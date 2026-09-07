//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through their exported `driver` symbol and compares the captured
//! stdout byte-for-byte, over many randomized inputs from that row's region
//! (fixed seed) plus that row's boundary values.

mod common;

use common::*;

/// Build `PER_ROW` pairs from a generator, prepending any boundary values.
fn mk(seed: u64, boundary: &[(i32, i32)], mut f: impl FnMut(&mut Rng) -> (i32, i32)) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    let mut v: Vec<(i32, i32)> = boundary.to_vec();
    while v.len() < PER_ROW + boundary.len() {
        let (x, y) = f(&mut rng);
        if !traps(x, y) {
            v.push((x, y));
        }
    }
    v
}

// --- Axis 1×2×3: sign quadrants × magnitude relation × divisibility ---------

#[test]
fn row01_pos_pos_bigger_inexact() {
    let pairs = mk(0x0101, &[(7, 3), (i32::MAX, 2), (i32::MAX, i32::MAX - 1)], |r| {
        let y = r.magnitude(i32::MAX - 1);
        let x = r.range_i32(y, i32::MAX);
        if x % y == 0 {
            (x.wrapping_add(1).max(1), y)
        } else {
            (x, y)
        }
    });
    assert_same_each("CONFIGS row 1 (+,+) |x|>|y| inexact", &pairs);
}

#[test]
fn row02_pos_pos_bigger_exact() {
    let pairs = mk(0x0102, &[(6, 3), (i32::MAX, 1), (2147483646, 2)], |r| {
        let y = r.magnitude(46341);
        let k = r.range_i32(1, i32::MAX / y);
        (y.wrapping_mul(k), y)
    });
    assert_same_each("CONFIGS row 2 (+,+) |x|>|y| exact", &pairs);
}

#[test]
fn row03_pos_pos_smaller() {
    let pairs = mk(0x0103, &[(1, 2), (3, i32::MAX), (i32::MAX - 1, i32::MAX)], |r| {
        let y = r.range_i32(2, i32::MAX);
        let x = r.range_i32(1, y - 1);
        (x, y)
    });
    assert_same_each("CONFIGS row 3 (+,+) |x|<|y|", &pairs);
}

#[test]
fn row04_pos_pos_equal() {
    let pairs = mk(0x0104, &[(1, 1), (i32::MAX, i32::MAX)], |r| {
        let y = r.magnitude(i32::MAX);
        (y, y)
    });
    assert_same_each("CONFIGS row 4 (+,+) |x|==|y|", &pairs);
}

#[test]
fn row05_pos_neg_bigger_inexact() {
    let pairs = mk(0x0105, &[(7, -3), (i32::MAX, -2), (i32::MAX, i32::MIN + 1)], |r| {
        let ay = r.magnitude(i32::MAX - 1);
        let x = r.range_i32(ay, i32::MAX);
        let x = if x % ay == 0 { x.wrapping_add(1).max(1) } else { x };
        (x, -ay)
    });
    assert_same_each("CONFIGS row 5 (+,-) |x|>|y| inexact", &pairs);
}

#[test]
fn row06_pos_neg_bigger_exact() {
    let pairs = mk(0x0106, &[(6, -3), (i32::MAX, -1), (2147483646, -2)], |r| {
        let ay = r.magnitude(46341);
        let k = r.range_i32(1, i32::MAX / ay);
        (ay.wrapping_mul(k), -ay)
    });
    assert_same_each("CONFIGS row 6 (+,-) |x|>|y| exact", &pairs);
}

#[test]
fn row07_pos_neg_smaller() {
    let pairs = mk(0x0107, &[(1, -2), (3, i32::MIN), (i32::MAX, i32::MIN)], |r| {
        let ay = r.range_i32(2, i32::MAX);
        let x = r.range_i32(1, ay - 1);
        (x, -ay)
    });
    assert_same_each("CONFIGS row 7 (+,-) |x|<|y|", &pairs);
}

#[test]
fn row08_neg_pos_bigger_inexact() {
    let pairs = mk(0x0108, &[(-7, 3), (i32::MIN, 2), (i32::MIN + 1, 2)], |r| {
        let y = r.magnitude(i32::MAX - 1);
        let ax = r.range_i32(y, i32::MAX);
        let ax = if ax % y == 0 { ax.wrapping_add(1).max(1) } else { ax };
        (-ax, y)
    });
    assert_same_each("CONFIGS row 8 (-,+) |x|>|y| inexact", &pairs);
}

#[test]
fn row09_neg_pos_bigger_exact() {
    let pairs = mk(0x0109, &[(-6, 3), (i32::MIN, 1), (i32::MIN, 2)], |r| {
        let y = r.magnitude(46341);
        let k = r.range_i32(1, i32::MAX / y);
        (-(y.wrapping_mul(k)), y)
    });
    assert_same_each("CONFIGS row 9 (-,+) |x|>|y| exact", &pairs);
}

#[test]
fn row10_neg_pos_smaller() {
    let pairs = mk(0x010A, &[(-1, 2), (-3, i32::MAX), (i32::MIN, i32::MAX)], |r| {
        let y = r.range_i32(2, i32::MAX);
        let ax = r.range_i32(1, y - 1);
        (-ax, y)
    });
    assert_same_each("CONFIGS row 10 (-,+) |x|<|y|", &pairs);
}

#[test]
fn row11_neg_neg_bigger_inexact() {
    let pairs = mk(0x010B, &[(-7, -3), (i32::MIN, -2), (i32::MIN + 1, -2)], |r| {
        let ay = r.magnitude(i32::MAX - 1);
        let ax = r.range_i32(ay, i32::MAX);
        let ax = if ax % ay == 0 { ax.wrapping_add(1).max(1) } else { ax };
        (-ax, -ay)
    });
    assert_same_each("CONFIGS row 11 (-,-) |x|>|y| inexact", &pairs);
}

#[test]
fn row12_neg_neg_bigger_exact() {
    let pairs = mk(0x010C, &[(-6, -3), (i32::MIN + 1, -1), (i32::MIN, -2)], |r| {
        let ay = r.magnitude(46341);
        let k = r.range_i32(1, i32::MAX / ay);
        (-(ay.wrapping_mul(k)), -ay)
    });
    assert_same_each("CONFIGS row 12 (-,-) |x|>|y| exact", &pairs);
}

#[test]
fn row13_neg_neg_smaller() {
    let pairs = mk(0x010D, &[(-1, -2), (-3, i32::MIN), (i32::MIN + 1, i32::MIN)], |r| {
        let ay = r.range_i32(2, i32::MAX);
        let ax = r.range_i32(1, ay - 1);
        (-ax, -ay)
    });
    assert_same_each("CONFIGS row 13 (-,-) |x|<|y|", &pairs);
}

#[test]
fn row14_neg_neg_equal() {
    let pairs = mk(0x010E, &[(-1, -1), (i32::MIN, i32::MIN), (i32::MIN + 1, i32::MIN + 1)], |r| {
        let ay = r.magnitude(i32::MAX);
        (-ay, -ay)
    });
    assert_same_each("CONFIGS row 14 (-,-) |x|==|y|", &pairs);
}

// --- Axis 4/5: special divisors and numerators ------------------------------

#[test]
fn row15_zero_numerator() {
    let pairs = mk(0x010F, &[(0, 1), (0, -1), (0, i32::MAX), (0, i32::MIN)], |r| {
        (0, r.nonzero_i32())
    });
    assert_same_each("CONFIGS row 15 x==0", &pairs);
}

#[test]
fn row16_divisor_one() {
    let pairs = mk(0x0110, &[(i32::MIN, 1), (i32::MAX, 1), (0, 1), (-1, 1)], |r| {
        (r.any_i32(), 1)
    });
    assert_same_each("CONFIGS row 16 y==1", &pairs);
}

#[test]
fn row17_divisor_minus_one() {
    // x == INT_MIN is excluded here: it is ERRORS.md row 2 (a trap), not a
    // valid path.
    let pairs = mk(
        0x0111,
        &[(i32::MIN + 1, -1), (i32::MAX, -1), (0, -1), (1, -1), (-1, -1)],
        |r| (r.range_i32(i32::MIN + 1, i32::MAX), -1),
    );
    assert_same_each("CONFIGS row 17 y==-1", &pairs);
}

#[test]
fn row18_divisor_int_min() {
    let pairs = mk(
        0x0112,
        &[(i32::MIN, i32::MIN), (i32::MAX, i32::MIN), (0, i32::MIN), (-1, i32::MIN), (1, i32::MIN)],
        |r| (r.any_i32(), i32::MIN),
    );
    assert_same_each("CONFIGS row 18 y==INT_MIN", &pairs);
}

#[test]
fn row19_divisor_int_max() {
    let pairs = mk(
        0x0113,
        &[(i32::MIN, i32::MAX), (i32::MAX, i32::MAX), (0, i32::MAX), (-1, i32::MAX), (i32::MAX - 1, i32::MAX)],
        |r| (r.any_i32(), i32::MAX),
    );
    assert_same_each("CONFIGS row 19 y==INT_MAX", &pairs);
}

#[test]
fn row20_numerator_int_min() {
    // Every y except 0 (ERRORS row 1) and -1 (ERRORS row 2).
    let pairs = mk(
        0x0114,
        &[(i32::MIN, 1), (i32::MIN, -2), (i32::MIN, 2), (i32::MIN, i32::MIN), (i32::MIN, i32::MAX)],
        |r| {
            let y = loop {
                let y = r.any_i32();
                if y != 0 && y != -1 {
                    break y;
                }
            };
            (i32::MIN, y)
        },
    );
    assert_same_each("CONFIGS row 20 x==INT_MIN", &pairs);
}

#[test]
fn row21_numerator_int_max() {
    let pairs = mk(
        0x0115,
        &[(i32::MAX, 1), (i32::MAX, -1), (i32::MAX, 2), (i32::MAX, i32::MIN), (i32::MAX, i32::MAX)],
        |r| (i32::MAX, r.nonzero_i32()),
    );
    assert_same_each("CONFIGS row 21 x==INT_MAX", &pairs);
}

// --- Axis 6: printf("%d") field shapes -------------------------------------

#[test]
fn row22_printf_field_shapes() {
    let mut pairs: Vec<(i32, i32)> = vec![
        // widest possible field in the quotient (`-2147483648`)
        (i32::MIN, 1),
        // widest possible field in the remainder: |rem| can reach |y|-1
        (i32::MIN, i32::MIN + 1),
        (i32::MAX, i32::MIN),
        // sign in quotient only
        (-7, 2),
        // sign in remainder only -- impossible: rem takes the numerator's
        // sign, so a negative remainder forces a negative numerator; the
        // reachable case is sign in both.
        (-7, -2),
        // sign in neither
        (7, 2),
        // 1-digit / 2-digit / 10-digit fields
        (9, 4),
        (99, 10),
        (1_000_000_007, 3),
        (2_000_000_000, 999_999_999),
    ];
    pairs.extend(mk(0x0116, &[], |r| (r.any_i32(), r.nonzero_i32())));
    assert_same_each("CONFIGS row 22 printf field shapes", &pairs);
}

// --- Row 23: unconstrained fuzz --------------------------------------------

#[test]
fn row23_unconstrained_fuzz() {
    let mut rng = Rng::new(0xDEADBEEF_C0FFEE);
    let mut pairs = Vec::with_capacity(20_000);
    while pairs.len() < 20_000 {
        let x = rng.any_i32();
        let y = rng.any_i32();
        if !traps(x, y) {
            pairs.push((x, y));
        }
    }
    // Compare as one batch: 20k separate fd-1 redirections would dominate the
    // runtime, and the concatenated stream is a strictly stronger assertion.
    assert_same_batch("CONFIGS row 23 unconstrained fuzz", &pairs, false);

    // On divergence the batch assert reports a byte offset; re-run the
    // suspicious neighbourhood pair-by-pair to name the exact input. Only a
    // small sample, to keep this cheap.
    assert_same_each("CONFIGS row 23 sample", &pairs[..64]);
}

// --- Rows 24/25: repeated invocation, regular file vs pipe ------------------

fn batch_512() -> Vec<(i32, i32)> {
    let mut rng = Rng::new(0x5121_5121);
    let mut pairs = Vec::with_capacity(512);
    // Deliberately mix widths and signs so buffer boundaries land mid-line.
    let seeds: [(i32, i32); 8] = [
        (i32::MIN, 1),
        (i32::MAX, -1),
        (0, i32::MIN),
        (-1, i32::MAX),
        (7, -3),
        (-7, 3),
        (1_000_000_007, 3),
        (i32::MIN, i32::MIN),
    ];
    pairs.extend_from_slice(&seeds);
    while pairs.len() < 512 {
        let x = rng.any_i32();
        let y = rng.any_i32();
        if !traps(x, y) {
            pairs.push((x, y));
        }
    }
    pairs
}

#[test]
fn row24_repeated_calls_regular_file() {
    assert_same_batch("CONFIGS row 24 512 calls -> regular file", &batch_512(), false);
}

#[test]
fn row25_repeated_calls_pipe() {
    assert_same_batch("CONFIGS row 25 512 calls -> pipe", &batch_512(), true);
}
