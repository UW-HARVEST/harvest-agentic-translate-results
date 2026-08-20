//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of ERRORS.md, in order. Each one constructs the exact
//! invalid input or condition, drives BOTH the C and the Rust artifact through
//! their FFI / process boundary, and asserts they reject it identically (same
//! bytes, same exit code, same terminating signal) — never merely "both failed".

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Rows 1-7: `printLine`'s NULL guard and its argument surface (channel S)
// ---------------------------------------------------------------------------

/// Row 1 — `printLine(NULL)`: the `if (line != NULL)` guard rejects the call and
/// nothing at all is written.
#[test]
fn err_printline_null() {
    let c = so_print_line(Side::C, None);
    let r = so_print_line(Side::Rust, None);
    assert_bytes_eq("row1/printLine(NULL)", b"<NULL>", &c, &r);
    assert!(
        c.is_empty(),
        "C printLine(NULL) must write 0 bytes, wrote {:?}",
        pretty(&c)
    );
    assert!(r.is_empty(), "Rust printLine(NULL) must write 0 bytes");
}

/// Row 2 — empty (zero-length) string: passes the guard, `printf("%s\n","")`.
#[test]
fn err_printline_empty() {
    let c = so_print_line(Side::C, Some(b""));
    let r = so_print_line(Side::Rust, Some(b""));
    assert_bytes_eq("row2/printLine(\"\")", b"", &c, &r);
    assert_eq!(c, b"\n", "C printLine(\"\") must print exactly one newline");
}

/// Row 3 — the payload is *data*, never a format string.
#[test]
fn err_printline_format_specifiers() {
    for p in [
        &b"%s"[..],
        b"%d",
        b"%n",
        b"%%",
        b"%999999d",
        b"%s %d %n %% %p %x",
        b"%",
        b"%%%%%%",
        b"100%",
        b"%c%c%c%c%c%c%c%c",
        b"%hn%hhn%lln",
    ] {
        assert_so_print_line_same("row3/format-specifiers", Some(p));
    }
}

/// Row 4 — non-UTF-8 payloads; `printf("%s")` is byte-oriented.
#[test]
fn err_printline_non_utf8() {
    for p in [
        &b"\xff"[..],
        b"\xff\xfe\x80",
        b"\x80\x81\x82\x83",
        b"\xc3",            // truncated 2-byte sequence
        b"\xe2\x82",        // truncated 3-byte sequence
        b"\xf0\x9f\x92",    // truncated 4-byte sequence
        b"ok\xffmore\xfe",  // valid + invalid mixed
        b"\xed\xa0\x80",    // surrogate half
        b"\xf4\x90\x80\x80" // above U+10FFFF
    ] {
        assert_so_print_line_same("row4/non-utf8", Some(p));
    }
}

/// Row 5 — embedded whitespace / control bytes are copied verbatim.
#[test]
fn err_printline_embedded_newline() {
    for p in [
        &b"\n"[..],
        b"\n\n\n",
        b"a\nb",
        b"\r\n",
        b"a\rb\tc\x0bd\x0ce",
        b"trailing\n",
        b"\x01\x02\x03\x04\x05\x06\x07\x08",
        b"\x7f",
    ] {
        assert_so_print_line_same("row5/embedded-whitespace", Some(p));
    }
}

/// Row 6 — oversized payload (64 KiB), far past any stdio buffer.
#[test]
fn err_printline_oversized() {
    let big = vec![b'A'; 65536];
    let c = so_print_line(Side::C, Some(&big));
    let r = so_print_line(Side::Rust, Some(&big));
    assert_bytes_eq("row6/oversized-64KiB", &big, &c, &r);
    assert_eq!(c.len(), 65537, "64 KiB payload + newline");
}

/// Row 7 — every single non-NUL byte value as a one-byte string (255 cases).
#[test]
fn err_printline_every_byte() {
    for b in 1u8..=255 {
        let payload = [b];
        let c = so_print_line(Side::C, Some(&payload));
        let r = so_print_line(Side::Rust, Some(&payload));
        assert_bytes_eq(&format!("row7/byte-0x{b:02x}"), &payload, &c, &r);
        assert_eq!(c, [b, b'\n'], "C must print the byte then a newline");
    }
}

// ---------------------------------------------------------------------------
// Rows 8-18: every way `scanf("%d", &x)` fails or surprises (channel E)
// ---------------------------------------------------------------------------

/// Row 8 — EOF before any character: `scanf` returns `EOF`, stores nothing.
#[test]
fn err_scanf_eof_empty() {
    assert_exe_same("row8/empty-stdin", b"");
    assert_exe_same_with(
        "row8/devnull-stdin",
        b"",
        StdinKind::DevNull,
        StdoutKind::Pipe,
    );
}

/// Row 9 — whitespace only, then EOF: the `%d` skip loop runs into EOF.
#[test]
fn err_scanf_whitespace_only_eof() {
    for s in [
        " ", "  ", "\t", "\n", "\r", "\x0b", "\x0c", " \t\n\x0b\x0c\r", "\n\n\n\n",
        "                                                                ",
    ] {
        assert_exe_same_str("row9/whitespace-only", s);
    }
    // A whitespace run longer than any plausible stdio buffer.
    assert_exe_same("row9/whitespace-8193", &vec![b' '; 8193]);
}

/// Row 10 — matching failure: the first non-space byte can never start a `%d`.
#[test]
fn err_scanf_matching_failure() {
    let cases: [&[u8]; 18] = [
        b"abc", b"a", b".", b"x", b"/", b":", b"\x00", b"\x00123", b"!", b"~",
        b"e5", b"E5", b"#1", b"'1'", b"\"5\"", b"\xff", b"\x80\x81", b"[]",
    ];
    for c in cases {
        assert_exe_same("row10/matching-failure", c);
    }
}

/// Row 11 — sign consumed, then EOF.
#[test]
fn err_scanf_sign_then_eof() {
    assert_exe_same_str("row11/minus-eof", "-");
    assert_exe_same_str("row11/plus-eof", "+");
    assert_exe_same_str("row11/ws-minus-eof", "   -");
    assert_exe_same_str("row11/ws-plus-eof", "\n\t+");
}

/// Row 12 — sign followed by something that is not a digit.
#[test]
fn err_scanf_sign_then_nondigit() {
    for s in [
        "-a", "+.", "--1", "++1", "+-1", "-+1", "- 5", "+ 5", "-\n5", "+\t5",
        "-.5", "+.5", "-x10", "+e3", "-\x00", "-#",
    ] {
        assert_exe_same_str("row12/sign-then-nondigit", s);
    }
}

/// Row 13 — positive overflow of glibc's `long` accumulator (clamped to LONG_MAX,
/// then truncated to `int` => -1 => non-zero => `good()`).
#[test]
fn err_scanf_overflow_positive() {
    for s in [
        "9223372036854775808",  // LONG_MAX + 1
        "9223372036854775809",
        "18446744073709551615", // ULONG_MAX
        "18446744073709551616", // 2^64 — would be 0 if glibc wrapped
        "99999999999999999999",
        "170141183460469231731687303715884105728", // 2^127
    ] {
        assert_exe_same_str("row13/positive-overflow", s);
    }
    // Digit runs far past any buffer boundary.
    for n in [100usize, 1000, 4095, 4096, 5000] {
        let s = vec![b'9'; n];
        assert_exe_same(&format!("row13/{n}-nines"), &s);
        let mut s2 = vec![b'1'];
        s2.extend(std::iter::repeat(b'0').take(n));
        assert_exe_same(&format!("row13/1e{n}"), &s2);
    }
}

/// Row 14 — negative overflow (clamped to LONG_MIN, whose low word is 0 =>
/// `x == 0` => `bad()`).
#[test]
fn err_scanf_overflow_negative() {
    for s in [
        "-9223372036854775809", // LONG_MIN - 1
        "-9223372036854775810",
        "-18446744073709551616",
        "-99999999999999999999",
        "-170141183460469231731687303715884105728",
    ] {
        assert_exe_same_str("row14/negative-overflow", s);
    }
    for n in [100usize, 1000, 5000] {
        let mut s = vec![b'-'];
        s.extend(std::iter::repeat(b'9').take(n));
        assert_exe_same(&format!("row14/minus-{n}-nines"), &s);
    }
}

/// Row 15 — `%d` truncates the `long` to `int`; it does not saturate.
#[test]
fn err_scanf_int_truncation() {
    for s in [
        "4294967296",  // 2^32      -> low word 0 -> bad()
        "-4294967296",
        "8589934592",  // 2^33      -> low word 0 -> bad()
        "4294967297",  // 2^32 + 1  -> 1          -> good()
        "2147483648",  // 2^31      -> INT_MIN    -> good()
        "-2147483648",
        "4294967295",  // 2^32 - 1  -> -1         -> good()
        "-4294967295",
        "1099511627776",
        "281474976710656",
    ] {
        assert_exe_same_str("row15/int-truncation", s);
    }
}

/// Row 16 — the `int` boundaries and one step past them; `%d` never
/// range-checks, so none of these is rejected.
#[test]
fn err_scanf_int_boundaries() {
    for s in [
        "2147483647",  // INT_MAX
        "2147483648",  // INT_MAX + 1
        "-2147483648", // INT_MIN
        "-2147483649", // INT_MIN - 1
        "2147483646",
        "-2147483647",
    ] {
        assert_exe_same_str("row16/int-boundary", s);
    }
}

/// Row 17 — every spelling that yields the falsy value 0.
#[test]
fn err_scanf_zero_forms() {
    for s in [
        "0", "-0", "+0", "00", "000", "-00", "+000", "0x10", "0X10", "0abc",
        "0.5", "0 1", "0\n1", "0e5",
    ] {
        assert_exe_same_str("row17/zero-forms", s);
    }
    assert_exe_same("row17/5000-zeros", &vec![b'0'; 5000]);
    let mut s = vec![b'-'];
    s.extend(std::iter::repeat(b'0').take(5000));
    assert_exe_same("row17/minus-5000-zeros", &s);
}

/// Row 18 — fd 0 closed: every read fails with `EBADF`, so `scanf` fails.
#[test]
fn err_stdin_closed() {
    assert_exe_same_with(
        "row18/stdin-closed",
        b"",
        StdinKind::Closed,
        StdoutKind::Pipe,
    );
}

// ---------------------------------------------------------------------------
// Rows 19-20: output-side failures, whose status main.c discards
// ---------------------------------------------------------------------------

/// Row 19 — fd 1 closed: `printf` fails with `EBADF`, the result is discarded,
/// the exit status stays 0 and nothing is said on stderr.
#[test]
fn err_stdout_closed() {
    for input in [&b"1"[..], b"0", b"abc", b""] {
        assert_exe_same_with(
            "row19/stdout-closed",
            input,
            StdinKind::Pipe,
            StdoutKind::Closed,
        );
    }
}

/// Row 20 — fd 1 is a pipe with no reader: the write raises `SIGPIPE`.
///
/// This is the row that caught a genuine divergence: Rust's runtime installs
/// `SIG_IGN` for `SIGPIPE` before `main`, so the untreated translation exited 0
/// where the C program is killed by signal 13. `src/main.rs` now restores
/// `SIG_DFL`.
#[test]
fn err_stdout_broken_pipe() {
    for input in [&b"1"[..], b"0", b"abc", b"", b"-5"] {
        assert_exe_same_with(
            "row20/stdout-broken-pipe",
            input,
            StdinKind::Pipe,
            StdoutKind::BrokenPipe,
        );
    }
    // And make sure the C really does die from the signal, i.e. that the row is
    // testing what it claims to test.
    let c = run_exe(&c_exe(), b"1", StdinKind::Pipe, StdoutKind::BrokenPipe);
    assert_eq!(
        c.signal,
        Some(13),
        "expected the C executable to be killed by SIGPIPE, got {c:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 21: the exit status
// ---------------------------------------------------------------------------

/// Row 21 — `main` has exactly one `return 0;` and no `exit()`: every input,
/// valid or not, exits 0 (unless a signal kills the process, row 20).
#[test]
fn err_exit_status_always_zero() {
    for input in [
        &b""[..],
        b"0",
        b"1",
        b"-1",
        b"abc",
        b"   ",
        b"-",
        b"99999999999999999999",
        b"\x00\x01\x02",
        b"\xff\xfe",
    ] {
        // channel E
        let c = run_exe(&c_exe(), input, StdinKind::Pipe, StdoutKind::Pipe);
        let r = run_exe(&rust_exe(), input, StdinKind::Pipe, StdoutKind::Pipe);
        assert_eq!(c.code, Some(0), "C exit code for {:?}", pretty(input));
        assert_eq!(r.code, Some(0), "Rust exit code for {:?}", pretty(input));
        assert_eq!(c.signal, None);
        assert_eq!(r.signal, None);
        // channel S: the `int` returned across the FFI boundary
        assert_so_main_same("row21/so-main-status", input);
    }
}

// ---------------------------------------------------------------------------
// Row 22: the uninitialised read in bad()
// ---------------------------------------------------------------------------

/// Row 22, executable form — the artifact `c_src/CMakeLists.txt` builds. Here the
/// undefined value is a non-NULL pointer to a NUL byte, so exactly `"\n"` comes
/// out, and the Rust translation reproduces that byte-for-byte.
#[test]
fn err_bad_uninitialised_via_exe() {
    for input in [&b"0"[..], b"", b"abc", b"-0", b"4294967296", b"   ", b"-"] {
        assert_exe_same("row22/bad-path", input);
        let c = run_exe(&c_exe(), input, StdinKind::Pipe, StdoutKind::Pipe);
        assert_eq!(
            c.stdout,
            b"\n",
            "the C executable's bad() path must print exactly one newline for {:?}",
            pretty(input)
        );
    }
}

/// Row 22, shared-library form — the *documented* UB divergence.
///
/// Called in isolation (rather than from `main`, the only way the program itself
/// reaches it), the C `bad()` dereferences whatever its caller left in that stack
/// slot. Three different outcomes have been measured from this one `main.c`:
/// `"\n"` (executable), a run of the library's own machine code, and `SIGSEGV`
/// (release-profile test binary). There is therefore no C behaviour here for any
/// translation to match, and asserting one would be asserting an artefact of a
/// particular link.
///
/// What this test does pin down:
/// * the **Rust** side is deterministic and equals the behaviour of the artifact
///   being reproduced — exactly `"\n"`, clean exit;
/// * the C side's outcome is captured and printed as the evidence behind that
///   claim (the crash cannot take the test process down: the call is made in a
///   forked child).
///
/// The *real* differential assertion for `bad()` lives where the C is
/// well-defined and reproducible: reached through `main`, in
/// `err_bad_uninitialised_via_exe` (executable) and `cfg_so_main_bad_path`
/// (shared library), both of which do assert byte equality.
#[test]
fn err_bad_uninitialised_ub_documented() {
    let (c, c_status) = so_call_bad_tolerant(Side::C, 1);
    let (r, r_status) = so_call_bad_tolerant(Side::Rust, 1);

    // The Rust side must be deterministic and match the reproduced artifact.
    assert!(
        r_status.is_clean(),
        "Rust bad() must not crash, but {}",
        r_status.describe()
    );
    assert_eq!(
        r, b"\n",
        "Rust bad() must print exactly the newline the C *executable* prints"
    );

    eprintln!(
        "row22 (documented UB) isolated bad():\n  \
         C   : {} — {} bytes {:?} hex [{}]\n  \
         Rust: {} — {} bytes {:?} hex [{}]",
        c_status.describe(),
        c.len(),
        pretty(&c),
        hex(&c),
        r_status.describe(),
        r.len(),
        pretty(&r),
        hex(&r)
    );

    // Whatever the C did, it must be one of the UB outcomes ERRORS.md records:
    // either it survived and emitted a single NUL-terminated line, or it was
    // killed by a memory-access signal.
    match c_status.signal() {
        Some(sig) => assert!(
            sig == 11 || sig == 7 || sig == 10,
            "unexpected fatal signal {sig} from C bad()"
        ),
        None => assert!(
            c.ends_with(b"\n") && c.iter().filter(|&&b| b == b'\n').count() == 1,
            "a surviving C bad() emits exactly one line, got {:?}",
            pretty(&c)
        ),
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary coverage required beyond the table
// ---------------------------------------------------------------------------

/// The C API takes no enum parameter anywhere (`grep -c enum c_src/src/main.c`
/// is 0), so the "invalid enum value across FFI" class has no instance. What the
/// boundary does carry is one `const char *` and one `int` return — both are
/// pinned down here so the claim is checked rather than asserted in prose.
#[test]
fn ffi_boundary_surface_is_exactly_pointer_in_int_out() {
    let src = std::fs::read_to_string(manifest_dir().join("c_src/src/main.c")).expect("read main.c");
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !code.contains("enum"),
        "the C source grew an enum; ERRORS.md's invalid-enum row must be revisited"
    );

    // `int main(void)` across the boundary: 0, for both libraries.
    for so in [c_so(), rust_so()] {
        let out = run_so_subprocess(&so, "main", None, b"1");
        assert_eq!(out.code, Some(0), "{} main() must return 0", so.display());
    }

    // NULL and the zero-length string are the two degenerate pointer values.
    assert_so_print_line_same("ffi/null", None);
    assert_so_print_line_same("ffi/empty", Some(b""));
}

/// Calling the exports repeatedly must not accumulate state on either side.
#[test]
fn ffi_repeated_calls_have_no_hidden_state() {
    for times in [1usize, 2, 7, 64] {
        let c = so_call_void(Side::C, "good", times);
        let r = so_call_void(Side::Rust, "good", times);
        assert_bytes_eq(&format!("ffi/good-x{times}"), b"", &c, &r);
        assert_eq!(c.len(), times * 7, "`string\\n` per call");
    }
    for times in [1usize, 5, 32] {
        let mut c_all = Vec::new();
        let mut r_all = Vec::new();
        for _ in 0..times {
            c_all.extend(so_print_line(Side::C, Some(b"abc")));
            r_all.extend(so_print_line(Side::Rust, Some(b"abc")));
        }
        assert_bytes_eq(&format!("ffi/printLine-x{times}"), b"abc", &c_all, &r_all);
    }
}
