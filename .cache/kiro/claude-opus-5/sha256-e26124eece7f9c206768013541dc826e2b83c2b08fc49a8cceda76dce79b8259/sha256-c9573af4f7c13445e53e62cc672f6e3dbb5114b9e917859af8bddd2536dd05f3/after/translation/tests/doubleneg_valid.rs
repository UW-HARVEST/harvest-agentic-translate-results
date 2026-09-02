//! Phase B — `CONFIGS.md` rows 32-43: `doubleneg`, the composed pipeline.
//!
//! Deliberately a SINGLE `#[test]` in its own binary: comparing the printed
//! bytes requires redirecting fd 1 process-wide, and libtest writes its own
//! progress lines to fd 1 from other test threads.

mod harness;

use std::ffi::c_int;

use harness::{Rng, diff_doubleneg};

#[test]
fn doubleneg_valid_configurations() {
    // Rows 32-42: one hand-derived configuration per row.
    let cases: &[(&str, [c_int; 4])] = &[
        ("row32 all zero", [0, 0, 0, 0]),
        ("row33 param1 zero", [0, 3, 5, 7]),
        ("row33 param1 zero, negatives", [0, -3, -5, -7]),
        ("row34 param2 zero", [11, 0, 5, 7]),
        ("row34 param2 zero, negative param1", [-11, 0, 5, 7]),
        ("row35 all ones", [1, 1, 1, 1]),
        ("row36 all negative", [-1, -3, -5, -7]),
        ("row36 all negative big", [-1000, -12345, -999, -7]),
        ("row37 mixed signs", [-5, 7, -9, 11]),
        ("row37 mixed signs 2", [5, -7, 9, -11]),
        ("row38 all INT_MAX", [i32::MAX, i32::MAX, i32::MAX, i32::MAX]),
        ("row38 all INT_MIN", [i32::MIN, i32::MIN, i32::MIN, i32::MIN]),
        (
            "row38 INT_MIN/MAX mix",
            [i32::MIN, i32::MAX, i32::MIN, i32::MAX],
        ),
        ("row38 INT_MIN+1", [i32::MIN + 1, i32::MAX - 1, -1, 1]),
        ("row39 negative exponent", [7, 3, -4, 9]),
        ("row39 negative exponent 2", [123456, 7, -9, 1]),
        ("row40 overflowing double", [i32::MAX, 1, 9, 3]),
        ("row40 overflowing double 2", [-2147483647, 1, 9, 3]),
        ("row40 overflowing double 3", [2000000000, 1, 8, 3]),
        ("row41 byte 100 absent", [1, 1, 1, 1]),
        ("row42 byte 42 absent", [43, 1, 1, 1]),
        ("row42 byte 42 absent 2", [100, 0, 0, 0]),
        ("param3 zero", [3, 5, 0, 7]),
        ("param4 zero", [3, 5, 7, 0]),
        ("multiples of 256", [256, 512, 768, 1024]),
        ("byte boundaries", [255, 128, 127, 256]),
        ("exponent 0 with b!=0", [12345, 7, 10, 20]),
        ("a == 0 with b != 0", [0, 7, 3, 5]),
    ];
    for (label, [a, b, c, d]) in cases {
        diff_doubleneg(*a, *b, *c, *d, label);
    }

    // Neighbourhood sweep around each case: perturb one parameter at a time.
    let mut rng = Rng::new(0xEEEE_0001);
    for (label, params) in cases {
        for _ in 0..4 {
            let mut q = *params;
            let idx = rng.below(4) as usize;
            let delta = [-2i32, -1, 1, 2][rng.below(4) as usize];
            q[idx] = q[idx].wrapping_add(delta);
            diff_doubleneg(q[0], q[1], q[2], q[3], &format!("{label} perturbed"));
        }
    }

    // Every exponent `param3 % 10` reaches, crossed with a found/not-found
    // buffer seed.
    for p3 in -12..=12 {
        for p1 in [0, 1, 42, 100, -100, 12345] {
            diff_doubleneg(p1, 3, p3, 7, &format!("exponent sweep p1={p1} p3={p3}"));
        }
    }

    // Row 43 — randomized tuples, full stdout byte comparison.
    let mut rng = Rng::new(0xEEEE_0002);
    for i in 0..4000 {
        diff_doubleneg(
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            &format!("row43 random #{i}"),
        );
    }
}
