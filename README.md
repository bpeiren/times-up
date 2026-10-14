# Times Up

A small always-on-top class-hour timer for Windows: a bar that fills green, then orange, then red as the hour ends.
Written in Rust against the Win32 API (`windows` crate). Portable: `times-up.exe` plus `times-up.toml` in one folder.

## Development setup (Windows 10/11 x64)

```powershell
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
rustup default stable-x86_64-pc-windows-msvc
rustup component add clippy rustfmt rust-analyzer
```

Open a new terminal afterwards so `~/.cargo/bin` is on `PATH`.

## Build

```powershell
cargo run              # debug
cargo build --release  # size-optimised exe in target\release\times-up.exe
cargo test
cargo clippy
```
