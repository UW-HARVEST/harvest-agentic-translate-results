//! Top level: `matrixsum`, the only function declared in `include/lib.h`. It
//! composes every lower-level routine, so these tests are the end-to-end check.
//!
//! All tests here take `matrix_guard()` because `matrixsum` reads the shared
//! `matrix` global via `calculate_matrix_checksum`.

mod common;

use common::{INT_PROBES, MatrixT, both, matrix_guard};
use std::ffi::c_int;

fn check(label: &str, a: c_int, b: c_int, cc: c_int, d: c_int) {
    let (c, rust) = both();
    let got_c = unsafe { (c.matrixsum)(a, b, cc, d) };
    let got_rust = unsafe { (rust.matrixsum)(a, b, cc, d) };
    assert_eq!(got_c, got_rust, "{label}: matrixsum({a}, {b}, {cc}, {d})");
}

/// The four parameters only ever feed the flag bits through a `!!` test, so the
/// 16 zero/non-zero combinations are the distinct control-flow cases.
#[test]
fn matrixsum_matches_on_all_zero_nonzero_combinations() {
    let _guard = matrix_guard();
    let nonzero = [1, -1, c_int::MAX, c_int::MIN, 0x1234];
    for &nz in &nonzero {
        for mask in 0..16u32 {
            let pick = |bit: u32| if mask & (1 << bit) != 0 { nz } else { 0 };
            check(
                &format!("mask={mask:#06b} nz={nz}"),
                pick(0),
                pick(1),
                pick(2),
                pick(3),
            );
        }
    }
}

#[test]
fn matrixsum_matches_on_small_grid() {
    let _guard = matrix_guard();
    for a in -6..=6 {
        for b in -6..=6 {
            for c in -6..=6 {
                for d in -6..=6 {
                    check("small grid", a, b, c, d);
                }
            }
        }
    }
}

/// Cartesian product of the edge-value probe list: 28^4 combinations, which
/// covers every mix of extremes, so any wrap-around difference in
/// `(sum * 0x10) + (flag_count * 0xFF) + (matrix_sum & 0xFFF)` shows up.
#[test]
fn matrixsum_matches_on_probe_product() {
    let _guard = matrix_guard();
    let (c, rust) = both();
    for &a in INT_PROBES {
        for &b in INT_PROBES {
            for &cc in INT_PROBES {
                for &d in INT_PROBES {
                    let got_c = unsafe { (c.matrixsum)(a, b, cc, d) };
                    let got_rust = unsafe { (rust.matrixsum)(a, b, cc, d) };
                    assert_eq!(got_c, got_rust, "matrixsum({a}, {b}, {cc}, {d})");
                }
            }
        }
    }
}

/// Values chosen so the intermediate sum and the final multiply overflow `int`.
#[test]
fn matrixsum_matches_on_overflowing_inputs() {
    let _guard = matrix_guard();
    let cases: &[[c_int; 4]] = &[
        [c_int::MAX, c_int::MAX, c_int::MAX, c_int::MAX],
        [c_int::MIN, c_int::MIN, c_int::MIN, c_int::MIN],
        [c_int::MAX, 1, 0, 0],
        [c_int::MIN, -1, 0, 0],
        [c_int::MAX, c_int::MIN, c_int::MAX, c_int::MIN],
        [0x1000_0000, 0x1000_0000, 0x1000_0000, 0x1000_0000],
        [0x0800_0000, 0, 0, 0],
        [0x1000_0000, 0, 0, 0],
        [-0x1000_0000, -0x1000_0000, -0x1000_0000, -0x1000_0000],
        [0x7FFF_FFFF, 0x7FFF_FFFF, -1, -1],
        [0x0FFF_FFFF, 0x0FFF_FFFF, 0x0FFF_FFFF, 0x0FFF_FFFF],
        [1 << 27, 1 << 27, 1 << 27, 1 << 27],
        [1 << 28, -(1 << 28), 1 << 28, -(1 << 28)],
    ];
    for v in cases {
        check("overflow", v[0], v[1], v[2], v[3]);
    }
}

/// Deterministic pseudo-random sweep over the full `int` range.
#[test]
fn matrixsum_matches_on_pseudorandom_inputs() {
    let _guard = matrix_guard();
    let (c, rust) = both();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 32) as u32 as c_int
    };
    for _ in 0..20_000 {
        let (a, b, cc, d) = (next(), next(), next(), next());
        let got_c = unsafe { (c.matrixsum)(a, b, cc, d) };
        let got_rust = unsafe { (rust.matrixsum)(a, b, cc, d) };
        assert_eq!(got_c, got_rust, "matrixsum({a}, {b}, {cc}, {d})");
    }
}

/// `matrixsum` folds in `calculate_matrix_checksum() & 0xFFF`, so mutating the
/// exported `matrix` global must shift both results the same way, including
/// where the mask discards high bits.
#[test]
fn matrixsum_matches_after_mutating_the_matrix_global() {
    let _guard = matrix_guard();
    let (c, rust) = both();
    let saved_c = c.read_matrix();
    let saved_rust = rust.read_matrix();
    assert_eq!(saved_c, saved_rust, "initial matrix");

    let matrices: &[MatrixT] = &[
        [[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[0xFFF, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        // Sums to exactly 0x1000, so the & 0xFFF mask yields 0.
        [[0x1000, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[0x1001, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[-1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[c_int::MIN, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [
            [c_int::MAX, 1, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ],
        [[100, 200, 300, 400], [500, 600, 700, 800], [900, 1000, 1100, 1200]],
        saved_c,
    ];

    for m in matrices {
        c.write_matrix(m);
        rust.write_matrix(m);
        for &(a, b, cc, d) in &[
            (0, 0, 0, 0),
            (1, 1, 1, 1),
            (-1, 2, -3, 4),
            (c_int::MAX, c_int::MIN, 1, -1),
            (1000, 0, 0, 7),
        ] {
            let got_c = unsafe { (c.matrixsum)(a, b, cc, d) };
            let got_rust = unsafe { (rust.matrixsum)(a, b, cc, d) };
            assert_eq!(
                got_c, got_rust,
                "matrixsum({a}, {b}, {cc}, {d}) with matrix {m:?}"
            );
        }
    }

    c.write_matrix(&saved_c);
    rust.write_matrix(&saved_rust);
}

/// Anchors the default-matrix behaviour against a value computed from the C
/// source by hand, so a change that broke *both* sides identically would still
/// be caught.
#[test]
fn matrixsum_default_matrix_reference_values() {
    let _guard = matrix_guard();
    let (c, rust) = both();

    // matrix checksum = (0x01+0x02+0x03+0x04) + (0x10+0x20+0x30+0x40)
    //                 + (0xA1+0xB2+0xC3+0xD4)
    //                 = 10 + 160 + 746 = 916 (0x394), unchanged by & 0xFFF.
    let matrix_term = 916;
    for &(a, b, cc, d) in &[
        (0, 0, 0, 0),
        (1, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (5, 0, 0, 0),
    ] {
        let sum = a + b + cc + d;
        let flags = [a, b, cc, d].iter().filter(|&&x| x != 0).count() as c_int;
        let expected = sum * 0x10 + flags * 0xFF + matrix_term;

        let got_c = unsafe { (c.matrixsum)(a, b, cc, d) };
        assert_eq!(got_c, expected, "hand-computed reference for C");
        assert_eq!(
            unsafe { (rust.matrixsum)(a, b, cc, d) },
            expected,
            "hand-computed reference for Rust"
        );
    }
}
