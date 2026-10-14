//! "Start with Windows": one value in the per-user `Run` key that launches this exe at sign-in.
//! It needs no administrator rights and is the only thing the app ever writes outside its folder.
//!
//! The app is portable, so the entry only counts as enabled when it points at *this* copy of the
//! exe; a copy that was moved or renamed shows the option as off.

use std::path::{Path, PathBuf};

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    REG_VALUE_TYPE, RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows::core::PCWSTR;

use super::wide;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "TimesUp";
const ERROR_FILE_NOT_FOUND: u32 = 2;

pub struct Autostart {
    root: HKEY,
    subkey: String,
    name: String,
    exe: PathBuf,
}

/// An open registry key, closed when dropped.
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by us and is closed exactly once.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

impl Autostart {
    /// The real thing: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `TimesUp`.
    pub fn current_user(exe: impl Into<PathBuf>) -> Self {
        Self::with_key(HKEY_CURRENT_USER, RUN_KEY, VALUE_NAME, exe)
    }

    /// Any key and value name (the tests use a throwaway key instead of the real `Run` key).
    pub fn with_key(root: HKEY, subkey: &str, name: &str, exe: impl Into<PathBuf>) -> Self {
        Self {
            root,
            subkey: subkey.to_string(),
            name: name.to_string(),
            exe: exe.into(),
        }
    }

    pub fn exe(&self) -> &Path {
        &self.exe
    }

    /// The command line Windows runs at sign-in: the quoted path of the exe.
    pub fn command(&self) -> String {
        format!("\"{}\"", self.exe.display())
    }

    fn open(&self, access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<Key> {
        let mut key = HKEY::default();
        let subkey = wide(&self.subkey);
        // SAFETY: valid null-terminated string and out pointer.
        let status =
            unsafe { RegOpenKeyExW(self.root, PCWSTR(subkey.as_ptr()), None, access, &mut key) };
        status.is_ok().then_some(Key(key))
    }

    /// Whether the entry exists and starts this exe.
    pub fn is_enabled(&self) -> bool {
        let Some(key) = self.open(KEY_READ) else {
            return false;
        };
        let name = wide(&self.name);
        let mut kind = REG_VALUE_TYPE::default();
        let mut len = 0u32;
        // SAFETY: first call only asks for the size; second reads into a buffer of that size.
        unsafe {
            let size = RegQueryValueExW(
                key.0,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                None,
                Some(&mut len),
            );
            if size.is_err() || kind != REG_SZ || len == 0 || len > 32 * 1024 {
                return false;
            }
            let mut buf = vec![0u16; (len as usize).div_ceil(2)];
            let mut len = (buf.len() * 2) as u32;
            let read = RegQueryValueExW(
                key.0,
                PCWSTR(name.as_ptr()),
                None,
                None,
                Some(buf.as_mut_ptr().cast::<u8>()),
                Some(&mut len),
            );
            if read.is_err() {
                return false;
            }
            let text = String::from_utf16_lossy(&buf)
                .trim_end_matches('\0')
                .to_string();
            text.eq_ignore_ascii_case(&self.command())
        }
    }

    /// Add (or correct) the entry. On failure nothing is changed.
    pub fn enable(&self) -> Result<(), String> {
        let subkey = wide(&self.subkey);
        let name = wide(&self.name);
        let value = wide(&self.command());
        let mut key = HKEY::default();
        // SAFETY: valid strings and out pointers; the key is closed by `Key`.
        unsafe {
            RegCreateKeyExW(
                self.root,
                PCWSTR(subkey.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
            .ok()
            .map_err(|e| e.message())?;
            let key = Key(key);
            let bytes = std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * 2);
            RegSetValueExW(key.0, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes))
                .ok()
                .map_err(|e| e.message())
        }
    }

    /// Remove the entry; fine when it is not there.
    pub fn disable(&self) -> Result<(), String> {
        let Some(key) = self.open(KEY_SET_VALUE) else {
            return Ok(()); // no Run key, so nothing to remove
        };
        let name = wide(&self.name);
        // SAFETY: valid key and null-terminated name.
        let status = unsafe { RegDeleteValueW(key.0, PCWSTR(name.as_ptr())) };
        if status.is_ok() || status.0 == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(status.ok().map_err(|e| e.message()).unwrap_err())
        }
    }

    /// Turn it off if it is on, on if it is off; returns the new state.
    pub fn toggle(&self) -> Result<bool, String> {
        if self.is_enabled() {
            self.disable().map(|()| false)
        } else {
            self.enable().map(|()| true)
        }
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::Registry::RegDeleteKeyW;

    use super::*;

    /// A throwaway key under HKCU, removed afterwards.
    struct Scratch(String);

    impl Scratch {
        fn new(label: &str) -> Self {
            Self(format!(
                r"Software\TimesUpTest-{}-{label}",
                std::process::id()
            ))
        }

        fn autostart(&self, exe: &str) -> Autostart {
            Autostart::with_key(HKEY_CURRENT_USER, &self.0, "TimesUp", exe)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let sub = wide(&self.0);
            // SAFETY: deleting the key we created for the test.
            unsafe {
                let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(sub.as_ptr()));
            }
        }
    }

    const EXE: &str = r"C:\Tools\Times Up\times-up.exe";

    #[test]
    fn command_is_the_quoted_exe_path() {
        let s = Scratch::new("command");
        assert_eq!(
            s.autostart(EXE).command(),
            r#""C:\Tools\Times Up\times-up.exe""#
        );
        assert_eq!(s.autostart(EXE).exe(), Path::new(EXE));
    }

    #[test]
    fn disabled_when_nothing_is_there() {
        let s = Scratch::new("none");
        assert!(!s.autostart(EXE).is_enabled(), "no key at all");
    }

    #[test]
    fn enable_then_disable_round_trips() {
        let s = Scratch::new("roundtrip");
        let a = s.autostart(EXE);
        a.enable().unwrap();
        assert!(a.is_enabled());
        a.enable().unwrap(); // enabling twice is fine
        assert!(a.is_enabled());
        a.disable().unwrap();
        assert!(!a.is_enabled());
        a.disable().unwrap(); // removing twice is fine too
    }

    #[test]
    fn toggle_flips_and_reports_the_new_state() {
        let s = Scratch::new("toggle");
        let a = s.autostart(EXE);
        assert_eq!(a.toggle(), Ok(true));
        assert!(a.is_enabled());
        assert_eq!(a.toggle(), Ok(false));
        assert!(!a.is_enabled());
        assert_eq!(a.toggle(), Ok(true));
    }

    #[test]
    fn an_entry_for_another_copy_of_the_exe_does_not_count() {
        let s = Scratch::new("other");
        s.autostart(r"D:\Old\times-up.exe").enable().unwrap();
        let here = s.autostart(EXE);
        assert!(!here.is_enabled(), "points at a different exe");
        // Switching it on from this copy repoints the entry instead of adding a second one.
        assert_eq!(here.toggle(), Ok(true));
        assert!(here.is_enabled());
        assert!(!s.autostart(r"D:\Old\times-up.exe").is_enabled());
    }

    #[test]
    fn path_comparison_ignores_case() {
        let s = Scratch::new("case");
        s.autostart(r"C:\TOOLS\times-up.exe").enable().unwrap();
        assert!(s.autostart(r"c:\tools\TIMES-UP.EXE").is_enabled());
    }

    #[test]
    fn a_failing_registry_write_reports_an_error_and_changes_nothing() {
        // Registry key names are limited to 255 characters, so this key can never be created.
        let long = "x".repeat(300);
        let sub = format!(r"Software\{long}");
        let a = Autostart::with_key(HKEY_CURRENT_USER, &sub, "TimesUp", EXE);
        let err = a.enable().expect_err("must fail");
        assert!(!err.is_empty());
        assert!(!a.is_enabled());
        assert!(
            a.toggle().is_err(),
            "toggling reports the failure instead of pretending"
        );
        assert!(!a.is_enabled());
        // Cleaning up an entry that never existed is not an error.
        assert_eq!(a.disable(), Ok(()));
    }

    #[test]
    fn the_real_run_key_is_where_windows_looks_for_startup_programs() {
        let a = Autostart::current_user(EXE);
        assert_eq!(a.subkey, r"Software\Microsoft\Windows\CurrentVersion\Run");
        assert_eq!(a.name, "TimesUp");
        // Only read it: this test never writes to the real key.
        let _ = a.is_enabled();
    }
}
