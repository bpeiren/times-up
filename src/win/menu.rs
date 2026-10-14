//! The right-click menu: the few things a teacher needs without a settings window.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, HMENU, MF_CHECKED, MF_SEPARATOR,
    MF_STRING, MF_UNCHECKED, PostMessageW, SetForegroundWindow, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, WM_NULL,
};
use windows::core::PCWSTR;

use super::wide;

pub const LABEL_OPEN_CONFIG: &str = "Open config (Notepad)";
pub const LABEL_AUTOSTART: &str = "Start with Windows";
pub const LABEL_QUIT: &str = "Quit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Open `times-up.toml` in Notepad.
    OpenConfig,
    /// Add or remove the "start with Windows" entry.
    ToggleAutostart,
    Quit,
}

impl Command {
    const fn id(self) -> usize {
        match self {
            Command::OpenConfig => 1,
            Command::ToggleAutostart => 2,
            Command::Quit => 3,
        }
    }

    pub const fn from_id(id: usize) -> Option<Self> {
        match id {
            1 => Some(Command::OpenConfig),
            2 => Some(Command::ToggleAutostart),
            3 => Some(Command::Quit),
            _ => None,
        }
    }
}

/// The popup menu; "Start with Windows" carries a check mark when `autostart` is on. The caller
/// owns the menu and destroys it.
pub fn build(autostart: bool) -> Option<HMENU> {
    // SAFETY: plain menu creation; the strings outlive each call.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let check = if autostart { MF_CHECKED } else { MF_UNCHECKED };
        let open = wide(LABEL_OPEN_CONFIG);
        let auto = wide(LABEL_AUTOSTART);
        let quit = wide(LABEL_QUIT);
        let ok = AppendMenuW(
            menu,
            MF_STRING,
            Command::OpenConfig.id(),
            PCWSTR(open.as_ptr()),
        )
        .and_then(|()| {
            AppendMenuW(
                menu,
                MF_STRING | check,
                Command::ToggleAutostart.id(),
                PCWSTR(auto.as_ptr()),
            )
        })
        .and_then(|()| AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()))
        .and_then(|()| AppendMenuW(menu, MF_STRING, Command::Quit.id(), PCWSTR(quit.as_ptr())));
        if ok.is_err() {
            let _ = DestroyMenu(menu);
            return None;
        }
        Some(menu)
    }
}

/// Show the menu at the pointer and wait for a choice. Blocks (in the system's menu loop, which
/// keeps dispatching messages such as the timer) until the menu closes.
pub fn show(hwnd: HWND, autostart: bool) -> Option<Command> {
    let menu = build(autostart)?;
    let mut pos = POINT::default();
    // SAFETY: our own window and menu; the menu is destroyed before returning.
    let chosen = unsafe {
        let _ = GetCursorPos(&mut pos);
        // Without this a menu of a window that is not in the foreground does not close when the
        // user clicks elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let id = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            pos.x,
            pos.y,
            None,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        id.0
    };
    usize::try_from(chosen).ok().and_then(Command::from_id)
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetMenuItemCount, GetMenuState, GetMenuStringW, MF_BYPOSITION,
    };

    use super::*;

    fn label(menu: HMENU, position: u32) -> String {
        let mut buf = [0u16; 64];
        // SAFETY: reading a menu we built.
        let len = unsafe { GetMenuStringW(menu, position, Some(&mut buf), MF_BYPOSITION) };
        String::from_utf16_lossy(&buf[..len.max(0) as usize])
    }

    fn state(menu: HMENU, position: u32) -> u32 {
        // SAFETY: reading a menu we built.
        unsafe { GetMenuState(menu, position, MF_BYPOSITION) }
    }

    #[test]
    fn menu_has_open_config_autostart_and_quit() {
        let menu = build(false).expect("menu");
        // SAFETY: reading and destroying a menu we built.
        unsafe {
            assert_eq!(
                GetMenuItemCount(Some(menu)),
                4,
                "three entries and a separator"
            );
            assert_eq!(label(menu, 0), LABEL_OPEN_CONFIG);
            assert_eq!(label(menu, 1), LABEL_AUTOSTART);
            assert_eq!(state(menu, 2) & MF_SEPARATOR.0, MF_SEPARATOR.0);
            assert_eq!(label(menu, 3), LABEL_QUIT);
            let _ = DestroyMenu(menu);
        }
    }

    #[test]
    fn the_autostart_entry_is_checked_only_when_it_is_on() {
        for (on, expected) in [(false, 0), (true, MF_CHECKED.0)] {
            let menu = build(on).unwrap();
            assert_eq!(state(menu, 1) & MF_CHECKED.0, expected, "autostart = {on}");
            assert_eq!(
                state(menu, 0) & MF_CHECKED.0,
                0,
                "the other entries are never checked"
            );
            assert_eq!(state(menu, 3) & MF_CHECKED.0, 0);
            // SAFETY: destroying a menu we built.
            unsafe {
                let _ = DestroyMenu(menu);
            }
        }
    }

    #[test]
    fn command_ids_round_trip() {
        for cmd in [Command::OpenConfig, Command::ToggleAutostart, Command::Quit] {
            assert_eq!(Command::from_id(cmd.id()), Some(cmd));
        }
        // 0 is what the menu returns when it is dismissed without a choice.
        assert_eq!(Command::from_id(0), None);
        assert_eq!(Command::from_id(4), None);
    }
}
