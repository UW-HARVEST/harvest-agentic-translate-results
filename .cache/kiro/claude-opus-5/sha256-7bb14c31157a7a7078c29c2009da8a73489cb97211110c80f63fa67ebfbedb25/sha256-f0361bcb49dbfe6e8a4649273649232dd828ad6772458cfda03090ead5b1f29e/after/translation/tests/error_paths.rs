//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundaries. Every
//! test constructs the exact condition, drives BOTH the C `.so` and the Rust
//! `.so`, and asserts they agree on the *specific* outcome (same signal number
//! or same exit code and the same stdout bytes) — never merely "both failed".

mod common;

use common::*;

const SIGFPE: i32 = 8;

/// ERRORS.md row 1 — `y == 0` for any `x`: `idiv` divide error.
///
/// Expected C result: killed by SIGFPE (8), with no stdout at all because the
/// `printf` is never reached.
#[test]
fn test_row1_divide_by_zero_signals_identically() {
    let mut rng = Rng::new(0x0000_D1F0);
    let mut xs: Vec<i32> = vec![0, 1, -1, 7, -7, i32::MAX, i32::MIN, i32::MIN + 1];
    for _ in 0..8 {
        xs.push(rng.any_i32());
    }

    for x in xs {
        let c = run_isolated(Impl::C, ChildStdout::Pipe, x, 0);
        let r = run_isolated(Impl::Rust, ChildStdout::Pipe, x, 0);

        assert_eq!(
            c.status,
            Status::Signaled(SIGFPE),
            "ERRORS row 1: C driver({x}, 0) was expected to die on SIGFPE, got {:?}",
            c.status
        );
        assert_eq!(
            r.status, c.status,
            "ERRORS row 1: driver({x}, 0) outcome diverged\n  C   : {:?}\n  Rust: {:?}",
            c.status, r.status
        );
        assert!(
            c.stdout.is_empty(),
            "ERRORS row 1: C unexpectedly printed {:?}",
            String::from_utf8_lossy(&c.stdout)
        );
        assert_eq!(
            r.stdout, c.stdout,
            "ERRORS row 1: driver({x}, 0) stdout diverged\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&r.stdout)
        );
    }
}

/// ERRORS.md row 2 — `INT_MIN / -1`: quotient is not representable, `idiv`
/// raises the same divide error. Killed by SIGFPE (8), no stdout.
#[test]
fn test_row2_int_min_div_neg_one_signals_identically() {
    let c = run_isolated(Impl::C, ChildStdout::Pipe, i32::MIN, -1);
    let r = run_isolated(Impl::Rust, ChildStdout::Pipe, i32::MIN, -1);

    assert_eq!(
        c.status,
        Status::Signaled(SIGFPE),
        "ERRORS row 2: C driver(INT_MIN, -1) was expected to die on SIGFPE, got {:?}",
        c.status
    );
    assert_eq!(
        r.status, c.status,
        "ERRORS row 2: driver(INT_MIN, -1) outcome diverged\n  C   : {:?}\n  Rust: {:?}\n\
         A Rust `x / y` panics with SIGABRT(6) instead of trapping, and \
         `wrapping_div` returns INT_MIN and prints a line C never prints.",
        c.status, r.status
    );
    assert!(c.stdout.is_empty(), "ERRORS row 2: C unexpectedly printed output");
    assert_eq!(
        r.stdout, c.stdout,
        "ERRORS row 2: stdout diverged\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
}

/// ERRORS.md row 3 — `printf` fails (fd 1 is read-only, every `write` returns
/// `EBADF`). The C discards `printf`'s return value, so this is NOT an error:
/// `driver` returns normally and the child exits 0. The Rust must not turn it
/// into a panic, an abort, or an early return.
#[test]
fn test_row3_printf_failure_is_ignored() {
    for (x, y) in [(7, 3), (i32::MIN, 1), (0, -5)] {
        let c = run_isolated(Impl::C, ChildStdout::ReadOnly, x, y);
        let r = run_isolated(Impl::Rust, ChildStdout::ReadOnly, x, y);

        assert_eq!(
            c.status,
            Status::Exited(0),
            "ERRORS row 3: C driver({x}, {y}) with a failing stdout should still return \
             normally, got {:?}",
            c.status
        );
        assert_eq!(
            r.status, c.status,
            "ERRORS row 3: driver({x}, {y}) with a failing stdout diverged\n  C   : {:?}\n  Rust: {:?}",
            c.status, r.status
        );
    }
}

/// ERRORS.md row 4 — one step past the row-2 trap on each axis. All four are
/// representable, so C returns normally and prints; the trap is exactly one
/// point, not a range. Confirms the Rust does not over-reject.
#[test]
fn test_row4_one_step_past_the_overflow_point() {
    let cases = [
        (i32::MIN, -2),
        (i32::MIN, 1),
        (i32::MIN + 1, -1),
        (i32::MAX, -1),
        (i32::MIN, i32::MIN),
        (i32::MIN + 1, i32::MIN),
    ];

    // Each must return normally (exit 0) and print identical bytes.
    for &(x, y) in &cases {
        let c = run_isolated(Impl::C, ChildStdout::Pipe, x, y);
        let r = run_isolated(Impl::Rust, ChildStdout::Pipe, x, y);
        assert_eq!(
            c.status,
            Status::Exited(0),
            "ERRORS row 4: C driver({x}, {y}) should NOT trap, got {:?}",
            c.status
        );
        assert_eq!(
            r.status, c.status,
            "ERRORS row 4: driver({x}, {y}) outcome diverged\n  C   : {:?}\n  Rust: {:?}",
            c.status, r.status
        );
        assert!(!c.stdout.is_empty(), "ERRORS row 4: C printed nothing for ({x}, {y})");
        assert_eq!(
            String::from_utf8_lossy(&r.stdout),
            String::from_utf8_lossy(&c.stdout),
            "ERRORS row 4: driver({x}, {y}) stdout diverged"
        );
    }

    // And in-process, byte-for-byte through the normal capture path.
    assert_same_each("ERRORS row 4 one step past the trap", &cases);
}

/// ERRORS.md row 5 — the full grid of extreme bit patterns in both positions.
/// `driver` takes no enum and no pointer, so the FFI-boundary analogue of "an
/// out-of-range enum value" is the extreme end of the accepted `int` domain:
/// every pattern must be accepted (and must trap only where rows 1–2 say).
#[test]
fn test_row5_full_extreme_value_grid() {
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];

    let mut valid = Vec::new();
    for &x in &extremes {
        for &y in &extremes {
            if traps(x, y) {
                // Must trap identically, with the same signal.
                let c = run_isolated(Impl::C, ChildStdout::Pipe, x, y);
                let r = run_isolated(Impl::Rust, ChildStdout::Pipe, x, y);
                assert_eq!(
                    c.status,
                    Status::Signaled(SIGFPE),
                    "ERRORS row 5: expected C driver({x}, {y}) to trap, got {:?}",
                    c.status
                );
                assert_eq!(
                    r.status, c.status,
                    "ERRORS row 5: driver({x}, {y}) trap outcome diverged\n  C   : {:?}\n  Rust: {:?}",
                    c.status, r.status
                );
                assert_eq!(r.stdout, c.stdout, "ERRORS row 5: driver({x}, {y}) stdout diverged");
            } else {
                valid.push((x, y));
            }
        }
    }

    assert_eq!(
        valid.len(),
        extremes.len() * extremes.len() - 8,
        "expected 7 divide-by-zero points plus (INT_MIN, -1) to be the only traps in the grid"
    );
    assert_same_each("ERRORS row 5 extreme-value grid", &valid);
}

/// Generic C-API boundaries beyond the table.
///
/// `driver` has no pointer parameters, so there is no null-pointer path and no
/// length parameter to make zero or oversized. What *is* reachable across the
/// FFI boundary is passing bit patterns that no C `int` literal in the source
/// would produce, and passing the same operands repeatedly to confirm no hidden
/// per-call state. Both are checked here.
#[test]
fn test_generic_boundaries_no_hidden_state() {
    // The same operand pair 32 times in a row: identical line every time, and
    // identical between C and Rust. Catches any accumulated state in either
    // implementation (there should be none).
    let repeated: Vec<(i32, i32)> = std::iter::repeat((i32::MIN, 3)).take(32).collect();
    assert_same_batch("generic: repeated identical call", &repeated, false);

    let c = capture_file(Impl::C, &repeated);
    let lines: Vec<&[u8]> = c.split(|&b| b == b'\n').filter(|s| !s.is_empty()).collect();
    assert_eq!(lines.len(), 32, "expected exactly one line per call");
    assert!(
        lines.windows(2).all(|w| w[0] == w[1]),
        "C output for a repeated identical call is not itself repeatable"
    );

    // Interleaving the two implementations must not change either one's output.
    let pairs = [(7, -3), (i32::MIN, 7), (0, i32::MIN), (i32::MAX, 2)];
    for &(x, y) in &pairs {
        let a = capture_file(Impl::C, &[(x, y)]);
        let b = capture_file(Impl::Rust, &[(x, y)]);
        let a2 = capture_file(Impl::C, &[(x, y)]);
        let b2 = capture_file(Impl::Rust, &[(x, y)]);
        assert_eq!(a, a2, "C output for ({x}, {y}) changed after a Rust call");
        assert_eq!(b, b2, "Rust output for ({x}, {y}) changed after a C call");
        assert_eq!(a, b, "C/Rust diverged for ({x}, {y})");
    }
}

/// Both `.so`s must export `driver` and nothing about the symbol's calling
/// convention may differ: exercised implicitly everywhere above, asserted
/// explicitly here so a missing export fails with a clear message.
#[test]
fn test_both_libraries_export_driver() {
    let _c = driver_fn(Impl::C);
    let _r = driver_fn(Impl::Rust);
}
