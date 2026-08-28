//! Step 8: every dynamic symbol the C `.so` exports must also be exported by
//! the Rust `.so` under the exact same name. Driven by `nm -D`.
mod common;

use common::{both, find_c_so, find_rust_so};
use std::path::PathBuf;
use std::process::Command;

fn c_so() -> PathBuf {
    find_c_so()
}

fn rust_so() -> PathBuf {
    find_rust_so()
}

/// Defined (not undefined) dynamic symbols, excluding compiler/linker-generated
/// housekeeping entries that are not part of the library's API.
fn exported_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ignored = [
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__libc_csu_init",
        "__libc_csu_fini",
        "_IO_stdin_used",
        "__odr_asan_gen_",
    ];
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .filter(|s| !ignored.contains(&s.as_str()))
        .filter(|s| !s.starts_with("__rust") && !s.starts_with("rust_"))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());
    assert!(!c.is_empty(), "nm reported no symbols for the C library");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}\nC: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn expected_public_api_is_present_in_both() {
    // The full list from src/lib.c, including the helpers that are not in
    // include/lib.h but still have external linkage in the C object.
    const EXPECTED: [&str; 12] = [
        "add_operation",
        "allocate_results",
        "divide_operation",
        "get_computation_timestamp",
        "get_operation_priority",
        "is_valid_operation",
        "mathop",
        "modulo_operation",
        "multiply_operation",
        "perform_computation_with_history",
        "select_operation",
        "subtract_operation",
    ];
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());
    for s in EXPECTED {
        assert!(c.contains(&s.to_string()), "C .so lacks {s}");
        assert!(r.contains(&s.to_string()), "Rust .so lacks {s}");
    }
}

#[test]
fn every_symbol_is_dlsym_resolvable_on_both() {
    // `both()` performs a dlsym for each of the 12 symbols on each library and
    // panics if any lookup fails, so simply loading is the assertion.
    let b = both();
    assert_ne!(b.c.mathop as usize, 0);
    assert_ne!(b.rust.mathop as usize, 0);
    // The two libraries must be genuinely distinct objects.
    assert_ne!(
        b.c.mathop as usize, b.rust.mathop as usize,
        "both handles resolved to the same library"
    );
}
