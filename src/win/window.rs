//! The borderless, transparent, always-on-top window.
//!
//! * `WS_POPUP` + `WS_EX_LAYERED`: no frame, per-pixel alpha.
//! * `WS_EX_TOOLWINDOW`: no taskbar button (and not in Alt+Tab).
//! * `WS_EX_NOACTIVATE`: clicking the bar never steals focus from the teacher's slides.
//! * `WS_EX_TOPMOST`, re-asserted every tick because other topmost windows can cover it.

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, MSG,
    PostQuitMessage, RegisterClassExW, SM_CXSCREEN, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TranslateMessage, WM_DESTROY, WM_NCDESTROY, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::w;

use super::surface::Surface;
use crate::canvas::Canvas;

const TIMER_ID: usize = 1;
const TICK_MS: u32 = 1000;
/// Logical size and distance from the top of the screen, at 96 DPI.
pub const DEFAULT_WIDTH: u32 = 600;
pub const DEFAULT_HEIGHT: u32 = 60;
const TOP_MARGIN: i32 = 20;

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
}

/// Where and how large (device pixels) the window starts; `None` means the default position.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
}

struct State {
    surface: Surface,
    canvas: Canvas,
    handler: Box<dyn Handler>,
    shown: bool,
}

impl State {
    /// Draw, show or hide the window accordingly, and arm the timer for the next tick.
    fn redraw(&mut self, hwnd: HWND) {
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
            SetTimer(Some(hwnd), TIMER_ID, self.handler.next_tick_ms(), None);
        }
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
    let state = Box::new(State {
        canvas: Canvas::new(placement.width, placement.height),
        surface,
        handler,
        shown: false,
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

        let x = placement
            .x
            .unwrap_or_else(|| (GetSystemMetrics(SM_CXSCREEN) - placement.width as i32) / 2);
        let y = placement
            .y
            .unwrap_or((TOP_MARGIN as f32 * dpi_scale()) as i32);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("TimesUpBar"),
            w!("Times Up"),
            WS_POPUP,
            x,
            y,
            placement.width as i32,
            placement.height as i32,
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

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: GWLP_USERDATA is either null or the `State` stored by `create` until WM_NCDESTROY.
    unsafe {
        match msg {
            WM_TIMER if wparam.0 == TIMER_ID => {
                if let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State).as_mut()
                {
                    state.redraw(hwnd);
                    if state.shown {
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
    use std::cell::Cell;
    use std::rc::Rc;

    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::HiDpi::{
        DPI_AWARENESS_PER_MONITOR_AWARE, GetAwarenessFromDpiAwarenessContext,
        GetThreadDpiAwarenessContext,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetWindowRect, IsWindowVisible, SendMessageW, WS_CAPTION,
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
        let screen = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        assert_eq!(rect.left, (screen - 200) / 2);
        assert!(rect.top > 0 && rect.top < 100, "top {}", rect.top);
        destroy(hwnd);
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
