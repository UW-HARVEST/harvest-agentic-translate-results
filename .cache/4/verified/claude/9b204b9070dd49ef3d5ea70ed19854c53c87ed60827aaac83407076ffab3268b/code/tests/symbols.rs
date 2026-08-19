// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Every symbol the C library exports must be exported by the Rust library under
// the exact same name, and must be resolvable with `dlsym` (which is what
// actually proves the `#[unsafe(no_mangle)] extern "C"` wrappers are correct).

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Parse `nm -D --defined-only` output into the set of exported global symbol
/// names, ignoring the CRT/toolchain bookkeeping symbols that are not API.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("run `nm -D --defined-only {so:?}`: {e}"));
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // "<addr> <type> <name>"  (addr may be blank for undefined, but we
        // passed --defined-only so every line here has one)
        let mut it = line.split_whitespace();
        let (Some(_addr), Some(kind), Some(name)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        // Only exported code/data: T = text, D/B/R = data, W/V/i = weak/indirect.
        if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i") {
            continue;
        }
        if is_toolchain_symbol(name) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

fn is_toolchain_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "_edata",
        "_end",
        "__bss_start",
        "__bss_start__",
        "_bss_end__",
        "__bss_end__",
        "__end__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__register_frame_info",
        "__deregister_frame_info",
        "_TLS_MODULE_BASE_",
    ];
    EXACT.contains(&name)
        // Rust std/panic-runtime machinery that the cdylib re-exports.
        || name.starts_with("rust_")
        || name.starts_with("_R")            // v0 mangled Rust symbols
        || name.starts_with("_ZN")           // legacy mangled Rust/C++ symbols
        || name.starts_with("__rust")
        || name.starts_with("__rdl_")
        || name.starts_with("__rg_")
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let (c_so, rust_so) = so_paths();
    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    assert!(
        !c_syms.is_empty(),
        "no exported symbols parsed from the C library {c_so:?} — the test would be vacuous"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c_syms,
        rust_syms
    );
}

/// The C library's complete public surface, transcribed from
/// `nm -D --defined-only c_src/build/libdriver.so`.  Pinned so that a future
/// change to the C side which adds a symbol cannot silently go untested.
const EXPECTED_C_SURFACE: &[&str] = &["driver", "printLine"];

#[test]
fn phase_d_c_surface_matches_symbols_md() {
    let (c_so, _) = so_paths();
    let c_syms = exported_symbols(&c_so);
    let expected: std::collections::BTreeSet<String> =
        EXPECTED_C_SURFACE.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "the C library's exported surface changed; SYMBOLS.md, CONFIGS.md and \
         ERRORS.md must be re-derived and new tests written"
    );
}

#[test]
fn phase_d_every_c_symbol_is_dlsym_resolvable_in_rust() {
    // `harness()` resolves both `driver` and `printLine` out of BOTH libraries
    // with `dlsym` and panics if either lookup fails, so simply constructing it
    // proves the exports are real and correctly named.
    let h = harness();
    assert_eq!(h.c.name, "C");
    assert_eq!(h.rust.name, "Rust");

    // Also assert the resolved addresses really come from two distinct objects,
    // i.e. we are not accidentally comparing a library against itself.
    assert_ne!(
        h.c.driver as usize, h.rust.driver as usize,
        "C and Rust `driver` resolved to the same address — the two libraries \
         are not actually independent, so every differential test is vacuous"
    );
    assert_ne!(
        h.c.print_line as usize, h.rust.print_line as usize,
        "C and Rust `printLine` resolved to the same address"
    );
}

/// The Rust `.so` must not have picked up an undefined dependency on anything
/// outside libc / the Rust runtime.
#[test]
fn phase_d_rust_so_has_no_unexpected_undefined_symbols() {
    let (_, rust_so) = so_paths();
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&rust_so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm --undefined-only failed");
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|name| {
            // libc imports carry a @GLIBC_x.y version tag; weak CRT hooks and
            // Rust runtime symbols are fine too.
            !name.contains("@GLIBC")
                && !name.contains("@GCC")
                && !is_toolchain_symbol(name)
                && !name.starts_with("_ITM_")
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unexpected undefined (unresolved non-libc) symbols: {bad:?}"
    );
}
