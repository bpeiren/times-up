//! The Settings window: an ordinary resizable top-level window whose whole client area is painted
//! by us (no native controls). The system draws only the title bar and the frame, so moving,
//! resizing, minimising and closing behave as people expect. There is at most one; asking for it
//! again brings the open one to the front.
//!
//! The window reads the shared [`Store`] and repaints when its revision changed, so it always
//! shows what the live bar shows. Where it was left is kept in `[settings_window]`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, DIB_RGB_COLORS, EndPaint, InvalidateRect,
    PAINTSTRUCT, ScreenToClient, SetDIBitsToDevice,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, VK_DELETE, VK_ESCAPE, VK_RETURN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, GetWindowRect, IDC_ARROW, IsIconic, IsWindow,
    IsZoomed, KillTimer, LoadCursorW, MINMAXINFO, RegisterClassExW, SW_RESTORE, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetTimer, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_EXITSIZEMOVE,
    WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SIZE, WM_TIMER,
    WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, HTCLIENT, IDC_SIZEALL, IDC_SIZEWE, SetCursor, WM_CAPTURECHANGED, WM_KEYDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_SETCURSOR,
};
use windows::core::{PCWSTR, w};

use super::text::{render_mask, render_mask_aligned};
use super::{monitors, wide, window};
use crate::banner::{self, Answer};
use crate::canvas::{Align, Canvas};
use crate::config::{Rgba, WindowRect};
use crate::detail::{self, Detail};
use crate::interaction::{CursorShape, Rect};
use crate::schedule::{Secs, Weekday};
use crate::settings::{self, Layout, Section};
use crate::store::Store;
use crate::strings;
use crate::timetable::{self, Timetable};

const TIMER_ID: usize = 1;
/// How often the window checks whether the store changed; the file poll itself runs on the bar.
const REVISION_CHECK_MS: u32 = 500;
/// Tab label height as a share of the tab bar height.
const LABEL_SCALE: f32 = 0.42;
/// Text height as a share of the height of the area it is centred in.
const TEXT_SCALE: f32 = 0.5;
const LABEL_SELECTED: Rgba = Rgba::rgb(0xf2, 0xf2, 0xf2);

/// A class by its day, start and end.
type ClassKey = (Weekday, Secs, Secs);
/// The local day and time.
type Clock = fn() -> (Weekday, Secs);

thread_local! {
    /// The open Settings window (an `HWND` as a number), or 0.
    static OPEN: Cell<isize> = const { Cell::new(0) };
}

struct State {
    store: Rc<RefCell<Store>>,
    canvas: Canvas,
    section: Section,
    timetable: Timetable,
    detail: Detail,
    /// The running class (day, start, end) whose temporary changes are being asked about.
    banner: Option<ClassKey>,
    /// The answer given for the running class, until that class ends.
    answered: Option<(ClassKey, Answer)>,
    clock: Clock,
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
        let (w, h) = client_size(hwnd);
        settings::layout(w as i32, h as i32, Self::scale(hwnd))
    }

    /// Where the week overview sits for what the store and the drag in progress hold now.
    fn week_layout(&self, hwnd: HWND) -> timetable::Layout {
        let (days, axis) = self.timetable.days_and_axis(&self.store.borrow());
        timetable::layout(self.areas(hwnd).0, Self::scale(hwnd), &days, axis)
    }

    /// The area of the week and the area of the detail bar below it (under the banner, if any).
    fn areas(&self, hwnd: HWND) -> (Rect, Rect) {
        let content = self.layout(hwnd).content;
        let scale = Self::scale(hwnd);
        let rest = if self.banner.is_some() {
            banner::split(content, scale).1
        } else {
            content
        };
        detail::split(rest, scale)
    }

    fn banner_layout(&self, hwnd: HWND) -> Option<banner::Layout> {
        self.banner?;
        let scale = Self::scale(hwnd);
        Some(banner::layout(
            banner::split(self.layout(hwnd).content, scale).0,
            scale,
        ))
    }

    /// An edit saved in Settings: when it changed the running class that has temporary changes
    /// on the bar, apply the answer given before, or ask.
    fn after_edit(&mut self) {
        let touched_week = self.timetable.take_live_touched();
        let touched_detail = self.detail.take_live_touched();
        if !(touched_week || touched_detail) {
            return;
        }
        let key = self
            .store
            .borrow()
            .live()
            .map(|l| (l.day, l.class.start, l.class.end));
        let Some(key) = key else {
            return;
        };
        match self.answered {
            Some((answered, answer)) if answered == key => self.apply(answer),
            _ => self.banner = Some(key),
        }
    }

    fn apply(&mut self, answer: Answer) {
        if answer == Answer::Replace {
            self.store.borrow_mut().clear_live();
        }
    }

    /// The teacher chose; the answer holds for this class until it ends.
    fn answer(&mut self, hwnd: HWND, answer: Answer) {
        if let Some(key) = self.banner.take() {
            self.answered = Some((key, answer));
            self.apply(answer);
            invalidate(hwnd);
        }
    }

    /// Forget an answer and a question whose class is over. Returns whether the banner went.
    fn forget_ended(&mut self) -> bool {
        let (day, now) = (self.clock)();
        let store = self.store.borrow();
        let schedule = store.schedule();
        let running = |key: ClassKey| {
            key.0 == day
                && schedule
                    .current_class(day, now)
                    .and_then(|i| schedule.classes(day).get(i))
                    .is_some_and(|c| (c.start, c.end) == (key.1, key.2))
        };
        if self.answered.is_some_and(|(key, _)| !running(key)) {
            self.answered = None;
        }
        let gone =
            self.banner.is_some() && (store.live().is_none() || !running(self.banner.unwrap()));
        if gone {
            self.banner = None;
        }
        gone
    }

    fn detail_layout(&self, hwnd: HWND) -> detail::Layout {
        detail::layout(self.areas(hwnd).1, Self::scale(hwnd))
    }

    /// End the Add break mode when the class it was started for is no longer the selected one.
    fn sync_detail(&mut self) {
        let store = self.store.borrow();
        let selection = self.timetable.selected(&store);
        self.detail.sync(&store, selection);
    }

    fn is_dragging(&self) -> bool {
        self.timetable.is_dragging() || self.detail.is_dragging()
    }

    /// Resize the canvas to the client area.
    fn fit_canvas(&mut self, hwnd: HWND) {
        let (w, h) = client_size(hwnd);
        if (w, h) != (self.canvas.width(), self.canvas.height()) {
            self.canvas = Canvas::new(w.max(1), h.max(1));
        }
    }

    fn paint_canvas(&mut self, hwnd: HWND) {
        self.sync_detail();
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
        match self.section {
            Section::Timetable => {
                let (week, below) = (self.week_layout(hwnd), self.detail_layout(hwnd));
                let banner = self.banner_layout(hwnd);
                let store = self.store.borrow();
                let canvas = &mut self.canvas;
                let mut text = |canvas: &mut Canvas, text: &str, area, color, align| {
                    draw_text(canvas, text, area, color, align);
                };
                self.timetable.paint(canvas, &store, &week, &mut text);
                let selection = self.timetable.selected(&store);
                self.detail
                    .paint(canvas, &store, selection, &below, &mut text);
                if let Some(layout) = banner {
                    banner::paint(canvas, &layout, &mut text);
                }
            }
        }
    }

    fn press(&mut self, hwnd: HWND, x: i32, y: i32) {
        if let Some(section) = self.layout(hwnd).tab_at(x, y) {
            if section != self.section {
                self.section = section;
                invalidate(hwnd);
            }
            return;
        }
        if let Some(layout) = self.banner_layout(hwnd)
            && contains(layout.area, x, y)
        {
            if let Some(answer) = layout.answer_at(x, y) {
                self.answer(hwnd, answer);
            }
            return;
        }
        match self.section {
            Section::Timetable if contains(self.areas(hwnd).1, x, y) => {
                let below = self.detail_layout(hwnd);
                let mut store = self.store.borrow_mut();
                let selection = self.timetable.selected(&store);
                self.detail.press(&mut store, selection, &below, x, y);
            }
            Section::Timetable => {
                let week = self.week_layout(hwnd);
                self.timetable
                    .press(&mut self.store.borrow_mut(), &week, x, y);
            }
        }
        self.sync_detail();
        self.after_edit();
        invalidate(hwnd);
    }

    fn drag(&mut self, hwnd: HWND, x: i32, y: i32) {
        let below = self.detail_layout(hwnd);
        let mut repaint = self.detail.moved(&below, x, y);
        if self.timetable.is_dragging() {
            let week = self.week_layout(hwnd);
            self.timetable.drag_to(&self.store.borrow(), &week, x, y);
            repaint = true;
        }
        if repaint {
            invalidate(hwnd);
        }
    }

    fn release(&mut self, hwnd: HWND) {
        if self.is_dragging() {
            self.timetable.release(&mut self.store.borrow_mut());
            self.detail.release(&mut self.store.borrow_mut());
            self.after_edit();
            invalidate(hwnd);
        }
    }

    /// The mouse capture is gone: whatever was being dragged is given up.
    fn cancel_drags(&mut self, hwnd: HWND) {
        self.timetable.cancel();
        self.detail.cancel();
        invalidate(hwnd);
    }

    /// Escape answers the question with Save only; otherwise it leaves the Add break mode (or
    /// gives up a drag of the detail bar).
    fn escape(&mut self, hwnd: HWND) {
        if self.banner.is_some() {
            self.answer(hwnd, Answer::SaveOnly);
        } else if self.detail.escape() {
            invalidate(hwnd);
        }
    }

    fn delete_key(&mut self, hwnd: HWND) {
        self.timetable.remove_selected(&mut self.store.borrow_mut());
        self.after_edit();
        invalidate(hwnd);
    }

    fn cursor(&self, hwnd: HWND, x: i32, y: i32) -> CursorShape {
        if y < self.layout(hwnd).content.y {
            return CursorShape::Arrow;
        }
        let store = self.store.borrow();
        if contains(self.areas(hwnd).1, x, y) || self.detail.is_dragging() {
            let below = self.detail_layout(hwnd);
            let selection = self.timetable.selected(&store);
            return self.detail.cursor(&store, selection, &below, x, y);
        }
        let week = self.week_layout(hwnd);
        self.timetable.cursor(&store, &week, x, y)
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

/// `text` centred in `area`, in a font that fits the height of `area`.
fn draw_text(canvas: &mut Canvas, text: &str, area: Rect, color: Rgba, align: Align) {
    if area.w <= 0 || area.h <= 0 {
        return;
    }
    let (w, h) = (area.w as u32, area.h as u32);
    // Not wider than the area: a long word gets a smaller font instead of being cut off.
    let by_width = (area.w as f32 / (text.chars().count().max(1) as f32 * 0.62)) as i32;
    let font_px = ((area.h as f32 * TEXT_SCALE) as i32)
        .clamp(9, 22)
        .min(by_width.max(9));
    if let Some(mask) = render_mask_aligned(text, w, h, font_px, align) {
        canvas.blend_mask(&mask, w, h, area.x, area.y, color);
    }
}

fn contains(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.right() && y >= r.y && y < r.bottom()
}

fn cursor_id(shape: CursorShape) -> PCWSTR {
    match shape {
        CursorShape::SizeAll => IDC_SIZEALL,
        CursorShape::SizeWestEast => IDC_SIZEWE,
        _ => IDC_ARROW,
    }
}

fn client_xy(lparam: LPARAM) -> (i32, i32) {
    (
        i32::from((lparam.0 & 0xffff) as u16 as i16),
        i32::from(((lparam.0 >> 16) & 0xffff) as u16 as i16),
    )
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
    open_with_clock(store, super::app::clock)
}

/// Like [`open`], with the clock that tells which class is running.
pub fn open_with_clock(store: Rc<RefCell<Store>>, clock: Clock) -> windows::core::Result<HWND> {
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
        timetable: Timetable::default(),
        detail: Detail::default(),
        banner: None,
        answered: None,
        clock,
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
                    && (state.forget_ended() || state.store.borrow().revision() != state.revision)
                {
                    invalidate(hwnd);
                }
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                let (x, y) = client_xy(lparam);
                let dragging = state_of(hwnd).is_some_and(|state| {
                    state.press(hwnd, x, y);
                    state.is_dragging()
                });
                if dragging {
                    let _ = SetCapture(hwnd);
                }
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                if let Some(state) = state_of(hwnd) {
                    let (x, y) = client_xy(lparam);
                    state.drag(hwnd, x, y);
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                if let Some(state) = state_of(hwnd) {
                    state.release(hwnd);
                }
                let _ = ReleaseCapture();
                return LRESULT(0);
            }
            // The capture was taken away in the middle of a drag: give the drag up.
            WM_CAPTURECHANGED => {
                if let Some(state) = state_of(hwnd) {
                    state.cancel_drags(hwnd);
                }
                return LRESULT(0);
            }
            WM_KEYDOWN if wparam.0 == usize::from(VK_RETURN.0) => {
                if let Some(state) = state_of(hwnd) {
                    state.answer(hwnd, Answer::Replace);
                }
                return LRESULT(0);
            }
            WM_KEYDOWN if wparam.0 == usize::from(VK_ESCAPE.0) => {
                if let Some(state) = state_of(hwnd) {
                    state.escape(hwnd);
                }
                return LRESULT(0);
            }
            WM_KEYDOWN if wparam.0 == usize::from(VK_DELETE.0) => {
                if let Some(state) = state_of(hwnd) {
                    state.delete_key(hwnd);
                }
                return LRESULT(0);
            }
            WM_SETCURSOR if (lparam.0 & 0xffff) as u32 == HTCLIENT => {
                if let Some(state) = state_of(hwnd) {
                    let mut p = POINT::default();
                    let _ = GetCursorPos(&mut p);
                    let _ = ScreenToClient(hwnd, &mut p);
                    let shape = state.cursor(hwnd, p.x, p.y);
                    if let Ok(cursor) = LoadCursorW(None, cursor_id(shape)) {
                        SetCursor(Some(cursor));
                    }
                    return LRESULT(1);
                }
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
                520,
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
                height: Some(520),
            }
        );
        close(first);

        let again = open(store).unwrap();
        assert_eq!(
            window_rect(again),
            Rect::new(area.x + 30, area.y + 40, 700, 520)
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

    /// Send a mouse message at client position `(x, y)`.
    fn mouse(hwnd: HWND, msg: u32, x: i32, y: i32) {
        let lparam = LPARAM(((y & 0xffff) << 16 | (x & 0xffff)) as isize);
        // SAFETY: on the window's thread.
        unsafe {
            SendMessageW(hwnd, msg, None, Some(lparam));
        }
    }

    const MONDAY: &str = "[monday]
classes = [{ start = \"09:00\", end = \"09:50\" }]
";

    /// The middle of Monday's first class and the plus button of Monday.
    fn class_and_plus(hwnd: HWND) -> ((i32, i32), (i32, i32)) {
        // SAFETY: on the window's thread; the borrow ends with this function.
        let state = unsafe { state_of(hwnd) }.unwrap();
        let week = state.week_layout(hwnd);
        let class = state.store.borrow().schedule().classes(Weekday::Mon)[0].clone();
        let r = week.class_rect(0, &class);
        let p = week.plus[0];
        ((r.x + r.w / 2, r.y + r.h / 2), (p.x + 3, p.y + 3))
    }

    #[test]
    fn dragging_a_class_with_the_mouse_saves_on_release_and_the_bar_sees_it() {
        use windows::Win32::UI::WindowsAndMessaging::{WM_LBUTTONUP, WM_MOUSEMOVE};
        let dir = Scratch::new("mouse-drag", MONDAY);
        let store = dir.store();
        let hwnd = open(store.clone()).unwrap();
        let ((x, y), _) = class_and_plus(hwnd);

        mouse(hwnd, WM_LBUTTONDOWN, x, y);
        mouse(hwnd, WM_MOUSEMOVE, x + 90, y);
        assert_eq!(store.borrow().revision(), 0, "nothing is saved mid-drag");
        assert!(dir.text().contains("09:00"));
        mouse(hwnd, WM_LBUTTONUP, x + 90, y);

        assert_eq!(store.borrow().revision(), 1);
        let moved = store.borrow().schedule().classes(Weekday::Mon)[0].clone();
        assert!(
            moved.start > hm(9, 0) && moved.start.is_multiple_of(300),
            "{}",
            moved.start
        );
        assert_eq!(moved.duration(), 50 * 60);
        assert!(
            dir.text()
                .contains(&crate::schedule::format_clock(moved.start))
        );
        close(hwnd);
    }

    #[test]
    fn the_plus_button_and_the_delete_key_add_and_remove_classes() {
        use windows::Win32::UI::WindowsAndMessaging::{WM_KEYDOWN, WM_LBUTTONUP};
        let dir = Scratch::new("mouse-add", MONDAY);
        let store = dir.store();
        let hwnd = open(store.clone()).unwrap();
        let (_, (px, py)) = class_and_plus(hwnd);

        mouse(hwnd, WM_LBUTTONDOWN, px, py);
        mouse(hwnd, WM_LBUTTONUP, px, py);
        assert_eq!(store.borrow().schedule().classes(Weekday::Mon).len(), 2);
        assert!(dir.text().contains("10:50"));

        // The new class is selected: Delete takes it away again.
        // SAFETY: on the window's thread.
        unsafe {
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x2e)), None);
        }
        assert_eq!(store.borrow().schedule().classes(Weekday::Mon).len(), 1);
        assert_eq!(store.borrow().revision(), 2);
        close(hwnd);
    }

    #[test]
    fn losing_the_mouse_capture_gives_a_drag_up_without_saving() {
        use windows::Win32::UI::WindowsAndMessaging::{WM_CAPTURECHANGED, WM_MOUSEMOVE};
        let dir = Scratch::new("mouse-cancel", MONDAY);
        let store = dir.store();
        let hwnd = open(store.clone()).unwrap();
        let ((x, y), _) = class_and_plus(hwnd);
        mouse(hwnd, WM_LBUTTONDOWN, x, y);
        mouse(hwnd, WM_MOUSEMOVE, x + 90, y);
        // SAFETY: on the window's thread.
        unsafe {
            SendMessageW(hwnd, WM_CAPTURECHANGED, None, None);
            assert!(!state_of(hwnd).unwrap().timetable.is_dragging());
        }
        assert_eq!(store.borrow().revision(), 0);
        close(hwnd);
    }

    /// Select Monday's class with a click, and return the detail layout.
    fn select_monday(hwnd: HWND) -> detail::Layout {
        use windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP;
        let ((x, y), _) = class_and_plus(hwnd);
        mouse(hwnd, WM_LBUTTONDOWN, x, y);
        mouse(hwnd, WM_LBUTTONUP, x, y);
        // SAFETY: on the window's thread.
        unsafe { state_of(hwnd) }.unwrap().detail_layout(hwnd)
    }

    #[test]
    fn add_break_works_through_the_window_and_escape_leaves_the_mode() {
        use windows::Win32::UI::WindowsAndMessaging::{WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE};
        let dir = Scratch::new("detail-add", MONDAY);
        let store = dir.store();
        let hwnd = open(store.clone()).unwrap();
        let l = select_monday(hwnd);

        // Escape leaves the mode...
        mouse(hwnd, WM_LBUTTONDOWN, l.add.x + 3, l.add.y + 3);
        mouse(hwnd, WM_LBUTTONUP, l.add.x + 3, l.add.y + 3);
        // SAFETY: on the window's thread.
        unsafe {
            assert!(state_of(hwnd).unwrap().detail.is_adding());
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1b)), None);
            assert!(!state_of(hwnd).unwrap().detail.is_adding());
        }

        // ...and a click on the bar places the break and saves it.
        mouse(hwnd, WM_LBUTTONDOWN, l.add.x + 3, l.add.y + 3);
        mouse(hwnd, WM_LBUTTONUP, l.add.x + 3, l.add.y + 3);
        let x = l.bar.x + l.bar.w * 2 / 5;
        let y = l.bar.y + l.bar.h / 2;
        mouse(hwnd, WM_MOUSEMOVE, x, y);
        mouse(hwnd, WM_LBUTTONDOWN, x, y);
        mouse(hwnd, WM_LBUTTONUP, x, y);
        let class = store.borrow().schedule().classes(Weekday::Mon)[0].clone();
        assert!(class.brk.is_some());
        assert!(dir.text().contains("break_len"), "{}", dir.text());
        close(hwnd);
    }

    #[test]
    fn dragging_a_break_edge_through_the_window_saves_on_release() {
        use windows::Win32::UI::WindowsAndMessaging::{WM_LBUTTONUP, WM_MOUSEMOVE};
        let long = "[monday]
classes = [{ start = \"09:00\", end = \"10:40\" }]
";
        let dir = Scratch::new("detail-drag", long);
        let store = dir.store();
        let hwnd = open(store.clone()).unwrap();
        let l = select_monday(hwnd);
        let before = store.borrow().schedule().classes(Weekday::Mon)[0]
            .brk
            .unwrap();
        let duration = 100 * 60;
        let at = |secs: u32| {
            l.bar.x + (i64::from(secs) * i64::from(l.bar.w) / i64::from(duration)) as i32
        };
        let y = l.bar.y + l.bar.h / 2;

        mouse(hwnd, WM_LBUTTONDOWN, at(before.start), y);
        mouse(hwnd, WM_MOUSEMOVE, at(before.start - 5 * 60), y);
        assert_eq!(store.borrow().revision(), 0, "not saved while dragging");
        mouse(hwnd, WM_LBUTTONUP, at(before.start - 5 * 60), y);

        let after = store.borrow().schedule().classes(Weekday::Mon)[0]
            .brk
            .unwrap();
        assert!(after.start < before.start, "{before:?} -> {after:?}");
        assert_eq!(after.end(), before.end());
        assert_eq!(store.borrow().revision(), 1);
        close(hwnd);
    }

    #[test]
    fn the_longest_day_name_fits_its_column() {
        let mut canvas = Canvas::new(160, 70);
        let area = Rect::new(20, 0, 120, 70);
        draw_text(
            &mut canvas,
            "Wednesday",
            area,
            Rgba::rgb(255, 255, 255),
            Align::Center,
        );
        let ink = |x: u32| (0..70).any(|y| canvas.pixel(x, y) != 0);
        assert!((20..140).any(ink), "it is drawn");
        assert!(
            !ink(20) && !ink(139),
            "and not cut off at the edges of its area"
        );
    }

    #[test]
    fn leaving_the_class_ends_the_add_break_mode_for_good() {
        use windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP;
        let dir = Scratch::new("mode-ends", MONDAY);
        let store = dir.store();
        let hwnd = open(store.clone()).unwrap();
        let l = select_monday(hwnd);
        mouse(hwnd, WM_LBUTTONDOWN, l.add.x + 3, l.add.y + 3);
        mouse(hwnd, WM_LBUTTONUP, l.add.x + 3, l.add.y + 3);
        // SAFETY: on the window's thread.
        unsafe { assert!(state_of(hwnd).unwrap().detail.is_adding()) };

        // Click on empty space in the week, then on the class again.
        mouse(hwnd, WM_LBUTTONDOWN, 5, l.area.y - 20);
        mouse(hwnd, WM_LBUTTONUP, 5, l.area.y - 20);
        let ((x, y), _) = class_and_plus(hwnd);
        mouse(hwnd, WM_LBUTTONDOWN, x, y);
        mouse(hwnd, WM_LBUTTONUP, x, y);
        // SAFETY: as above.
        unsafe {
            let state = state_of(hwnd).unwrap();
            assert!(state.timetable.selected(&store.borrow()).is_some());
            assert!(!state.detail.is_adding());
        }
        close(hwnd);
    }

    // ---- the running class with temporary changes ----

    thread_local! {
        static NOW: Cell<(Weekday, Secs)> = const { Cell::new((Weekday::Mon, 9 * 3600 + 10 * 60)) };
    }

    fn fake_clock() -> (Weekday, Secs) {
        NOW.get()
    }

    const LONG: &str = "[monday]\nclasses = [{ start = \"09:00\", end = \"10:40\" }]\n";

    /// A window on Monday 09:00-10:40 that is running, with a temporary change on the bar.
    fn running_with_live(name: &str) -> (Scratch, Rc<RefCell<Store>>, HWND) {
        NOW.set((Weekday::Mon, 9 * 3600 + 10 * 60));
        let dir = Scratch::new(name, LONG);
        let store = dir.store();
        let mut live = store.borrow().schedule().classes(Weekday::Mon)[0].clone();
        live.warnings[0] = crate::schedule::Warnings::new(8 * 60, 2 * 60).unwrap();
        store.borrow_mut().set_live(Weekday::Mon, live);
        let hwnd = open_with_clock(store.clone(), fake_clock).unwrap();
        (dir, store, hwnd)
    }

    fn click(hwnd: HWND, x: i32, y: i32) {
        use windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP;
        mouse(hwnd, WM_LBUTTONDOWN, x, y);
        mouse(hwnd, WM_LBUTTONUP, x, y);
    }

    fn key(hwnd: HWND, vk: usize) {
        use windows::Win32::UI::WindowsAndMessaging::WM_KEYDOWN;
        // SAFETY: on the window's thread.
        unsafe {
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(vk)), None);
        }
    }

    fn banner_shown(hwnd: HWND) -> bool {
        // SAFETY: on the window's thread.
        unsafe { state_of(hwnd) }.unwrap().banner.is_some()
    }

    fn remove_break(hwnd: HWND, l: &detail::Layout) {
        click(hwnd, l.remove.x + 3, l.remove.y + 3);
    }

    #[test]
    fn editing_the_running_class_with_temporary_changes_asks_what_to_do() {
        let (_dir, store, hwnd) = running_with_live("ask");
        let l = select_monday(hwnd);
        assert!(!banner_shown(hwnd));
        remove_break(hwnd, &l);
        assert!(
            banner_shown(hwnd),
            "the saved class changed under the live one"
        );
        assert!(store.borrow().live().is_some(), "until the teacher chooses");
        assert!(
            store.borrow().schedule().classes(Weekday::Mon)[0]
                .brk
                .is_none()
        );
        close(hwnd);
    }

    #[test]
    fn enter_replaces_the_temporary_changes_with_the_saved_class() {
        let (_dir, store, hwnd) = running_with_live("replace");
        let l = select_monday(hwnd);
        remove_break(hwnd, &l);
        key(hwnd, 0x0d);
        assert!(!banner_shown(hwnd));
        assert!(store.borrow().live().is_none());
        assert_eq!(
            store.borrow().live_schedule().classes(Weekday::Mon),
            store.borrow().schedule().classes(Weekday::Mon)
        );
        close(hwnd);
    }

    #[test]
    fn escape_saves_only_and_keeps_the_temporary_changes() {
        let (dir, store, hwnd) = running_with_live("save-only");
        let l = select_monday(hwnd);
        remove_break(hwnd, &l);
        key(hwnd, 0x1b);
        assert!(!banner_shown(hwnd));
        assert!(store.borrow().live().is_some());
        let shown = store.borrow().live_schedule().classes(Weekday::Mon)[0].clone();
        assert_eq!(
            shown.warnings[0].orange,
            8 * 60,
            "the bar keeps its version"
        );
        assert!(
            dir.text().contains("break_len = 0"),
            "while the file has the new one"
        );
        close(hwnd);
    }

    #[test]
    fn the_buttons_of_the_banner_answer_too() {
        let (_dir, store, hwnd) = running_with_live("buttons");
        let l = select_monday(hwnd);
        remove_break(hwnd, &l);
        // SAFETY: on the window's thread.
        let banner = unsafe { state_of(hwnd) }
            .unwrap()
            .banner_layout(hwnd)
            .unwrap();
        click(hwnd, banner.replace.x + 3, banner.replace.y + 3);
        assert!(!banner_shown(hwnd));
        assert!(store.borrow().live().is_none());
        close(hwnd);
    }

    #[test]
    fn the_answer_holds_for_the_running_class_so_it_is_asked_once() {
        use windows::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;
        let (_dir, store, hwnd) = running_with_live("once");
        let l = select_monday(hwnd);
        remove_break(hwnd, &l);
        key(hwnd, 0x1b);

        // Another edit of the same class: add a break again. No question this time.
        // SAFETY: on the window's thread.
        let l = unsafe { state_of(hwnd) }.unwrap().detail_layout(hwnd);
        click(hwnd, l.add.x + 3, l.add.y + 3);
        let (x, y) = (l.bar.x + l.bar.w / 2, l.bar.y + l.bar.h / 2);
        mouse(hwnd, WM_MOUSEMOVE, x, y);
        click(hwnd, x, y);
        assert!(
            store.borrow().schedule().classes(Weekday::Mon)[0]
                .brk
                .is_some()
        );
        assert!(!banner_shown(hwnd), "already answered for this class");
        assert!(store.borrow().live().is_some(), "Save only still holds");
        close(hwnd);
    }

    #[test]
    fn the_answer_is_forgotten_when_the_class_ends() {
        let (_dir, store, hwnd) = running_with_live("ends");
        let l = select_monday(hwnd);
        remove_break(hwnd, &l);
        key(hwnd, 0x1b);
        // SAFETY: on the window's thread.
        unsafe { assert!(state_of(hwnd).unwrap().answered.is_some()) };
        NOW.set((Weekday::Mon, 12 * 3600));
        // SAFETY: as above.
        unsafe {
            SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER_ID)), None);
            assert!(state_of(hwnd).unwrap().answered.is_none());
        }
        assert!(store.borrow().live().is_some(), "the bar clears it itself");
        close(hwnd);
    }

    #[test]
    fn nothing_is_asked_without_temporary_changes_or_for_another_class() {
        NOW.set((Weekday::Mon, 9 * 3600 + 10 * 60));
        let dir = Scratch::new("quiet", LONG);
        let store = dir.store();
        let hwnd = open_with_clock(store.clone(), fake_clock).unwrap();
        let l = select_monday(hwnd);
        remove_break(hwnd, &l);
        assert!(
            !banner_shown(hwnd),
            "no temporary changes: the bar follows the file"
        );
        close(hwnd);

        let (_dir, store, hwnd) = running_with_live("other");
        let (_, (px, py)) = class_and_plus(hwnd);
        click(hwnd, px, py); // adds a second class
        assert_eq!(store.borrow().schedule().classes(Weekday::Mon).len(), 2);
        assert!(
            !banner_shown(hwnd),
            "the running class itself was not edited"
        );
        assert!(store.borrow().live().is_some());
        close(hwnd);
    }

    #[test]
    fn closing_the_window_counts_as_save_only() {
        let (_dir, store, hwnd) = running_with_live("close");
        let l = select_monday(hwnd);
        remove_break(hwnd, &l);
        assert!(banner_shown(hwnd));
        close(hwnd);
        assert!(
            store.borrow().live().is_some(),
            "the temporary changes stay"
        );
        // Opening it again starts without a question.
        let again = open_with_clock(store.clone(), fake_clock).unwrap();
        assert!(!banner_shown(again));
        close(again);
    }

    #[test]
    fn moving_the_running_class_drops_its_temporary_changes_without_a_question() {
        use windows::Win32::UI::WindowsAndMessaging::{WM_LBUTTONUP, WM_MOUSEMOVE};
        let (_dir, store, hwnd) = running_with_live("moved");
        let ((x, y), _) = class_and_plus(hwnd);
        mouse(hwnd, WM_LBUTTONDOWN, x, y);
        mouse(hwnd, WM_MOUSEMOVE, x + 60, y);
        mouse(hwnd, WM_LBUTTONUP, x + 60, y);
        assert!(store.borrow().revision() > 0);
        assert!(!banner_shown(hwnd));
        assert_eq!(
            store.borrow().live_base(),
            None,
            "no longer the class they belonged to"
        );
        close(hwnd);
    }
}
