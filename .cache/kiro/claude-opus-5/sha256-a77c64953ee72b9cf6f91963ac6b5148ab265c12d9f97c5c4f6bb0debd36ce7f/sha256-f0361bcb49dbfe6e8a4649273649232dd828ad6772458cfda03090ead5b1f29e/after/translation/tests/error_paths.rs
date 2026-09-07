//! Phase C — error/boundary-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library has no rejection path (void return, no pointers, no range
//! checks), so "same error" means: for each boundary/invalid-shaped input both
//! `.so`s must produce the SAME observable result — identical stdout bytes and
//! identical non-rejection (neither aborts, traps, or panics across the FFI
//! boundary). Any divergence, including a Rust-side overflow panic where C
//! wraps, fails these tests.

mod common;

use common::*;

/// Rows 1 & 2 — signed overflow in `2*x` at both ends of the domain.
fn err01_int_max() {
    assert_same("err01", i32::MAX);
    assert_eq!(c_driver(i32::MAX), b"298\n".to_vec());
    assert_eq!(rust_driver(i32::MAX), b"298\n".to_vec());
}

fn err02_int_min() {
    assert_same("err02", i32::MIN);
    assert_eq!(c_driver(i32::MIN), b"300\n".to_vec());
    assert_eq!(rust_driver(i32::MIN), b"300\n".to_vec());
}

/// Row 3 — smallest positive `x` where `2*x` overflows.
fn err03_first_mul_overflow() {
    let x = 1_073_741_824; // INT_MAX/2 + 1
    assert_same("err03", x);
    assert_eq!(c_driver(x), b"-2147483348\n".to_vec());
    assert_eq!(rust_driver(x), b"-2147483348\n".to_vec());
}

/// Row 4 — `2*x` in range but `y += 300` overflows.
fn err04_add_overflow_at_max_half() {
    let x = 1_073_741_823; // INT_MAX/2
    assert_same("err04", x);
    assert_eq!(c_driver(x), b"-2147483350\n".to_vec());
    assert_eq!(rust_driver(x), b"-2147483350\n".to_vec());
}

/// Row 5 — largest `x` with no overflow anywhere.
fn err05_last_non_overflowing() {
    let x = 1_073_741_673;
    assert_same("err05", x);
    assert_eq!(c_driver(x), b"2147483646\n".to_vec());
    assert_eq!(rust_driver(x), b"2147483646\n".to_vec());
}

/// Row 6 — one step past row 5.
fn err06_one_past_last_non_overflowing() {
    let x = 1_073_741_674;
    assert_same("err06", x);
    assert_eq!(c_driver(x), b"-2147483648\n".to_vec());
    assert_eq!(rust_driver(x), b"-2147483648\n".to_vec());
}

/// Row 7 — most negative `x` with `2*x == INT_MIN` (no wrap).
fn err07_min_half() {
    let x = -1_073_741_824; // INT_MIN/2
    assert_same("err07", x);
    assert_eq!(c_driver(x), b"-2147483348\n".to_vec());
    assert_eq!(rust_driver(x), b"-2147483348\n".to_vec());
}

/// Row 8 — one step past row 7: negative multiply overflow.
fn err08_one_past_min_half() {
    let x = -1_073_741_825;
    assert_same("err08", x);
    assert_eq!(c_driver(x), b"-2147483350\n".to_vec());
    assert_eq!(rust_driver(x), b"-2147483350\n".to_vec());
}

/// Row 9 — result exactly zero: `printf("%d")` zero formatting.
fn err09_zero_result_formatting() {
    let x = -150;
    assert_same("err09", x);
    let out = c_driver(x);
    assert_eq!(out, b"0\n".to_vec(), "no sign, no padding, no leading blank");
    assert_eq!(rust_driver(x), out);
}

/// Row 10 — smallest-magnitude negative result: minus-sign formatting.
fn err10_minus_sign_formatting() {
    let x = -151;
    assert_same("err10", x);
    assert_eq!(c_driver(x), b"-2\n".to_vec());
    assert_eq!(rust_driver(x), b"-2\n".to_vec());
}

/// Row 11 — out-of-range/garbage bit patterns crossing the FFI boundary.
///
/// A C `int` parameter accepts any 32-bit pattern (this is the C-enum
/// "no valid variant" case: there is no validation to fail), so both sides must
/// accept it and produce identical output rather than rejecting it.
fn err11_out_of_range_bit_patterns() {
    let pats: [u32; 8] = [
        0xFFFF_FFFF, // -1
        0x8000_0000, // INT_MIN
        0x7FFF_FFFF, // INT_MAX
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x0000_0000,
        0xFFFF_FFFE,
        0x8000_0001,
    ];
    for &p in &pats {
        let x = p as i32;
        assert_same("err11", x);
    }
    assert_eq!(c_driver(-1), b"298\n".to_vec());
    assert_eq!(rust_driver(-1), b"298\n".to_vec());
    assert_eq!(c_driver(0x8000_0000u32 as i32), b"300\n".to_vec());
    assert_eq!(rust_driver(0x8000_0000u32 as i32), b"300\n".to_vec());
}

/// Row 12 — 64-bit argument truncation across the `int` parameter.
///
/// Calls both `.so`s through a `extern "C" fn(i64)` view of the same symbol, so
/// the callee sees only the low 32 bits. C and Rust must truncate identically.
fn err12_wide_argument_truncation() {
    use std::ffi::c_int;
    use libloading::{Library, Symbol};

    type DriverWide = unsafe extern "C" fn(i64);

    let wide_inputs: [i64; 6] = [
        0x1_0000_0001,
        0x7FFF_FFFF_FFFF_FFFF,
        -1,
        0x1_0000_0000,
        0xABCD_EF01_2345_6789u64 as i64,
        0x0000_0002_0000_012Cu64 as i64,
    ];

    unsafe {
        let c_lib = Library::new(c_so_path()).expect("dlopen C .so");
        let r_lib = Library::new(rust_so_path()).expect("dlopen Rust .so");
        let c_wide: Symbol<DriverWide> = c_lib.get(b"driver\0").unwrap();
        let r_wide: Symbol<DriverWide> = r_lib.get(b"driver\0").unwrap();
        let c_narrow: Symbol<DriverFn> = c_lib.get(b"driver\0").unwrap();
        let _ = &c_narrow;

        for &w in &wide_inputs {
            // Both sides receive the identical wide value; only the low 32 bits
            // are part of the ABI-defined argument.
            let c_out = {
                let f = *c_wide;
                capture_pub(|| f(w))
            };
            let r_out = {
                let f = *r_wide;
                capture_pub(|| f(w))
            };
            assert_eq!(
                c_out,
                r_out,
                "err12 wide arg {w:#x} diverged:\n  C    = {:?}\n  Rust = {:?}",
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
            // And it matches the properly-typed call with the truncated value.
            let truncated = w as u32 as c_int;
            assert_eq!(
                c_out,
                c_driver(truncated),
                "err12 truncation of {w:#x} is not the low 32 bits"
            );
        }
    }

    assert_eq!(c_driver(1), b"302\n".to_vec());
    assert_eq!(rust_driver(1), b"302\n".to_vec());
}

/// Generic boundary sweep required by Phase C: every value one step past each
/// interesting boundary in both directions, plus the domain extremes.
fn err_generic_boundary_neighbourhoods() {
    let centers: [i32; 9] = [
        0,
        -150,
        1_073_741_673,
        1_073_741_674,
        1_073_741_823,
        1_073_741_824,
        -1_073_741_824,
        i32::MAX,
        i32::MIN,
    ];
    let mut xs = Vec::new();
    for &c in &centers {
        for d in -3i32..=3 {
            xs.push(c.wrapping_add(d));
        }
    }
    for &x in &xs {
        assert_same("err-boundary", x);
    }
    assert_same_batch("err-boundary-batch", &xs);
}

fn main() {
    common::run_suite(
        "Phase C — ERRORS.md error/boundary rows",
        &[
            ("err01_int_max", err01_int_max),
            ("err02_int_min", err02_int_min),
            ("err03_first_mul_overflow", err03_first_mul_overflow),
            ("err04_add_overflow_at_max_half", err04_add_overflow_at_max_half),
            ("err05_last_non_overflowing", err05_last_non_overflowing),
            ("err06_one_past_last_non_overflowing", err06_one_past_last_non_overflowing),
            ("err07_min_half", err07_min_half),
            ("err08_one_past_min_half", err08_one_past_min_half),
            ("err09_zero_result_formatting", err09_zero_result_formatting),
            ("err10_minus_sign_formatting", err10_minus_sign_formatting),
            ("err11_out_of_range_bit_patterns", err11_out_of_range_bit_patterns),
            ("err12_wide_argument_truncation", err12_wide_argument_truncation),
            ("err_generic_boundary_neighbourhoods", err_generic_boundary_neighbourhoods),
        ],
    );
}
