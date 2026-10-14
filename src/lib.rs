//! Times Up core logic. Everything here is free of Win32 so it can be unit tested.

pub mod canvas;
pub mod config;
pub mod schedule;
#[cfg(windows)]
pub mod win;
