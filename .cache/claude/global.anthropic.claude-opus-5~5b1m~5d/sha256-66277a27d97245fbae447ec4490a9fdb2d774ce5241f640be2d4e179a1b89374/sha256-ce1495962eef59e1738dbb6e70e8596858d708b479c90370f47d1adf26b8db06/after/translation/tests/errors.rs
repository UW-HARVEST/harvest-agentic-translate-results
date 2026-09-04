// Phase C -- error/rejection-path differential tests, one test per ERRORS.md row.
//
// `driver` returns void and has no error codes, so its rejection surface is the
// set of inputs it refuses to do work for (guard conditions) plus the degenerate
// and extreme values a caller can push across the FFI boundary. For each, both
// implementations must reject IDENTICALLY: the same sentinel behaviour (exactly
// zero bytes of output, immediate return, no crash) or the same emitted bytes.

mod common;
use common::*;

// E1: guard rejects the call outright (x <= 0 && y <= 0).
#[test]
fn err_e1_guard_rejects_nonpositive() {
    for x in -8..=0 {
        for y in -8..=0 {
            let c = run_c(x, y);
            let r = run_rust(x, y);
            assert_eq!(c, r, "divergent rejection for driver({x}, {y})");
            assert!(
                c.is_empty(),
                "driver({x}, {y}) must be rejected with zero output, got {:?}",
                String::from_utf8_lossy(&c)
            );
        }
    }
}

// E2: one step past the guard on the x axis.
#[test]
fn err_e2_one_past_x_guard() {
    assert_same_and_eq(0, 0, "");
    assert_same_and_eq(1, 0, "loop\nx\n");
    assert_same_and_eq(-1, 0, "");
}

// E3: one step past the guard on the y axis.
#[test]
fn err_e3_one_past_y_guard() {
    assert_same_and_eq(0, 0, "");
    assert_same_and_eq(0, 1, "loop\ny\n");
    assert_same_and_eq(0, -1, "");
    assert_same_and_eq(-1, 1, "loop\ny\n");
}

// E4: most-negative arguments on both axes.
#[test]
fn err_e4_int_min_both() {
    for (x, y) in [
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MIN + 1),
        (i32::MIN + 1, i32::MIN),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MIN, 0),
        (0, i32::MIN),
    ] {
        let c = run_c(x, y);
        let r = run_rust(x, y);
        assert_eq!(c, r, "divergence for driver({x}, {y})");
        assert!(c.is_empty(), "driver({x}, {y}) must print nothing");
    }
}

// E5: x = INT_MIN with y > 0 -- `x > 0` never true, `x < 3` always true.
#[test]
fn err_e5_int_min_x_positive_y() {
    assert_same_and_eq(i32::MIN, 1, "loop\ny\n");
    assert_same_and_eq(i32::MIN, 3, "loop\ny\ny\ny\n");
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..40 {
        let y = rng.range(1, 200);
        let out = assert_same(i32::MIN, y);
        // Shape check: "loop\n" once then "y\n" y times.
        let expect: String = format!("loop\n{}", "y\n".repeat(y as usize));
        assert_eq!(String::from_utf8_lossy(&out), expect);
    }
}

// E7: y < 0 with x <= 0 -- guard false even though y != 0.
#[test]
fn err_e7_negative_y_nonpositive_x() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..200 {
        let x = rng.range(i32::MIN, 0);
        let y = rng.range(i32::MIN, -1);
        let c = run_c(x, y);
        let r = run_rust(x, y);
        assert_eq!(c, r, "divergence for driver({x}, {y})");
        assert!(c.is_empty(), "driver({x}, {y}) must print nothing");
    }
}

// E8: largest feasible magnitudes (INT_MAX itself needs ~2^31 iterations and is
// documented as untestable; these are the largest values runnable in budget).
#[test]
fn err_e8_large_but_feasible() {
    for (x, y) in [
        (100_000, 0),
        (0, 100_000),
        (50_000, 50_000),
        (1, 100_000),
        (100_000, 1),
    ] {
        assert_same(x, y);
    }
}

// E9: the only input taking the `goto label2` branch.
#[test]
fn err_e9_goto_label2_special_case() {
    assert_same_and_eq(1, 4, "loop\ny\nx\ny\ny\ny\n");
    // Neighbours must NOT take that branch.
    assert_same(1, 3);
    assert_same(1, 5);
    assert_same(2, 4);
    assert_same(0, 4);
}

// Generic FFI boundary sweep: since the API takes two plain `int`s there are no
// pointers, lengths or enums; the analogue is every "one step past a range" and
// out-of-any-valid-domain integer value. All are legal inputs to the C ABI and
// must behave identically.
#[test]
fn err_generic_boundary_values() {
    let interesting: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -70000,
        -1000,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        16,
        17,
        255,
        256,
        257,
        1000,
    ];
    for &x in &interesting {
        for &y in &interesting {
            // Skip only the pairs where the C code provably does not terminate
            // (x > 0 with y < 0 -> ~2^31 iterations then signed overflow), and
            // the pairs whose output is too large to run in the time budget.
            if x > 0 && y < 0 {
                continue;
            }
            if x > 100_000 || y > 100_000 {
                continue;
            }
            let c = run_c(x, y);
            let r = run_rust(x, y);
            assert_eq!(c, r, "divergence for driver({x}, {y})");
        }
    }
}
