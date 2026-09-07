//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `driver` symbol and compares the emitted stdout byte-for-byte.

mod common;

use common::{assert_same, assert_same_all, Rng};

/// C1 — `while (x > 0 || y > 0)` false on entry: no output at all.
#[allow(dead_code)]
fn c1_guard_false_on_entry() {
    let mut rng = Rng::new(1);
    assert_same_all([
        (0, 0),
        (-1, -1),
        (-1, 0),
        (0, -1),
        (i32::MIN, i32::MIN),
        (i32::MIN, 0),
        (0, i32::MIN),
        (i32::MIN, -1),
    ]);
    for _ in 0..300 {
        let x = rng.range_i32(i32::MIN, 0);
        let y = rng.range_i32(i32::MIN, 0);
        let out = assert_same(x, y);
        assert!(out.is_empty(), "driver({x}, {y}) should print nothing");
    }
}

/// C2 — `x > 0`, `y == 0`: pure `x`-drain exercising the `if (y == 0) continue;`
/// path on every iteration.
#[allow(dead_code)]
fn c2_x_positive_y_zero() {
    for x in 1..=40 {
        let out = assert_same(x, 0);
        assert!(!out.is_empty());
    }
    let mut rng = Rng::new(2);
    for _ in 0..200 {
        assert_same(rng.range_i32(1, 40), 0);
    }
}

/// C3 — `x <= 0`, `y > 0`: pure `y`-drain, `if (x > 0)` never taken.
#[allow(dead_code)]
fn c3_x_nonpositive_y_positive() {
    let mut rng = Rng::new(3);
    for x in -5..=0 {
        for y in 1..=20 {
            assert_same(x, y);
        }
    }
    for _ in 0..300 {
        assert_same(rng.range_i32(-40, 0), rng.range_i32(1, 40));
    }
}

/// C4 — the exact `if (x == 1 && y == 4) goto label2;` special case.
#[allow(dead_code)]
fn c4_special_case_exact() {
    let out = assert_same(1, 4);
    // Sanity: the `goto label2` must make the first pass print "loop" then "y"
    // with no "x" in between, i.e. the `label1` block was skipped exactly once.
    assert!(
        out.starts_with(b"loop\ny\n"),
        "goto label2 did not skip label1: {:?}",
        String::from_utf8_lossy(&out)
    );
}

/// C5 — `x == 1` but `y != 4`: special case rejected on the `y` half.
#[allow(dead_code)]
fn c5_x_one_y_not_four() {
    for y in (1..=40).filter(|y| *y != 4) {
        assert_same(1, y);
    }
    assert_same(1, 0);
}

/// C6 — `y == 4` but `x != 1`: special case rejected on the `x` half.
#[allow(dead_code)]
fn c6_y_four_x_not_one() {
    for x in (-10..=40).filter(|x| *x != 1) {
        assert_same(x, 4);
    }
}

/// C7 — `x == 2`, `y > 0`: `x < 3` is true so the backwards `goto label1` fires.
#[allow(dead_code)]
fn c7_x_two_y_positive() {
    for y in 1..=40 {
        assert_same(2, y);
    }
}

/// C8 — `x >= 3`, `y > 0`: `x < 3` false at first (body ends, `while` guard
/// re-tested), then the run crosses into `x < 3` and switches mode mid-flight.
#[allow(dead_code)]
fn c8_x_ge_three_y_positive() {
    let mut rng = Rng::new(8);
    for x in 3..=20 {
        for y in 1..=20 {
            assert_same(x, y);
        }
    }
    for _ in 0..300 {
        assert_same(rng.range_i32(3, 40), rng.range_i32(1, 40));
    }
}

/// C9 — exhaustive grid straddling the `x < 3` and `y == 0` boundaries.
#[allow(dead_code)]
fn c9_boundary_x_lt_3_and_y_zero() {
    for x in 2..=4 {
        for y in 0..=2 {
            assert_same(x, y);
        }
    }
}

/// C10 — exhaustive grid straddling the `x == 1 && y == 4` special case.
#[allow(dead_code)]
fn c10_boundary_special_case() {
    for x in 0..=2 {
        for y in 3..=5 {
            assert_same(x, y);
        }
    }
}

/// C11 — `y` ≫ `x`: many `while` iterations before `continue` is ever reached.
#[allow(dead_code)]
fn c11_y_much_larger_than_x() {
    let mut rng = Rng::new(11);
    for x in 0..=3 {
        for _ in 0..25 {
            assert_same(x, rng.range_i32(50, 200));
        }
    }
}

/// C12 — `x` ≫ `y`: long `x`-drain after `y` reaches 0.
#[allow(dead_code)]
fn c12_x_much_larger_than_y() {
    let mut rng = Rng::new(12);
    for y in 0..=3 {
        for _ in 0..25 {
            assert_same(rng.range_i32(50, 200), y);
        }
    }
}

/// C13 — dense exhaustive cross-product of every branch at small magnitudes.
#[allow(dead_code)]
fn c13_dense_exhaustive_grid() {
    for x in -3..=12 {
        for y in 0..=12 {
            assert_same(x, y);
        }
    }
}

/// C14 — randomized property sweep with the fixed default seed.
#[allow(dead_code)]
fn c14_randomized_property_sweep() {
    let mut rng = Rng::new(Rng::DEFAULT_SEED);
    for _ in 0..2000 {
        let x = rng.range_i32(-50, 300);
        let y = rng.range_i32(0, 300);
        assert_same(x, y);
    }
}

/// C15 — `x` at the non-positive extremes with `y` in each positive class.
#[allow(dead_code)]
fn c15_extreme_nonpositive_x_with_positive_y() {
    for x in [i32::MIN, i32::MIN + 1, -1, 0] {
        for y in [1, 2, 3, 4, 5, 64] {
            assert_same(x, y);
        }
    }
}

/// C16 — statefulness check: the very same `.so` handles are re-invoked many
/// times; each call must be independent of the previous ones.
#[allow(dead_code)]
fn c16_repeated_invocation_is_stateless() {
    let cases = [(1, 4), (3, 7), (0, 5), (9, 0), (2, 2), (5, 5)];
    let first: Vec<Vec<u8>> = cases.iter().map(|&(x, y)| assert_same(x, y)).collect();
    for _round in 0..5 {
        for (i, &(x, y)) in cases.iter().enumerate() {
            let again = assert_same(x, y);
            assert_eq!(
                again, first[i],
                "driver({x}, {y}) is not stateless across calls"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Aggregator
//
// `driver`'s only observable output goes to fd 1, which is process-global, so
// the rows MUST NOT run concurrently (libtest's own progress banner would land
// inside a capture window). Running every row from a single `#[test]` makes the
// suite correct under any `--test-threads` setting.
// ---------------------------------------------------------------------------

macro_rules! rows {
    ($($f:ident),* $(,)?) => {
        #[test]
        fn phase_b_all_config_rows() {
            $(
                eprintln!("CONFIGS.md row: {}", stringify!($f));
                $f();
            )*
            eprintln!("Phase B: all CONFIGS.md rows passed");
        }
    };
}

rows!(
    c1_guard_false_on_entry,
    c2_x_positive_y_zero,
    c3_x_nonpositive_y_positive,
    c4_special_case_exact,
    c5_x_one_y_not_four,
    c6_y_four_x_not_one,
    c7_x_two_y_positive,
    c8_x_ge_three_y_positive,
    c9_boundary_x_lt_3_and_y_zero,
    c10_boundary_special_case,
    c11_y_much_larger_than_x,
    c12_x_much_larger_than_y,
    c13_dense_exhaustive_grid,
    c14_randomized_property_sweep,
    c15_extreme_nonpositive_x_with_positive_y,
    c16_repeated_invocation_is_stateless,
);
