//! Phase C — error-path differential tests. One test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejecting condition and asserts that C and Rust
//! produce the *same* rejection — the same message bytes and the same exit
//! status — not merely that both "failed somehow".

mod common;
use common::ffi::{Call, MixedOp};
use common::{exe, ffi, Rng, SEED};

/// The four message strings the C can emit on a rejection. Tests assert against
/// these literally so a silently-reworded message cannot pass.
const MSG_FGETS_FAILED: &[u8] = b"fgets() failed.\n";
const MSG_NEGATIVE: &[u8] = b"ERROR: Array index is negative.\n";
const MSG_OOB: &[u8] = b"ERROR: Array index is out-of-bounds\n";

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// Rows 1-3, 21: printLine's `line != NULL` guard
// ---------------------------------------------------------------------------

/// Rows 1 & 21 — a NULL pointer must print *nothing at all*.
#[test]
fn err_printline_null() {
    let (c, r) = ffi::both(&Call::PrintLine(None), b"");
    assert_eq!(c, r, "printLine(NULL) diverged: C={c:?} Rust={r:?}");
    assert!(
        c.stdout.is_empty(),
        "printLine(NULL) must emit nothing, C emitted {c:?}"
    );
    assert_eq!(c.status, Ok(0), "printLine(NULL) must return normally");

    // And the guard must not suppress a *later* legitimate call.
    let ops = [
        MixedOp::Line(b"before"),
        MixedOp::Int(1),
        MixedOp::Line(b"after"),
    ];
    ffi::assert_same(&Call::Mixed(&ops), b"", "calls surrounding a NULL");
}

/// Row 2 — a valid pointer to an empty string still prints the newline.
#[test]
fn err_printline_empty() {
    let (c, r) = ffi::both(&Call::PrintLine(Some(b"")), b"");
    assert_eq!(c, r, "printLine(\"\") diverged");
    assert_eq!(c.stdout, b"\n", "printLine(\"\") must emit exactly one newline");
}

/// Row 3 — non-UTF-8 bytes pass through verbatim.
#[test]
fn err_printline_non_utf8() {
    for s in [&b"\xff"[..], b"\xfe\xff", b"\x80\x80", b"\xc0\x80", b"\xed\xa0\x80"] {
        let (c, r) = ffi::both(&Call::PrintLine(Some(s)), b"");
        assert_eq!(c, r, "printLine non-UTF-8 diverged for {s:?}");
        let mut want = s.to_vec();
        want.push(b'\n');
        assert_eq!(c.stdout, want, "bytes were not passed through verbatim");
    }
}

// ---------------------------------------------------------------------------
// Rows 4-5, 7-9: the fgets-NULL and index-guard rejections
// ---------------------------------------------------------------------------

/// Row 4 — `bad()`'s `fgets` returns NULL at EOF, and `data` keeps its -1.
#[test]
fn err_bad_fgets_eof() {
    let (c, r) = ffi::both(&Call::Bad, b"");
    assert_eq!(c, r, "bad() at EOF diverged");
    assert!(
        contains(&c.stdout, MSG_FGETS_FAILED),
        "expected the fgets failure message, got {c:?}"
    );
    // ...which then falls through to the negative-index rejection (row 5).
    assert!(
        contains(&c.stdout, MSG_NEGATIVE),
        "expected the negative-index message after the fgets failure, got {c:?}"
    );
}

/// Row 5 — `data < 0` in `bad()` prints the negative message and no values.
#[test]
fn err_bad_negative() {
    let mut rng = Rng::new(SEED ^ 105);
    let mut cases: Vec<i64> = vec![-1, -2, -10, -1000, i32::MIN as i64];
    for _ in 0..40 {
        cases.push(rng.in_range(i32::MIN as i64, -1));
    }
    for k in cases {
        let stdin = format!("{k}\n");
        let (c, r) = ffi::both(&Call::Bad, stdin.as_bytes());
        assert_eq!(c, r, "bad({k}) diverged");
        assert_eq!(
            c.stdout, MSG_NEGATIVE,
            "bad({k}) must print only the negative message"
        );
    }
}

/// Row 7 — `goodB2G`'s `fgets` returns NULL at EOF. `goodG2B` still runs first.
#[test]
fn err_b2g_fgets_eof() {
    let (c, r) = ffi::both(&Call::Good, b"");
    assert_eq!(c, r, "good() at EOF diverged");
    assert!(
        contains(&c.stdout, MSG_FGETS_FAILED),
        "expected the fgets failure message, got {c:?}"
    );
    assert!(
        contains(&c.stdout, MSG_OOB),
        "expected goodB2G's out-of-bounds message, got {c:?}"
    );
    // goodG2B is unconditional and prints ten values with a 1 at index 7.
    assert!(
        contains(&c.stdout, b"0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n"),
        "goodG2B's output is missing, got {c:?}"
    );
}

/// Row 8 — `goodB2G` rejects a negative index (first conjunct of the guard).
#[test]
fn err_b2g_negative() {
    let mut rng = Rng::new(SEED ^ 108);
    let mut cases: Vec<i64> = vec![-1, -2, i32::MIN as i64];
    for _ in 0..40 {
        cases.push(rng.in_range(i32::MIN as i64, -1));
    }
    for k in cases {
        let stdin = format!("{k}\n");
        let (c, r) = ffi::both(&Call::Good, stdin.as_bytes());
        assert_eq!(c, r, "good({k}) diverged");
        assert!(
            contains(&c.stdout, MSG_OOB),
            "good({k}) must reject with the out-of-bounds message, got {c:?}"
        );
        assert!(
            !contains(&c.stdout, MSG_NEGATIVE),
            "goodB2G must use the out-of-bounds wording, not the negative one"
        );
    }
}

/// Row 9 — `goodB2G` rejects `data >= 10` (second conjunct) *without* the
/// out-of-bounds write, so even huge indices are safe here.
#[test]
fn err_b2g_too_large() {
    let mut rng = Rng::new(SEED ^ 109);
    let mut cases: Vec<i64> = vec![10, 11, 16, 26, 27, 100, 100_000, i32::MAX as i64];
    for _ in 0..60 {
        cases.push(rng.in_range(10, i32::MAX as i64));
    }
    for k in cases {
        let stdin = format!("{k}\n");
        let (c, r) = ffi::both(&Call::Good, stdin.as_bytes());
        assert_eq!(c, r, "good({k}) diverged");
        assert_eq!(c.status, Ok(0), "goodB2G is bounds-checked and must not crash");
        assert!(
            contains(&c.stdout, MSG_OOB),
            "good({k}) must reject with the out-of-bounds message, got {c:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 6: the missing upper-bound check in bad() -- the CWE itself
// ---------------------------------------------------------------------------

/// Row 6 — `bad()` has *no* upper-bound check, so `data >= 10` writes out of
/// bounds. Asserted here for the caller-independent classes; the full class
/// breakdown lives in `tests/exe_diff.rs` rows 12–17.
#[test]
fn err_bad_oob_no_upper_bound_check() {
    // 10..=15 land in dead storage: the ten values still print, all zero, and the
    // out-of-bounds message is never emitted because no such check exists.
    for k in 10..=15 {
        let stdin = format!("{k}\n");
        let (c, r) = ffi::both(&Call::Bad, stdin.as_bytes());
        assert_eq!(c, r, "bad({k}) diverged");
        assert_eq!(
            c.stdout, b"0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n",
            "bad({k}) must print ten zeros"
        );
        assert!(
            !contains(&c.stdout, MSG_OOB),
            "bad() has no bounds check and must never print the out-of-bounds message"
        );
    }
    // 18..=19 clobber bad()'s own *return address*: fatal for any caller.
    // (16..=17 clobber only the saved rbp, which an optimized caller such as this
    // harness never reloads, so C survives them here -- see imp.rs.)
    for k in 18..=19 {
        let stdin = format!("{k}\n");
        let (c, r) = ffi::both(&Call::Bad, stdin.as_bytes());
        assert_eq!(c, r, "bad({k}) diverged");
        assert!(c.status.is_err(), "bad({k}) must be fatal, got {c:?}");
        assert!(c.stdout.is_empty(), "bad({k}) must lose its buffered output");
    }
}

/// Row 6, continued — the boundary between "valid index" and "out of bounds" is
/// exactly 9 / 10, and index 9 must still write inside the array.
#[test]
fn err_bad_oob_boundary_is_exactly_ten() {
    let (c9, r9) = ffi::both(&Call::Bad, b"9\n");
    assert_eq!(c9, r9, "bad(9) diverged");
    assert_eq!(
        c9.stdout, b"0\n0\n0\n0\n0\n0\n0\n0\n0\n1\n",
        "index 9 is the last in-bounds slot"
    );
    let (c10, r10) = ffi::both(&Call::Bad, b"10\n");
    assert_eq!(c10, r10, "bad(10) diverged");
    assert_eq!(
        c10.stdout, b"0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n",
        "index 10 is past the end, so no printed slot changes"
    );
}

// ---------------------------------------------------------------------------
// Rows 10-15: atoi's non-rejections and truncation
// ---------------------------------------------------------------------------

/// Row 10 — unparseable text is NOT an error: `atoi` yields 0, a valid index.
#[test]
fn err_atoi_unparseable() {
    for s in ["abc", "x", "+", "-", ".5", "0x10", "e5", "--5", "++5", " a", "\t\tz"] {
        let stdin = format!("{s}\n");
        let (c, r) = ffi::both(&Call::Bad, stdin.as_bytes());
        assert_eq!(c, r, "bad({s:?}) diverged");
        assert_eq!(
            c.stdout, b"1\n0\n0\n0\n0\n0\n0\n0\n0\n0\n",
            "{s:?} must behave exactly like index 0, got {c:?}"
        );
    }
}

/// Row 11 — a bare newline is a *successful* fgets, and `atoi("\n")` is 0.
#[test]
fn err_atoi_empty_line() {
    let (c, r) = ffi::both(&Call::Bad, b"\n");
    assert_eq!(c, r, "bad(\"\\n\") diverged");
    assert!(
        !contains(&c.stdout, MSG_FGETS_FAILED),
        "a bare newline is not an fgets failure, got {c:?}"
    );
    assert_eq!(
        c.stdout, b"1\n0\n0\n0\n0\n0\n0\n0\n0\n0\n",
        "atoi(\"\\n\") must be 0"
    );
}

/// Rows 12-14 — values beyond `int` are truncated, not rejected.
#[test]
fn err_atoi_int_truncation() {
    // 9999999999999 fits in a long; (int) keeps the low 32 bits.
    let expect = 9_999_999_999_999i64 as i32;
    assert!(expect > 10, "sanity: truncation yields a large positive index");
    for s in ["9999999999999", "4294967296", "4294967306", "2147483648"] {
        // goodB2G is bounds-checked, so the truncated value is safely observable.
        let stdin = format!("{s}\n");
        let (c, r) = ffi::both(&Call::Good, stdin.as_bytes());
        assert_eq!(c, r, "good({s}) diverged");
    }
    // 4294967296 == 2^32 truncates to exactly 0, which is a *valid* index.
    let (c, r) = ffi::both(&Call::Bad, b"4294967296\n");
    assert_eq!(c, r, "bad(2^32) diverged");
    assert_eq!(
        c.stdout, b"1\n0\n0\n0\n0\n0\n0\n0\n0\n0\n",
        "2^32 must truncate to index 0"
    );
    // 4294967301 == 2^32 + 5 truncates to 5.
    let (c, r) = ffi::both(&Call::Bad, b"4294967301\n");
    assert_eq!(c, r, "bad(2^32+5) diverged");
    assert_eq!(
        c.stdout, b"0\n0\n0\n0\n0\n1\n0\n0\n0\n0\n",
        "2^32+5 must truncate to index 5"
    );
}

/// Row 13 — the negative side of truncation.
#[test]
fn err_atoi_neg_truncation() {
    for s in ["-9999999999999", "-999999999999", "-4294967296", "-2147483649"] {
        let stdin = format!("{s}\n");
        let (c, r) = ffi::both(&Call::Bad, stdin.as_bytes());
        assert_eq!(c, r, "bad({s}) diverged");
    }
    // -4294967296 == -(2^32) truncates to 0 -- positive, so NOT the negative path.
    let (c, _) = ffi::both(&Call::Bad, b"-4294967296\n");
    assert_eq!(
        c.stdout, b"1\n0\n0\n0\n0\n0\n0\n0\n0\n0\n",
        "-(2^32) must truncate to index 0, not be rejected as negative"
    );
    // Truncation does NOT preserve the sign: atoi("-999999999999") is +727379969,
    // so goodB2G rejects it as out-of-bounds rather than as negative. Asserted
    // explicitly because it is exactly the kind of claim that is easy to get wrong.
    assert_eq!(
        (-999_999_999_999i64) as i32,
        727_379_969,
        "sanity check of the truncation arithmetic itself"
    );
    let (c, r) = ffi::both(&Call::Good, b"-999999999999\n");
    assert_eq!(c, r, "good(-999999999999) diverged");
    assert!(
        contains(&c.stdout, MSG_OOB),
        "the truncated value is positive, so goodB2G must report out-of-bounds, got {c:?}"
    );
    assert!(
        !contains(&c.stdout, MSG_NEGATIVE),
        "the truncated value is positive; the negative path must NOT be taken"
    );
}

/// Row 14 — INT_MAX and INT_MIN pass through `atoi` unchanged.
#[test]
fn err_atoi_int_limits() {
    let (c, r) = ffi::both(&Call::Bad, b"-2147483648\n");
    assert_eq!(c, r, "bad(INT_MIN) diverged");
    assert_eq!(c.stdout, MSG_NEGATIVE, "INT_MIN must take the negative path");

    // INT_MAX in the bounds-checked sink, where it is safely observable.
    let (c, r) = ffi::both(&Call::Good, b"2147483647\n");
    assert_eq!(c, r, "good(INT_MAX) diverged");
    assert!(
        contains(&c.stdout, MSG_OOB),
        "INT_MAX must be rejected as out of bounds by goodB2G"
    );
}

/// Row 15 — `fgets` keeps bytes after an embedded NUL, but `atoi` stops there.
#[test]
fn err_atoi_embedded_nul() {
    let (c, r) = ffi::both(&Call::Bad, b"5\x006\n");
    assert_eq!(c, r, "bad(\"5\\0 6\") diverged");
    assert_eq!(
        c.stdout, b"0\n0\n0\n0\n0\n1\n0\n0\n0\n0\n",
        "atoi must stop at the NUL and yield 5, got {c:?}"
    );
    // A leading NUL yields the empty string -> 0.
    let (c, r) = ffi::both(&Call::Bad, b"\x005\n");
    assert_eq!(c, r, "bad(\"\\0 5\") diverged");
    assert_eq!(
        c.stdout, b"1\n0\n0\n0\n0\n0\n0\n0\n0\n0\n",
        "a leading NUL must make atoi return 0"
    );
}

// ---------------------------------------------------------------------------
// Rows 16-18: fgets truncation and EOF between the two reads
// ---------------------------------------------------------------------------

/// Row 16 — a line longer than 13 bytes is truncated and the remainder is left
/// for the *next* `fgets`, so one long line feeds both sinks.
#[test]
fn err_fgets_truncation() {
    // 13 zeros make goodB2G see index 0; the tail "7" makes bad() see index 7.
    exe::assert_same(b"00000000000007\n", "14 bytes: 13 zeros + tail 7");
    // Exactly 13 bytes: the newline is NOT consumed, so bad() reads just "\n".
    exe::assert_same(b"0000000000000\n5\n", "exactly 13 bytes then a line");
    let (c, _) = exe::both(b"00000000000007\n");
    assert!(
        contains(&c.stdout, b"0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n"),
        "bad() should have received the truncated tail 7, got {c:?}"
    );
}

/// Row 17 — a final line with no newline is a successful `fgets`, not an error.
#[test]
fn err_fgets_no_newline() {
    for s in [&b"5"[..], b"5\n3", b"-1", b"abc"] {
        exe::assert_same(s, "unterminated final line");
    }
    let (c, _) = exe::both(b"5\n3");
    assert!(
        !contains(&c.stdout, MSG_FGETS_FAILED),
        "an unterminated line is not an fgets failure, got {c:?}"
    );
}

/// Row 18 — only one line of input: `goodB2G` takes it, `bad()` hits EOF.
#[test]
fn err_bad_second_fgets_eof() {
    let (c, r) = exe::both(b"5\n");
    assert_eq!(c, r, "single-line input diverged");
    assert!(
        contains(&c.stdout, MSG_FGETS_FAILED),
        "bad()'s fgets must fail, got {c:?}"
    );
    assert!(
        contains(&c.stdout, MSG_NEGATIVE),
        "and then reject the -1 index, got {c:?}"
    );
    // Exactly one "fgets() failed." -- goodB2G's read succeeded.
    let n = c
        .stdout
        .windows(MSG_FGETS_FAILED.len())
        .filter(|w| *w == MSG_FGETS_FAILED)
        .count();
    assert_eq!(n, 1, "expected exactly one fgets failure, saw {n}");
}

// ---------------------------------------------------------------------------
// Row 19: goodG2B's dead else-branch
// ---------------------------------------------------------------------------

/// Row 19 — `goodG2B` hardcodes `data = 7`, so its `else` is unreachable and its
/// output is identical for every possible input.
#[test]
fn err_g2b_else_unreachable() {
    let expected: &[u8] = b"0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n";
    for stdin in [&b""[..], b"\n", b"-1\n", b"5\n", b"99999\n", b"abc\n"] {
        let (c, r) = ffi::both(&Call::Good, stdin);
        assert_eq!(c, r, "good() diverged for {stdin:?}");
        assert!(
            c.stdout.starts_with(expected),
            "goodG2B must always print a 1 at index 7 first, got {c:?}"
        );
        assert!(
            !contains(&c.stdout, MSG_NEGATIVE),
            "goodG2B's negative branch is dead code and must never be reached"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 20, 22-26: generic FFI-boundary cases
// ---------------------------------------------------------------------------

/// Row 20 — a non-crashing run always exits 0.
#[test]
fn err_main_returns_zero() {
    for stdin in [&b""[..], b"5\n5\n", b"-1\n-1\n", b"abc\nabc\n", b"\n\n"] {
        let (c, r) = exe::both(stdin);
        assert_eq!(c, r, "diverged for {stdin:?}");
        assert_eq!(c.status, Ok(0), "expected exit 0 for {stdin:?}, got {c:?}");
    }
}

/// Rows 22-24 — `printIntLine` at the extremes of `int`, including a bit pattern
/// with the sign bit set.
#[test]
fn err_printintline_extremes() {
    for v in [0i32, -1, 1, i32::MAX, i32::MIN, i32::MIN + 1, -2147483648] {
        let (c, r) = ffi::both(&Call::PrintIntLine(v), b"");
        assert_eq!(c, r, "printIntLine({v}) diverged");
        assert_eq!(
            c.stdout,
            format!("{v}\n").into_bytes(),
            "printIntLine({v}) formatting is wrong"
        );
    }
    // 0x80000000 reinterpreted as a C int is INT_MIN; %d prints it as negative.
    let v = 0x8000_0000u32 as i32;
    let (c, r) = ffi::both(&Call::PrintIntLine(v), b"");
    assert_eq!(c, r, "printIntLine(0x80000000) diverged");
    assert_eq!(c.stdout, b"-2147483648\n");
}

/// Row 25 — `main` ignores `argc`/`argv`, so `argc = 0, argv = NULL` is fine.
#[test]
fn err_main_null_argv() {
    for stdin in [&b""[..], b"3\n4\n", b"-1\n-1\n"] {
        let (c, r) = ffi::both(&Call::Main { with_args: false }, stdin);
        assert_eq!(c, r, "main(0, NULL) diverged for {stdin:?}");
        assert_eq!(c.status, Ok(0), "main(0, NULL) must still exit 0");
    }
}

/// Row 26 — stdin closed outright (not merely at EOF) still takes the NULL path.
#[test]
fn err_stdin_closed() {
    // `Stdio::null()` gives an immediately-EOF stdin; a *closed* fd 0 makes the
    // read fail with EBADF. Both must reach the same "fgets() failed." path.
    use std::process::{Command, Stdio};
    common::ensure_built();
    let mut outs = Vec::new();
    for exe in [common::c_exe(), common::rust_exe()] {
        let out = Command::new(&exe)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("spawn");
        outs.push((out.status.code(), out.stdout));
    }
    assert_eq!(outs[0], outs[1], "closed/empty stdin diverged");
    assert!(
        contains(&outs[0].1, MSG_FGETS_FAILED),
        "expected the fgets failure path, got {:?}",
        String::from_utf8_lossy(&outs[0].1)
    );
}

/// There are no enum parameters anywhere in this C API, so the "out-of-range enum
/// value" class reduces to "arbitrary `int` reaching a value-dispatched path".
/// That path is the array index, swept exhaustively over the deterministic range
/// and randomized over the whole of `i32`.
#[test]
fn err_arbitrary_int_across_ffi_boundary() {
    let mut rng = Rng::new(SEED ^ 126);
    // goodB2G's sink is bounds-checked, so *any* i32 is safe to compare there.
    for _ in 0..250 {
        let v = rng.i32_any();
        let stdin = format!("{v}\n");
        ffi::assert_same(&Call::Good, stdin.as_bytes(), &format!("good({v})"));
    }
    // bad()'s sink: sweep every caller-independent class exhaustively.
    for v in -4..=19 {
        let stdin = format!("{v}\n");
        ffi::assert_same(&Call::Bad, stdin.as_bytes(), &format!("bad({v})"));
    }
}
