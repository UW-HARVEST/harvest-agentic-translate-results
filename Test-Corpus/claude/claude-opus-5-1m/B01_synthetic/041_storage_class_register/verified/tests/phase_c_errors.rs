//! Phase C — error-path differential tests: **one test per row of `ERRORS.md`**.
//!
//! Each test constructs that row's exact invalid input/condition, runs *both*
//! implementations, and asserts
//!
//! 1. they agree on everything observable (`stdout`, `stderr`, exit code,
//!    terminating signal), and
//! 2. the result is the *specific* sentinel the C produces (e.g. `300\n`
//!    because the rejected conversion leaves `x` at its initialiser `0`, or
//!    death by signal 13 for a closed pipe) — never merely "both failed".
//!
//! The Rust side is only ever reached through the executable's process boundary
//! or through the `.so`'s exported symbols.

mod common;

use common::*;
use std::path::Path;

/// `2*0 + 300` — what the C prints whenever `scanf` rejects the input and `x`
/// keeps the initialiser it was given in `int x = 0;`.
const REJECTED: &[u8] = b"300\n";

fn pair() -> (std::path::PathBuf, std::path::PathBuf) {
    (c_exe(), rust_exe_release())
}

/// Runs one payload through both executables, asserts equality, and asserts the
/// exact expected `stdout`.
#[track_caller]
fn expect(row: &str, stdin: &[u8], want_stdout: &[u8]) {
    let (c, r) = pair();
    let co = run(&c, stdin);
    let ro = run(&r, stdin);
    assert_same_outcome(row, &format!("stdin={}", show(stdin)), &co, &ro);
    assert_eq!(
        show(&co.stdout),
        show(want_stdout),
        "[{row}] C produced an unexpected sentinel for stdin={}",
        show(stdin)
    );
    assert_eq!(co.code, Some(0), "[{row}] C exit code changed");
    assert_eq!(co.signal, None, "[{row}] C died from a signal unexpectedly");
}

#[track_caller]
fn expect_cfg(
    row: &str,
    stdin: &[u8],
    sk: StdinKind,
    ok: StdoutKind,
    want_stdout: &[u8],
    want_code: Option<i32>,
    want_signal: Option<i32>,
) {
    let (c, r) = pair();
    let co = run_cfg(&c, &[], stdin, sk, ok, &[]);
    let ro = run_cfg(&r, &[], stdin, sk, ok, &[]);
    assert_same_outcome(row, &format!("stdin={sk:?} stdout={ok:?}"), &co, &ro);
    assert_eq!(
        show(&co.stdout),
        show(want_stdout),
        "[{row}] unexpected stdout"
    );
    assert_eq!(co.code, want_code, "[{row}] unexpected exit code");
    assert_eq!(co.signal, want_signal, "[{row}] unexpected signal");
}

// ===========================================================================
// Rows 1–4: `scanf` input failures (the stream cannot supply a character).
// ===========================================================================

/// Row 1 — immediate EOF: empty `stdin`.
#[test]
fn err01_input_failure_empty_stdin() {
    expect("err01", b"", REJECTED);
    expect_cfg(
        "err01/devnull",
        b"",
        StdinKind::DevNull,
        StdoutKind::Pipe,
        REJECTED,
        Some(0),
        None,
    );
}

/// Row 2 — EOF reached while skipping leading whitespace.
#[test]
fn err02_input_failure_whitespace_only() {
    for p in [
        &b" "[..],
        b"   \n\t ",
        b"\n",
        b"\t\t",
        b"\r\n",
        b"\x0b\x0c",
        &[b' '; 5000][..],
    ] {
        expect("err02", p, REJECTED);
    }
}

/// Row 3 — `stdin` closed outright: every `read` fails with `EBADF`.
#[test]
fn err03_input_failure_stdin_closed() {
    expect_cfg(
        "err03",
        b"42",
        StdinKind::Closed,
        StdoutKind::Pipe,
        REJECTED,
        Some(0),
        None,
    );
}

/// Row 4 — `stdin` is a directory: every `read` fails with `EISDIR`.
#[test]
fn err04_input_failure_stdin_is_directory() {
    expect_cfg(
        "err04",
        b"42",
        StdinKind::Directory,
        StdoutKind::Pipe,
        REJECTED,
        Some(0),
        None,
    );
}

// ===========================================================================
// Rows 5–10: `scanf` matching failures (a character is available but cannot
// start a `%d` conversion). `x` must keep its initialiser.
// ===========================================================================

/// Row 5 — first non-space byte is a letter.
#[test]
fn err05_matching_failure_letter() {
    for p in [
        &b"abc"[..],
        b"Z",
        b"x42",
        b"  \n hello 42",
        b"nan",
        b"inf",
        b"nil",
        b"(nil)",
    ] {
        expect("err05", p, REJECTED);
    }
}

/// Row 6 — first non-space byte is punctuation.
#[test]
fn err06_matching_failure_punctuation() {
    for p in [
        &b".5"[..], b"-.5", b"*", b"/", b"#", b",5", b"'42", b"\"42\"", b"[42]", b"%d", b"\\42",
        b"~", b"=42",
    ] {
        expect("err06", p, REJECTED);
    }
}

/// Row 7 — a NUL byte is a perfectly ordinary input byte that is neither
/// `isspace` nor `isdigit`.
#[test]
fn err07_matching_failure_nul_byte() {
    for p in [&b"\0"[..], b"\0 42", b"  \0 42", b"\0\0\0", b"\042"] {
        expect("err07", p, REJECTED);
    }
}

/// Row 8 — bytes ≥ 0x80 are not whitespace in the `"C"` locale.
#[test]
fn err08_matching_failure_high_bytes() {
    for p in [
        &b"\x80"[..],
        b"\x80\x8142",
        b"\xff42",
        b"\xa0 42",              // NBSP in latin-1
        "\u{a0}42".as_bytes(),   // NBSP in UTF-8
        "٣".as_bytes(),          // ARABIC-INDIC DIGIT THREE
        "０".as_bytes(),         // FULLWIDTH DIGIT ZERO
        "\u{feff}42".as_bytes(), // BOM
    ] {
        expect("err08", p, REJECTED);
    }
}

/// Row 9 — a lone sign followed by EOF: glibc's work buffer holds only the
/// sign, which is an explicit `conv_error`.
#[test]
fn err09_matching_failure_lone_sign_then_eof() {
    for p in [&b"-"[..], b"+", b"   -", b"\n+"] {
        expect("err09", p, REJECTED);
    }
}

/// Row 10 — a sign followed by something that is not a digit.
#[test]
fn err10_matching_failure_sign_then_non_digit() {
    for p in [
        &b"-abc"[..],
        b"+abc",
        b"- 42",
        b"+ 42",
        b"-\n42",
        b"--5",
        b"++5",
        b"+-5",
        b"-+5",
        b"-.5",
        b"-\0",
        b"-\xff",
    ] {
        expect("err10", p, REJECTED);
    }
}

// ===========================================================================
// Rows 11–17: range and truncation behaviour of the conversion.
// ===========================================================================

/// Row 11 — magnitude past `LONG_MAX`: `strtol` clamps to `LONG_MAX`, whose low
/// 32 bits are `-1`, so the program prints `2*(-1)+300 == 298`.
#[test]
fn err11_erange_positive_clamps_to_long_max() {
    for p in [
        &b"9223372036854775808"[..],
        b"9223372036854775809",
        b"99999999999999999999",
        b"18446744073709551616",
        b"+99999999999999999999",
    ] {
        expect("err11", p, b"298\n");
    }
    // Far past any internal buffer.
    expect("err11/huge", &vec![b'9'; 1_000_000], b"298\n");
}

/// Row 12 — magnitude past `|LONG_MIN|`: clamps to `LONG_MIN`, whose low 32
/// bits are `0`, so the program prints `300`.
#[test]
fn err12_erange_negative_clamps_to_long_min() {
    for p in [
        &b"-9223372036854775809"[..],
        b"-9223372036854775810",
        b"-99999999999999999999",
        b"-18446744073709551616",
    ] {
        expect("err12", p, REJECTED);
    }
    expect("err12/huge", &[b"-".as_ref(), &vec![b'7'; 1_000_000]].concat(), REJECTED);
}

/// Row 13 — the exact `long` boundaries convert without clamping but are still
/// truncated into the `int`.
#[test]
fn err13_long_boundaries_truncate() {
    expect("err13/LONG_MAX", b"9223372036854775807", b"298\n");
    expect("err13/LONG_MIN", b"-9223372036854775808", REJECTED);
}

/// Row 14 — in-`long` but out-of-`int` values keep only their low 32 bits.
#[test]
fn err14_out_of_int_range_truncates() {
    // INT_MAX + 1 -> INT_MIN ; 2*INT_MIN wraps to 0 ; +300
    expect("err14/int_max+1", b"2147483648", REJECTED);
    // 2^32 -> 0
    expect("err14/2^32", b"4294967296", REJECTED);
    // 2^32 - 1 -> -1
    expect("err14/2^32-1", b"4294967295", b"298\n");
    // INT_MIN - 1 -> INT_MAX
    expect("err14/int_min-1", b"-2147483649", b"298\n");
}

/// Row 15 — with `%d` the base is fixed at 10, so a `0x` prefix is *rejected*
/// after the leading `0` has already been accepted.
#[test]
fn err15_hex_prefix_rejected() {
    for p in [&b"0x10"[..], b"0X10", b"0x", b"0xff", b"-0x10", b"0b101"] {
        expect("err15", p, REJECTED);
    }
}

/// Row 16 — no `'` flag, so a thousands separator terminates the number.
#[test]
fn err16_grouping_rejected() {
    expect("err16/1,000", b"1,000", b"302\n");
    expect("err16/1'000", b"1'000", b"302\n");
    expect("err16/1.000", b"1.000", b"302\n");
    expect("err16/12_000", b"12_000", b"324\n");
}

/// Row 17 — trailing junk is simply not consumed.
#[test]
fn err17_trailing_junk_truncates() {
    expect("err17/42abc", b"42abc", b"384\n");
    expect("err17/1e5", b"1e5", b"302\n");
    expect("err17/42 99", b"42 99", b"384\n");
    expect("err17/42\\n99", b"42\n99", b"384\n");
    expect("err17/0.9", b"0.9", REJECTED);
}

// ===========================================================================
// Rows 18–19: the output side fails.
// ===========================================================================

/// Row 18 — fd 1 closed: `printf` returns `-1`, the C ignores it, exit status 0.
#[test]
fn err18_stdout_closed_is_ignored() {
    expect_cfg(
        "err18",
        b"42",
        StdinKind::File,
        StdoutKind::Closed,
        b"",
        Some(0),
        None,
    );
}

/// Row 19 — writing into a pipe whose read end is gone: the default `SIGPIPE`
/// disposition terminates the process. Rust's runtime installs `SIG_IGN`, so
/// this row is exactly the divergence a naive translation exhibits.
#[test]
fn err19_stdout_closed_pipe_raises_sigpipe() {
    let (c, r) = pair();
    for payload in [&b"42"[..], b"", b"abc", b"-1"] {
        let co = run_cfg(
            &c,
            &[],
            payload,
            StdinKind::File,
            StdoutKind::ClosedPipe,
            &[],
        );
        let ro = run_cfg(
            &r,
            &[],
            payload,
            StdinKind::File,
            StdoutKind::ClosedPipe,
            &[],
        );
        assert_same_outcome("err19", &format!("payload={}", show(payload)), &co, &ro);
        assert_eq!(
            co.signal,
            Some(13),
            "[err19] expected C to die from SIGPIPE, got {co:?}"
        );
        assert_eq!(co.code, None, "[err19] expected no exit code");
        assert_eq!(
            ro.signal,
            Some(13),
            "[err19] expected Rust to die from SIGPIPE, got {ro:?}"
        );
    }
}

/// Row 23 — the same closed pipe, but `SIGPIPE` was already `SIG_IGN` when the
/// program was `exec`ed (inherited dispositions survive `execve`).
///
/// Here the C program must **not** die: it gets `EPIPE`, ignores it, and exits
/// `0`. Forcing `SIG_DFL` unconditionally to fix row 19 would break exactly this
/// case, so both directions are pinned.
#[test]
fn err23_inherited_sigpipe_ign_is_preserved() {
    let (c, r) = pair();
    for payload in [&b"42"[..], b"", b"abc", b"-2147483648"] {
        let co = run_closed_pipe_with_sigpipe_ignored(&c, payload);
        let ro = run_closed_pipe_with_sigpipe_ignored(&r, payload);
        assert_same_outcome("err23", &format!("payload={}", show(payload)), &co, &ro);
        assert_eq!(
            co.signal, None,
            "[err23] C must not die when SIGPIPE is inherited as SIG_IGN: {co:?}"
        );
        assert_eq!(co.code, Some(0), "[err23] C must exit 0: {co:?}");
        assert_eq!(
            ro.signal, None,
            "[err23] Rust re-armed SIG_DFL and died where C survived: {ro:?}"
        );
    }
}

/// Row 24 — a failing allocation must not turn into an abort.
///
/// glibc's `printf`/`scanf` fall back to the `FILE`'s one-byte `_shortbuf` when
/// `malloc` fails and keep working; Rust's allocation-failure handler prints to
/// `stderr` and raises `SIGABRT`. The translation is therefore allocation-free.
#[test]
fn err24_tight_address_space_limit_does_not_abort() {
    let (c, r) = pair();
    // Every limit here is large enough for the dynamic loader to map both
    // binaries (the Rust binary is larger and needs a few more objects, which no
    // translation can change), and small enough that a 4 KiB or 1 KiB heap
    // buffer would have failed.
    for mb in [4u64, 6, 8, 12, 16, 32, 64] {
        let bytes = mb * 1024 * 1024;
        for payload in [&b"42rest"[..], b"-2147483648", b"abc", b""] {
            let co = run_with_address_space_limit(&c, payload, bytes);
            let ro = run_with_address_space_limit(&r, payload, bytes);
            assert_same_outcome(
                "err24",
                &format!("RLIMIT_AS={mb}MiB payload={}", show(payload)),
                &co,
                &ro,
            );
            assert_eq!(
                ro.signal, None,
                "[err24] Rust aborted under RLIMIT_AS={mb}MiB: {ro:?}"
            );
            assert!(
                ro.stderr.is_empty(),
                "[err24] Rust wrote to stderr under RLIMIT_AS={mb}MiB: {}",
                show(&ro.stderr)
            );
        }
    }
}

// ===========================================================================
// Rows 20–21: the arithmetic in `driver` overflows (UB in C; the target wraps).
// Exercised here through the executable; `phase_c_ffi` repeats them through the
// exported symbol.
// ===========================================================================

/// Row 20 — `2*x` overflows `int`.
#[test]
fn err20_multiply_overflow_wraps() {
    expect("err20/int_min", b"-2147483648", REJECTED);
    expect("err20/int_max", b"2147483647", b"298\n");
    expect("err20/2^30", b"1073741824", b"-2147483348\n");
    expect("err20/-2^30", b"-1073741824", b"-2147483348\n");
    expect("err20/2^31-1", b"2147483647", b"298\n");
}

/// Row 21 — `2*x` fits but `y += 300` overflows.
#[test]
fn err21_add_overflow_wraps() {
    // 2*1073741823 = 2147483646; +300 wraps.
    expect("err21/max_div2", b"1073741823", b"-2147483350\n");
    // The last x for which no overflow happens: 2x + 300 == INT_MAX -> x = 1073741673 (odd part)
    expect("err21/edge-", b"1073741673", b"2147483646\n");
    expect("err21/edge+", b"1073741674", b"-2147483648\n");
}

// ===========================================================================
// Row 22 + generic FFI boundaries, through the external loader process.
// ===========================================================================

/// Row 22 — extreme `c_int` arguments handed to the exported `driver` symbol by
/// a real external consumer (`dlopen` + `dlsym` from a C program).
#[test]
fn err22_driver_extreme_args_via_loader() {
    let loader = loader_exe();
    let cso = c_so().to_string_lossy().to_string();
    let rso = rust_so().to_string_lossy().to_string();
    for x in [
        0i32,
        -1,
        1,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0xDEAD_BEEFu32 as i32,
        0x7FFF_FFFF,
        1_073_741_824,
        -1_073_741_824,
    ] {
        let arg = x.to_string();
        let co = run_args(&loader, &[&cso, "driver", &arg], b"");
        let ro = run_args(&loader, &[&rso, "driver", &arg], b"");
        assert_same_outcome("err22", &format!("driver({x})"), &co, &ro);
        assert_eq!(co.code, Some(0), "[err22] loader exit code for driver({x})");
    }
}

/// Generic boundary: the exported `main` of both `.so`s must reject the same
/// inputs identically when driven by an external loader (not just the
/// executables).
#[test]
fn generic_so_main_rejects_identically() {
    let loader = loader_exe();
    let cso = c_so().to_string_lossy().to_string();
    let rso = rust_so().to_string_lossy().to_string();
    for p in [
        &b""[..],
        b"   ",
        b"abc",
        b"-",
        b"+",
        b"- 42",
        b"\0 42",
        b"\xff",
        b"0x10",
        b"1,000",
        b"9223372036854775808",
        b"-9223372036854775809",
        b"2147483648",
        b"4294967296",
    ] {
        let co = run_args(&loader, &[&cso, "main"], p);
        let ro = run_args(&loader, &[&rso, "main"], p);
        assert_same_outcome("generic/so-main", &format!("stdin={}", show(p)), &co, &ro);
    }
}

/// Generic boundary: oversized input (far past every internal buffer) and
/// zero-length input, in the same test so the pair is always compared.
#[test]
fn generic_zero_and_oversized_input() {
    let (c, r) = pair();
    let zero: Vec<u8> = Vec::new();
    let mut oversized = vec![b' '; 100_000];
    oversized.extend_from_slice(b"-424242");
    oversized.extend_from_slice(&vec![b'x'; 100_000]);
    let mut all_digits = vec![b'1'; 2_000_000];
    all_digits.push(b'\n');
    for p in [zero, oversized, all_digits] {
        assert_same_exe("generic/size", &c, &r, &p);
    }
}

/// Generic boundary: one step past every documented valid range, on both sides
/// of the boundary, so an off-by-one in the accumulator would show up.
#[test]
fn generic_one_step_past_every_range() {
    let (c, r) = pair();
    let mut payloads: Vec<Vec<u8>> = Vec::new();
    for centre in [
        0i128,
        i32::MAX as i128,
        i32::MIN as i128,
        u32::MAX as i128,
        u32::MAX as i128 + 1,
        i64::MAX as i128,
        i64::MIN as i128,
        u64::MAX as i128,
        i32::MAX as i128 / 2,
        i32::MIN as i128 / 2,
    ] {
        for d in [-2i128, -1, 0, 1, 2] {
            payloads.push(format!("{}", centre + d).into_bytes());
        }
    }
    for p in &payloads {
        assert_same_exe("generic/one-past", &c, &r, p);
    }
}

/// Sanity check that the two executables actually exist and are distinct
/// programs, so a mis-wired path cannot make the whole suite vacuously pass.
#[test]
fn harness_compares_two_distinct_programs() {
    let (c, r) = pair();
    assert!(Path::new(&c).is_file(), "C executable missing: {c:?}");
    assert!(Path::new(&r).is_file(), "Rust executable missing: {r:?}");
    assert_ne!(c, r, "both sides resolved to the same binary");
    let co = run(&c, b"7");
    assert_eq!(co.stdout, b"314\n", "C reference output changed");
}
