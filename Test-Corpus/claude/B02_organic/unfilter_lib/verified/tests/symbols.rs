//! Phase A / Phase D — exported-symbol parity between the C `.so` and the Rust
//! `.so`, and byte-identical contents of the 7 exported data objects.

mod common;

use common::libs;
use std::process::Command;

/// Every symbol `nm -D --defined-only` reports for the C library must also be
/// exported by the Rust library, with the exact same name.
#[test]
fn c_and_rust_export_the_same_symbols() {
    let (c, r) = libs();
    let cs = defined_dynamic_symbols(&c.path.display().to_string());
    let rs = defined_dynamic_symbols(&r.path.display().to_string());
    assert!(!cs.is_empty(), "nm produced no symbols for the C library");

    let missing: Vec<_> = cs.iter().filter(|s| !rs.contains(*s)).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Also assert the exact expected set, so a future regression that drops one
    // is caught even if `nm` is unavailable.
    let expected = [
        "cp_dist_base",
        "cp_dist_extra_bits",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_inflate",
        "cp_len_base",
        "cp_len_extra_bits",
        "cp_permutation_order",
        "unfilter",
    ];
    let mut got: Vec<String> = cs.iter().cloned().collect();
    got.sort();
    assert_eq!(got, expected, "unexpected C symbol set");
    for e in expected {
        assert!(rs.contains(&e.to_string()), "Rust .so lacks {e}");
    }
}

/// `nm -D --defined-only` sizes of the *data* objects must match exactly.
#[test]
fn exported_object_sizes_match() {
    let (c, r) = libs();
    let cs = symbol_sizes(&c.path.display().to_string());
    let rs = symbol_sizes(&r.path.display().to_string());
    for name in [
        "cp_error_reason",
        "cp_fixed_table",
        "cp_permutation_order",
        "cp_len_extra_bits",
        "cp_len_base",
        "cp_dist_extra_bits",
        "cp_dist_base",
    ] {
        let a = cs.iter().find(|(n, _)| n == name).map(|(_, s)| *s);
        let b = rs.iter().find(|(n, _)| n == name).map(|(_, s)| *s);
        assert_eq!(a, b, "size mismatch for {name}: C={a:?} Rust={b:?}");
        assert!(a.is_some(), "{name} not found");
    }
}

/// I27 — the initial contents of all 7 exported data objects, read through
/// `dlsym`, are byte-for-byte identical.
#[test]
fn exported_globals_have_identical_contents() {
    let (c, r) = libs();
    unsafe {
        assert!(
            (*c.cp_error_reason).is_null(),
            "C cp_error_reason should start NULL"
        );
        assert!(
            (*r.cp_error_reason).is_null(),
            "Rust cp_error_reason should start NULL"
        );

        cmp_u8("cp_fixed_table", c.cp_fixed_table, r.cp_fixed_table, 288 + 32);
        cmp_u8(
            "cp_permutation_order",
            c.cp_permutation_order,
            r.cp_permutation_order,
            19,
        );
        cmp_u8(
            "cp_len_extra_bits",
            c.cp_len_extra_bits,
            r.cp_len_extra_bits,
            31,
        );
        cmp_u8(
            "cp_dist_extra_bits",
            c.cp_dist_extra_bits,
            r.cp_dist_extra_bits,
            32,
        );
        cmp_u32("cp_len_base", c.cp_len_base, r.cp_len_base, 31);
        cmp_u32("cp_dist_base", c.cp_dist_base, r.cp_dist_base, 32);
    }
}

/// E19 — `cp_chunk` / `cp_find` (and every other `static` helper) must not be
/// exported by either library.
#[test]
fn no_png_chunk_symbols_exported() {
    let (c, r) = libs();
    for lib in [c, r] {
        let syms = defined_dynamic_symbols(&lib.path.display().to_string());
        for hidden in [
            "cp_chunk",
            "cp_find",
            "cp_make32",
            "cp_paeth",
            "cp_build",
            "cp_decode",
            "cp_block",
            "cp_stored",
            "cp_fixed",
            "cp_dynamic",
            "cp_read_bits",
            "cp_peak_bits",
            "cp_consume_bits",
            "cp_would_overflow",
            "cp_ptr",
            "cp_rev16",
            "cp_make_pixel",
            "cp_make_pixel_a",
        ] {
            assert!(
                !syms.contains(&hidden.to_string()),
                "{} unexpectedly exports {hidden}",
                lib.name
            );
        }
    }
}

// ---------------------------------------------------------------------------

unsafe fn cmp_u8(name: &str, a: *const u8, b: *const u8, n: usize) {
    let sa = std::slice::from_raw_parts(a, n);
    let sb = std::slice::from_raw_parts(b, n);
    assert_eq!(sa, sb, "{name} contents differ");
}

unsafe fn cmp_u32(name: &str, a: *const u32, b: *const u32, n: usize) {
    let sa = std::slice::from_raw_parts(a, n);
    let sb = std::slice::from_raw_parts(b, n);
    assert_eq!(sa, sb, "{name} contents differ");
}

fn nm(path: &str, extra: &[&str]) -> String {
    let mut cmd = Command::new("nm");
    cmd.arg("-D").arg("--defined-only");
    for e in extra {
        cmd.arg(e);
    }
    let out = cmd.arg(path).output().expect("run nm");
    assert!(out.status.success(), "nm failed on {path}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn defined_dynamic_symbols(path: &str) -> Vec<String> {
    nm(path, &[])
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.starts_with('_'))
        .collect()
}

/// `(name, size)` from `nm -D -S`
fn symbol_sizes(path: &str) -> Vec<(String, u64)> {
    nm(path, &["-S"])
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() == 4 {
                u64::from_str_radix(f[1], 16)
                    .ok()
                    .map(|sz| (f[3].to_string(), sz))
            } else {
                None
            }
        })
        .collect()
}
