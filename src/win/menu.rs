//! The right-click menu: the few things a teacher needs without a settings window.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, HMENU, MENU_ITEM_FLAGS, MF_CHECKED,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, PostMessageW, SetForegroundWindow, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_NULL,
};
use windows::core::PCWSTR;

use super::wide;
use crate::strings::{LABEL_AUTOSTART, LABEL_QUIT, LABEL_SAVE_DEFAULT, LABEL_SETTINGS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Store the break and warning thresholds of the current class as its defaults.
    SaveDefault,
    /// Open the Settings window.
    Settings,
    /// Add or remove the "start with Windows" entry.
    ToggleAutostart,
    Quit,
}

impl Command {
    const fn id(self) -> usize {
        match self {
            Command::Settings => 1,
            Command::ToggleAutostart => 2,
            Command::Quit => 3,
            Command::SaveDefault => 4,
        }
    }

    pub const fn from_id(id: usize) -> Option<Self> {
        match id {
            1 => Some(Command::Settings),
            2 => Some(Command::ToggleAutostart),
            3 => Some(Command::Quit),
            4 => Some(Command::SaveDefault),
            _ => None,
        }
    }
}

/// What the menu needs to know to look right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuState {
    /// "Start with Windows" is on (gets a check mark).
    pub autostart: bool,
    /// The current class has changes that can be saved as its defaults (otherwise the entry is
    /// greyed out).
    pub can_save_default: bool,
}

/// The popup menu. The caller owns it and destroys it.
pub fn build(state: MenuState) -> Option<HMENU> {
    // SAFETY: plain menu creation; the strings outlive each call.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let add = |flags: MENU_ITEM_FLAGS, command: Option<Command>, label: &str| {
            let text = wide(label);
            let id = command.map_or(0, Command::id);
            let text = if flags == MF_SEPARATOR {
                PCWSTR::null()
            } else {
                PCWSTR(text.as_ptr())
            };
            AppendMenuW(menu, flags, id, text)
        };
        let save = if state.can_save_default {
            MF_STRING
        } else {
            MF_STRING | MF_GRAYED
        };
        let auto = if state.autostart {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let built = add(save, Some(Command::SaveDefault), LABEL_SAVE_DEFAULT)
            .and_then(|()| add(MF_SEPARATOR, None, ""))
            .and_then(|()| add(MF_STRING, Some(Command::Settings), LABEL_SETTINGS))
            .and_then(|()| add(auto, Some(Command::ToggleAutostart), LABEL_AUTOSTART))
            .and_then(|()| add(MF_SEPARATOR, None, ""))
            .and_then(|()| add(MF_STRING, Some(Command::Quit), LABEL_QUIT));
        if built.is_err() {
            let _ = DestroyMenu(menu);
            return None;
        }
        Some(menu)
    }
}

/// Show the menu at the pointer and wait for a choice. Blocks (in the system's menu loop, which
/// keeps dispatching messages such as the timer) until the menu closes.
pub fn show(hwnd: HWND, state: MenuState) -> Option<Command> {
    let menu = build(state)?;
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

    fn destroy(menu: HMENU) {
        // SAFETY: destroying a menu we built.
        unsafe {
            let _ = DestroyMenu(menu);
        }
    }

    #[test]
    fn menu_has_save_settings_autostart_and_quit() {
        let menu = build(MenuState::default()).expect("menu");
        // SAFETY: reading a menu we built.
        let count = unsafe { GetMenuItemCount(Some(menu)) };
        assert_eq!(count, 6, "four entries and two separators");
        assert_eq!(label(menu, 0), LABEL_SAVE_DEFAULT);
        assert_eq!(state(menu, 1) & MF_SEPARATOR.0, MF_SEPARATOR.0);
        assert_eq!(label(menu, 2), LABEL_SETTINGS);
        assert_eq!(label(menu, 3), LABEL_AUTOSTART);
        assert_eq!(state(menu, 4) & MF_SEPARATOR.0, MF_SEPARATOR.0);
        assert_eq!(label(menu, 5), LABEL_QUIT);
        destroy(menu);
    }

    #[test]
    fn the_autostart_entry_is_checked_only_when_it_is_on() {
        for (on, expected) in [(false, 0), (true, MF_CHECKED.0)] {
            let menu = build(MenuState {
                autostart: on,
                can_save_default: false,
            })
            .unwrap();
            assert_eq!(state(menu, 3) & MF_CHECKED.0, expected, "autostart = {on}");
            for other in [0, 2, 5] {
                assert_eq!(
                    state(menu, other) & MF_CHECKED.0,
                    0,
                    "entry {other} is never checked"
                );
            }
            destroy(menu);
        }
    }

    #[test]
    fn save_as_default_is_greyed_out_unless_there_is_something_to_save() {
        for (can, grayed) in [(false, MF_GRAYED.0), (true, 0)] {
            let menu = build(MenuState {
                autostart: false,
                can_save_default: can,
            })
            .unwrap();
            assert_eq!(
                state(menu, 0) & MF_GRAYED.0,
                grayed,
                "can_save_default = {can}"
            );
            // The other entries are always usable.
            for other in [2, 3, 5] {
                assert_eq!(state(menu, other) & MF_GRAYED.0, 0, "entry {other}");
            }
            destroy(menu);
        }
    }

    #[test]
    fn command_ids_round_trip() {
        for cmd in [
            Command::SaveDefault,
            Command::Settings,
            Command::ToggleAutostart,
            Command::Quit,
        ] {
            assert_eq!(Command::from_id(cmd.id()), Some(cmd));
        }
        // 0 is what the menu returns when it is dismissed without a choice.
        assert_eq!(Command::from_id(0), None);
        assert_eq!(Command::from_id(5), None);
    }
}
