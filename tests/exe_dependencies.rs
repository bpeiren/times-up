//! The app ships as one portable file, so the exe may only depend on DLLs that Windows itself
//! provides. In particular it must not need the Visual C++ Redistributable (`vcruntime140.dll`),
//! which Rust's MSVC target pulls in unless the C runtime is linked statically
//! (`.cargo/config.toml`).
//!
//! The test reads the PE import table of the exe built by `cargo test`. Set `TIMES_UP_EXE` to
//! check another file instead, e.g. the release build that is packaged.

use std::path::PathBuf;

fn u16_at(b: &[u8], o: usize) -> usize {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap()) as usize
}

fn u32_at(b: &[u8], o: usize) -> usize {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize
}

/// Machine type and the names of the DLLs a 64-bit PE file imports.
fn parse_pe(bytes: &[u8]) -> (usize, Vec<String>) {
    assert_eq!(&bytes[0..2], b"MZ", "not an executable");
    let pe = u32_at(bytes, 0x3c);
    assert_eq!(&bytes[pe..pe + 4], b"PE\0\0", "no PE header");
    let coff = pe + 4;
    let machine = u16_at(bytes, coff);
    let sections = u16_at(bytes, coff + 2);
    let optional_size = u16_at(bytes, coff + 16);
    let optional = coff + 20;
    assert_eq!(
        u16_at(bytes, optional),
        0x20b,
        "expected a PE32+ (64-bit) file"
    );
    // Data directory 1 is the import table; the directories start 112 bytes into a PE32+ header.
    let import_rva = u32_at(bytes, optional + 112 + 8);
    let section_table = optional + optional_size;

    let to_offset = |rva: usize| -> usize {
        for i in 0..sections {
            let s = section_table + i * 40;
            let (size, address, raw) = (
                u32_at(bytes, s + 8),
                u32_at(bytes, s + 12),
                u32_at(bytes, s + 20),
            );
            let raw_size = u32_at(bytes, s + 16);
            if rva >= address && rva < address + size.max(raw_size) {
                return rva - address + raw;
            }
        }
        panic!("RVA {rva:#x} is in no section");
    };

    let mut names = Vec::new();
    let mut descriptor = to_offset(import_rva);
    loop {
        let name_rva = u32_at(bytes, descriptor + 12);
        if name_rva == 0 {
            break;
        }
        let start = to_offset(name_rva);
        let end = start + bytes[start..].iter().position(|&c| c == 0).unwrap();
        names.push(String::from_utf8_lossy(&bytes[start..end]).to_string());
        descriptor += 20;
    }
    (machine, names)
}

fn exe_path() -> PathBuf {
    std::env::var_os("TIMES_UP_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_times-up")))
}

/// Libraries that are not part of Windows itself (or that need a redistributable).
fn is_redistributable(dll: &str) -> bool {
    let d = dll.to_ascii_lowercase();
    [
        "vcruntime",
        "msvcp",
        "msvcr",
        "concrt",
        "vcomp",
        "ucrtbase",
        "api-ms-win-crt-",
    ]
    .iter()
    .any(|prefix| d.starts_with(prefix))
}

/// Windows DLLs and API sets that exist on every Windows 10 and 11 installation.
fn is_part_of_windows(dll: &str) -> bool {
    let d = dll.to_ascii_lowercase();
    if d.starts_with("api-ms-win-core-") || d.starts_with("api-ms-win-shcore-") {
        return true; // API sets are resolved by the loader, there is no file for them
    }
    let system =
        std::env::var_os("SystemRoot").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    system.join("System32").join(&d).exists()
}

#[test]
fn the_exe_is_a_64_bit_windows_program_with_the_expected_imports() {
    let bytes = std::fs::read(exe_path()).expect("the exe exists");
    let (machine, imports) = parse_pe(&bytes);
    assert_eq!(machine, 0x8664, "x64");
    let lower: Vec<String> = imports.iter().map(|d| d.to_ascii_lowercase()).collect();
    // Sanity check that the parser really sees the import table of a windowing app.
    for expected in ["kernel32.dll", "user32.dll", "gdi32.dll"] {
        assert!(
            lower.contains(&expected.to_string()),
            "{expected} missing in {imports:?}"
        );
    }
}

#[test]
fn the_exe_needs_no_redistributable_runtime() {
    let bytes = std::fs::read(exe_path()).expect("the exe exists");
    let (_, imports) = parse_pe(&bytes);
    let bad: Vec<&String> = imports.iter().filter(|d| is_redistributable(d)).collect();
    assert!(
        bad.is_empty(),
        "the exe depends on the C runtime ({bad:?}); link it statically (see .cargo/config.toml)"
    );
}

#[test]
fn every_import_is_part_of_windows() {
    let bytes = std::fs::read(exe_path()).expect("the exe exists");
    let (_, imports) = parse_pe(&bytes);
    let foreign: Vec<&String> = imports.iter().filter(|d| !is_part_of_windows(d)).collect();
    assert!(
        foreign.is_empty(),
        "not Windows DLLs: {foreign:?} (all imports: {imports:?})"
    );
}

#[test]
fn the_dependency_rules_recognise_what_they_are_meant_to() {
    for dll in [
        "VCRUNTIME140.dll",
        "vcruntime140_1.dll",
        "MSVCP140.dll",
        "ucrtbase.dll",
        "api-ms-win-crt-heap-l1-1-0.dll",
    ] {
        assert!(is_redistributable(dll), "{dll}");
    }
    for dll in [
        "kernel32.dll",
        "USER32.dll",
        "gdi32.dll",
        "api-ms-win-core-synch-l1-2-0.dll",
    ] {
        assert!(!is_redistributable(dll), "{dll}");
        assert!(is_part_of_windows(dll), "{dll}");
    }
    assert!(!is_part_of_windows("definitely-not-a-windows-dll.dll"));
}
