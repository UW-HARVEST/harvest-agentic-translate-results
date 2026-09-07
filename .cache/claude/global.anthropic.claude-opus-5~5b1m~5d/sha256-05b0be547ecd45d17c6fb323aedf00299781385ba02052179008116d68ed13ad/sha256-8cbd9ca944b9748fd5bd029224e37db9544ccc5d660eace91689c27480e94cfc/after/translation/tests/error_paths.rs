//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! `bin2hex`'s only rejection is `abort()` (it never returns NULL and never
//! touches `errno`), so the observable "error code" is the process
//! termination status. Each call therefore runs in a `fork()`ed child and the
//! parent compares the exact wait-status classification (exited-with-code N
//! vs. killed-by-signal N) between the C `.so` and the Rust `.so`.
//!
//! Both `.so`s are `dlopen`ed in the parent (before forking) and invoked only
//! through `dlsym`.

mod common;

use common::*;

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

const SIGABRT: i32 = 6;
const SIGSEGV: i32 = 11;
const SIGBUS: i32 = 7;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Status {
    Exited(i32),
    Signalled(i32),
    Other(i32),
}

fn classify(raw: i32) -> Status {
    let low = raw & 0x7f;
    if low == 0x7f {
        Status::Other(raw) // stopped
    } else if low == 0 {
        Status::Exited((raw >> 8) & 0xff)
    } else {
        Status::Signalled(low)
    }
}

/// Runs `f(hex, hex_maxlen, bin, bin_len)` in a forked child and returns the
/// child's termination status. The child does nothing but the FFI call and
/// `_exit(0)`, so the status is entirely determined by the callee.
fn status_of(f: &Bin2Hex, hex: *mut i8, hex_maxlen: usize, bin: *const u8, bin_len: usize) -> Status {
    // Flush so the child cannot duplicate buffered parent output.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // Child: async-signal-safe only — one FFI call, then _exit.
        unsafe {
            let r = f(hex, hex_maxlen, bin, bin_len);
            // Use the return value so the call cannot be optimized away, and
            // report whether it equalled `hex` via the exit code.
            if r == hex {
                _exit(0)
            } else if r.is_null() {
                _exit(2)
            } else {
                _exit(3)
            }
        }
    }
    let mut raw: i32 = 0;
    let w = unsafe { waitpid(pid, &mut raw, 0) };
    assert_eq!(w, pid, "waitpid failed");
    classify(raw)
}

/// The Phase-C differential assertion.
///
/// `expected` is the status derived from `ERRORS.md` (from reading the C
/// source). We assert C == Rust *and* that both equal the expected class, so a
/// test cannot silently pass by both implementations doing the wrong thing.
fn assert_same_status(
    label: &str,
    hex_maxlen: usize,
    bin_len: usize,
    hex_null: bool,
    bin_null: bool,
    expected: &[Status],
) -> Status {
    let l = libs();
    let cf = l.c_fn();
    let rf = l.rust_fn();

    // Generous real buffers so that *only* the C logic decides the outcome.
    let mut out = vec![GUARD; 4096];
    let input = vec![0x5Au8; 4096];

    let hex = if hex_null {
        std::ptr::null_mut()
    } else {
        out.as_mut_ptr() as *mut i8
    };
    let bin = if bin_null {
        std::ptr::null()
    } else {
        input.as_ptr()
    };

    let c = status_of(&cf, hex, hex_maxlen, bin, bin_len);
    let r = status_of(&rf, hex, hex_maxlen, bin, bin_len);

    assert_eq!(
        c, r,
        "[{label}] status divergence: C={c:?} Rust={r:?} \
         (hex_maxlen={hex_maxlen} bin_len={bin_len} hex_null={hex_null} bin_null={bin_null})"
    );
    assert!(
        expected.contains(&c),
        "[{label}] both agreed on {c:?} but ERRORS.md expects one of {expected:?}"
    );
    c
}

const SIZE_MAX: usize = usize::MAX;
const HALF: usize = SIZE_MAX / 2; // 0x7FFF_FFFF_FFFF_FFFF

// ------------------------------------------------------------------ row 1
#[test]
fn err01_bin_len_exactly_size_max_over_2() {
    // First term `bin_len >= SIZE_MAX/2` is true at the exact boundary.
    assert_same_status("err01", SIZE_MAX, HALF, false, false, &[Status::Signalled(SIGABRT)]);
}

// ------------------------------------------------------------------ row 2
#[test]
fn err02_bin_len_one_past_boundary() {
    assert_same_status(
        "err02",
        SIZE_MAX,
        HALF + 1, // 0x8000_0000_0000_0000
        false,
        false,
        &[Status::Signalled(SIGABRT)],
    );
}

// ------------------------------------------------------------------ row 3
#[test]
fn err03_bin_len_size_max() {
    assert_same_status("err03", SIZE_MAX, SIZE_MAX, false, false, &[Status::Signalled(SIGABRT)]);
}

// ------------------------------------------------------------------ row 4
#[test]
fn err04_largest_passing_bin_len_but_hex_maxlen_too_small() {
    // bin_len = SIZE_MAX/2 - 1 -> first term FALSE. bin_len*2 = 0xFFFF...FC.
    // hex_maxlen = 0 -> second term TRUE -> abort. Crucially the product must
    // NOT be treated as overflowing.
    assert_same_status(
        "err04",
        0,
        HALF - 1,
        false,
        false,
        &[Status::Signalled(SIGABRT)],
    );
}

// ------------------------------------------------------------------ row 5
#[test]
fn err05_hex_maxlen_exactly_bin_len_times_two() {
    // `<=` => no room for the NUL terminator => abort.
    for bin_len in [1usize, 2, 3, 7, 8, 15, 16, 100, 1000] {
        assert_same_status(
            "err05",
            bin_len * 2,
            bin_len,
            false,
            false,
            &[Status::Signalled(SIGABRT)],
        );
    }
}

// ------------------------------------------------------------------ row 6
#[test]
fn err06_hex_maxlen_less_than_bin_len_times_two() {
    let mut rng = Rng::new(0xE6);
    // hex_maxlen = 0 with bin_len = 4, plus randomized strictly-too-small values.
    assert_same_status("err06-zero", 0, 4, false, false, &[Status::Signalled(SIGABRT)]);
    for _ in 0..40 {
        let bin_len = rng.range(1, 256);
        let hex_maxlen = rng.range(0, bin_len * 2 - 1);
        assert_same_status(
            "err06-rand",
            hex_maxlen,
            bin_len,
            false,
            false,
            &[Status::Signalled(SIGABRT)],
        );
    }
}

// ------------------------------------------------------------------ row 7
#[test]
fn err07_empty_input_with_zero_hex_maxlen_aborts() {
    // bin_len = 0: second term is `0 <= 0` which is TRUE, so even the empty
    // input aborts when hex_maxlen == 0.
    assert_same_status("err07", 0, 0, false, false, &[Status::Signalled(SIGABRT)]);
}

// ------------------------------------------------------------------ row 8
#[test]
fn err08_empty_input_hex_maxlen_one_is_accepted() {
    // One step INTO the valid range: must NOT abort, child exits 0 having
    // returned `hex`.
    assert_same_status("err08", 1, 0, false, false, &[Status::Exited(0)]);
    // And the actual bytes written must match (checked in-process, no abort).
    assert_same("err08-bytes", 16, 0, 1, &[], 0);
}

// ------------------------------------------------------------------ row 9
#[test]
fn err09_minimum_accepted_hex_maxlen() {
    for bin_len in [1usize, 2, 3, 7, 8, 15, 16, 100, 1000] {
        assert_same_status(
            "err09",
            bin_len * 2 + 1,
            bin_len,
            false,
            false,
            &[Status::Exited(0)],
        );
    }
}

// ------------------------------------------------------------------ row 10
#[test]
fn err10_largest_passing_bin_len_passes_both_checks_then_faults() {
    // bin_len = SIZE_MAX/2 - 1 and hex_maxlen = SIZE_MAX: neither term rejects
    // (proving `bin_len * 2` is computed in size_t and does not overflow into a
    // rejection), so the loop is entered and both implementations run off the
    // end of the 4 KiB buffer identically.
    let s = assert_same_status(
        "err10",
        SIZE_MAX,
        HALF - 1,
        false,
        false,
        &[Status::Signalled(SIGSEGV), Status::Signalled(SIGBUS)],
    );
    assert_ne!(
        s,
        Status::Signalled(SIGABRT),
        "err10 must NOT be rejected by the argument checks"
    );
}

// ------------------------------------------------------------------ row 11
#[test]
fn err11_null_hex_is_not_checked() {
    // No null check in the C: the checks pass, then the store faults.
    let s = assert_same_status(
        "err11",
        64,
        8,
        true,
        false,
        &[Status::Signalled(SIGSEGV), Status::Signalled(SIGBUS)],
    );
    assert_ne!(s, Status::Signalled(SIGABRT), "err11 is not an abort in C");

    // Also with bin_len = 0: the terminator store `hex[0] = 0` still faults.
    let s0 = assert_same_status(
        "err11-empty",
        1,
        0,
        true,
        false,
        &[Status::Signalled(SIGSEGV), Status::Signalled(SIGBUS)],
    );
    assert_ne!(s0, Status::Signalled(SIGABRT));

    // But a null `hex` combined with a REJECTED length must still abort first
    // (short-circuit order: the checks precede any dereference).
    assert_same_status(
        "err11-abort-first",
        0,
        4,
        true,
        false,
        &[Status::Signalled(SIGABRT)],
    );
    assert_same_status(
        "err11-abort-first2",
        SIZE_MAX,
        SIZE_MAX,
        true,
        false,
        &[Status::Signalled(SIGABRT)],
    );
}

// ------------------------------------------------------------------ row 12
#[test]
fn err12_null_bin_with_nonzero_len_is_not_checked() {
    let s = assert_same_status(
        "err12",
        4096,
        64,
        false,
        true,
        &[Status::Signalled(SIGSEGV), Status::Signalled(SIGBUS)],
    );
    assert_ne!(s, Status::Signalled(SIGABRT), "err12 is not an abort in C");
}

// ------------------------------------------------------------------ row 13
#[test]
fn err13_null_bin_with_zero_len_is_fine() {
    // Loop body never runs, so the null `bin` is never dereferenced.
    assert_same_status("err13", 1, 0, false, true, &[Status::Exited(0)]);
    assert_same_status("err13-b", 4096, 0, false, true, &[Status::Exited(0)]);

    // Verify the written bytes too, in-process.
    let l = libs();
    let cf = l.c_fn();
    let rf = l.rust_fn();
    for hex_maxlen in [1usize, 2, 4096, usize::MAX] {
        let mut cbuf = vec![GUARD; 8];
        let mut rbuf = vec![GUARD; 8];
        unsafe {
            let cr = cf(cbuf.as_mut_ptr() as *mut i8, hex_maxlen, std::ptr::null(), 0);
            let rr = rf(rbuf.as_mut_ptr() as *mut i8, hex_maxlen, std::ptr::null(), 0);
            assert_eq!(cr as *mut u8, cbuf.as_mut_ptr());
            assert_eq!(rr as *mut u8, rbuf.as_mut_ptr());
        }
        assert_eq!(cbuf, rbuf, "err13 bytes differ (hex_maxlen={hex_maxlen})");
        assert_eq!(cbuf, [0, GUARD, GUARD, GUARD, GUARD, GUARD, GUARD, GUARD]);
    }
}

// ------------------------------------------------------------------ row 14
// `bin2hex` has no enum / flag / mode parameter (`char*`, `size_t`,
// `const uint8_t*`, `size_t`), so there is no out-of-range-enum input to
// smuggle across the FFI boundary. The full `size_t` domain around every
// boundary is covered by err01..err10 and the sweep below; the full `uint8_t`
// domain is covered exhaustively by CONFIGS.md rows 3-5.

// ------------------------------------------- extra: dense boundary sweep
#[test]
fn err_boundary_sweep_around_every_threshold() {
    // Walk one step either side of both thresholds and of assorted lengths,
    // asserting C and Rust agree on accept vs. abort at every single point.
    let mut cases: Vec<(usize, usize)> = Vec::new(); // (hex_maxlen, bin_len)

    // Threshold 1: bin_len vs SIZE_MAX/2.
    for d in 0..4usize {
        cases.push((SIZE_MAX, HALF - d));
        cases.push((SIZE_MAX, HALF + d));
        cases.push((0, HALF - d));
        cases.push((0, HALF + d));
        cases.push((SIZE_MAX, SIZE_MAX - d));
    }

    // Threshold 2: hex_maxlen vs bin_len*2, one step either side.
    for bin_len in [0usize, 1, 2, 3, 4, 5, 16, 17, 255, 256, 257, 1023, 1024] {
        let need = bin_len * 2;
        for hex_maxlen in [
            need.saturating_sub(1),
            need,
            need + 1,
            need + 2,
            need + 3,
            SIZE_MAX,
        ] {
            cases.push((hex_maxlen, bin_len));
        }
    }

    for (hex_maxlen, bin_len) in cases {
        // Predict from the C source, then require both to match it.
        let expect_abort = bin_len >= HALF || hex_maxlen <= bin_len.wrapping_mul(2);
        // Huge non-rejected lengths would fault; skip those here (err10 covers them).
        if !expect_abort && bin_len > 4096 / 2 - 1 {
            continue;
        }
        let expected = if expect_abort {
            Status::Signalled(SIGABRT)
        } else {
            Status::Exited(0)
        };
        assert_same_status(
            "err-sweep",
            hex_maxlen,
            bin_len,
            false,
            false,
            &[expected],
        );
    }
}
