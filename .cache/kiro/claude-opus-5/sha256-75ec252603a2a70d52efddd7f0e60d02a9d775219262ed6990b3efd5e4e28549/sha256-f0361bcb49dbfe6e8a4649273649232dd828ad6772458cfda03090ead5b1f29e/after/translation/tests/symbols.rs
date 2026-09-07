//! Phase A / D: exported-symbol parity and exported-table content parity.
//! Everything is read through `dlopen`/`dlsym`, not from the Rust crate.

mod common;

use common::*;
use std::process::Command;

const REQUIRED: &[&str] = &[
    "cp_inflate",
    "load_png_mem",
    "cp_fixed_table",
    "cp_permutation_order",
    "cp_len_extra_bits",
    "cp_len_base",
    "cp_dist_extra_bits",
    "cp_dist_base",
    "cp_error_reason",
];

const MUST_NOT_EXPORT: &[&str] = &[
    "cp_make_pixel_a",
    "cp_make_pixel",
    "cp_would_overflow",
    "cp_ptr",
    "cp_peak_bits",
    "cp_consume_bits",
    "cp_read_bits",
    "cp_rev16",
    "cp_build",
    "cp_stored",
    "cp_fixed",
    "cp_decode",
    "cp_dynamic",
    "cp_block",
    "cp_paeth",
    "cp_make32",
    "cp_chunk",
    "cp_find",
    "cp_unfilter",
    "cp_convert",
    "cp_get_alpha_for_indexed_image",
    "cp_depalette",
    "cp_get_chunk_byte_length",
    "cp_out_size",
];

fn dyn_syms(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

#[test]
fn symbol_diff_is_empty() {
    let c = dyn_syms(&c_so_path());
    let r = dyn_syms(&rust_so_path());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\nC={c:?}\nRust={r:?}"
    );
    for want in REQUIRED {
        assert!(c.contains(&want.to_string()), "C .so lacks {want}");
        assert!(r.contains(&want.to_string()), "Rust .so lacks {want}");
    }
    // Every C symbol accounted for by REQUIRED (no new C symbol slipped in).
    for s in &c {
        assert!(REQUIRED.contains(&s.as_str()), "unaccounted C symbol {s}");
    }
}

#[test]
fn private_helpers_are_not_exported() {
    let c = dyn_syms(&c_so_path());
    let r = dyn_syms(&rust_so_path());
    for s in MUST_NOT_EXPORT {
        assert!(!c.contains(&s.to_string()), "C unexpectedly exports {s}");
        assert!(!r.contains(&s.to_string()), "Rust unexpectedly exports {s}");
    }
}

/// "0 missing/undefined non-libc symbols": every undefined symbol in the Rust
/// `.so` must be resolvable from the system libraries it links against.
/// `ldd -r` performs exactly that relocation check.
#[test]
fn no_undefined_non_libc_symbols() {
    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd")
            .args(["-r", so.to_str().unwrap()])
            .output()
            .expect("ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(bad.is_empty(), "{so:?} has unresolved symbols: {bad:?}");
    }
}

#[test]
fn exported_table_contents_match() {
    let p = pair();
    unsafe {
        for (i, (name, cptr, len)) in p.c.tables.iter().enumerate() {
            let (rname, rptr, rlen) = p.rs.tables[i];
            assert_eq!(*name, rname);
            assert_eq!(*len, rlen);
            let a = std::slice::from_raw_parts(*cptr, *len);
            let b = std::slice::from_raw_parts(rptr, rlen);
            assert_eq!(a, b, "table {name} differs\nC   = {a:?}\nRust= {b:?}");
        }
    }
}

#[test]
fn error_reason_symbol_is_writable_and_readable() {
    // Goes through the locking helpers in `common`, since `cp_error_reason` is a
    // process-global that parallel test threads would otherwise stomp on.
    let junk = [0u8; 32];
    let r = c_png(&junk, 32);
    assert!(r.null);
    assert_eq!(
        r.error.as_deref(),
        Some("incorrect file signature (is this a png file?)")
    );
    // and both libraries agree on the message
    diff_png("error_reason readable", &junk);
}
