// A GUI app: no console window in release builds (debug builds keep one for eprintln!).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::process::exit(times_up::win::app::run());
}
