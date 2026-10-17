//! Time's up. `schedule`, `config` and `canvas` are plain Rust without Win32 so they are easy to
//! unit test; everything that talks to Windows lives in `win`. (`win` is not behind
//! `#[cfg(windows)]` because this is a Windows-only app and Groma's scanner skips cfg-gated modules.)

pub mod bar;
pub mod canvas;
pub mod config;
pub mod editing;
pub mod icon;
pub mod interaction;
pub mod schedule;
pub mod store;
pub mod win;
