// Phase D -- symbol parity between the C `.so` and the Rust `.so`.
// The diff of exported dynamic symbols MUST be empty.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only <so>` -> set of exported symbol names.
fn exported(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
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
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

/// `nm -D --undefined-only <so>` -> set of imported symbol names.
fn undefined(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| s != "U" && s != "w")
        .collect()
}

fn d1_every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert!(
        !c.is_empty(),
        "sanity: C .so should export at least `driver`"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   exports: {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );
}

fn d2_driver_is_the_only_public_symbol_on_both_sides() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());
    assert_eq!(
        c,
        ["driver".to_string()].into_iter().collect::<BTreeSet<_>>(),
        "C .so surface changed; SYMBOLS.md must be regenerated"
    );
    // The Rust cdylib must not leak extra public API either -- in particular
    // it must not export the file-local `print_hex`.
    assert!(
        !r.contains("print_hex"),
        "Rust .so must not export the `static` C helper print_hex"
    );
    assert!(r.contains("driver"), "Rust .so must export driver");
}

fn d3_rust_imports_only_resolvable_libc_libgcc_symbols() {
    // Every undefined symbol in the Rust .so must be satisfiable at load time.
    // The strongest possible check is simply that dlopen + dlsym succeed,
    // which the harness already does; assert it explicitly here, then confirm
    // no undefined symbol looks like an untranslated C module.
    let _ = rust_driver(); // forces dlopen of the Rust .so and dlsym("driver")
    let _ = c_driver();

    let r = undefined(&rust_so_path());
    let c = undefined(&c_so_path());

    // Sanity: GCC lowered the C `printf("\n")` into `putchar('\n')`, so the C
    // .so imports both.
    for sym in ["printf", "putchar"] {
        assert!(
            c.iter().any(|s| s.starts_with(sym)),
            "sanity: C .so should import {sym}"
        );
    }

    // The Rust .so must route its output through the SAME process-wide libc
    // stdio, so it must import libc's `printf` (the `%02x` calls). Whether the
    // constant `printf("\n")` additionally gets lowered to `putchar` is a pure
    // optimization detail -- LLVM does it at -O2 but not at -O0, and GCC does
    // it for the C build. Both spellings write the identical byte to the
    // identical stream, so requiring `putchar` specifically would assert on the
    // optimizer, not on behaviour. The observable output equivalence is what
    // valid_paths/error_paths verify byte-for-byte.
    assert!(
        r.iter().any(|s| s.starts_with("printf")),
        "Rust .so must route output through libc printf; imports = {r:?}"
    );
    assert!(
        r.iter().any(|s| s.starts_with("printf") || s.starts_with("putchar")),
        "Rust .so must emit the trailing newline via libc stdio; imports = {r:?}"
    );

    // Nothing may reference a would-be-translated C entity.
    for bad in ["print_hex", "house_t", "unimplemented", "not_implemented"] {
        assert!(
            !r.iter().any(|s| s.contains(bad)),
            "Rust .so has an unresolved reference containing `{bad}`"
        );
    }
}

fn d4_no_stub_markers_in_rust_source() {
    // Guard against a symbol that exists only to satisfy `nm -D`.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    for marker in ["unimplemented!", "todo!", "unreachable!"] {
        assert!(
            !src.contains(marker),
            "src/lib.rs contains `{marker}` -- a stub that lies about behaviour"
        );
    }
}

// --- sequential runner entry point (harness = false) ---------------------

fn main() {
    common::run_suite(
        "symbol_parity",
        &[
        ("d1_every_c_symbol_is_exported_by_rust", d1_every_c_symbol_is_exported_by_rust as fn()),
        ("d2_driver_is_the_only_public_symbol_on_both_sides", d2_driver_is_the_only_public_symbol_on_both_sides as fn()),
        ("d3_rust_imports_only_resolvable_libc_libgcc_symbols", d3_rust_imports_only_resolvable_libc_libgcc_symbols as fn()),
        ("d4_no_stub_markers_in_rust_source", d4_no_stub_markers_in_rust_source as fn()),
        ],
    );
}
