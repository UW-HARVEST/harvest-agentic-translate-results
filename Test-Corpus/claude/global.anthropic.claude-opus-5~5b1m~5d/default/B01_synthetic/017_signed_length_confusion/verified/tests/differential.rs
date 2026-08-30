//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin, and require byte-identical stdout, stderr
//! and identical exit status (including death-by-signal).
//!
//! The Rust code is never used as a library here; only the built binary is driven.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Repository root: the directory containing both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C executable, building it with CMake on first use.
fn c_binary() -> &'static PathBuf {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build_dir = c_src.join("build");
        let bin = build_dir.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build_dir).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );
            let build = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("run cmake --build");
            assert!(
                build.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }
        assert!(bin.exists(), "C binary not found at {}", bin.display());
        bin
    })
}

/// Path to the Rust executable under test (built by cargo before tests run).
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The child may die (SIGSEGV) before draining stdin; a broken pipe here
        // is expected and must not fail the test.
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => Err(out.status.signal().expect("exited by signal")),
    };
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Assert the C program and the Rust program agree on stdout, stderr and status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_binary(), stdin_bytes);
    let r = run(&rust_binary(), stdin_bytes);

    assert_eq!(
        c.status, r.status,
        "[{label}] exit status differs for input {:?}: C={:?} Rust={:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "[{label}] stdout differs for input {:?}",
        show(stdin_bytes)
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "[{label}] stderr differs for input {:?}",
        show(stdin_bytes)
    );
    // Byte-exactness (show() is only for readable diffs).
    assert_eq!(c.stdout, r.stdout, "[{label}] stdout bytes differ");
    assert_eq!(c.stderr, r.stderr, "[{label}] stderr bytes differ");
}

// ---------------------------------------------------------------------------
// fgets() branch: EOF with nothing read -> NULL -> "fgets() failed." and data
// stays -1, which then reaches the negative-length strncpy (fatal signal, so
// the buffered message is never actually written).
// ---------------------------------------------------------------------------

#[test]
fn empty_input_fgets_fails() {
    assert_same("empty", b"");
}

#[test]
fn lone_newline_is_read_successfully() {
    // fgets() succeeds with "\n"; atoi("\n") == 0.
    assert_same("lone newline", b"\n");
}

// ---------------------------------------------------------------------------
// data < 0 branch: strncpy() receives a negative length as a huge size_t.
// ---------------------------------------------------------------------------

#[test]
fn negative_one() {
    assert_same("-1", b"-1");
}

#[test]
fn negative_with_leading_space() {
    assert_same(" -5", b" -5\n");
}

#[test]
fn int_min() {
    assert_same("INT_MIN", b"-2147483648");
}

#[test]
fn negative_via_int_truncation() {
    // strtol() returns 4294967295, (int) of that is -1.
    assert_same("4294967295", b"4294967295");
}

#[test]
fn long_overflow_saturates_negative() {
    // 13 chars: strtol saturates to LONG_MAX; (int)LONG_MAX == -1.
    assert_same("9999999999999", b"9999999999999");
}

// ---------------------------------------------------------------------------
// 0 <= data < 100 branch: strncpy copies `data` 'A's.
// ---------------------------------------------------------------------------

#[test]
fn zero_copies_nothing() {
    assert_same("0", b"0");
}

#[test]
fn negative_zero() {
    assert_same("-0", b"-0");
}

#[test]
fn single_item() {
    assert_same("1", b"1");
}

#[test]
fn small_value_with_newline() {
    assert_same("5\\n", b"5\n");
}

#[test]
fn max_handled_length() {
    // 99 == the largest length the `data < 100` branch accepts; dest[99] = '\0'
    // is the last in-bounds byte of dest[100].
    assert_same("99", b"99");
}

#[test]
fn ninety_eight() {
    assert_same("98", b"98");
}

#[test]
fn ninety_seven_with_newline() {
    assert_same("97\\n", b"97\n");
}

#[test]
fn zero_via_int_truncation() {
    // strtol -> 8589934592 (2^33), (int) of that is 0.
    assert_same("8589934592", b"8589934592");
}

#[test]
fn small_positive_via_int_truncation() {
    // strtol -> 4294967301, (int) of that is 5.
    assert_same("4294967301", b"4294967301");
}

// ---------------------------------------------------------------------------
// data >= 100 branch: strncpy is skipped entirely, dest stays "" -> blank line.
// ---------------------------------------------------------------------------

#[test]
fn exactly_one_hundred() {
    assert_same("100", b"100");
}

#[test]
fn just_over_one_hundred() {
    assert_same("101", b"101");
}

#[test]
fn int_max() {
    assert_same("INT_MAX", b"2147483647");
}

// ---------------------------------------------------------------------------
// atoi() parsing corner cases (non-numeric prefixes, partial parses).
// ---------------------------------------------------------------------------

#[test]
fn non_numeric_input() {
    assert_same("abc", b"abc");
}

#[test]
fn thirteen_non_digits_fills_buffer() {
    assert_same("13 A's", b"AAAAAAAAAAAAA");
}

#[test]
fn only_spaces() {
    assert_same("spaces", b"     ");
}

#[test]
fn sign_with_no_digits() {
    assert_same("-", b"-");
}

#[test]
fn plus_sign() {
    assert_same("+8", b"+8");
}

#[test]
fn leading_whitespace_and_trailing_junk() {
    assert_same("   7  ", b"   7  ");
}

#[test]
fn leading_tab() {
    assert_same("\\t3", b"\t3");
}

#[test]
fn hex_prefix_is_parsed_as_zero_base_ten() {
    assert_same("0x10", b"0x10");
}

#[test]
fn float_like_input_truncates_at_dot() {
    assert_same("10.9", b"10.9");
}

#[test]
fn exponent_notation_stops_at_e() {
    assert_same("1e3", b"1e3");
}

#[test]
fn digits_separated_by_space() {
    assert_same("2 2", b"2 2");
}

#[test]
fn non_utf8_bytes() {
    assert_same("invalid utf-8", b"\xff\xfe 7\n");
}

// ---------------------------------------------------------------------------
// fgets() length limit: at most 13 bytes are consumed, and it stops at the
// first newline (it does not read across newlines the way scanf would).
// ---------------------------------------------------------------------------

#[test]
fn buffer_exactly_full_thirteen_digits() {
    // 13 digits, no newline: strtol -> 1234567890123, (int) of that is 1912276171.
    assert_same("13 digits", b"1234567890123");
}

#[test]
fn input_longer_than_buffer_is_truncated() {
    // Only the first 13 bytes are read; the rest is never consumed.
    assert_same("17 digits", b"12345678901234567");
}

#[test]
fn fourteenth_digit_would_change_the_value() {
    // "0000000000042" fills the buffer exactly; value 42.
    assert_same("padded 42", b"0000000000042");
}

#[test]
fn stops_at_newline_ignoring_second_line() {
    assert_same("42 then 99", b"42\n99\n");
}

#[test]
fn newline_before_digits_yields_zero() {
    assert_same("\\n5", b"\n5");
}

#[test]
fn newline_inside_buffer_window() {
    assert_same("7\\n99999999", b"7\n99999999");
}

#[test]
fn embedded_nul_byte() {
    assert_same("NUL byte", b"\x0042\n");
}

#[test]
fn carriage_return_line_ending() {
    assert_same("CRLF", b"12\r\n");
}

// ---------------------------------------------------------------------------
// stdout buffering mode is observable: with an empty stdin the program prints
// "fgets() failed." and then dies from a fatal signal.  Through a pipe stdout
// is fully buffered, so the message is lost; on a terminal stdout is line
// buffered, so the message *is* written before the crash.  Both programs must
// agree in both modes.
// ---------------------------------------------------------------------------

/// Run a program with its stdout attached to a pty (via `script`), stdin empty.
/// Returns (combined pty output, exit status) or None when `script` is absent.
fn run_on_pty(program: &Path) -> Option<(Vec<u8>, i32)> {
    let out = Command::new("script")
        .arg("-qec")
        .arg(format!("{} < /dev/null", program.display()))
        .arg("/dev/null")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    Some((out.stdout, out.status.code().unwrap_or(-1)))
}

#[test]
fn stdout_buffering_mode_matches() {
    match (run_on_pty(c_binary()), run_on_pty(&rust_binary())) {
        (Some((c_out, c_st)), Some((r_out, r_st))) => {
            assert_eq!(
                show(&c_out),
                show(&r_out),
                "line-buffered (tty) stdout differs on empty input"
            );
            assert_eq!(c_out, r_out, "line-buffered stdout bytes differ");
            assert_eq!(c_st, r_st, "tty-wrapped exit status differs");
        }
        // No `script(1)` available: still assert the fully buffered (pipe) case,
        // which is the mode the rest of the suite exercises.
        _ => assert_same("empty (pipe fallback)", b""),
    }
}
