//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `driver` has no return value and no error
//! codes, so a "rejection" is observable as *the exact bytes it does or does
//! not write*: the guard that returns immediately (zero bytes), and each guard
//! that suppresses a branch. Both `.so`s must agree byte-for-byte.

mod common;

use common::{Rng, SEED, assert_same_labelled, assert_same_prefix, c_driver, rust_driver};
use std::ffi::c_int;

const INT_MIN: c_int = c_int::MIN;
const INT_MAX: c_int = c_int::MAX;

/// Helper: assert both libraries agree AND that the C output equals `expect`
/// (the ground truth captured from the real C library).
fn assert_same_and_exact(label: &str, x: c_int, y: c_int, expect: &str) {
    let cf = c_driver();
    let rf = rust_driver();
    let c_out = common::capture(|| unsafe { cf(x, y) });
    let r_out = common::capture(|| unsafe { rf(x, y) });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        expect,
        "{label}: C ground truth changed for driver({x}, {y})"
    );
    assert_eq!(
        String::from_utf8_lossy(&r_out),
        String::from_utf8_lossy(&c_out),
        "{label}: Rust diverged from C for driver({x}, {y})"
    );
}

/// Row 1 — `while (x > 0 || y > 0)` false on entry ⇒ immediate return, no bytes.
#[test]
fn row01_loop_never_entered() {
    for (x, y) in [(0, 0), (-1, -1), (-5, 0), (0, -5), (-1, 0), (0, -1)] {
        assert_same_and_exact("row01", x, y, "");
    }
}

/// Row 2 — both operands negative, randomized.
#[test]
fn row02_both_negative_random() {
    let mut rng = Rng::new(SEED ^ 0xE02);
    for _ in 0..500 {
        let x = rng.range(INT_MIN, -1);
        let y = rng.range(INT_MIN, -1);
        assert_same_and_exact("row02", x, y, "");
    }
}

/// Row 3 — the most-negative boundary pair.
#[test]
fn row03_int_min_pair() {
    assert_same_and_exact("row03", INT_MIN, INT_MIN, "");
    assert_same_and_exact("row03", INT_MIN + 1, INT_MIN + 1, "");
    assert_same_and_exact("row03", INT_MIN, INT_MIN + 1, "");
}

/// Row 4 — exactly `0` on one side (boundary of the strict `> 0` tests).
#[test]
fn row04_zero_boundary_of_gt0() {
    for (x, y) in [
        (0, -1),
        (-1, 0),
        (0, INT_MIN),
        (INT_MIN, 0),
        (0, 0),
        (-1, INT_MIN),
        (INT_MIN, -1),
    ] {
        assert_same_and_exact("row04", x, y, "");
    }
}

/// Row 5 — `if (x > 0)` at `label1` rejected (`x == 0`): no `x\n`, no `x--`.
#[test]
fn row05_label1_x_not_positive() {
    assert_same_and_exact("row05", 0, 3, "loop\ny\ny\ny\n");
    assert_same_and_exact("row05", 0, 1, "loop\ny\n");
    assert_same_and_exact("row05", 0, 4, "loop\ny\ny\ny\ny\n");
    let mut rng = Rng::new(SEED ^ 0xE05);
    for _ in 0..64 {
        let y = rng.range(1, 200);
        let expect = format!("loop\n{}", "y\n".repeat(y as usize));
        assert_same_and_exact("row05/random", 0, y, &expect);
    }
}

/// Row 6 — `if (x > 0)` rejected with `x == INT_MIN` (must never decrement).
#[test]
fn row06_label1_x_int_min() {
    assert_same_and_exact("row06", INT_MIN, 5, "loop\ny\ny\ny\ny\ny\n");
    assert_same_and_exact("row06", INT_MIN, 1, "loop\ny\n");
    assert_same_and_exact("row06", INT_MIN + 1, 4, "loop\ny\ny\ny\ny\n");
    assert_same_and_exact("row06", -1, 4, "loop\ny\ny\ny\ny\n");
}

/// Row 7 — `if (y == 0) continue` at `label2` on entry.
#[test]
fn row07_label2_y_zero_continue() {
    assert_same_and_exact("row07", 3, 0, "loop\nx\nloop\nx\nloop\nx\n");
    assert_same_and_exact("row07", 1, 0, "loop\nx\n");
    assert_same_and_exact("row07", 4, 0, "loop\nx\nloop\nx\nloop\nx\nloop\nx\n");
    assert_same_and_exact("row07", 2, 0, "loop\nx\nloop\nx\n");
    let mut rng = Rng::new(SEED ^ 0xE07);
    for _ in 0..64 {
        let x = rng.range(1, 200);
        let expect = "loop\nx\n".repeat(x as usize);
        assert_same_and_exact("row07/random", x, 0, &expect);
    }
}

/// Row 8 — `if (y == 0) continue` reached with `y` having become 0 mid-body.
#[test]
fn row08_label2_y_became_zero() {
    assert_same_and_exact("row08", 2, 2, "loop\nx\ny\nx\ny\n");
    assert_same_and_exact("row08", 1, 1, "loop\nx\ny\n");
    assert_same_and_exact("row08", 2, 1, "loop\nx\ny\nx\n");
    assert_same_and_exact("row08", 1, 2, "loop\nx\ny\ny\n");
}

/// Row 9 — `x == 1 && y == 4` ⇒ `goto label2` skips the whole `label1` block.
#[test]
fn row09_x1_y4_goto_label2() {
    assert_same_and_exact("row09", 1, 4, "loop\ny\nx\ny\ny\ny\n");
}

/// Row 10 — both short-circuit halves of `x == 1 && y == 4` failing.
#[test]
fn row10_x1_y4_short_circuit_halves() {
    assert_same_and_exact("row10/y!=4", 1, 3, "loop\nx\ny\ny\ny\n");
    assert_same_and_exact("row10/y!=4", 1, 5, "loop\nx\ny\ny\ny\ny\ny\n");
    assert_same_and_exact("row10/x!=1", 0, 4, "loop\ny\ny\ny\ny\n");
    assert_same_and_exact("row10/x!=1", 2, 4, "loop\nx\ny\nx\ny\ny\ny\n");
    assert_same_and_exact("row10/x!=1", 4, 4, "loop\nx\ny\nloop\nx\ny\nx\ny\nx\ny\n");
    assert_same_and_exact("row10/x!=1", -1, 4, "loop\ny\ny\ny\ny\n");
}

/// Row 11 — `if (x < 3)` true ⇒ backward `goto label1` (no `while` re-test,
/// hence no extra `loop\n`).
#[test]
fn row11_backward_goto_taken() {
    assert_same_and_exact("row11", 1, 1, "loop\nx\ny\n");
    assert_same_and_exact("row11", 2, 5, "loop\nx\ny\nx\ny\ny\ny\ny\n");
    assert_same_and_exact("row11", 2, 2, "loop\nx\ny\nx\ny\n");
    assert_same_and_exact("row11", 1, 2, "loop\nx\ny\ny\n");
    // No repeated "loop" while the back-edge keeps firing.
    let cf = c_driver();
    let out = common::capture(|| unsafe { cf(2, 50) });
    assert_eq!(
        out.windows(5).filter(|w| *w == b"loop\n").count(),
        1,
        "row11: expected exactly one while-iteration for driver(2, 50)"
    );
    let rf = rust_driver();
    let rout = common::capture(|| unsafe { rf(2, 50) });
    assert_eq!(out, rout, "row11: driver(2, 50) diverged");
}

/// Row 12 — `if (x < 3)` false at the boundary ⇒ back-edge rejected, `while`
/// condition re-tested (extra `loop\n`).
#[test]
fn row12_backward_goto_boundary_x3() {
    assert_same_and_exact("row12", 4, 4, "loop\nx\ny\nloop\nx\ny\nx\ny\nx\ny\n");
    assert_same_and_exact(
        "row12",
        5,
        5,
        "loop\nx\ny\nloop\nx\ny\nloop\nx\ny\nx\ny\nx\ny\n",
    );
    assert_same_and_exact(
        "row12",
        6,
        2,
        "loop\nx\ny\nloop\nx\ny\nloop\nx\nloop\nx\nloop\nx\nloop\nx\n",
    );
    assert_same_and_exact("row12", 3, 3, "loop\nx\ny\nx\ny\nx\ny\n");
}

/// Row 13 — extreme / "garbage" `int` bit patterns across the FFI boundary.
///
/// The API takes no pointers and no enums, so the analogue of "out-of-range
/// enum value" is an arbitrary 32-bit pattern: all of them are legal input and
/// none may be rejected differently by the two implementations.
#[test]
fn row13_extreme_int_bit_patterns() {
    let patterns: [c_int; 12] = [
        INT_MIN,
        INT_MIN + 1,
        -2147483647,
        -1000000,
        -2,
        -1,
        0,
        i32::from_le_bytes([0xFF, 0xFF, 0xFF, 0xFF]),
        i32::from_le_bytes([0x00, 0x00, 0x00, 0x80]),
        i32::from_le_bytes([0xAA, 0xAA, 0xAA, 0xAA]),
        i32::from_le_bytes([0x55, 0x55, 0x55, 0x80]),
        i32::from_le_bytes([0xEF, 0xBE, 0xAD, 0xDE]),
    ];
    // All of these are <= 0, so every pair must produce zero bytes.
    for x in patterns {
        for y in patterns {
            assert!(x <= 0 && y <= 0);
            assert_same_and_exact("row13", x, y, "");
        }
    }
    // ... and combined with a positive counterpart they must still agree.
    for x in patterns {
        for y in [1, 2, 3, 4, 5, 40] {
            assert_same_labelled("row13/mixed", x, y);
        }
    }
}

/// Row 14 — `x > 0 && y < 0`: `y--` walks toward `INT_MIN` (signed-overflow UB
/// in C) for ≈2^31 iterations. Compared as a byte-capped prefix, produced in a
/// forked child that is killed once enough output has been seen.
#[test]
fn row14_negative_y_unbounded_prefix_matches() {
    for (x, y) in [
        (1, -1),
        (2, -1),
        (3, -1),
        (5, -7),
        (1, INT_MIN + 1),
        (40, -100000),
        (INT_MAX, -1),
    ] {
        assert_same_prefix("row14", x, y, 4096);
    }
}

/// Row 15 — `x == INT_MAX`: ≈2^31 iterations. Prefix comparison plus bounded
/// surrogates that take the identical code path.
#[test]
fn row15_int_max_x_prefix_matches() {
    assert_same_prefix("row15/(INT_MAX,0)", INT_MAX, 0, 8192);
    assert_same_prefix("row15/(INT_MAX,1)", INT_MAX, 1, 8192);
    assert_same_prefix("row15/(INT_MAX,INT_MAX)", INT_MAX, INT_MAX, 8192);
    assert_same_prefix("row15/(INT_MAX-1,4)", INT_MAX - 1, 4, 8192);
    assert_same_prefix("row15/(0,INT_MAX)", 0, INT_MAX, 8192);
    assert_same_prefix("row15/(INT_MIN,INT_MAX)", INT_MIN, INT_MAX, 8192);
}

#[test]
fn row15_large_x_surrogates() {
    for x in [5_000, 50_000, 200_000] {
        for y in [0, 7] {
            assert_same_labelled("row15/surrogate", x, y);
        }
    }
}

/// Generic FFI boundary sweep: values one step past every documented/implicit
/// range boundary of the two parameters.
#[test]
fn generic_boundary_sweep() {
    let interesting: [c_int; 11] = [INT_MIN, INT_MIN + 1, -2, -1, 0, 1, 2, 3, 4, 5, 6];
    for x in interesting {
        for y in interesting {
            if x > 0 && y < 0 {
                continue; // unbounded in C (row 14 covers it via prefix)
            }
            assert_same_labelled("generic-boundary", x, y);
        }
    }
}

/// Row 16 — write-error surface: fd 1 closed by the caller. Every `puts` fails;
/// both libraries must still terminate the same way.
#[test]
fn row16_stdout_closed() {
    for mode in common::BufMode::ALL {
        for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 4), (4, 4), (2, 5), (40, 40)] {
            let c = common::outcome_closed_stdout(c_driver(), x, y, mode);
            let r = common::outcome_closed_stdout(rust_driver(), x, y, mode);
            assert_eq!(
                c.status, r.status,
                "row16 {mode:?}: driver({x}, {y}) status differs (C={:#x} Rust={:#x})",
                c.status, r.status
            );
        }
    }
}

/// Row 17 — write-error surface: fd 1 is a pipe with no reader (EPIPE/SIGPIPE).
#[test]
fn row17_broken_pipe() {
    for mode in common::BufMode::ALL {
        for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 4), (4, 4), (2, 5), (40, 40)] {
            let c = common::outcome_broken_pipe(c_driver(), x, y, mode);
            let r = common::outcome_broken_pipe(rust_driver(), x, y, mode);
            assert_eq!(
                c.status, r.status,
                "row17 {mode:?}: driver({x}, {y}) status differs (C={:#x} Rust={:#x})",
                c.status, r.status
            );
        }
    }
}

/// Row 18 — huge positive `y` with `x <= 0` is the *other* ~2^31-iteration
/// shape (`y` drains one per back-edge pass): capped-prefix comparison.
#[test]
fn row18_huge_y_prefix_matches() {
    for (x, y) in [
        (0, INT_MAX),
        (-1, INT_MAX),
        (INT_MIN, INT_MAX),
        (0, INT_MAX - 1),
        (0, 2_000_000_000),
    ] {
        assert_same_prefix("row18", x, y, 8192);
    }
}

/// Row 19 — the special case `x == 1 && y == 4` arriving at `label1`/`label2`
/// through the backward `goto` must NOT re-trigger `goto label2` (the test sits
/// above `label1`). Entry `(2,5)` reaches `label1` with exactly `x==1, y==4`.
#[test]
fn row19_special_case_state_reached_via_back_edge() {
    // (2,5): loop -> x(2->1) -> y(5->4) -> x<3 -> label1 with x==1,y==4.
    assert_same_and_exact("row19", 2, 5, "loop\nx\ny\nx\ny\ny\ny\ny\n");
    // (5,8) passes through the same state after several while-iterations.
    assert_same_labelled("row19", 5, 8);
    for x in 2..=8 {
        for y in 4..=10 {
            assert_same_labelled("row19/reachability", x, y);
        }
    }
}
