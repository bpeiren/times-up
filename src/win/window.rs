//! The borderless, transparent, always-on-top window.
//!
//! * `WS_POPUP` + `WS_EX_LAYERED`: no frame, per-pixel alpha.
//! * `WS_EX_TOOLWINDOW`: no taskbar button (and not in Alt+Tab).
//! * `WS_EX_NOACTIVATE`: clicking the bar never steals focus from the teacher's slides.
//! * `WS_EX_TOPMOST`, re-asserted every tick because other topmost windows can cover it.
//!
//! Moving and resizing are handled here with mouse capture instead of the system's caption and
//! frame, so the same hit test can later tell the break and zone handles from the body of the bar.
//! The geometry itself lives in [`crate::interaction`].

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, GetWindowRect, HTCLIENT, HWND_TOPMOST, IDC_ARROW, IDC_SIZENESW,
    IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, KillTimer, LoadCursorW, MB_ICONWARNING, MB_OK,
    MB_SETFOREGROUND, MB_TOPMOST, MSG, MessageBoxW, PostQuitMessage, RegisterClassExW, SW_HIDE,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetCursor, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, WM_CAPTURECHANGED, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCDESTROY, WM_RBUTTONUP,
    WM_SETCURSOR, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::menu::{self, Command};
use super::monitors;
use super::surface::Surface;
use super::wide;
use crate::canvas::Canvas;
use crate::interaction::{
    CursorShape, Hit, Rect, default_rect, drag_rect, fit_on_screen, hit_test,
};

const TIMER_ID: usize = 1;
const TICK_MS: u32 = 1000;
/// Logical size and distance from the top of the screen, at 96 DPI.
pub const DEFAULT_WIDTH: u32 = 600;
pub const DEFAULT_HEIGHT: u32 = 60;
const TOP_MARGIN: i32 = 20;
/// Width of the resize grip along the edges, and the smallest the bar may get (96 DPI).
const GRIP: i32 = 6;
const MIN_WIDTH: i32 = 160;
const MIN_HEIGHT: i32 = 24;

/// What the window shows. Called on every tick of the timer.
pub trait Handler {
    /// Paint the canvas (already cleared). Return `false` when there is nothing to show; the
    /// window then hides itself until a later tick returns `true`.
    fn draw(&mut self, canvas: &mut Canvas) -> bool;

    /// Milliseconds until the next tick. The app can aim just past a second boundary so a
    /// countdown never repeats or skips a second.
    fn next_tick_ms(&self) -> u32 {
        TICK_MS
    }

    /// The user finished moving or resizing the window; `rect` is its place in screen pixels.
    fn window_changed(&mut self, _rect: Rect) {}

    /// Whether "Start with Windows" is on, for the check mark in the right-click menu.
    fn autostart_enabled(&self) -> bool {
        false
    }

    /// A menu command other than Quit (which the window handles itself). Returns the text of an
    /// error to show to the user, if something went wrong.
    fn run_command(&mut self, _command: Command) -> Option<String> {
        None
    }
}

/// Where and how large (device pixels) the window starts; `None` means the default position.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
}

/// The smallest the bar may be, in device pixels.
pub fn min_size() -> (i32, i32) {
    let scale = dpi_scale();
    (
        (MIN_WIDTH as f32 * scale) as i32,
        (MIN_HEIGHT as f32 * scale) as i32,
    )
}

/// Distance of the default position from the top of the screen, in device pixels.
pub fn top_margin() -> i32 {
    (TOP_MARGIN as f32 * dpi_scale()) as i32
}

/// A drag in progress.
#[derive(Clone, Copy)]
struct Drag {
    hit: Hit,
    /// Where the pointer was on the screen when the button went down.
    start_screen: (i32, i32),
    /// The window rectangle at that moment.
    start_rect: Rect,
}

struct State {
    surface: Surface,
    canvas: Canvas,
    handler: Box<dyn Handler>,
    shown: bool,
    drag: Option<Drag>,
    grip: i32,
    min: (i32, i32),
    margin: i32,
}

impl State {
    /// Draw, and show or hide the window accordingly.
    fn paint(&mut self, hwnd: HWND) {
        self.canvas.clear();
        let visible = self.handler.draw(&mut self.canvas);
        // SAFETY: calls on our own window from its own thread.
        unsafe {
            if visible {
                let _ = self.surface.present(hwnd, &self.canvas);
                if !self.shown {
                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    self.shown = true;
                }
            } else if self.shown {
                let _ = ShowWindow(hwnd, SW_HIDE);
                self.shown = false;
            }
        }
    }

    /// Paint, then arm the timer for the next tick.
    fn redraw(&mut self, hwnd: HWND) {
        self.paint(hwnd);
        // SAFETY: timer on our own window.
        unsafe {
            SetTimer(Some(hwnd), TIMER_ID, self.handler.next_tick_ms(), None);
        }
    }

    /// Move and/or resize the window to `rect` (the bar is repainted at its new size).
    fn apply_rect(&mut self, hwnd: HWND, rect: Rect) {
        let resized = {
            let cur = window_rect(hwnd);
            (cur.w, cur.h) != (rect.w, rect.h)
        };
        if resized {
            let (w, h) = (rect.w.max(1) as u32, rect.h.max(1) as u32);
            let Some(surface) = Surface::new(w, h) else {
                return;
            };
            self.surface = surface;
            self.canvas = Canvas::new(w, h);
        }
        // SAFETY: our own window; the flags keep it from changing z-order or taking focus.
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                None,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        if resized {
            self.paint(hwnd);
        }
    }

    fn begin_drag(&mut self, hwnd: HWND, client: (i32, i32)) {
        let rect = window_rect(hwnd);
        self.drag = Some(Drag {
            hit: hit_test(client.0, client.1, rect.w, rect.h, self.grip),
            start_screen: (rect.x + client.0, rect.y + client.1),
            start_rect: rect,
        });
    }

    fn drag_to(&mut self, hwnd: HWND, client: (i32, i32)) {
        let Some(drag) = self.drag else {
            return;
        };
        let cur = window_rect(hwnd);
        // The pointer in screen coordinates; the window may already have moved under it.
        let (sx, sy) = (cur.x + client.0, cur.y + client.1);
        let new = drag_rect(
            drag.start_rect,
            drag.hit,
            sx - drag.start_screen.0,
            sy - drag.start_screen.1,
            self.min,
        );
        if new != cur {
            self.apply_rect(hwnd, new);
        }
    }

    /// Finish a drag; tells the handler when the window ended up somewhere else.
    fn end_drag(&mut self, hwnd: HWND) {
        if let Some(drag) = self.drag.take() {
            let now = window_rect(hwnd);
            if now != drag.start_rect {
                self.handler.window_changed(now);
            }
        }
    }

    /// Pull the window back onto a connected monitor if it is (almost) off screen.
    fn refit(&mut self, hwnd: HWND) {
        let cur = window_rect(hwnd);
        let fitted = fit_on_screen(cur, &monitors::work_areas(), self.margin);
        if fitted != cur {
            self.apply_rect(hwnd, fitted);
        }
    }

    fn cursor_shape(&self, hwnd: HWND) -> CursorShape {
        if let Some(drag) = self.drag {
            return drag.hit.cursor();
        }
        let rect = window_rect(hwnd);
        let mut pos = POINT::default();
        // SAFETY: plain query.
        if unsafe { GetCursorPos(&mut pos) }.is_err() {
            return CursorShape::Arrow;
        }
        hit_test(pos.x - rect.x, pos.y - rect.y, rect.w, rect.h, self.grip).cursor()
    }
}

fn window_rect(hwnd: HWND) -> Rect {
    let mut r = RECT::default();
    // SAFETY: plain query of a window of ours.
    let _ = unsafe { GetWindowRect(hwnd, &mut r) };
    Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}

/// Client coordinates packed in a mouse message (each a signed 16-bit value).
fn client_xy(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam.0 & 0xffff) as u16 as i16;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16;
    (i32::from(x), i32::from(y))
}

fn cursor_id(shape: CursorShape) -> PCWSTR {
    match shape {
        CursorShape::Arrow => IDC_ARROW,
        CursorShape::SizeWestEast => IDC_SIZEWE,
        CursorShape::SizeNorthSouth => IDC_SIZENS,
        CursorShape::SizeNorthwestSoutheast => IDC_SIZENWSE,
        CursorShape::SizeNortheastSouthwest => IDC_SIZENESW,
    }
}

/// Opt in to per-monitor DPI awareness (v2) so the bar is crisp on every monitor. Must run before
/// any window is created; calling it again is harmless.
pub fn enable_dpi_awareness() {
    // SAFETY: plain call; it fails (ignored) when the awareness is already set.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Scale factor of the primary display (1.0 at 96 DPI).
pub fn dpi_scale() -> f32 {
    // SAFETY: plain query.
    unsafe { GetDpiForSystem() as f32 / 96.0 }
}

/// Create and show the window and draw it once. The returned handle stays valid until the
/// window is destroyed.
pub fn create(placement: Placement, handler: Box<dyn Handler>) -> windows::core::Result<HWND> {
    let surface = Surface::new(placement.width, placement.height)
        .ok_or_else(windows::core::Error::from_thread)?;
    let margin = top_margin();
    let state = Box::new(State {
        canvas: Canvas::new(placement.width, placement.height),
        surface,
        handler,
        shown: false,
        drag: None,
        grip: ((GRIP as f32 * dpi_scale()) as i32).max(1),
        min: min_size(),
        margin,
    });
    // SAFETY: standard window creation; see the ownership note below.
    unsafe {
        let module = GetModuleHandleW(None)?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: w!("TimesUpBar"),
            ..Default::default()
        };
        // Registering twice (e.g. in tests) fails harmlessly with "class already exists".
        RegisterClassExW(&class);

        let (w, h) = (placement.width as i32, placement.height as i32);
        let home = monitors::work_areas()
            .first()
            .map_or(Rect::new(0, 0, w, h), |area| {
                default_rect(w, h, *area, margin)
            });
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("TimesUpBar"),
            w!("Times Up"),
            WS_POPUP,
            placement.x.unwrap_or(home.x),
            placement.y.unwrap_or(home.y),
            w,
            h,
            None,
            None,
            Some(instance),
            None,
        )?;
        // From here the window owns the state; WM_NCDESTROY frees it. Until then the window
        // procedure sees a null pointer and just uses the default handling.
        let raw = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        (*raw).redraw(hwnd); // shows the window (or not) and starts the timer
        Ok(hwnd)
    }
}

/// Carry out a menu command. Returns an error text for the caller to show: it is not shown here
/// because a message box runs its own message loop and no borrow of the state may be alive then.
pub fn execute(hwnd: HWND, command: Command) -> Option<String> {
    match command {
        Command::Quit => {
            destroy(hwnd);
            None
        }
        other => {
            // SAFETY: on the window's thread; the borrow ends with this statement.
            unsafe { state_of(hwnd) }.and_then(|state| state.handler.run_command(other))
        }
    }
}

/// Tell the user about a failed command.
fn report_error(text: &str) {
    let text = wide(text);
    // SAFETY: modal message box with valid strings; no window borrow is alive.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("Times Up"),
            MB_OK | MB_ICONWARNING | MB_TOPMOST | MB_SETFOREGROUND,
        );
    }
}

/// Destroy the window (used by tests and on quit).
pub fn destroy(hwnd: HWND) {
    // SAFETY: `hwnd` came from `create` and is destroyed from the thread that owns it.
    unsafe {
        let _ = KillTimer(Some(hwnd), TIMER_ID);
        let _ = DestroyWindow(hwnd);
    }
}

/// Run the message loop until the window is closed; returns the exit code.
pub fn run_message_loop() -> i32 {
    let mut msg = MSG::default();
    // SAFETY: standard message loop on the thread that created the window.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    msg.wParam.0 as i32
}

/// The window's state, if it has been attached yet. The reference must not be kept across a call
/// that can send messages back to this window (`SetCapture`, `ReleaseCapture`).
///
/// # Safety
/// Only valid on the window's own thread, between `create` and WM_NCDESTROY.
unsafe fn state_of<'a>(hwnd: HWND) -> Option<&'a mut State> {
    // SAFETY: GWLP_USERDATA is either null or the `State` stored by `create`.
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
            WM_TIMER if wparam.0 == TIMER_ID => {
                if let Some(state) = state_of(hwnd) {
                    state.redraw(hwnd);
                    if state.shown && state.drag.is_none() {
                        // Other topmost windows can push us down; stay on top without taking
                        // focus.
                        let _ = SetWindowPos(
                            hwnd,
                            Some(HWND_TOPMOST),
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                if let Some(state) = state_of(hwnd) {
                    state.begin_drag(hwnd, client_xy(lparam));
                }
                let _ = SetCapture(hwnd);
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                if let Some(state) = state_of(hwnd) {
                    state.drag_to(hwnd, client_xy(lparam));
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                if let Some(state) = state_of(hwnd) {
                    state.end_drag(hwnd);
                }
                let _ = ReleaseCapture();
                return LRESULT(0);
            }
            WM_RBUTTONUP => {
                // Read what the menu needs, then let go of the state: the menu runs its own message
                // loop, which keeps delivering timer ticks to this window.
                let autostart = match state_of(hwnd) {
                    Some(state) if state.drag.is_none() => state.handler.autostart_enabled(),
                    _ => return LRESULT(0),
                };
                if let Some(command) = menu::show(hwnd, autostart)
                    && let Some(problem) = execute(hwnd, command)
                {
                    report_error(&problem);
                }
                return LRESULT(0);
            }
            // Capture taken away (e.g. another window grabbed it): the drag is over.
            WM_CAPTURECHANGED => {
                if let Some(state) = state_of(hwnd) {
                    state.end_drag(hwnd);
                }
                return LRESULT(0);
            }
            WM_SETCURSOR if (lparam.0 & 0xffff) as u32 == HTCLIENT => {
                if let Some(state) = state_of(hwnd) {
                    let shape = state.cursor_shape(hwnd);
                    if let Ok(cursor) = LoadCursorW(None, cursor_id(shape)) {
                        SetCursor(Some(cursor));
                    }
                    return LRESULT(1);
                }
            }
            // A monitor was connected or removed, or the resolution changed.
            WM_DISPLAYCHANGE => {
                if let Some(state) = state_of(hwnd) {
                    state.refit(hwnd);
                }
            }
            WM_DESTROY => {
                PostQuitMessage(0);
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
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::HiDpi::{
        DPI_AWARENESS_PER_MONITOR_AWARE, GetAwarenessFromDpiAwarenessContext,
        GetThreadDpiAwarenessContext,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetWindowRect, IsWindow, IsWindowVisible, SendMessageW, WS_CAPTION,
        WS_THICKFRAME,
    };

    use super::*;
    use crate::canvas::RectF;
    use crate::config::Rgba;

    struct Counting {
        draws: Rc<Cell<u32>>,
    }

    impl Handler for Counting {
        fn draw(&mut self, canvas: &mut Canvas) -> bool {
            self.draws.set(self.draws.get() + 1);
            canvas.fill_rounded_rect(
                RectF::new(0.0, 0.0, canvas.width() as f32, canvas.height() as f32),
                8.0,
                Rgba::rgb(0, 200, 0),
                None,
            );
            true
        }
    }

    /// A handler whose visibility and next tick the test controls.
    struct Switch {
        visible: Rc<Cell<bool>>,
        tick_ms: Rc<Cell<u32>>,
    }

    impl Handler for Switch {
        fn draw(&mut self, _: &mut Canvas) -> bool {
            self.visible.get()
        }

        fn next_tick_ms(&self) -> u32 {
            self.tick_ms.get()
        }
    }

    fn make(draws: &Rc<Cell<u32>>) -> HWND {
        enable_dpi_awareness();
        let placement = Placement {
            x: Some(50),
            y: Some(60),
            width: 300,
            height: 40,
        };
        create(
            placement,
            Box::new(Counting {
                draws: draws.clone(),
            }),
        )
        .expect("window")
    }

    #[test]
    fn window_is_borderless_transparent_topmost_and_not_on_the_taskbar() {
        let draws = Rc::new(Cell::new(0));
        let hwnd = make(&draws);
        // SAFETY: queries on the window created above.
        unsafe {
            let ex = GetWindowLongPtrW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX(GWL_EXSTYLE.0),
            ) as u32;
            let style = GetWindowLongPtrW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX(GWL_STYLE.0),
            ) as u32;
            assert_ne!(ex & WS_EX_LAYERED.0, 0, "layered (per-pixel alpha)");
            assert_ne!(ex & WS_EX_TOPMOST.0, 0, "topmost");
            assert_ne!(ex & WS_EX_TOOLWINDOW.0, 0, "tool window: no taskbar button");
            assert_ne!(ex & WS_EX_NOACTIVATE.0, 0, "never takes focus");
            assert_eq!(style & WS_CAPTION.0, 0, "no title bar");
            assert_eq!(style & WS_THICKFRAME.0, 0, "no sizing frame");
            assert_ne!(style & WS_POPUP.0, 0);
            assert!(IsWindowVisible(hwnd).as_bool());
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).unwrap();
            assert_eq!(
                (
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top
                ),
                (50, 60, 300, 40)
            );
        }
        assert_eq!(draws.get(), 1, "drawn once on creation");
        destroy(hwnd);
    }

    #[test]
    fn process_is_per_monitor_dpi_aware() {
        enable_dpi_awareness();
        // SAFETY: plain queries.
        let awareness =
            unsafe { GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()) };
        assert_eq!(awareness, DPI_AWARENESS_PER_MONITOR_AWARE);
        assert!(dpi_scale() >= 1.0);
    }

    #[test]
    fn window_hides_and_shows_with_the_handler() {
        enable_dpi_awareness();
        let visible = Rc::new(Cell::new(false));
        let tick_ms = Rc::new(Cell::new(1000));
        let hwnd = create(
            Placement {
                x: Some(10),
                y: Some(10),
                width: 100,
                height: 20,
            },
            Box::new(Switch {
                visible: visible.clone(),
                tick_ms,
            }),
        )
        .unwrap();
        let tick = || {
            // SAFETY: synchronous message to our own window on this thread.
            unsafe { SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER_ID)), Some(LPARAM(0))) };
        };
        // SAFETY: query on our own window.
        let shown = || unsafe { IsWindowVisible(hwnd).as_bool() };

        assert!(!shown(), "nothing to show at creation");
        tick();
        assert!(!shown());
        visible.set(true);
        tick();
        assert!(shown(), "shown when the handler has something to show");
        tick();
        assert!(shown());
        visible.set(false);
        tick();
        assert!(!shown(), "hidden again when there is nothing to show");
        destroy(hwnd);
    }

    #[test]
    fn timer_tick_redraws() {
        let draws = Rc::new(Cell::new(0));
        let hwnd = make(&draws);
        // SAFETY: synchronous message to our own window on this thread.
        unsafe {
            SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER_ID)), Some(LPARAM(0)));
            SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER_ID)), Some(LPARAM(0)));
            // A different timer id is not ours.
            SendMessageW(hwnd, WM_TIMER, Some(WPARAM(99)), Some(LPARAM(0)));
        }
        assert_eq!(draws.get(), 3);
        destroy(hwnd);
    }

    #[test]
    fn default_position_is_centred_near_the_top() {
        enable_dpi_awareness();
        let draws = Rc::new(Cell::new(0));
        let hwnd = create(
            Placement {
                x: None,
                y: None,
                width: 200,
                height: 30,
            },
            Box::new(Counting { draws }),
        )
        .unwrap();
        // SAFETY: query on the window created above.
        let rect = unsafe {
            let mut r = RECT::default();
            GetWindowRect(hwnd, &mut r).unwrap();
            r
        };
        let primary = monitors::work_areas()[0];
        let expected = default_rect(200, 30, primary, top_margin());
        assert_eq!((rect.left, rect.top), (expected.x, expected.y));
        assert!(rect.top >= primary.y, "top {}", rect.top);
        destroy(hwnd);
    }

    // ---- moving and resizing with the mouse ----

    /// Remembers what the window reports and the size of the canvas it was last asked to draw.
    struct Recorder {
        changes: Rc<RefCell<Vec<Rect>>>,
        canvas_size: Rc<Cell<(u32, u32)>>,
    }

    impl Handler for Recorder {
        fn draw(&mut self, canvas: &mut Canvas) -> bool {
            self.canvas_size.set((canvas.width(), canvas.height()));
            true
        }

        fn window_changed(&mut self, rect: Rect) {
            self.changes.borrow_mut().push(rect);
        }
    }

    struct Bar {
        hwnd: HWND,
        changes: Rc<RefCell<Vec<Rect>>>,
        canvas_size: Rc<Cell<(u32, u32)>>,
    }

    impl Bar {
        /// A 300 x 40 bar at (200, 200).
        fn new() -> Self {
            enable_dpi_awareness();
            let changes = Rc::new(RefCell::new(Vec::new()));
            let canvas_size = Rc::new(Cell::new((0, 0)));
            let hwnd = create(
                Placement {
                    x: Some(200),
                    y: Some(200),
                    width: 300,
                    height: 40,
                },
                Box::new(Recorder {
                    changes: changes.clone(),
                    canvas_size: canvas_size.clone(),
                }),
            )
            .unwrap();
            Self {
                hwnd,
                changes,
                canvas_size,
            }
        }

        fn send(&self, msg: u32, x: i32, y: i32) {
            let lparam = ((i64::from(y as u16) << 16) | i64::from(x as u16)) as isize;
            // SAFETY: synchronous message to our own window on this thread.
            unsafe {
                SendMessageW(self.hwnd, msg, Some(WPARAM(0)), Some(LPARAM(lparam)));
            }
        }

        fn rect(&self) -> Rect {
            window_rect(self.hwnd)
        }

        fn changes(&self) -> Vec<Rect> {
            self.changes.borrow().clone()
        }

        fn has_capture(&self) -> bool {
            // SAFETY: plain query.
            unsafe { GetCapture() == self.hwnd }
        }
    }

    impl Drop for Bar {
        fn drop(&mut self) {
            destroy(self.hwnd);
        }
    }

    #[test]
    fn dragging_the_body_moves_the_window() {
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 150, 20);
        assert!(bar.has_capture(), "the mouse is captured during the drag");
        assert_eq!(
            bar.rect(),
            Rect::new(200, 200, 300, 40),
            "nothing moves yet"
        );

        // The pointer moves 30 right and 10 down on the screen.
        bar.send(WM_MOUSEMOVE, 180, 30);
        assert_eq!(bar.rect(), Rect::new(230, 210, 300, 40));
        // Another 5 to the right: relative to the window that has moved with it, so +5 only.
        bar.send(WM_MOUSEMOVE, 155, 20);
        assert_eq!(bar.rect(), Rect::new(235, 210, 300, 40));
        assert!(bar.changes().is_empty(), "nothing is reported mid-drag");

        bar.send(WM_LBUTTONUP, 155, 20);
        assert!(!bar.has_capture(), "released at the end");
        assert_eq!(bar.changes(), vec![Rect::new(235, 210, 300, 40)]);
        // The size did not change, so the canvas was not rebuilt.
        assert_eq!(bar.canvas_size.get(), (300, 40));

        // Moving the mouse afterwards does nothing.
        bar.send(WM_MOUSEMOVE, 10, 10);
        assert_eq!(bar.rect(), Rect::new(235, 210, 300, 40));
    }

    #[test]
    fn a_click_without_moving_reports_nothing() {
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 150, 20);
        bar.send(WM_LBUTTONUP, 150, 20);
        assert!(bar.changes().is_empty());
        assert_eq!(bar.rect(), Rect::new(200, 200, 300, 40));
        // Out and back to the start is also "no change".
        bar.send(WM_LBUTTONDOWN, 150, 20);
        bar.send(WM_MOUSEMOVE, 190, 20);
        bar.send(WM_MOUSEMOVE, 110, 20);
        bar.send(WM_LBUTTONUP, 150, 20);
        assert!(bar.changes().is_empty());
    }

    #[test]
    fn dragging_the_right_edge_resizes_and_repaints_at_the_new_size() {
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 298, 20);
        bar.send(WM_MOUSEMOVE, 348, 20); // 50 px to the right
        assert_eq!(bar.rect(), Rect::new(200, 200, 350, 40));
        assert_eq!(bar.canvas_size.get(), (350, 40), "drawn at the new size");
        bar.send(WM_LBUTTONUP, 348, 20);
        assert_eq!(bar.changes(), vec![Rect::new(200, 200, 350, 40)]);
    }

    #[test]
    fn dragging_the_left_edge_moves_the_left_side_only() {
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 2, 20);
        bar.send(WM_MOUSEMOVE, -38, 20); // 40 px to the left
        assert_eq!(bar.rect(), Rect::new(160, 200, 340, 40));
        bar.send(WM_LBUTTONUP, -38, 20);
        assert_eq!(bar.changes(), vec![Rect::new(160, 200, 340, 40)]);
    }

    #[test]
    fn corners_resize_both_ways() {
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 298, 38);
        bar.send(WM_MOUSEMOVE, 318, 48);
        assert_eq!(bar.rect(), Rect::new(200, 200, 320, 50));
        assert_eq!(bar.canvas_size.get(), (320, 50));
        bar.send(WM_LBUTTONUP, 318, 48);
        assert_eq!(bar.changes().len(), 1);
    }

    #[test]
    fn the_bar_cannot_be_made_smaller_than_the_minimum() {
        let (min_w, min_h) = min_size();
        // From the left: the right edge (500) stays where it is.
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 2, 20);
        bar.send(WM_MOUSEMOVE, 2 + 2000, 20);
        let r = bar.rect();
        assert_eq!((r.w, r.right()), (min_w, 500));
        bar.send(WM_LBUTTONUP, 0, 0);
        drop(bar);
        // From the top: the bottom edge (240) stays.
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 150, 2);
        bar.send(WM_MOUSEMOVE, 150, 2 + 1000);
        let r = bar.rect();
        assert_eq!((r.h, r.bottom()), (min_h, 240));
        assert_eq!(bar.canvas_size.get(), (300, min_h as u32));
        // From the right edge and the bottom edge too.
        bar.send(WM_LBUTTONUP, 0, 0);
        drop(bar);
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 298, 38);
        bar.send(WM_MOUSEMOVE, 298 - 2000, 38 - 1000);
        let r = bar.rect();
        assert_eq!((r.w, r.h), (min_w, min_h));
    }

    #[test]
    fn losing_the_mouse_capture_ends_the_drag() {
        let bar = Bar::new();
        bar.send(WM_LBUTTONDOWN, 150, 20);
        bar.send(WM_MOUSEMOVE, 170, 20);
        // Another window grabs the mouse.
        bar.send(WM_CAPTURECHANGED, 0, 0);
        assert_eq!(bar.changes(), vec![Rect::new(220, 200, 300, 40)]);
        bar.send(WM_MOUSEMOVE, 300, 20);
        assert_eq!(
            bar.rect(),
            Rect::new(220, 200, 300, 40),
            "no longer following the mouse"
        );
    }

    #[test]
    fn moving_the_mouse_without_a_button_does_nothing() {
        let bar = Bar::new();
        bar.send(WM_MOUSEMOVE, 100, 10);
        bar.send(WM_LBUTTONUP, 100, 10);
        assert_eq!(bar.rect(), Rect::new(200, 200, 300, 40));
        assert!(bar.changes().is_empty());
    }

    #[test]
    fn a_window_stranded_off_screen_is_pulled_back_when_the_displays_change() {
        enable_dpi_awareness();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let hwnd = create(
            Placement {
                x: Some(30_000),
                y: Some(20_000),
                width: 300,
                height: 40,
            },
            Box::new(Recorder {
                changes: changes.clone(),
                canvas_size: Rc::new(Cell::new((0, 0))),
            }),
        )
        .unwrap();
        assert_eq!(window_rect(hwnd).x, 30_000);
        // SAFETY: synchronous message to our own window on this thread.
        unsafe {
            SendMessageW(hwnd, WM_DISPLAYCHANGE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        let primary = monitors::work_areas()[0];
        assert_eq!(
            window_rect(hwnd),
            default_rect(300, 40, primary, top_margin())
        );
        assert!(
            changes.borrow().is_empty(),
            "a display change is not a user move, so it is not saved"
        );
        destroy(hwnd);
    }

    #[test]
    fn a_visible_window_stays_put_when_the_displays_change() {
        let bar = Bar::new();
        bar.send(WM_DISPLAYCHANGE, 0, 0);
        assert_eq!(bar.rect(), Rect::new(200, 200, 300, 40));
    }

    // ---- menu commands ----

    /// A handler with commands: reports an error for "open config" and records them all.
    struct Commands {
        seen: Rc<RefCell<Vec<Command>>>,
        autostart: bool,
    }

    impl Handler for Commands {
        fn draw(&mut self, _: &mut Canvas) -> bool {
            true
        }

        fn autostart_enabled(&self) -> bool {
            self.autostart
        }

        fn run_command(&mut self, command: Command) -> Option<String> {
            self.seen.borrow_mut().push(command);
            (command == Command::OpenConfig).then(|| "could not open".to_string())
        }
    }

    fn commands_window(autostart: bool) -> (HWND, Rc<RefCell<Vec<Command>>>) {
        enable_dpi_awareness();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let hwnd = create(
            Placement {
                x: Some(10),
                y: Some(10),
                width: 100,
                height: 20,
            },
            Box::new(Commands {
                seen: seen.clone(),
                autostart,
            }),
        )
        .unwrap();
        (hwnd, seen)
    }

    #[test]
    fn menu_commands_reach_the_handler_and_its_errors_come_back() {
        let (hwnd, seen) = commands_window(false);
        assert_eq!(
            execute(hwnd, Command::OpenConfig),
            Some("could not open".to_string())
        );
        assert_eq!(execute(hwnd, Command::ToggleAutostart), None);
        assert_eq!(
            *seen.borrow(),
            vec![Command::OpenConfig, Command::ToggleAutostart]
        );
        // SAFETY: query on our own window.
        assert!(
            unsafe { IsWindow(Some(hwnd)).as_bool() },
            "only Quit closes the window"
        );
        destroy(hwnd);
    }

    #[test]
    fn quit_closes_the_window_without_asking_the_handler() {
        let (hwnd, seen) = commands_window(false);
        assert_eq!(execute(hwnd, Command::Quit), None);
        // SAFETY: query on a window that should be gone.
        assert!(!unsafe { IsWindow(Some(hwnd)).as_bool() });
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn handlers_without_commands_ignore_them() {
        let draws = Rc::new(Cell::new(0));
        let hwnd = make(&draws);
        assert_eq!(execute(hwnd, Command::OpenConfig), None);
        assert_eq!(execute(hwnd, Command::ToggleAutostart), None);
        destroy(hwnd);
        // The default handler also says autostart is off.
        let handler = Counting { draws };
        assert!(!handler.autostart_enabled());
    }

    #[test]
    fn the_cursor_message_is_handled_for_the_client_area_only() {
        let bar = Bar::new();
        // SAFETY: synchronous messages to our own window on this thread.
        unsafe {
            let handled = SendMessageW(
                bar.hwnd,
                WM_SETCURSOR,
                Some(WPARAM(bar.hwnd.0 as usize)),
                Some(LPARAM(HTCLIENT as isize)),
            );
            assert_eq!(handled.0, 1, "we set the cursor ourselves");
        }
    }

    #[test]
    fn destroying_the_window_releases_the_state() {
        // The handler is dropped with the window (WM_NCDESTROY frees the State).
        struct Probe(Rc<Cell<bool>>);
        impl Handler for Probe {
            fn draw(&mut self, _: &mut Canvas) -> bool {
                true
            }
        }
        impl Drop for Probe {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        enable_dpi_awareness();
        let dropped = Rc::new(Cell::new(false));
        let hwnd = create(
            Placement {
                x: Some(0),
                y: Some(0),
                width: 20,
                height: 20,
            },
            Box::new(Probe(dropped.clone())),
        )
        .unwrap();
        assert!(!dropped.get());
        destroy(hwnd);
        assert!(dropped.get());
    }
}
