//! Phase D — symbol parity between the C and Rust shared objects.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Defined dynamic symbol names, from `nm -D --defined-only`.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let a = it.next();
        let b = it.next();
        let c = it.next();
        // "<addr> <type> <name>" or "         <type> <name>" for undefined
        let name = match (a, b, c) {
            (Some(_), Some(_), Some(n)) => n,
            _ => continue,
        };
        set.insert(name.to_string());
    }
    set
}

/// Undefined (imported) dynamic symbols.
fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| s != "U" && s != "w")
        .collect()
}

#[test]
fn c_and_rust_export_the_same_symbols() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c:?}\n\
         RUST({}): {r:?}",
        c.len(),
        r.len()
    );

    // sanity: the nine documented symbols really are there
    for name in [
        "convert_pix",
        "cp_inflate",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_permutation_order",
        "cp_len_extra_bits",
        "cp_len_base",
        "cp_dist_extra_bits",
        "cp_dist_base",
    ] {
        assert!(c.contains(name), "C .so does not export {name}");
        assert!(r.contains(name), "Rust .so does not export {name}");
    }
}

#[test]
fn statics_stay_private_in_both() {
    // These are `static` in the C translation unit; the Rust .so must not leak
    // them either, or an external consumer would see a different ABI.
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());
    for name in [
        "cp_build",
        "cp_stored",
        "cp_fixed",
        "cp_dynamic",
        "cp_block",
        "cp_decode",
        "cp_read_bits",
        "cp_peak_bits",
        "cp_consume_bits",
        "cp_would_overflow",
        "cp_ptr",
        "cp_rev16",
        "cp_paeth",
        "cp_make32",
        "cp_chunk",
        "cp_find",
        "cp_unfilter",
        "cp_make_pixel",
        "cp_make_pixel_a",
    ] {
        assert!(!c.contains(name), "C .so unexpectedly exports {name}");
        assert!(!r.contains(name), "Rust .so exports the private helper {name}");
    }
}

#[test]
fn rust_imports_only_platform_symbols() {
    let u = undefined_symbols(&rust_so_path());
    // Anything the Rust .so imports must be resolvable from libc/libgcc/ld —
    // there must be no leftover reference to a symbol the translation forgot to
    // provide. Loading the library already proves resolvability, so assert that
    // and additionally that no `cp_*` / `convert_pix` symbol is imported.
    let _ = pair(); // dlopen succeeded => every import resolved
    for s in &u {
        assert!(
            !s.starts_with("cp_") && s != "convert_pix",
            "Rust .so imports library symbol {s} instead of defining it"
        );
    }
}
