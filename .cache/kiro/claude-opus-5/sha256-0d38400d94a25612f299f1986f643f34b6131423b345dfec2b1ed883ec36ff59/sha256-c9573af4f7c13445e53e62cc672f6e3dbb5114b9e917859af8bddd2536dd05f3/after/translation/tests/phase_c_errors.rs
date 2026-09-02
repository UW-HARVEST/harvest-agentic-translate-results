//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundaries. Each test
//! asserts the two `.so`s return the *same* sentinel (`1`) **and** print the
//! *same* error message — not merely that both failed.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const E_START: &[u8] = b"Error: start is off the end of the string!\n";
const E_STOP_END: &[u8] = b"Error: stop is off the end of the string!\n";
const E_ORDER: &[u8] = b"Error: stop must come after start!\n";

const ITERS: usize = 64;

#[track_caller]
fn expect_err(row: &str, call: &Call<'_>, msg: &[u8]) {
    let o = assert_same_ret(row, call, 1);
    assert_eq!(
        o.out,
        msg,
        "[{row}] wrong error message for {call:?}\n  got     : {:?}\n  expected: {:?}",
        String::from_utf8_lossy(&o.out),
        String::from_utf8_lossy(msg)
    );
}

/// row 1 — start > len
#[test]
fn err01_start_past_end() {
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let over = rng.range_incl(1, 1000) as c_int;
        expect_err("err01", &Call::new(&s, Some(len as c_int + over), None), E_START);
        // exactly one past the end — the off-by-one boundary
        expect_err("err01", &Call::new(&s, Some(len as c_int + 1), None), E_START);
        // and with a stop pointer present, which is never reached
        expect_err(
            "err01",
            &Call::new(&s, Some(len as c_int + 1), Some(0)),
            E_START,
        );
    }
}

/// row 2 — negative start (unsigned promotion ⇒ "off the end")
#[test]
fn err02_start_negative() {
    let mut rng = Rng::new(SEED ^ 102);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let neg = -(rng.range_incl(1, 100_000) as i64) as c_int;
        expect_err("err02", &Call::new(&s, Some(neg), None), E_START);
        expect_err("err02", &Call::new(&s, Some(-1), None), E_START);
        expect_err("err02", &Call::new(&s, Some(-1), Some(len as c_int)), E_START);
    }
}

/// row 3 — start == INT_MIN
#[test]
fn err03_start_int_min() {
    let s = b"hello world";
    expect_err("err03", &Call::new(s, Some(c_int::MIN), None), E_START);
    expect_err("err03", &Call::new(s, Some(c_int::MIN), Some(1)), E_START);
    expect_err("err03", &Call::new(b"", Some(c_int::MIN), None), E_START);
}

/// row 4 — start == INT_MAX
#[test]
fn err04_start_int_max() {
    let s = b"hello world";
    expect_err("err04", &Call::new(s, Some(c_int::MAX), None), E_START);
    expect_err("err04", &Call::new(s, Some(c_int::MAX), Some(1)), E_START);
    expect_err("err04", &Call::new(b"", Some(c_int::MAX), None), E_START);
    expect_err("err04", &Call::new(s, Some(c_int::MAX - 1), None), E_START);
}

/// row 5 — empty string, start >= 1
#[test]
fn err05_empty_start_positive() {
    for st in [1, 2, 7, 1000, c_int::MAX] {
        expect_err("err05", &Call::new(b"", Some(st), None), E_START);
        expect_err("err05", &Call::new(b"", Some(st), Some(0)), E_START);
    }
}

/// row 6 — stop > len
#[test]
fn err06_stop_past_end() {
    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let l = len as c_int;
        let over = rng.range_incl(1, 1000) as c_int;
        expect_err("err06", &Call::new(&s, None, Some(l + over)), E_STOP_END);
        expect_err("err06", &Call::new(&s, None, Some(l + 1)), E_STOP_END);
        // with a valid start pointer too
        expect_err("err06", &Call::new(&s, Some(0), Some(l + 1)), E_STOP_END);
        expect_err("err06", &Call::new(&s, Some(l), Some(l + 1)), E_STOP_END);
    }
}

/// row 7 — negative stop reports "off the end", NOT the ordering error
#[test]
fn err07_stop_negative() {
    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let neg = -(rng.range_incl(1, 100_000) as i64) as c_int;
        expect_err("err07", &Call::new(&s, None, Some(neg)), E_STOP_END);
        expect_err("err07", &Call::new(&s, None, Some(-1)), E_STOP_END);
        expect_err("err07", &Call::new(&s, Some(0), Some(-1)), E_STOP_END);
    }
}

/// row 8 — stop == INT_MIN
#[test]
fn err08_stop_int_min() {
    expect_err("err08", &Call::new(b"hello", None, Some(c_int::MIN)), E_STOP_END);
    expect_err(
        "err08",
        &Call::new(b"hello", Some(0), Some(c_int::MIN)),
        E_STOP_END,
    );
    expect_err("err08", &Call::new(b"", None, Some(c_int::MIN)), E_STOP_END);
}

/// row 9 — stop == INT_MAX
#[test]
fn err09_stop_int_max() {
    expect_err("err09", &Call::new(b"hello", None, Some(c_int::MAX)), E_STOP_END);
    expect_err(
        "err09",
        &Call::new(b"hello", Some(2), Some(c_int::MAX)),
        E_STOP_END,
    );
    expect_err("err09", &Call::new(b"", None, Some(c_int::MAX)), E_STOP_END);
}

/// row 10 — empty string, stop >= 1
#[test]
fn err10_empty_stop_positive() {
    for e in [1, 2, 9, 4096, c_int::MAX] {
        expect_err("err10", &Call::new(b"", None, Some(e)), E_STOP_END);
        expect_err("err10", &Call::new(b"", Some(0), Some(e)), E_STOP_END);
    }
}

/// row 11 — stop < start, both in range
#[test]
fn err11_stop_before_start() {
    let mut rng = Rng::new(SEED ^ 111);
    for _ in 0..ITERS {
        let len = rng.range_incl(2, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let st = rng.range_incl(1, len as u64) as c_int;
        let e = rng.below(st as u64) as c_int; // 0 <= e < st
        expect_err("err11", &Call::new(&s, Some(st), Some(e)), E_ORDER);
    }
}

/// row 12 — stop == start (equality is rejected by `<=`)
#[test]
fn err12_stop_equals_start() {
    let mut rng = Rng::new(SEED ^ 112);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let v = rng.range_incl(0, len as u64) as c_int;
        expect_err("err12", &Call::new(&s, Some(v), Some(v)), E_ORDER);
    }
}

/// row 13 — implicit start 0 with stop == 0
#[test]
fn err13_null_start_stop_zero() {
    let mut rng = Rng::new(SEED ^ 113);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        expect_err("err13", &Call::new(&s, None, Some(0)), E_ORDER);
    }
}

/// row 14 — empty string, stop == 0 (passes the range check, fails ordering)
#[test]
fn err14_empty_stop_zero() {
    expect_err("err14", &Call::new(b"", None, Some(0)), E_ORDER);
    expect_err("err14", &Call::new(b"", Some(0), Some(0)), E_ORDER);
}

/// row 15 — aliased start_ptr == stop_ptr
#[test]
fn err15_aliased_pointers() {
    let mut rng = Rng::new(SEED ^ 115);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        // in-range value ⇒ ordering error
        let v = rng.range_incl(0, len as u64) as c_int;
        expect_err("err15", &Call::aliased(&s, v), E_ORDER);
        // out-of-range value ⇒ the *start* check fires first
        expect_err("err15", &Call::aliased(&s, len as c_int + 1), E_START);
        expect_err("err15", &Call::aliased(&s, -1), E_START);
    }
}

/// row 16 — both indices at the accepted `len` boundary ⇒ ordering error
#[test]
fn err16_both_eq_len() {
    let mut rng = Rng::new(SEED ^ 116);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let l = len as c_int;
        expect_err("err16", &Call::new(&s, Some(l), Some(l)), E_ORDER);
    }
}

/// row 17 — both indices out of range: only the START message is printed
#[test]
fn err17_precedence_start_wins() {
    let mut rng = Rng::new(SEED ^ 117);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let l = len as c_int;
        expect_err("err17", &Call::new(&s, Some(l + 5), Some(l + 9)), E_START);
        expect_err("err17", &Call::new(&s, Some(-3), Some(-4)), E_START);
        expect_err(
            "err17",
            &Call::new(&s, Some(c_int::MAX), Some(c_int::MIN)),
            E_START,
        );
    }
}

/// row 18 — stop is both out of range and <= start: the range message wins
#[test]
fn err18_precedence_stop_range_before_order() {
    let mut rng = Rng::new(SEED ^ 118);
    for _ in 0..ITERS {
        let len = rng.range_incl(1, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let st = rng.range_incl(0, len as u64) as c_int;
        // negative stop is <= start AND (as unsigned) > len
        expect_err("err18", &Call::new(&s, Some(st), Some(-1)), E_STOP_END);
        expect_err(
            "err18",
            &Call::new(&s, Some(st), Some(c_int::MIN)),
            E_STOP_END,
        );
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundaries
// ---------------------------------------------------------------------------

/// A dense sweep of the whole reachable index space for small strings, checking
/// that C and Rust agree on *which* branch fires for every combination —
/// including every out-of-range value one step past the valid range.
#[test]
fn boundary_dense_index_sweep() {
    let mut rng = Rng::new(SEED ^ 200);
    for len in 0usize..=6 {
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let l = len as c_int;
        let interesting: Vec<c_int> = vec![
            c_int::MIN,
            c_int::MIN + 1,
            -100_000,
            -2,
            -1,
            0,
            1,
            l - 1,
            l,
            l + 1,
            l + 2,
            100_000,
            c_int::MAX - 1,
            c_int::MAX,
        ];
        for &st in &interesting {
            // start only
            assert_same("sweep", &Call::new(&s, Some(st), None));
            for &e in &interesting {
                assert_same("sweep", &Call::new(&s, Some(st), Some(e)));
            }
        }
        for &e in &interesting {
            // stop only
            assert_same("sweep", &Call::new(&s, None, Some(e)));
        }
        assert_same("sweep", &Call::new(&s, None, None));
    }
}

/// The same sweep for a longer string, so `len` no longer sits next to 0.
#[test]
fn boundary_dense_index_sweep_long() {
    let mut rng = Rng::new(SEED ^ 201);
    let s = Alpha::AnyNonZero.make(&mut rng, 300);
    let l = 300 as c_int;
    let interesting: Vec<c_int> = vec![
        c_int::MIN,
        -1,
        0,
        1,
        149,
        150,
        l - 1,
        l,
        l + 1,
        1000,
        c_int::MAX,
    ];
    for &st in &interesting {
        assert_same("sweep_long", &Call::new(&s, Some(st), None));
        for &e in &interesting {
            assert_same("sweep_long", &Call::new(&s, Some(st), Some(e)));
        }
    }
}

// --- mystr == NULL: undefined behaviour in the C, compared in a child process.

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

/// Runs `f(NULL, NULL, NULL)` in a forked child and reports
/// `Ok(exit_code)` or `Err(signal)`.
fn run_null_in_child(f: SliceFn) -> Result<c_int, c_int> {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: nothing async-signal-unsafe before the call.
            let r = f(
                std::ptr::null_mut::<c_char>(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            _exit(if r == 0 { 10 } else { 11 });
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        let sig = status & 0x7f;
        if sig != 0 {
            Err(sig)
        } else {
            Ok((status >> 8) & 0xff)
        }
    }
}

/// `mystr == NULL` is not a rejection the C implements (`strlen` is called
/// unconditionally); both libraries must fault in the same way.
#[test]
fn null_mystr_faults_identically() {
    let l = libs();
    let c = run_null_in_child(l.c_slice);
    let r = run_null_in_child(l.rust_slice);
    assert_eq!(
        c, r,
        "mystr=NULL behaved differently: C={c:?} Rust={r:?} (expected the same signal)"
    );
    // Documented expectation on Linux/glibc: SIGSEGV (11).
    assert_eq!(c, Err(11), "expected SIGSEGV from both, got {c:?}");
}

/// Both index pointers NULL is the documented *valid* default path; asserted
/// here as well since it is the "null pointer" generic boundary.
#[test]
fn null_index_pointers_are_valid() {
    let mut rng = Rng::new(SEED ^ 300);
    for _ in 0..ITERS {
        let len = rng.range_incl(0, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret("null_idx", &Call::new(&s, None, None), 0);
    }
}
