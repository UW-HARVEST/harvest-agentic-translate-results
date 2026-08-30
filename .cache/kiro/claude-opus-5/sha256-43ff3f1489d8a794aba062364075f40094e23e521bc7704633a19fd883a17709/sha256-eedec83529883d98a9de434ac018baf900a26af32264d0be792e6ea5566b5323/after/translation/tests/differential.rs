//! Differential tests: the C `driver` and the Rust `driver` are both run as
//! subprocesses on identical stdin, and stdout, stderr and exit status are
//! compared byte for byte.
//!
//! The input classes below come from reading `c_src/src/main.c`. Every branch in
//! `main` is driven, plus the `fgets`/`sscanf` behaviours the loop depends on.

mod harness;

use harness::{assert_same, c_bin, run, rust_bin, Status};

// ---------------------------------------------------------------------------
// Loop termination: `if (!fgets(...)) break;`
// ---------------------------------------------------------------------------

/// Empty input. The menu is printed once, `fgets` returns NULL immediately, the
/// loop breaks and `main` returns 0.
#[test]
fn empty_input() {
    assert_same("empty", b"");
}

/// A single newline: `fgets` succeeds with "\n", `sscanf` converts nothing.
#[test]
fn single_newline() {
    assert_same("single newline", b"\n");
}

/// Input that ends without a trailing newline: `fgets` returns the partial line,
/// the demo runs, then the next `fgets` returns NULL.
#[test]
fn no_trailing_newline() {
    assert_same("no trailing newline on 6", b"6");
    assert_same("no trailing newline on 7", b"7");
    assert_same("no trailing newline, invalid", b"x");
}

/// EOF reached after a demo has run, without ever selecting Exit.
#[test]
fn eof_after_demo() {
    assert_same("eof after demo 1", b"1\n");
    assert_same("eof after demo 6", b"6\n");
}

// ---------------------------------------------------------------------------
// The switch: one test per `case`, plus `default`.
// ---------------------------------------------------------------------------

#[test]
fn each_demo_individually() {
    for choice in 1..=6 {
        let input = format!("{choice}\n7\n");
        assert_same(&format!("demo {choice}"), input.as_bytes());
    }
}

/// `case 7` is the only early `return 0` in `main`; it prints "\nGoodbye!\n" and
/// leaves the rest of stdin unread.
#[test]
fn exit_choice_leaves_stdin_unread() {
    assert_same("exit immediately", b"7\n");
    assert_same("exit before more input", b"7\n6\n6\n6\n");
}

/// `default:` -> "Invalid choice". Includes 0, negatives and out-of-range.
#[test]
fn invalid_choice_branch() {
    for token in ["0", "8", "9", "-1", "-0", "100", "2147483647", "-2147483648"] {
        let input = format!("{token}\n7\n");
        assert_same(&format!("invalid choice {token}"), input.as_bytes());
    }
}

/// `if (sscanf(input, "%d", &choice) != 1)` -> "Invalid input". Reached when the
/// line holds no convertible integer at all.
#[test]
fn invalid_input_branch() {
    for token in [
        "abc", "", " ", "   ", "\t", "-", "+", "--3", ".5", "x1", "#", "/", "\x0b", "\x0c",
    ] {
        let input = format!("{token}\n7\n");
        assert_same(&format!("invalid input {token:?}"), input.as_bytes());
    }
}

/// Every menu choice exercised in one session, mixed with both error branches.
#[test]
fn all_branches_in_one_session() {
    assert_same(
        "full sweep",
        b"1\n2\n3\n4\n5\n6\n0\n8\n-4\nabc\n\n \n7\n",
    );
}

/// Demos hold no state between runs, so repeating one must repeat its output.
#[test]
fn repeated_demos() {
    assert_same("demo 6 three times", b"6\n6\n6\n7\n");
    assert_same("demo 3 twice", b"3\n3\n7\n");
}

// ---------------------------------------------------------------------------
// `sscanf("%d")` conversion details.
// ---------------------------------------------------------------------------

/// Leading whitespace is skipped, an optional sign is accepted, and conversion
/// stops at the first non-digit.
#[test]
fn scanf_accepts_whitespace_sign_and_trailing_junk() {
    for token in [
        "  3", "\t3", "\x0b7", "\x0c7", " +3", "+3", "007", "3abc", "3 4", "7 junk here",
        "0x10", "1e5", "  \t +6",
    ] {
        let input = format!("{token}\n7\n");
        assert_same(&format!("scanf {token:?}"), input.as_bytes());
    }
}

/// `\r\n` line endings: `\r` is not stripped by `fgets`, but `%d` stops at it.
#[test]
fn carriage_return_line_endings() {
    assert_same("crlf exit", b"7\r\n");
    assert_same("crlf sweep", b"1\r\n6\r\n0\r\n7\r\n");
    assert_same("lone cr", b"\r\n7\n");
}

/// glibc converts the digit run with `strtol` and assigns the low 32 bits to an
/// `int`. These inputs pin down truncation and `long` saturation.
#[test]
fn scanf_integer_truncation_and_saturation() {
    for token in [
        "2147483648",           // INT_MAX + 1 -> INT_MIN
        "-2147483649",          // INT_MIN - 1 -> INT_MAX
        "4294967297",           // wraps to 1  -> demo 1
        "4294967303",           // wraps to 7  -> Goodbye
        "8589934592",           // wraps to 0  -> Invalid choice
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // LONG_MAX + 1, saturates
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809", // LONG_MIN - 1, saturates
        "99999999999999999999999999",
        "-99999999999999999999999999",
        "00000000000000000000000007",
    ] {
        let input = format!("{token}\n7\n");
        assert_same(&format!("scanf overflow {token}"), input.as_bytes());
    }
}

/// `fgets` copies NUL bytes into the buffer; `sscanf` then treats them as the
/// end of the string.
#[test]
fn embedded_nul_bytes() {
    assert_same("nul then digit", b"\x006\n7\n");
    assert_same("digit then nul", b"6\x00\n7\n");
    assert_same("nul between digits", b"1\x002\n7\n");
    assert_same("many nuls", &[b'\0'; 260]);
}

/// Bytes outside ASCII are not whitespace and not digits, so they land in the
/// "Invalid input" branch.
#[test]
fn non_ascii_bytes() {
    assert_same("high bytes", b"\xff\xfe\n7\n");
    assert_same("utf8 bom then digit", "\u{feff}7\n".as_bytes());
    assert_same("latin1 soup", b"\xc3\xa9\xc3\xa8\n6\n7\n");
}

// ---------------------------------------------------------------------------
// `char input[256]` / `fgets(input, sizeof(input), stdin)` boundary.
// ---------------------------------------------------------------------------

/// A line of exactly 254 characters plus '\n' fits in one `fgets` (255 bytes
/// read, one byte left for the NUL).
#[test]
fn line_exactly_fills_buffer() {
    let mut input = vec![b' '; 254];
    input[253] = b'7';
    input.push(b'\n');
    assert_same("254 chars + newline", &input);
}

/// A 255-character line means `fgets` stops one byte short: the '\n' is left in
/// the stream and becomes the next (empty, invalid) line.
#[test]
fn line_one_byte_over_buffer() {
    let mut input = vec![b' '; 255];
    input[254] = b'7';
    input.push(b'\n');
    assert_same("255 chars + newline", &input);
}

/// A line far longer than the buffer is delivered as several `fgets` results,
/// each of which is validated separately.
#[test]
fn line_much_longer_than_buffer() {
    let mut input = vec![b' '; 300];
    input.extend_from_slice(b"7\n");
    assert_same("300 spaces then 7", &input);

    let mut input = b"3".to_vec();
    input.extend(std::iter::repeat(b'x').take(300));
    input.extend_from_slice(b"\n7\n");
    assert_same("digit then 300 junk chars", &input);

    let mut input = vec![b'1'; 400];
    input.extend_from_slice(b"\n7\n");
    assert_same("400 digits", &input);

    // Fills the buffer with digits exactly, no newline anywhere.
    assert_same("255 sevens, no newline", &vec![b'7'; 255]);
    assert_same("1000 sevens, no newline", &vec![b'7'; 1000]);
}

/// A long run of blank lines before a real choice.
#[test]
fn many_blank_lines() {
    let mut input = vec![b'\n'; 50];
    input.extend_from_slice(b"7\n");
    assert_same("50 blank lines", &input);
}

// ---------------------------------------------------------------------------
// Exit status, including death by signal.
// ---------------------------------------------------------------------------

/// Nothing in the C program writes to stderr, and it always exits 0 when its
/// output is fully consumed. Assert that directly rather than only relatively.
#[test]
fn stderr_is_empty_and_exit_is_zero() {
    for input in [&b""[..], b"7\n", b"6\n7\n", b"abc\n", b"0\n"] {
        let c = run(c_bin(), input);
        let r = run(rust_bin(), input);
        assert!(c.stderr.is_empty(), "C wrote to stderr for {input:?}");
        assert!(r.stderr.is_empty(), "Rust wrote to stderr for {input:?}");
        assert_eq!(c.status.code, Some(0), "C exit code for {input:?}");
        assert_eq!(r.status.code, Some(0), "Rust exit code for {input:?}");
    }
}

/// When the consumer of stdout goes away mid-write, the C program is killed by
/// `SIGPIPE`. The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main`,
/// so this only matches because the translation restores the default.
#[cfg(unix)]
#[test]
fn broken_stdout_pipe_matches() {
    // Enough output that the child must block on a full pipe rather than
    // finishing before the reader closes.
    let input = b"6\n".repeat(400);

    let c = harness::run_with_early_close(c_bin(), &input, 64);
    let r = harness::run_with_early_close(rust_bin(), &input, 64);

    assert_eq!(
        c,
        Status {
            code: None,
            signal: Some(13),
        },
        "expected the C program to die from SIGPIPE"
    );
    assert_eq!(c, r, "exit status differs when stdout is closed early");
}

// ---------------------------------------------------------------------------
// Unreadable stdin: the other way `fgets` returns NULL.
// ---------------------------------------------------------------------------

/// `/dev/null` on stdin: `fgets` hits EOF on the very first call.
#[test]
fn stdin_is_dev_null() {
    let c = harness::run_without_stdin(c_bin(), false);
    let r = harness::run_without_stdin(rust_bin(), false);
    assert_eq!(c.stdout, r.stdout, "stdout differs with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs with /dev/null stdin");
    assert_eq!(c.status, r.status, "status differs with /dev/null stdin");
}

/// File descriptor 0 closed: `fgets` fails with `EBADF` and returns NULL, which
/// the loop treats the same as end of input.
#[cfg(unix)]
#[test]
fn stdin_closed() {
    let c = harness::run_without_stdin(c_bin(), true);
    let r = harness::run_without_stdin(rust_bin(), true);
    assert_eq!(c.stdout, r.stdout, "stdout differs with stdin closed");
    assert_eq!(c.stderr, r.stderr, "stderr differs with stdin closed");
    assert_eq!(c.status, r.status, "status differs with stdin closed");
}

// ---------------------------------------------------------------------------
// Scale.
// ---------------------------------------------------------------------------

/// Several megabytes of output and a 100 KB single "line", to exercise the
/// buffered writer and the `fgets` refill path well past their chunk sizes.
#[test]
fn megabyte_scale_session() {
    let mut input = b"6\n".repeat(500);
    input.extend(std::iter::repeat(b'1').take(100_000));
    input.extend_from_slice(b"\n7\n");
    assert_same("500 full demos plus a 100KB line", &input);
}
