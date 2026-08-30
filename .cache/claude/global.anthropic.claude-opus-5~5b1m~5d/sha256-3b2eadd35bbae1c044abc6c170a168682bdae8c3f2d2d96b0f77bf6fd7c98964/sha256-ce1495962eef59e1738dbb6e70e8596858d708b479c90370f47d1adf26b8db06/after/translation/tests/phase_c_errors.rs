//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `driver` contains **no** rejection branch
//! (it is `void`, takes no pointers and no enums, and has zero conditionals), so
//! every row asserts the *must-not-reject* contract: both implementations accept
//! the hostile input and emit identical bytes, with neither panicking nor
//! aborting.
//!
//! Rows E10 and E11 live in their own test binaries (`phase_c_dev_full.rs`,
//! `phase_c_closed_stdout.rs`) because they intentionally poison the process's
//! `stdout` error indicator.

mod common;

use common::*;
use std::ffi::c_int;

const IMIN: c_int = i32::MIN;
const IMAX: c_int = i32::MAX;

// E1 — x at the extreme low end, y swept.
#[test]
fn err_e1_x_int_min() {
    for y in [IMIN, IMIN + 1, -1, 0, 1, IMAX - 1, IMAX] {
        diff_one_expect(IMIN, y, &expected_text(IMIN, y));
    }
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..256 {
        let y = rng.next_i32();
        diff_one_expect(IMIN, y, &expected_text(IMIN, y));
    }
}

// E2 — y == INT_MIN: `~INT_MIN` is the shape that traps for arithmetic negation.
#[test]
fn err_e2_y_int_min() {
    assert_eq!(!IMIN, IMAX, "sanity: ~INT_MIN == INT_MAX");
    for x in [IMIN, IMIN + 1, -1, 0, 1, IMAX - 1, IMAX] {
        diff_one_expect(x, IMIN, &expected_text(x, IMIN));
    }
    let mut rng = Rng::new(SEED ^ 0xE2);
    for _ in 0..256 {
        let x = rng.next_i32();
        diff_one_expect(x, IMIN, &expected_text(x, IMIN));
    }
}

// E3 — both at INT_MAX.
#[test]
fn err_e3_both_int_max() {
    diff_one_expect(IMAX, IMAX, "-1\n");
}

// E4 — both at INT_MIN.
#[test]
fn err_e4_both_int_min() {
    diff_one_expect(IMIN, IMIN, "-1\n");
}

// E5 — result is exactly INT_MIN (the value with no positive magnitude).
#[test]
fn err_e5_result_int_min() {
    diff_one_expect(IMIN, IMAX, "-2147483648\n");
    // Any x whose bits are a subset of the sign bit, with ~y == INT_MIN.
    diff_one_expect(0, IMAX, "-2147483648\n");
}

// E6 — one step past the range of narrower integer types.
#[test]
fn err_e6_one_past_narrow_ranges() {
    let edges: [c_int; 20] = [
        -65537, -65536, -32769, -32768, -32767, -256, -129, -128, -127, -1, 0, 1, 127, 128, 255,
        256, 32767, 32768, 65535, 65536,
    ];
    for &x in &edges {
        for &y in &edges {
            diff_one_expect(x, y, &expected_text(x, y));
        }
    }
}

// E7 — "out-of-range enum value" analogue: arbitrary 32-bit patterns that carry
// no meaningful interpretation, crossed over the FFI boundary as `int`.
#[test]
fn err_e7_out_of_range_enum_like_ints() {
    let patterns: [u32; 12] = [
        0x0000_0000, 0x0000_0001, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF, 0xFFFF_FFFE, 0xDEAD_BEEF,
        0xCAFE_BABE, 0x5555_5555, 0xAAAA_AAAA, 0x0F0F_0F0F, 0xF0F0_F0F0,
    ];
    for &px in &patterns {
        for &py in &patterns {
            let (x, y) = (px as i32, py as i32);
            diff_one_expect(x, y, &expected_text(x, y));
        }
    }

    // Also: values far outside any plausible enum range, including ones that
    // would be sentinel-like for a 1-based enum.
    for v in [-1, 0, 1, 2, 3, 4, 5, 99, 1000, -1000, IMAX, IMIN] {
        diff_one_expect(v, v, &expected_text(v, v));
    }
}

// E8 — ABI: garbage in the upper halves of the 64-bit argument registers.
// The symbol is called through a deliberately mis-declared `fn(u64, u64)`.
#[test]
fn err_e8_high_garbage_bits_in_arg_registers() {
    let f = impls();
    let cases: [(u64, u64); 8] = [
        (0xDEAD_BEEF_0000_0005, 0x0000_0000_0000_0000),
        (0xFFFF_FFFF_0000_0000, 0xFFFF_FFFF_FFFF_FFFF),
        (0x1111_1111_8000_0000, 0x2222_2222_7FFF_FFFF),
        (0xCAFE_BABE_FFFF_FFFF, 0xBAAD_F00D_0000_0001),
        (0x0000_0001_0000_0000, 0x0000_0001_0000_0000),
        (0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000),
        (u64::MAX, 0),
        (0, u64::MAX),
    ];
    for &(hx, hy) in &cases {
        let cout = capture(|| unsafe { (f.c_u64)(hx, hy) });
        let rout = capture(|| unsafe { (f.rust_u64)(hx, hy) });
        assert_eq!(
            cout, rout,
            "mismatch for driver({hx:#018x}, {hy:#018x}) called with 64-bit args"
        );
        // Both must have used only the low 32 bits.
        let (x, y) = (hx as u32 as i32, hy as u32 as i32);
        assert_eq!(
            show(&cout),
            expected_text(x, y),
            "high bits leaked into the computation for ({hx:#018x}, {hy:#018x})"
        );
    }
}

// E9 — degenerate zero arguments.
#[test]
fn err_e9_degenerate_zero_args() {
    diff_one_expect(0, 0, "-1\n");
    diff_one_expect(0, -1, "0\n");
    diff_one_expect(-1, 0, "-1\n");
    diff_one_expect(-1, -1, "-1\n");
    // repeated, to show there is no latched state after a degenerate call
    for _ in 0..16 {
        diff_one_expect(0, 0, "-1\n");
        diff_one_expect(0, -1, "0\n");
    }
}

// E12 / E13 — documented N/A rows, asserted structurally against the header so
// the claim "no pointer / length / return channel exists" cannot silently rot.
#[test]
fn err_e12_e13_no_pointer_length_or_return_channel() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read driver.h");

    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("%:"))
        .collect();
    assert_eq!(
        decls,
        vec!["void driver(int x, int y);"],
        "public header surface changed; ERRORS.md N/A rows must be re-derived"
    );
    assert!(!decls[0].contains('*'), "a pointer parameter appeared");
    assert!(decls[0].starts_with("void "), "a return channel appeared");
    assert!(!decls[0].contains("size_t") && !decls[0].contains("len"), "a length parameter appeared");
    assert!(!decls[0].contains("enum"), "an enum parameter appeared");
}
