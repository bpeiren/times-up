//! The notification-area (tray) icon. It gives the app a visible presence when the bar is hidden
//! between classes, and the same menu: Open config, Start with Windows, Quit.
//!
//! The icon is drawn at run time ([`crate::icon`]) and turned into an `HICON` here, so the exe
//! needs no icon resource.

use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, HGDIOBJ,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, NOTIFYICONIDENTIFIER,
    Shell_NotifyIconGetRect, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, GetSystemMetrics, HICON, ICONINFO, RegisterWindowMessageW,
    SM_CXSMICON, WM_APP, WM_CONTEXTMENU, WM_LBUTTONUP, WM_RBUTTONUP,
};
use windows::core::w;

use crate::canvas::Canvas;
use crate::config::Colors;
use crate::icon::tray_icon;

/// The message the shell sends to the window when the pointer does something on the icon.
pub const CALLBACK_MESSAGE: u32 = WM_APP + 1;
const ICON_ID: u32 = 1;
const TOOLTIP: &str = "Times Up";

/// Whether a tray callback (`lParam` of [`CALLBACK_MESSAGE`]) is a click that should open the menu.
pub fn opens_menu(lparam: isize) -> bool {
    matches!(
        (lparam & 0xffff) as u32,
        WM_LBUTTONUP | WM_RBUTTONUP | WM_CONTEXTMENU
    )
}

/// The message Explorer broadcasts after it restarts; the icon has to be added again then.
pub fn taskbar_created_message() -> u32 {
    // SAFETY: registering (or looking up) a window message by name.
    unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) }
}

/// Turn the canvas into an icon with per-pixel alpha.
fn icon_from_canvas(canvas: &Canvas) -> Option<HICON> {
    let (w, h) = (canvas.width() as i32, canvas.height() as i32);
    if w == 0 || h == 0 {
        return None;
    }
    // SAFETY: GDI objects created and released here; the DIB holds `w * h` u32s as requested.
    unsafe {
        let dc = CreateCompatibleDC(None);
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
        let mut bits: *mut c_void = std::ptr::null_mut();
        let color = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0);
        let mask = CreateBitmap(w, h, 1, 1, None);
        let icon = match color {
            Ok(color) if !bits.is_null() && !mask.is_invalid() => {
                std::ptr::copy_nonoverlapping(
                    canvas.pixels().as_ptr(),
                    bits.cast::<u32>(),
                    canvas.pixels().len(),
                );
                let info = ICONINFO {
                    fIcon: true.into(),
                    xHotspot: 0,
                    yHotspot: 0,
                    hbmMask: mask,
                    hbmColor: color,
                };
                CreateIconIndirect(&info).ok()
            }
            _ => None,
        };
        // The icon keeps its own copies of the bitmaps.
        if let Ok(color) = color {
            let _ = DeleteObject(HGDIOBJ(color.0));
        }
        if !mask.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(mask.0));
        }
        let _ = DeleteDC(dc);
        icon
    }
}

fn notify_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: CALLBACK_MESSAGE,
        hIcon: icon,
        ..Default::default()
    };
    for (slot, unit) in data.szTip.iter_mut().zip(TOOLTIP.encode_utf16()) {
        *slot = unit;
    }
    data
}

/// The icon in the notification area. Dropping it removes the icon again.
pub struct Tray {
    hwnd: HWND,
    icon: HICON,
}

impl Tray {
    /// Add the icon for `hwnd`; clicks on it arrive as [`CALLBACK_MESSAGE`].
    pub fn add(hwnd: HWND) -> Option<Self> {
        // SAFETY: plain query.
        let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
        let icon = icon_from_canvas(&tray_icon(size, &Colors::default()))?;
        let tray = Self { hwnd, icon };
        // SAFETY: valid data for our own window; failure is handled below.
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &notify_data(hwnd, icon)) }.as_bool();
        added.then_some(tray) // otherwise dropping `tray` frees the icon
    }

    /// Add the icon again after Explorer restarted (the old one is gone with it).
    pub fn readd(&self) {
        // SAFETY: as in `add`.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &notify_data(self.hwnd, self.icon));
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // SAFETY: removing the icon we added and freeing the handle we created.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &notify_data(self.hwnd, self.icon));
            let _ = DestroyIcon(self.icon);
        }
    }
}

/// Where the shell shows the icon of `hwnd` on the screen, if there is one (used by tests).
pub fn icon_rect(hwnd: HWND) -> Option<RECT> {
    let id = NOTIFYICONIDENTIFIER {
        cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        ..Default::default()
    };
    // SAFETY: plain query with valid data.
    unsafe { Shell_NotifyIconGetRect(&id) }.ok()
}

/// Remove the icon of `hwnd` from the tray without telling its [`Tray`], like Explorer restarting
/// would (used by tests).
pub fn remove_icon_behind_the_apps_back(hwnd: HWND) {
    let data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        ..Default::default()
    };
    // SAFETY: removing an icon by window and id.
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::Graphics::Gdi::{BITMAP, GetObjectW};
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, WM_LBUTTONDOWN, WM_MOUSEMOVE};

    use super::*;
    use crate::win::window::{self, Handler, Placement};

    struct Quiet;

    impl Handler for Quiet {
        fn draw(&mut self, _: &mut Canvas) -> bool {
            false
        }
    }

    /// A hidden window to hang the icon on (no tray icon of its own).
    fn host() -> HWND {
        window::enable_dpi_awareness();
        window::create(
            Placement {
                x: Some(0),
                y: Some(0),
                width: 50,
                height: 20,
            },
            Box::new(Quiet),
        )
        .unwrap()
    }

    #[test]
    fn clicks_open_the_menu_and_other_pointer_messages_do_not() {
        for message in [WM_LBUTTONUP, WM_RBUTTONUP, WM_CONTEXTMENU] {
            assert!(opens_menu(message as isize), "{message:#x}");
        }
        for message in [WM_MOUSEMOVE, WM_LBUTTONDOWN, 0] {
            assert!(!opens_menu(message as isize), "{message:#x}");
        }
        // The shell may put extra data in the high word.
        assert!(opens_menu(((7 << 16) | WM_RBUTTONUP) as isize));
    }

    #[test]
    fn the_canvas_becomes_an_icon_of_the_same_size() {
        let canvas = tray_icon(24, &Colors::default());
        let icon = icon_from_canvas(&canvas).expect("icon");
        let mut info = ICONINFO::default();
        // SAFETY: reading back and freeing an icon we made.
        unsafe {
            GetIconInfo(icon, &mut info).unwrap();
            let mut bitmap = BITMAP::default();
            GetObjectW(
                HGDIOBJ(info.hbmColor.0),
                size_of::<BITMAP>() as i32,
                Some((&mut bitmap as *mut BITMAP).cast()),
            );
            assert_eq!((bitmap.bmWidth, bitmap.bmHeight), (24, 24));
            let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            let _ = DestroyIcon(icon);
        }
        assert!(icon_from_canvas(&Canvas::new(0, 0)).is_none());
    }

    #[test]
    fn the_icon_is_in_the_notification_area_while_the_tray_lives() {
        let hwnd = host();
        assert!(icon_rect(hwnd).is_none(), "nothing yet");
        let tray = Tray::add(hwnd).expect("tray icon");
        let rect = icon_rect(hwnd).expect("the shell knows the icon");
        assert!(rect.right > rect.left && rect.bottom > rect.top, "{rect:?}");
        drop(tray);
        assert!(
            icon_rect(hwnd).is_none(),
            "removed when the tray is dropped"
        );
        window::destroy(hwnd);
    }

    #[test]
    fn the_icon_can_be_added_again_after_explorer_restarts() {
        let hwnd = host();
        let tray = Tray::add(hwnd).unwrap();
        // Explorer restarting wipes the icons; simulate that by removing ours behind its back.
        // SAFETY: removing the icon this test added.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &notify_data(hwnd, tray.icon));
        }
        assert!(icon_rect(hwnd).is_none());
        tray.readd();
        assert!(icon_rect(hwnd).is_some(), "back after the re-add");
        drop(tray);
        window::destroy(hwnd);
    }

    #[test]
    fn the_registered_taskbar_message_is_stable() {
        let m = taskbar_created_message();
        assert!(
            m >= 0xc000,
            "registered messages start at 0xC000, got {m:#x}"
        );
        assert_eq!(m, taskbar_created_message());
    }

    #[test]
    fn the_tooltip_is_copied_into_the_notification_data() {
        let data = notify_data(HWND::default(), HICON::default());
        let text: String = String::from_utf16_lossy(&data.szTip);
        assert!(text.starts_with(TOOLTIP), "{text:?}");
        assert_eq!(data.uCallbackMessage, CALLBACK_MESSAGE);
        assert_eq!(data.uID, ICON_ID);
    }
}
