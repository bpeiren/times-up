//! The usable area of every monitor, so a saved window position can be checked against what is
//! connected right now.

use std::ffi::c_void;

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::core::BOOL;

use crate::interaction::Rect;

/// `MONITORINFO::dwFlags` bit set for the primary monitor.
const MONITORINFOF_PRIMARY: u32 = 1;

/// Work areas (monitor minus taskbar) of all monitors, the primary monitor first.
pub fn work_areas() -> Vec<Rect> {
    let mut found: Vec<(bool, Rect)> = Vec::new();
    // SAFETY: the callback only runs during this call and `found` outlives it.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut found as *mut Vec<(bool, Rect)> as isize),
        );
    }
    found.sort_by_key(|&(primary, _)| !primary);
    found.into_iter().map(|(_, rect)| rect).collect()
}

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
    // SAFETY: `data` is the pointer to the Vec passed by `work_areas`, valid for the whole call.
    unsafe {
        let found = &mut *(data.0 as *mut c_void as *mut Vec<(bool, Rect)>);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let r = info.rcWork;
            let primary = info.dwFlags & MONITORINFOF_PRIMARY != 0;
            found.push((
                primary,
                Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top),
            ));
        }
    }
    BOOL(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_is_at_least_one_monitor_and_the_primary_comes_first() {
        let areas = work_areas();
        assert!(!areas.is_empty());
        assert!(areas.iter().all(|a| a.w > 0 && a.h > 0), "{areas:?}");
        // The primary monitor starts at the origin of the virtual screen.
        let first = areas[0];
        assert!(first.x.abs() < 10_000 && first.y.abs() < 10_000);
    }

    #[test]
    fn the_default_placement_is_visible_on_the_current_monitors() {
        use crate::interaction::{default_rect, fit_on_screen};
        let areas = work_areas();
        let rect = default_rect(600, 60, areas[0], 20);
        assert_eq!(fit_on_screen(rect, &areas, 20), rect);
        // A position on a monitor that cannot exist ends up on the primary one.
        let far = Rect::new(900_000, 900_000, 600, 60);
        assert_eq!(fit_on_screen(far, &areas, 20), rect);
    }
}
