//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every row drives BOTH the C `.so` and the Rust `.so` through `dlsym` with
//! many pseudo-random inputs from a fixed seed and compares stdout byte-for-byte.

mod common;

use common::{assert_same, load, Impls, Rng, SEED};

const IMIN: i32 = i32::MIN; // -2147483648
const IMAX: i32 = i32::MAX; //  2147483647

/// Number of randomized inputs per shaped row.
const N: usize = 400;

fn imps() -> Impls {
    load()
}

// ---------------------------------------------------------------- C1
// x > 0, y > 0, exactly divisible -> rem == 0
#[test]
fn c1_pos_pos_exact() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let y = rng.range_i32(1, 46_340); // y*k stays in range below
        let k = rng.range_i32(0, IMAX / y);
        let x = y.wrapping_mul(k);
        if x <= 0 {
            continue;
        }
        assert_same(&imp, "C1", x, y);
    }
    // Deterministic anchors.
    for &(x, y) in &[(1, 1), (100, 10), (IMAX, IMAX), (46_340 * 46_340, 46_340)] {
        assert_same(&imp, "C1", x, y);
    }
}

// ---------------------------------------------------------------- C2
// x > 0, y > 0, not divisible -> quot > 0, rem > 0
#[test]
fn c2_pos_pos_inexact() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 2);
    let mut seen = 0;
    for _ in 0..(N * 3) {
        let x = rng.range_i32(1, IMAX);
        let y = rng.range_i32(1, IMAX);
        if x % y == 0 || x < y {
            continue;
        }
        assert_same(&imp, "C2", x, y);
        seen += 1;
        if seen >= N {
            break;
        }
    }
    assert!(seen > 0, "C2 generated no inexact positive pairs");
    for &(x, y) in &[(7, 2), (IMAX, 2), (IMAX, 3), (999_999, 7), (IMAX, IMAX - 1)] {
        assert_same(&imp, "C2", x, y);
    }
}

// ---------------------------------------------------------------- C3
// x < 0, y > 0 -> quot <= 0, rem <= 0
#[test]
fn c3_neg_pos() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let x = rng.range_i32(IMIN, -1);
        let y = rng.range_i32(1, IMAX);
        assert_same(&imp, "C3", x, y);
    }
    for &(x, y) in &[(-7, 2), (-1, IMAX), (IMIN, 2), (IMIN, IMAX), (-10, 10)] {
        assert_same(&imp, "C3", x, y);
    }
}

// ---------------------------------------------------------------- C4
// x > 0, y < 0 -> quot <= 0, rem >= 0
#[test]
fn c4_pos_neg() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let x = rng.range_i32(1, IMAX);
        let y = rng.range_i32(IMIN, -1);
        assert_same(&imp, "C4", x, y);
    }
    for &(x, y) in &[(7, -2), (IMAX, -1), (1, IMIN), (IMAX, IMIN), (10, -10)] {
        assert_same(&imp, "C4", x, y);
    }
}

// ---------------------------------------------------------------- C5
// x < 0, y < 0 -> quot >= 0, rem <= 0
#[test]
fn c5_neg_neg() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..N {
        let x = rng.range_i32(IMIN, -1);
        let y = rng.range_i32(IMIN, -1);
        if x == IMIN && y == -1 {
            continue; // error row E5
        }
        assert_same(&imp, "C5", x, y);
    }
    for &(x, y) in &[(-7, -2), (-1, -1), (IMIN, -2), (IMIN, IMIN), (-100, -10)] {
        assert_same(&imp, "C5", x, y);
    }
}

// ---------------------------------------------------------------- C6
// x == 0, y any non-zero -> quot == 0, rem == 0
#[test]
fn c6_zero_numerator() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..N {
        let y = rng.nonzero_i32();
        assert_same(&imp, "C6", 0, y);
    }
    for &y in &[1, -1, 2, -2, IMAX, IMIN, IMAX - 1, IMIN + 1] {
        assert_same(&imp, "C6", 0, y);
    }
}

// ---------------------------------------------------------------- C7
// |x| < |y| -> quot == 0, rem == x, in all four sign combinations
#[test]
fn c7_magnitude_less() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..N {
        let y = rng.range_i32(2, IMAX);
        let mag = rng.range_i32(1, y - 1);
        for &(sx, sy) in &[(1i32, 1i32), (-1, 1), (1, -1), (-1, -1)] {
            assert_same(&imp, "C7", sx * mag, sy * y);
        }
    }
    for &(x, y) in &[
        (1, 2),
        (-1, 2),
        (1, -2),
        (-1, -2),
        (IMAX - 1, IMAX),
        (IMIN + 1, IMIN),
    ] {
        assert_same(&imp, "C7", x, y);
    }
}

// ---------------------------------------------------------------- C8
// y == 1 -> quot == x, rem == 0, x over the full range
#[test]
fn c8_divisor_one() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        assert_same(&imp, "C8", rng.next_i32(), 1);
    }
    for &x in &[0, 1, -1, IMAX, IMIN, IMAX - 1, IMIN + 1] {
        assert_same(&imp, "C8", x, 1);
    }
}

// ---------------------------------------------------------------- C9
// y == -1, x != INT_MIN -> quot == -x, rem == 0
#[test]
fn c9_divisor_minus_one() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let x = rng.range_i32(IMIN + 1, IMAX);
        assert_same(&imp, "C9", x, -1);
    }
    for &x in &[0, 1, -1, IMAX, IMIN + 1, IMAX - 1] {
        assert_same(&imp, "C9", x, -1);
    }
}

// ---------------------------------------------------------------- C10
// x == INT_MIN, y non-zero and != -1 (the 11-byte "-2147483648" print path)
#[test]
fn c10_numerator_int_min() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        let y = loop {
            let v = rng.nonzero_i32();
            if v != -1 {
                break v;
            }
        };
        assert_same(&imp, "C10", IMIN, y);
    }
    for &y in &[1, 2, -2, 3, -3, IMAX, IMIN, IMIN + 1, IMAX - 1] {
        assert_same(&imp, "C10", IMIN, y);
    }
}

// ---------------------------------------------------------------- C11
// x == INT_MAX, y non-zero
#[test]
fn c11_numerator_int_max() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        assert_same(&imp, "C11", IMAX, rng.nonzero_i32());
    }
    for &y in &[1, -1, 2, -2, IMAX, IMIN, IMAX - 1, IMIN + 1] {
        assert_same(&imp, "C11", IMAX, y);
    }
}

// ---------------------------------------------------------------- C12
// y == INT_MIN, x arbitrary
#[test]
fn c12_divisor_int_min() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..N {
        assert_same(&imp, "C12", rng.next_i32(), IMIN);
    }
    for &x in &[0, 1, -1, IMAX, IMIN, IMIN + 1, IMAX - 1] {
        assert_same(&imp, "C12", x, IMIN);
    }
}

// ---------------------------------------------------------------- C13
// Boundary cross-product: one step inside / at the representable extremes.
#[test]
fn c13_boundary_cross_product() {
    let imp = imps();
    let vals = [IMIN, IMIN + 1, -2, -1, 0, 1, 2, IMAX - 1, IMAX];
    let mut pairs = 0;
    for &x in &vals {
        for &y in &vals {
            if y == 0 {
                continue; // error rows E1..E4
            }
            if x == IMIN && y == -1 {
                continue; // error row E5
            }
            assert_same(&imp, "C13", x, y);
            pairs += 1;
        }
    }
    assert_eq!(pairs, 9 * 8 - 1, "C13 did not cover the expected pair count");
}

// ---------------------------------------------------------------- C14
// Full-domain randomized sweep.
#[test]
fn c14_full_domain_random() {
    let imp = imps();
    let mut rng = Rng::new(SEED);
    let mut n = 0;
    while n < 20_000 {
        let x = rng.next_i32();
        let y = rng.nonzero_i32();
        if x == IMIN && y == -1 {
            continue;
        }
        assert_same(&imp, "C14", x, y);
        n += 1;
    }
    assert_eq!(n, 20_000);
}

// ---------------------------------------------------------------- C15
// Repeated / interleaved invocation against the same shared libc stdout:
// no hidden per-call state, no leaked buffer, identical stream buffering
// across a composed sequence of calls rather than one isolated call.
#[test]
fn c15_interleaved_repeated_calls() {
    let imp = imps();
    let mut rng = Rng::new(SEED ^ 15);

    // Same (x, y) called repeatedly must keep producing the same bytes.
    for _ in 0..50 {
        let x = rng.next_i32();
        let y = rng.nonzero_i32();
        if x == IMIN && y == -1 {
            continue;
        }
        let first_c = common::c_out(&imp, x, y);
        let first_r = common::rust_out(&imp, x, y);
        assert_eq!(first_c, first_r, "C15 single call diverged: ({x}, {y})");
        for rep in 0..4 {
            assert_eq!(
                common::c_out(&imp, x, y),
                first_c,
                "C15 C impl not idempotent at rep {rep} for ({x}, {y})"
            );
            assert_eq!(
                common::rust_out(&imp, x, y),
                first_r,
                "C15 Rust impl not idempotent at rep {rep} for ({x}, {y})"
            );
        }
    }

    // A whole batch emitted by many successive calls in ONE capture window:
    // compares the concatenated multi-line stream, so ordering and buffering
    // of the composed pipeline are compared, not just one line.
    let cases: Vec<(i32, i32)> = {
        let mut rng = Rng::new(SEED ^ 0xB16);
        let mut v = Vec::new();
        while v.len() < 500 {
            let x = rng.next_i32();
            let y = rng.nonzero_i32();
            if x == IMIN && y == -1 {
                continue;
            }
            v.push((x, y));
        }
        v
    };

    let mut c_all = Vec::new();
    let mut r_all = Vec::new();
    for &(x, y) in &cases {
        c_all.extend_from_slice(&common::c_out(&imp, x, y));
        r_all.extend_from_slice(&common::rust_out(&imp, x, y));
    }
    assert_eq!(
        c_all.len(),
        r_all.len(),
        "C15 batched stream length differs"
    );
    assert_eq!(c_all, r_all, "C15 batched stream bytes differ");
    assert_eq!(
        c_all.iter().filter(|&&b| b == b'\n').count(),
        cases.len(),
        "C15 expected exactly one newline per call"
    );
}

// ---------------------------------------------------------------- shape check
// Independent of the C/Rust comparison: confirm the captured bytes really are
// the "quotient: %d, remainder: %d\n" line, so the differential assertions
// above cannot be passing on two identically-empty buffers.
#[test]
fn output_shape_is_the_expected_line() {
    let imp = imps();
    assert_eq!(common::c_out(&imp, 7, 2), b"quotient: 3, remainder: 1\n");
    assert_eq!(common::rust_out(&imp, 7, 2), b"quotient: 3, remainder: 1\n");
    assert_eq!(common::c_out(&imp, -7, 2), b"quotient: -3, remainder: -1\n");
    assert_eq!(
        common::rust_out(&imp, -7, 2),
        b"quotient: -3, remainder: -1\n"
    );
    assert_eq!(
        common::rust_out(&imp, IMIN, 1),
        b"quotient: -2147483648, remainder: 0\n"
    );
}
