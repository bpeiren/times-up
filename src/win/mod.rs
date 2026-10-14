//! Everything that talks to Windows. The pure logic lives outside this module.

pub mod app;
pub mod autostart;
pub mod menu;
pub mod monitors;
pub mod surface;
pub mod text;
pub mod tray;
pub mod window;

/// `s` as a null-terminated UTF-16 string, as Windows APIs want it.
pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
