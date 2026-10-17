//! The Settings window: an ordinary resizable top-level window whose whole client area is painted
//! by us (no native controls). The system draws only the title bar and the frame, so moving,
//! resizing, minimising and closing behave as people expect. There is at most one; asking for it
//! again brings the open one to the front.
//!
//! The window reads the shared [`Store`] and repaints when its revision changed, so it always
//! shows what the live bar shows. Where it was left is kept in `[settings_window]`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, DIB_RGB_COLORS, EndPaint, InvalidateRect,
    PAINTSTRUCT, SetDIBitsToDevice,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, GetWindowRect, IDC_ARROW, IsIconic, IsWindow,
    IsZoomed, KillTimer, LoadCursorW, MINMAXINFO, RegisterClassExW, SW_RESTORE, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetTimer, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_EXITSIZEMOVE,
    WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE, WM_TIMER,
    WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, w};

use super::text::render_mask;
use super::{monitors, wide, window};
use crate::canvas::Canvas;
use crate::config::{Rgba, WindowRect};
use crate::interaction::Rect;
use crate::settings::{self, Layout, Section};
use crate::store::Store;
use crate::strings;

const TIMER_ID: usize = 1;
/// How often the window checks whether the store changed; the file poll itself runs on the bar.
const REVISION_CHECK_MS: u32 = 500;
/// Tab label height as a share of the tab bar height.
const LABEL_SCALE: f32 = 0.42;
const LABEL_SELECTED: Rgba = Rgba::rgb(0xf2, 0xf2, 0xf2);

thread_local! {
    /// The open Settings window (an `HWND` as a number), or 0.
    static OPEN: Cell<isize> = const { Cell::new(0) };
}

struct State {
    store: Rc<RefCell<Store>>,
    canvas: Canvas,
    section: Section,
    /// The store revision that was last painted.
    revision: u64,
}

impl State {
    fn scale(hwnd: HWND) -> f32 {
        // SAFETY: plain query on our own window.
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
    }

    fn layout(&self, hwnd: HWND) -> Layout {
        settings::layout(
            self.canvas.width() as i32,
            self.canvas.height() as i32,
            Self::scale(hwnd),
        )
    }

    /// Resize the canvas to the client area.
    fn fit_canvas(&mut self, hwnd: HWND) {
        let (w, h) = client_size(hwnd);
        if (w, h) != (self.canvas.width(), self.canvas.height()) {
            self.canvas = Canvas::new(w.max(1), h.max(1));
        }
    }

    fn paint_canvas(&mut self, hwnd: HWND) {
        self.fit_canvas(hwnd);
        self.revision = self.store.borrow().revision();
        let layout = self.layout(hwnd);
        let accent = self.store.borrow().config().colors.green;
        settings::paint_frame(&mut self.canvas, &layout, self.section, accent);
        let font_px = ((layout.tab_bar.h as f32 * LABEL_SCALE) as i32).max(9);
        for (section, tab) in &layout.tabs {
            let (w, h) = (tab.w as u32, tab.h as u32);
            if let Some(mask) = render_mask(section.label(), w, h, font_px) {
                self.canvas
                    .blend_mask(&mask, w, h, tab.x, tab.y, LABEL_SELECTED);
            }
        }
    }

    fn click(&mut self, hwnd: HWND, x: i32, y: i32) {
        if let Some(section) = self.layout(hwnd).tab_at(x, y)
            && section != self.section
        {
            self.section = section;
            invalidate(hwnd);
        }
    }

    /// Remember where the window is. A window that is minimised or maximised has no rectangle
    /// worth keeping. A config that cannot be written just means the place is not kept.
    fn save_placement(&self, hwnd: HWND) {
        // SAFETY: plain queries on our own window.
        let (iconic, zoomed) = unsafe { (IsIconic(hwnd).as_bool(), IsZoomed(hwnd).as_bool()) };
        if iconic || zoomed {
            return;
        }
        let rect = window_rect(hwnd);
        let _ = self.store.borrow_mut().save_settings_window(WindowRect {
            x: Some(rect.x),
            y: Some(rect.y),
            width: u32::try_from(rect.w).ok(),
            height: u32::try_from(rect.h).ok(),
        });
    }
}

fn window_rect(hwnd: HWND) -> Rect {
    let mut r = RECT::default();
    // SAFETY: plain query on our own window.
    unsafe {
        let _ = GetWindowRect(hwnd, &mut r);
    }
    Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}

fn client_size(hwnd: HWND) -> (u32, u32) {
    let mut r = RECT::default();
    // SAFETY: plain query on our own window.
    unsafe {
        let _ = GetClientRect(hwnd, &mut r);
    }
    (
        (r.right - r.left).max(0) as u32,
        (r.bottom - r.top).max(0) as u32,
    )
}

fn invalidate(hwnd: HWND) {
    // SAFETY: our own window; a null rectangle means all of it.
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

/// Open the Settings window on `store`, or bring the open one to the front. Returns its handle.
pub fn open(store: Rc<RefCell<Store>>) -> windows::core::Result<HWND> {
    let existing = HWND(OPEN.get() as *mut _);
    // SAFETY: plain calls on a handle that is checked first.
    unsafe {
        if !existing.0.is_null() && IsWindow(Some(existing)).as_bool() {
            if IsIconic(existing).as_bool() {
                let _ = ShowWindow(existing, SW_RESTORE);
            }
            let _ = SetForegroundWindow(existing);
            return Ok(existing);
        }
    }

    let scale = window::dpi_scale();
    let rect = settings::placement(
        &store.borrow().config().settings_window,
        scale,
        &monitors::work_areas(),
    );
    let revision = store.borrow().revision();
    let state = Box::new(State {
        store,
        canvas: Canvas::new(1, 1),
        section: Section::Timetable,
        revision,
    });
    let raw = Box::into_raw(state);
    // SAFETY: standard window creation. The state pointer travels in the creation parameters and
    // is owned by the window from WM_NCCREATE on; WM_NCDESTROY frees it, also when creation fails
    // after that. (A failure before WM_NCCREATE leaks the few bytes of state.)
    unsafe {
        let module = GetModuleHandleW(None)?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: w!("TimesUpSettings"),
            ..Default::default()
        };
        // Registering twice (a second window after the first was closed) fails harmlessly.
        RegisterClassExW(&class);

        let title = wide(strings::SETTINGS_TITLE);
        let created = CreateWindowExW(
            Default::default(),
            w!("TimesUpSettings"),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            None,
            None,
            Some(instance),
            Some(raw.cast()),
        );
        let hwnd = created?;
        OPEN.set(hwnd.0 as isize);
        SetTimer(Some(hwnd), TIMER_ID, REVISION_CHECK_MS, None);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        Ok(hwnd)
    }
}

/// The open Settings window, if any.
pub fn current() -> Option<HWND> {
    let hwnd = HWND(OPEN.get() as *mut _);
    // SAFETY: plain query on a handle that may be stale.
    (!hwnd.0.is_null() && unsafe { IsWindow(Some(hwnd)) }.as_bool()).then_some(hwnd)
}

/// Close the window the way its close button does.
pub fn close(hwnd: HWND) {
    // SAFETY: on the window's own thread; WM_CLOSE saves the placement and destroys it.
    unsafe {
        SendMessageW(hwnd, WM_CLOSE, None, None);
    }
}

/// # Safety
/// Only valid on the window's own thread, between WM_NCCREATE and WM_NCDESTROY.
unsafe fn state_of<'a>(hwnd: HWND) -> Option<&'a mut State> {
    // SAFETY: GWLP_USERDATA is either null or the `State` stored in WM_NCCREATE.
    unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State).as_mut() }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: see `state_of`; every borrow of the state ends before the next Win32 call that may
    // dispatch a message to this window.
    unsafe {
        match msg {
            WM_NCCREATE => {
                let create = &*(lparam.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            }
            WM_ERASEBKGND => return LRESULT(1),
            WM_PAINT => {
                if let Some(state) = state_of(hwnd) {
                    state.paint_canvas(hwnd);
                    let mut ps = PAINTSTRUCT::default();
                    let dc = BeginPaint(hwnd, &mut ps);
                    let (w, h) = (state.canvas.width() as i32, state.canvas.height() as i32);
                    let info = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: w,
                            biHeight: -h, // top-down
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    SetDIBitsToDevice(
                        dc,
                        0,
                        0,
                        w as u32,
                        h as u32,
                        0,
                        0,
                        0,
                        h as u32,
                        state.canvas.pixels().as_ptr().cast(),
                        &info,
                        DIB_RGB_COLORS,
                    );
                    let _ = EndPaint(hwnd, &ps);
                    return LRESULT(0);
                }
            }
            WM_SIZE => invalidate(hwnd),
            WM_TIMER if wparam.0 == TIMER_ID => {
                if let Some(state) = state_of(hwnd)
                    && state.store.borrow().revision() != state.revision
                {
                    invalidate(hwnd);
                }
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                if let Some(state) = state_of(hwnd) {
                    let x = (lparam.0 & 0xffff) as i16 as i32;
                    let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
                    state.click(hwnd, x, y);
                }
                return LRESULT(0);
            }
            WM_GETMINMAXINFO => {
                // The minimum is for the whole window, like the saved rectangle.
                let min = settings::min_size(State::scale(hwnd));
                let info = &mut *(lparam.0 as *mut MINMAXINFO);
                info.ptMinTrackSize.x = min.0;
                info.ptMinTrackSize.y = min.1;
                return LRESULT(0);
            }
            // Moved to a monitor with another scale: take the size Windows suggests.
            WM_DPICHANGED => {
                let r = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                invalidate(hwnd);
                return LRESULT(0);
            }
            WM_EXITSIZEMOVE => {
                if let Some(state) = state_of(hwnd) {
                    state.save_placement(hwnd);
                }
                return LRESULT(0);
            }
            WM_CLOSE => {
                if let Some(state) = state_of(hwnd) {
                    state.save_placement(hwnd);
                }
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            WM_DESTROY => {
                let _ = KillTimer(Some(hwnd), TIMER_ID);
                if OPEN.get() == hwnd.0 as isize {
                    OPEN.set(0);
                }
                return LRESULT(0);
            }
            WM_NCDESTROY => {
                let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                if !state.is_null() {
                    drop(Box::from_raw(state));
                }
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use windows::Win32::Graphics::Gdi::UpdateWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GWL_STYLE, GetWindowLongW, GetWindowTextW, IsWindowVisible, WS_CAPTION,
        WS_THICKFRAME,
    };
    use windows::core::BOOL;

    use super::*;
    use crate::config::{self, ConfigFile};
    use crate::schedule::{Class, DEFAULT_BREAK_LEN, Weekday, hm};

    /// A store on a scratch config file, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str, text: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("times-up-settings-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(config::FILE_NAME), text).unwrap();
            Self(dir)
        }

        fn store(&self) -> Rc<RefCell<Store>> {
            let mut file = ConfigFile::new(self.0.join(config::FILE_NAME));
            let config = file.load_or_create().config;
            Rc::new(RefCell::new(Store::new(config, Some(file))))
        }

        fn text(&self) -> String {
            fs::read_to_string(self.0.join(config::FILE_NAME)).unwrap()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn title(hwnd: HWND) -> String {
        let mut buf = [0u16; 128];
        // SAFETY: reading the title of our own window.
        let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
        String::from_utf16_lossy(&buf[..len.max(0) as usize])
    }

    fn child_count(hwnd: HWND) -> i32 {
        unsafe extern "system" fn count(_: HWND, data: LPARAM) -> BOOL {
            // SAFETY: `data` is the counter passed below, valid for the whole call.
            unsafe { *(data.0 as *mut i32) += 1 };
            BOOL(1)
        }
        let mut n = 0;
        // SAFETY: the callback only runs during this call.
        unsafe {
            let _ = EnumChildWindows(Some(hwnd), Some(count), LPARAM(&mut n as *mut i32 as isize));
        }
        n
    }

    #[test]
    fn it_is_a_resizable_titled_window_without_native_controls() {
        let dir = Scratch::new("shape", "");
        let hwnd = open(dir.store()).unwrap();
        // SAFETY: plain queries on our own window.
        let (style, visible) = unsafe {
            (
                GetWindowLongW(hwnd, GWL_STYLE) as u32,
                IsWindowVisible(hwnd).as_bool(),
            )
        };
        assert!(visible);
        assert_ne!(style & WS_THICKFRAME.0, 0, "resizable");
        assert_ne!(style & WS_CAPTION.0, 0, "has a title bar");
        assert_eq!(title(hwnd), strings::SETTINGS_TITLE);
        assert_eq!(child_count(hwnd), 0, "everything is drawn by us");
        close(hwnd);
    }

    #[test]
    fn asking_again_brings_the_same_window_forward() {
        let dir = Scratch::new("single", "");
        let store = dir.store();
        let first = open(store.clone()).unwrap();
        let second = open(store.clone()).unwrap();
        assert_eq!(first, second);
        assert_eq!(current(), Some(first));
        close(first);
        assert_eq!(current(), None, "closing forgets it");
        let third = open(store).unwrap();
        assert_eq!(current(), Some(third));
        close(third);
    }

    #[test]
    fn it_opens_where_it_was_left_and_remembers_a_move_and_resize() {
        let dir = Scratch::new("placement", "");
        let store = dir.store();
        let area = monitors::work_areas()[0];
        let first = open(store.clone()).unwrap();
        let rect = window_rect(first);
        let default = settings::placement(&WindowRect::default(), window::dpi_scale(), &[area]);
        assert_eq!((rect.w, rect.h), (default.w, default.h), "default size");

        // SAFETY: moving our own window.
        unsafe {
            let _ = SetWindowPos(
                first,
                None,
                area.x + 30,
                area.y + 40,
                700,
                450,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            SendMessageW(first, WM_EXITSIZEMOVE, None, None);
        }
        let text = dir.text();
        assert!(text.contains("[settings_window]"), "{text}");
        assert_eq!(
            config::parse(&text).config.settings_window,
            WindowRect {
                x: Some(area.x + 30),
                y: Some(area.y + 40),
                width: Some(700),
                height: Some(450),
            }
        );
        close(first);

        let again = open(store).unwrap();
        assert_eq!(
            window_rect(again),
            Rect::new(area.x + 30, area.y + 40, 700, 450)
        );
        close(again);
    }

    #[test]
    fn a_saved_place_that_is_off_screen_is_pulled_back() {
        let dir = Scratch::new("offscreen", "[settings_window]\nx = 90000\ny = 90000\n");
        let areas = monitors::work_areas();
        let hwnd = open(dir.store()).unwrap();
        let rect = window_rect(hwnd);
        assert!(
            areas
                .iter()
                .any(|a| rect.overlap_area(a) * 2 >= rect.area()),
            "{rect:?} is mostly on a monitor"
        );
        close(hwnd);
    }

    #[test]
    fn it_never_gets_smaller_than_its_minimum() {
        let dir = Scratch::new("minimum", "[settings_window]\nwidth = 10\nheight = 10\n");
        let hwnd = open(dir.store()).unwrap();
        let rect = window_rect(hwnd);
        let min = settings::min_size(window::dpi_scale());
        assert!(rect.w >= min.0 && rect.h >= min.1, "{rect:?}");
        // The user cannot drag it below the minimum either.
        let mut info = MINMAXINFO::default();
        // SAFETY: asking our own window for its limits.
        unsafe {
            SendMessageW(
                hwnd,
                WM_GETMINMAXINFO,
                None,
                Some(LPARAM(&mut info as *mut MINMAXINFO as isize)),
            );
        }
        assert_eq!((info.ptMinTrackSize.x, info.ptMinTrackSize.y), min);
        close(hwnd);
    }

    #[test]
    fn the_client_area_is_painted_by_us_and_follows_the_store() {
        let dir = Scratch::new("paint", "[monday]\nclasses = []\n");
        let store = dir.store();
        let hwnd = open(store.clone()).unwrap();
        // SAFETY: on the window's thread; the borrow of the state ends with each statement.
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
            let _ = UpdateWindow(hwnd);
        }
        let painted = |hwnd| {
            // SAFETY: as above.
            let state = unsafe { state_of(hwnd) }.unwrap();
            (state.canvas.width(), state.canvas.height(), state.revision)
        };
        let (w, h, revision) = painted(hwnd);
        assert_eq!((w, h), client_size(hwnd));
        assert!(w > 100 && h > 100);
        assert_eq!(revision, 0);
        // SAFETY: as above.
        let state = unsafe { state_of(hwnd) }.unwrap();
        assert!(state.canvas.pixels().iter().all(|p| p >> 24 == 0xff));

        // An edit bumps the store's revision; the next tick of the window repaints.
        let class = Class::new(hm(9, 0), hm(9, 50), DEFAULT_BREAK_LEN).unwrap();
        store
            .borrow_mut()
            .set_day(Weekday::Mon, vec![class])
            .unwrap();
        // SAFETY: as above.
        unsafe {
            SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER_ID)), None);
            let _ = UpdateWindow(hwnd);
        }
        assert_eq!(painted(hwnd).2, 1);
        close(hwnd);
    }

    #[test]
    fn clicking_the_only_tab_keeps_the_timetable_section() {
        let dir = Scratch::new("tab", "");
        let hwnd = open(dir.store()).unwrap();
        let lparam = LPARAM((10 << 16) | 10);
        // SAFETY: on the window's thread.
        unsafe {
            SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(lparam));
            assert_eq!(state_of(hwnd).unwrap().section, Section::Timetable);
        }
        close(hwnd);
    }
}
