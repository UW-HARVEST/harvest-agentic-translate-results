// Phase B -- valid-path differential tests, one test per row of CONFIGS.md.
//
// Every call goes through the .so exports of BOTH implementations (libloading);
// stdout is captured and compared byte-for-byte.

mod common;
use common::*;

// C1: guard false (x <= 0 && y <= 0) -- randomized.
#[test]
fn cfg_c1_guard_false() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..400 {
        let x = rng.range(-64, 0);
        let y = rng.range(-64, 0);
        let out = assert_same(x, y);
        assert!(out.is_empty(), "driver({x}, {y}) should print nothing");
    }
}

// C2: x > 0, y == 0 -- randomized.
#[test]
fn cfg_c2_positive_x_zero_y() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..120 {
        let x = rng.range(1, 300);
        assert_same(x, 0);
    }
    for x in 1..=32 {
        assert_same(x, 0);
    }
}

// C3: x == 0, y > 0 -- randomized.
#[test]
fn cfg_c3_zero_x_positive_y() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..120 {
        let y = rng.range(1, 300);
        assert_same(0, y);
    }
    for y in 1..=32 {
        assert_same(0, y);
    }
}

// C4: x < 0, y > 0 -- randomized.
#[test]
fn cfg_c4_negative_x_positive_y() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..200 {
        let x = rng.range(-300, -1);
        let y = rng.range(1, 300);
        assert_same(x, y);
    }
}

// C5: the `goto label2` special case, exactly (1, 4).
#[test]
fn cfg_c5_goto_label2_exact() {
    assert_same_and_eq(1, 4, "loop\ny\nx\ny\ny\ny\n");
}

// C6: near-misses of the (x == 1 && y == 4) condition.
#[test]
fn cfg_c6_goto_label2_near_misses() {
    // NB: (1, -4) is deliberately absent -- x > 0 with y < 0 never terminates in
    // the C code; that path is covered by the prefix tests in tests/unbounded.rs.
    for (x, y) in [(1, 3), (1, 5), (0, 4), (2, 4), (-1, 4), (4, 1), (4, 4), (1, 0)] {
        assert_same(x, y);
    }
}

// C7/C8/C9/C10: x pinned at each side of the `x < 3` boundary, y swept.
#[test]
fn cfg_c7_x_one_sweep_y() {
    for y in 0..=16 {
        assert_same(1, y);
    }
}

#[test]
fn cfg_c8_x_two_sweep_y() {
    for y in 0..=16 {
        assert_same(2, y);
    }
}

#[test]
fn cfg_c9_x_three_sweep_y() {
    for y in 0..=16 {
        assert_same(3, y);
    }
}

#[test]
fn cfg_c10_x_four_sweep_y() {
    for y in 0..=16 {
        assert_same(4, y);
    }
}

// C11: exhaustive small grid over every reachable branch combination.
#[test]
fn cfg_c11_exhaustive_small_grid() {
    for x in -6..=16 {
        for y in 0..=16 {
            assert_same(x, y);
        }
    }
}

// C12: exhaustive negative-y grid on the guard-false side.
#[test]
fn cfg_c12_negative_y_grid() {
    for x in -6..=0 {
        for y in -16..=-1 {
            let out = assert_same(x, y);
            assert!(out.is_empty(), "driver({x}, {y}) should print nothing");
        }
    }
}

// C13: "wide" shape x >= 3 && y >= 1 -- randomized.
#[test]
fn cfg_c13_wide_random() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..150 {
        let x = rng.range(3, 400);
        let y = rng.range(1, 400);
        assert_same(x, y);
    }
}

// C14: strongly asymmetric magnitudes -- randomized both ways.
#[test]
fn cfg_c14_asymmetric_random() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..80 {
        let big = rng.range(200, 1200);
        let small = rng.range(0, 4);
        assert_same(big, small);
        assert_same(small, big);
    }
}

// C15: largest feasible magnitudes.
#[test]
fn cfg_c15_large_feasible() {
    for (x, y) in [
        (3000, 0),
        (0, 3000),
        (3000, 3000),
        (3000, 1),
        (1, 3000),
        (2, 3000),
        (3, 3000),
        (2999, 3001),
    ] {
        assert_same(x, y);
    }
}

// C16: extreme representable inputs.
#[test]
fn cfg_c16_extremes() {
    for (x, y) in [
        (i32::MIN, i32::MIN),
        (i32::MIN + 1, i32::MIN + 1),
        (i32::MIN, 0),
        (0, i32::MIN),
        (-1, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, 0),      // huge, but only via the guard-false pairing below
        (i32::MIN, i32::MAX % 97 + 1),
    ]
    .into_iter()
    .filter(|&(x, y)| !(x > 0 && y > 4096) && !(x > 4096))
    {
        assert_same(x, y);
    }
    // x = INT_MIN with a positive y: `x > 0` is never true, `x < 3` always is.
    for y in [1, 2, 3, 4, 5, 17, 64] {
        assert_same(i32::MIN, y);
    }
    // INT_MAX on the x axis is not runnable (2^31 iterations); the guard-false
    // partner value is, and is covered by (i32::MIN, ...) rows above.
}

// C17: statelessness -- interleaved repeated calls must be independent.
#[test]
fn cfg_c17_statelessness_interleaved() {
    let mut rng = Rng::new(SEED ^ 17);
    let mut baseline = Vec::new();
    for _ in 0..40 {
        let x = rng.range(-3, 24);
        let y = rng.range(0, 24);
        baseline.push(((x, y), assert_same(x, y)));
    }
    // Replay in reverse order; results must be identical to the first pass.
    for ((x, y), expect) in baseline.iter().rev() {
        let c = run_c(*x, *y);
        let r = run_rust(*x, *y);
        assert_eq!(&c, expect, "C not stateless for driver({x}, {y})");
        assert_eq!(&r, expect, "Rust not stateless for driver({x}, {y})");
    }
    // Rust-then-C and C-then-Rust orderings.
    for _ in 0..40 {
        let x = rng.range(-3, 24);
        let y = rng.range(0, 24);
        let r = run_rust(x, y);
        let c = run_c(x, y);
        assert_eq!(r, c, "order-dependent divergence for driver({x}, {y})");
    }
}
