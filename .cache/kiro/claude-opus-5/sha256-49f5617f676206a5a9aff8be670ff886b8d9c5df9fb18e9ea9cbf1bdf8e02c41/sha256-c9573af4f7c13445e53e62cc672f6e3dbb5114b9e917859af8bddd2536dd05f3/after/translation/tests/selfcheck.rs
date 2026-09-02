//! Harness self-check: proves the differential tests are meaningful.
//!
//! Without these, a bug in the harness (e.g. loading the same `.so` twice, or
//! comparing nothing) would make every other test pass vacuously.

mod common;

use common::*;
use std::ffi::c_char;
use std::ffi::c_int;

#[test]
fn selfcheck_two_distinct_libraries_are_loaded() {
    let l = libs();
    let c = l.c as usize;
    let r = l.rs as usize;
    assert_ne!(
        c, r,
        "C and Rust hex2bin resolved to the SAME address — the harness is \
         loading one library twice and every differential test would pass vacuously"
    );
}

#[test]
fn selfcheck_both_libraries_decode_a_known_vector() {
    // Independently computed expectation, so a *shared* bug in both libraries
    // (or a harness that returns canned data) is still caught.
    let l = libs();
    let hex = b"0A1b2C3d4E5f";
    let expect: [u8; 6] = [0x0a, 0x1b, 0x2c, 0x3d, 0x4e, 0x5f];

    for (name, f) in [("C", l.c), ("Rust", l.rs)] {
        let mut bin = [0u8; 6];
        let mut end: *const c_char = std::ptr::null();
        let ret = unsafe {
            f(
                bin.as_mut_ptr(),
                bin.len(),
                hex.as_ptr() as *const c_char,
                hex.len(),
                std::ptr::null(),
                &mut end,
            )
        };
        assert_eq!(ret, 6, "{name}: return value");
        assert_eq!(bin, expect, "{name}: decoded bytes");
        assert_eq!(
            (end as usize) - (hex.as_ptr() as usize),
            hex.len(),
            "{name}: hex_end offset"
        );
    }
}

/// The comparison machinery must actually report a difference. Feed
/// `assert_same`-style comparison a deliberately wrong second implementation
/// and confirm the `Outcome` values differ.
#[test]
fn selfcheck_outcome_comparison_detects_a_difference() {
    let l = libs();
    let hex = b"ff00";

    let call_ok = Call {
        bin: BinArg::Buf(10),
        bin_maxlen: 2,
        hex: HexArg::Bytes(hex),
        hex_len: 4,
        ignore: None,
        want_hex_end: true,
    };
    // Identical config => must match.
    assert_same("selfcheck/identical", &call_ok);

    // A *different* config must produce a different observable outcome, which
    // proves `Outcome` captures ret, bin and hex_end (not a constant).
    let mut a_bin = [SENTINEL; 10];
    let mut b_bin = [SENTINEL; 10];
    let mut a_end: *const c_char = std::ptr::null();
    let mut b_end: *const c_char = std::ptr::null();
    let ra: c_int = unsafe {
        (l.c)(
            a_bin.as_mut_ptr(),
            2,
            hex.as_ptr() as *const c_char,
            4,
            std::ptr::null(),
            &mut a_end,
        )
    };
    let rb: c_int = unsafe {
        (l.rs)(
            b_bin.as_mut_ptr(),
            1, // <-- different bin_maxlen
            hex.as_ptr() as *const c_char,
            4,
            std::ptr::null(),
            &mut b_end,
        )
    };
    assert_ne!(
        (ra, a_bin, a_end as usize),
        (rb, b_bin, b_end as usize),
        "changing bin_maxlen produced an identical observation — the harness is \
         not observing the library's real output"
    );
}

/// The Rust `.so` under test must be the crate's cdylib, and it must be
/// distinct from — and no older than — the sources it was built from.
#[test]
fn selfcheck_loaded_paths_are_the_expected_two_files() {
    let c_so = find_c_so();
    let rs_so = find_rust_so();
    eprintln!("C   .so: {}", c_so.display());
    eprintln!("Rust.so: {}", rs_so.display());
    assert!(c_so.is_file());
    assert!(rs_so.is_file());
    assert_ne!(c_so, rs_so, "the same file is being loaded twice");
    assert!(
        rs_so.file_name().unwrap().to_str().unwrap().contains("hex2bin_lib"),
        "the \"Rust\" library is not the crate cdylib: {}",
        rs_so.display()
    );
    // Force the harness to initialise (this re-runs the staleness gate).
    let _ = libs();
}
