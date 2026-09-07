//! Phase D — symbol-parity gate, executed as a test so it runs under every
//! feature combination.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn phase_d_symbol_parity() {
    let root = workspace_root();

    let c_so = std::fs::read_dir(root.join("c_src/build"))
        .expect("c_src/build exists — build the C library first")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .expect("a .so under c_src/build");

    let rust_so = ["release", "debug"]
        .iter()
        .map(|p| {
            root.join("translation/target")
                .join(p)
                .join("libupdate_frame_header_lib.so")
        })
        .find(|p| p.exists())
        .expect("Rust cdylib built (cargo build --release)");

    let c_syms = defined_symbols(&c_so);
    let rust_syms = defined_symbols(&rust_so);

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C symbols:    {c_syms:?}\n\
         Rust symbols: {rust_syms:?}",
        missing.len()
    );

    // Sanity: the one documented public entry point is really there.
    assert!(
        c_syms.contains("update_frame_header"),
        "C .so unexpectedly lacks update_frame_header; got {c_syms:?}"
    );
    assert!(rust_syms.contains("update_frame_header"));
}

/// Both `.so`s must resolve `update_frame_header` through `dlsym`, and calling
/// it through that handle must work (exercises the `#[no_mangle]` wrapper).
#[test]
fn phase_d_dlsym_and_call_both() {
    let libs = Libs::load();
    let inp = Input::default();
    let mut c = inp.to_raw();
    let mut r = inp.to_raw();
    libs.call_c(&mut c);
    libs.call_rust(&mut r);
    assert_eq!(c, r, "default input diverged\nC={c:?}\nR={r:?}");
    // 4096 -> 0x0C, 44100 -> 0x09, INDEPENDENT with 2 channels -> 0x01, 16 -> 0x04
    let expected = (0xFFF8u32 << 16) | (0x0Cu32 << 12) | (0x09u32 << 8) | (0x01u32 << 4) | (0x04u32 << 1);
    assert_eq!(
        c.frame_header(),
        expected,
        "C ground truth for the default input changed"
    );
}
