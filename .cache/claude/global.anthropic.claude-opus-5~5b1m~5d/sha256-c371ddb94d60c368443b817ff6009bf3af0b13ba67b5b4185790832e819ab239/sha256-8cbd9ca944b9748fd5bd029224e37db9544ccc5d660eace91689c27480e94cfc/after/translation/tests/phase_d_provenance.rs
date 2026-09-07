//! Phase D — symbol provenance and export-surface checks.
//!
//! `wcscat` is also a **libc** symbol (with a completely different 2-argument
//! signature). If `dlsym` on either handle silently resolved to glibc's
//! `wcscat`, every differential assertion in phases B and C would be comparing
//! the wrong function. These tests pin the provenance down explicitly.

mod common;

use common::*;

/// The two resolved addresses must be distinct: if both handles collapsed onto
/// one definition (e.g. glibc's), the whole suite would be vacuous.
#[test]
fn d1_c_and_rust_symbols_are_distinct() {
    let l = libs();
    assert_ne!(
        l.c_wcscat as usize, l.rust_wcscat as usize,
        "C and Rust `wcscat` resolved to the same address"
    );
}

/// Neither resolved address may be glibc's `wcscat`. Proven behaviourally:
/// glibc's `wcscat(dst, src)` returns `dst` (a pointer, reinterpreted as a huge
/// / arbitrary `int`) and never inspects a `numElem` argument, so it cannot
/// return exactly 22 for `numElem == 0`. Both of our symbols must.
#[test]
fn d2_neither_symbol_is_libc_wcscat() {
    let l = libs();
    let src: [WcharT; 2] = [65, 0];
    let mut dst: [WcharT; 4] = [0, 0, 0, 0];

    for (name, f) in [("C", l.c_wcscat), ("Rust", l.rust_wcscat)] {
        let ret = unsafe { f(dst.as_mut_ptr(), 0, src.as_ptr()) };
        assert_eq!(
            ret, 22,
            "{name} `wcscat` returned {ret} for numElem == 0; glibc's 2-arg \
             wcscat would not return 22 -- the symbol was resolved from the wrong object"
        );
    }
}

/// Each resolved address must lie inside the memory mapping of the `.so` file it
/// was requested from, as reported by `/proc/self/maps`.
#[test]
fn d3_addresses_live_in_the_expected_objects() {
    let l = libs();
    let maps = std::fs::read_to_string("/proc/self/maps").expect("read /proc/self/maps");

    let owner_of = |addr: usize| -> Option<String> {
        for line in maps.lines() {
            let mut it = line.split_whitespace();
            let range = it.next()?;
            let (lo, hi) = range.split_once('-')?;
            let lo = usize::from_str_radix(lo, 16).ok()?;
            let hi = usize::from_str_radix(hi, 16).ok()?;
            if addr >= lo && addr < hi {
                return line.split_whitespace().nth(5).map(|s| s.to_string());
            }
        }
        None
    };

    let c_owner = owner_of(l.c_wcscat as usize).expect("C symbol not in any mapping");
    let r_owner = owner_of(l.rust_wcscat as usize).expect("Rust symbol not in any mapping");

    eprintln!("C   wcscat lives in: {c_owner}");
    eprintln!("Rust wcscat lives in: {r_owner}");

    assert!(
        !c_owner.contains("libc.so"),
        "C `wcscat` resolved into libc ({c_owner}), not the C library under test"
    );
    assert!(
        r_owner.contains("libwcscat_lib.so"),
        "Rust `wcscat` resolved into {r_owner}, expected libwcscat_lib.so"
    );
    assert_ne!(c_owner, r_owner, "both symbols came from the same object");
}

/// The Rust `.so` must export `wcscat` and the C `.so` must too -- verified by
/// the fact that `libs()` resolved both, but asserted here as a named check so
/// the Phase D gate has an explicit test.
#[test]
fn d4_both_objects_export_wcscat() {
    let l = libs();
    assert!(l.c_wcscat as usize != 0);
    assert!(l.rust_wcscat as usize != 0);
}
