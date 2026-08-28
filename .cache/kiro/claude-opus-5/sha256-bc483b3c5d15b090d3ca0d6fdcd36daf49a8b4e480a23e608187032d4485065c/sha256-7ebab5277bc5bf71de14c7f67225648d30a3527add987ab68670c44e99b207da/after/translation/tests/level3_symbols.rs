//! Every symbol the C `.so` exports must also be exported, under the exact same
//! name, by the Rust `.so` — including anything produced by preprocessor
//! macros. Verified two ways: by asking the dynamic loader for each name, and by
//! diffing `nm -D --defined-only` output.

mod common;

use common::*;
use std::ffi::c_void;
use std::process::Command;

/// The complete set of function symbols defined by `c_src/src/lib.c`.
const EXPECTED: &[&str] = &[
    "safe_double_to_int",
    "process_with_fallthrough",
    "copy_data_block",
    "handle_pointer_operations",
    "overunder",
];

fn every_expected_symbol_resolves_in_both_libraries() {
    let im = impls();
    for name in EXPECTED {
        let _c = im.c_sym::<unsafe extern "C" fn()>(name);
        let _r = im.rust_sym::<unsafe extern "C" fn()>(name);
    }
}

fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

fn rust_so_exports_every_c_so_symbol() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let rust_syms = defined_dynamic_symbols(&rust_so_path());

    // Sanity: the C library really did export the functions we expect, so an
    // empty/None nm parse cannot make this test vacuously pass.
    for name in EXPECTED {
        assert!(
            c_syms.iter().any(|s| s == name),
            "C .so unexpectedly does not export `{name}`; got {c_syms:?}"
        );
    }

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !rust_syms.iter().any(|r| r == *s))
        .collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\nC: {c_syms:?}\nRust: {rust_syms:?}"
    );
}

/// The exported symbols must be genuinely callable through the loader with the
/// C ABI, not merely present in the symbol table.
fn exported_symbols_are_callable_with_c_abi() {
    let im = impls();

    let (c, r) = (
        im.c_sym::<FnSafeDoubleToInt>("safe_double_to_int"),
        im.rust_sym::<FnSafeDoubleToInt>("safe_double_to_int"),
    );
    assert_eq!(unsafe { c(3.9) }, unsafe { r(3.9) });

    let (c, r) = (
        im.c_sym::<FnProcessWithFallthrough>("process_with_fallthrough"),
        im.rust_sym::<FnProcessWithFallthrough>("process_with_fallthrough"),
    );
    assert_eq!(unsafe { c(5, 1) }, unsafe { r(5, 1) });

    let (c, r) = (
        im.c_sym::<FnHandlePointerOperations>("handle_pointer_operations"),
        im.rust_sym::<FnHandlePointerOperations>("handle_pointer_operations"),
    );
    assert_eq!(unsafe { c(21) }, unsafe { r(21) });

    let (c, r) = (
        im.c_sym::<FnCopyDataBlock>("copy_data_block"),
        im.rust_sym::<FnCopyDataBlock>("copy_data_block"),
    );
    let src = make_block_bytes(9, 1.25, b"Source", 0);
    let mut cd = [0xAAu8; BLOCK_SCRATCH];
    let mut rd = [0xAAu8; BLOCK_SCRATCH];
    unsafe {
        c(cd.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void);
        r(rd.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void);
    }
    assert_eq!(cd, rd);

    let (c, r) = (
        im.c_sym::<FnOverunder>("overunder"),
        im.rust_sym::<FnOverunder>("overunder"),
    );
    let (cv, _) = capture_stdout(|| unsafe { c(1, 2, 3, 4) });
    let (rv, _) = capture_stdout(|| unsafe { r(1, 2, 3, 4) });
    assert_eq!(cv, rv);
}

/// Single entry point: see `capture_stdout` for why each test binary must
/// contain exactly one `#[test]`.
#[test]
fn symbol_exports_match_c() {
    every_expected_symbol_resolves_in_both_libraries();
    rust_so_exports_every_c_so_symbol();
    exported_symbols_are_callable_with_c_abi();
}
