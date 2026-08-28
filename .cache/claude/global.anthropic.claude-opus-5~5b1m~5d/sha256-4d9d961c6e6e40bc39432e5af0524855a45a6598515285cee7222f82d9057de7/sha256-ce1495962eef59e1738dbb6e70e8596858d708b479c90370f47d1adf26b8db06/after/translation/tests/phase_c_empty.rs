//! Phase C — error-path differential tests for the rows of ERRORS.md that are
//! reachable in the library exactly as shipped (rows 1, 2, 3, 4, 12) plus the
//! generic FFI boundaries.
//!
//! This binary never calls the init hook, so `node_count` stays 0 throughout.

mod common;

use common::*;
use std::ffi::c_int;

/// Every argument position gets the full set of `int` corner values.
const EXTREMES: [c_int; 12] = [
    0,
    1,
    -1,
    2,
    -2,
    127,
    -128,
    255,
    i32::MIN,
    i32::MIN + 1,
    i32::MAX,
    i32::MAX - 1,
];

// --- ERRORS.md row 1 ------------------------------------------------------

#[test]
fn err_row1_mode1_node_not_found() {
    let p = Pair::shipped();
    // STATUS_ERROR | 0020 == 18
    assert_eq!(ERR_MODE1_NOT_FOUND, 18);
    for &n in &EXTREMES {
        for &d in &EXTREMES {
            for &f in &EXTREMES {
                p.assert_same_eq(0o1, n, d, f, 18);
            }
        }
    }
    let mut rng = Rng::new(0xC000_0001);
    for _ in 0..30_000 {
        let (n, d, f) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        p.assert_same_eq(0o1, n, d, f, 18);
    }
}

// --- ERRORS.md row 2 ------------------------------------------------------

#[test]
fn err_row2_mode2_node_not_found() {
    let p = Pair::shipped();
    // STATUS_ERROR | 0040 == 34
    assert_eq!(ERR_MODE2_NOT_FOUND, 34);
    for &n in &EXTREMES {
        for &d in &EXTREMES {
            for &f in &EXTREMES {
                p.assert_same_eq(0o2, n, d, f, 34);
            }
        }
    }
    let mut rng = Rng::new(0xC000_0002);
    for _ in 0..30_000 {
        let (n, d, f) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        p.assert_same_eq(0o2, n, d, f, 34);
    }
}

// --- ERRORS.md row 3 ------------------------------------------------------

#[test]
fn err_row3_mode4_node_not_found() {
    let p = Pair::shipped();
    // STATUS_ERROR | 0100 == 66
    assert_eq!(ERR_MODE4_NOT_FOUND, 66);
    for &n in &EXTREMES {
        for &d in &EXTREMES {
            for &f in &EXTREMES {
                p.assert_same_eq(0o4, n, d, f, 66);
            }
        }
    }
    let mut rng = Rng::new(0xC000_0004);
    for _ in 0..30_000 {
        let (n, d, f) = (rng.i32_any(), rng.i32_any(), rng.i32_any());
        p.assert_same_eq(0o4, n, d, f, 66);
    }
}

// --- ERRORS.md row 4 ------------------------------------------------------

#[test]
fn err_row4_default_unknown_mode() {
    let p = Pair::shipped();
    // STATUS_ERROR | 0200 == 130
    assert_eq!(ERR_UNKNOWN_MODE, 130);
    for m in [0, 5, 6, 7, 8, -1, -2, 100, 255, 256, 0o1000] {
        p.assert_same_eq(m, 1, 2, 3, 130);
    }
}

#[test]
fn err_row4_exhaustive_mode_scan() {
    // Every mode in a wide contiguous window: exactly 1..4 must be handled,
    // everything else must fall to `default:`.
    let p = Pair::shipped();
    for m in -4096..=4096 {
        let got = p.assert_same(m, 1, 2, 3);
        if (1..=4).contains(&m) {
            assert_ne!(got, 130, "mode {m} must NOT hit the default branch");
        } else {
            assert_eq!(got, 130, "mode {m} must hit the default branch");
        }
    }
}

#[test]
fn err_row4_ffi_enum_edges() {
    // A C `switch` on an `int` accepts any bit pattern. These are the values an
    // out-of-range "enum" would arrive as across the FFI boundary, including
    // ones that alias 1..4 only in their low bits/bytes.
    let p = Pair::shipped();
    let mut cases: Vec<c_int> = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 4,
        -256,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        5,
        0x100,
        0x101,
        0x104,
        0x1_0000,
        0x1_0001,
        0x1_0004,
        0x7FFF_FFFC,
        i32::MAX - 1,
        i32::MAX,
        0x4000_0001,
        -0x7FFF_FFFF,
    ];
    // (u32) values whose truncation to 8/16 bits looks like a valid mode.
    for k in 1..=4i64 {
        for shift in [8, 16, 24] {
            for base in [1i64 << shift, -(1i64 << shift)] {
                let v = base + k;
                if (i32::MIN as i64..=i32::MAX as i64).contains(&v) {
                    cases.push(v as c_int);
                }
            }
        }
    }
    cases.retain(|m| !(1..=4).contains(m));
    cases.sort_unstable();
    cases.dedup();
    println!("out-of-range mode cases: {}", cases.len());
    for &m in &cases {
        for &n in &EXTREMES {
            p.assert_same_eq(m, n, 3, 4, 130);
        }
    }
}

// --- ERRORS.md row 12 -----------------------------------------------------

#[test]
fn err_row12_sprintf_widest() {
    // "Node_-2147483648_Depth_-2147483648" = 34 chars + NUL = 35 of 50 bytes.
    let p = Pair::shipped();
    let widest = 5 + 11 + 7 + 11;
    assert_eq!(widest, 34);
    let expected_metric = widest * 2 + 0o10; // 76
    for &f in &EXTREMES {
        p.assert_same_eq(0o3, i32::MIN, i32::MIN, f, expected_metric + (f & 0o177));
    }
    // All four widest/narrowest combinations of the two %d fields.
    for &n in &[i32::MIN, i32::MAX, 0, -1] {
        for &d in &[i32::MIN, i32::MAX, 0, -1] {
            for &f in &EXTREMES {
                p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
            }
        }
    }
}

// --- generic FFI boundaries ----------------------------------------------

#[test]
fn err_zero_arguments() {
    // All-zero: mode 0 is the `default:` arm.
    let p = Pair::shipped();
    p.assert_same_eq(0, 0, 0, 0, 130);
}

#[test]
fn err_int_extremes_cross_product() {
    // Full 4-way cross product of the corner values over every parameter,
    // including all four at once.
    let p = Pair::shipped();
    let mut modes: Vec<c_int> = vec![0o1, 0o2, 0o3, 0o4];
    modes.extend_from_slice(&EXTREMES);
    modes.sort_unstable();
    modes.dedup();
    let mut n_calls = 0u64;
    for &m in &modes {
        for &n in &EXTREMES {
            for &d in &EXTREMES {
                for &f in &EXTREMES {
                    p.assert_same(m, n, d, f);
                    n_calls += 1;
                }
            }
        }
    }
    println!("cross-product calls: {n_calls}");
    assert_eq!(n_calls as usize, modes.len() * 12 * 12 * 12);
}

#[test]
fn err_no_pointer_or_length_parameters() {
    // Documents why "null pointer" / "oversized length" rows are N/A: the whole
    // public surface is `int jumpnode(int, int, int, int)`.
    let header = std::fs::read_to_string(root_dir().join("c_src/include/lib.h")).unwrap();
    let decls: Vec<&str> = header
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect();
    assert_eq!(decls, vec!["int jumpnode(int a, int b, int c, int d);"]);
    assert!(!header.contains('*'), "header declares no pointer parameters");
}
