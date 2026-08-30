// Differential test harness: runs the original C program and the Rust
// translation as subprocesses with identical stdin and requires byte-for-byte
// identical stdout, byte-for-byte identical stderr, and the same exit status.
//
// The Rust program is never linked as a library here; it is executed the same
// way a shell would execute it, because that is how it is compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test, as built by cargo for this test run.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

/// Workspace root: the directory containing both `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Locate the compiled C executable, building it with CMake if necessary.
fn c_bin() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    for name in ["driver", "driver.exe"] {
        let candidate = build.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }

    std::fs::create_dir_all(&build).expect("failed to create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake ..` (is cmake installed?)");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr)
    );
    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("failed to run `cmake --build .`");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    for name in ["driver", "driver.exe"] {
        let candidate = build.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("C executable not found in {}", build.display());
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, Option<i32>>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sin = child.stdin.take().expect("stdin was piped");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock the test against a full pipe buffer.
        std::thread::spawn(move || {
            let _ = sin.write_all(&bytes);
            let _ = sin.flush();
        });
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", program.display()));

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => Err(signal_of(&out.status)),
    };

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.len() <= 400 => format!("{s:?}"),
        Ok(s) => format!("{:?}... ({} bytes total)", &s[..400], bytes.len()),
        Err(_) => format!("{bytes:?}"),
    }
}

/// Run both programs on `input` and require stdout, stderr and exit status to
/// match exactly.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = c_bin();
    let expected = run(&c, input);
    let actual = run(Path::new(RUST_BIN), input);

    assert_eq!(
        expected.status, actual.status,
        "[{label}] exit status differs for input {}\n  C:    {:?}\n  Rust: {:?}",
        show(input),
        expected.status,
        actual.status
    );
    assert_eq!(
        expected.stdout,
        actual.stdout,
        "[{label}] stdout differs for input {}\n  C:    {}\n  Rust: {}",
        show(input),
        show(&expected.stdout),
        show(&actual.stdout)
    );
    assert_eq!(
        expected.stderr,
        actual.stderr,
        "[{label}] stderr differs for input {}\n  C:    {}\n  Rust: {}",
        show(input),
        show(&expected.stderr),
        show(&actual.stderr)
    );
}

#[track_caller]
fn same(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_programs_are_runnable() {
    let c = c_bin();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(
        Path::new(RUST_BIN).is_file(),
        "Rust binary missing at {RUST_BIN}"
    );
    // A trivial run must succeed for both.
    assert_eq!(run(&c, b"1\n").status, Ok(0));
    assert_eq!(run(Path::new(RUST_BIN), b"1\n").status, Ok(0));
}

// ---------------------------------------------------------------------------
// Phase B: the loop-count branch in driver() — `i < x`.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_scanf_returns_eof() {
    // scanf fails, x keeps its initializer 0, the loop body never runs.
    same("empty", "");
}

#[test]
fn zero_produces_no_output() {
    same("zero", "0");
    same("zero_nl", "0\n");
}

#[test]
fn single_iteration() {
    same("one", "1");
    same("one_nl", "1\n");
}

#[test]
fn small_counts() {
    for n in 1..=12 {
        same(&format!("count_{n}"), &format!("{n}\n"));
    }
}

#[test]
fn negative_counts_skip_the_loop() {
    for s in ["-1", "-0", "-7", "-2147483648", "-000123"] {
        same(&format!("negative_{s}"), s);
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: every branch inside the `%d` conversion.
// ---------------------------------------------------------------------------

#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    // %d skips arbitrary whitespace, including newlines, unlike fgets.
    same("nl_then_value", "\n\n  7");
    same("tabs", "\t\t3");
    same("crlf", "\r\n4\r\n");
    same("vtab_ff", "\x0b\x0c5");
    same("mixed_ws", " \t\n \r\n\x0b\x0c 6\n");
    same("spaces_only", "   ");
    same("newlines_only", "\n\n\n");
    same("single_space", " ");
    same("tab_only", "\t");
}

#[test]
fn explicit_sign_branch() {
    same("plus", "  +4");
    same("plus_nl", "+3\n");
    same("minus_zero", "-0");
    same("plus_zero", "+0");
    // Sign followed immediately by EOF: conversion fails, x stays 0.
    same("plus_eof", "+");
    same("minus_eof", "-");
    // Sign followed by a non-digit: conversion fails.
    same("minus_nondigit", "-x");
    same("plus_nondigit", "+ 5");
    same("plus_newline", "+\n5");
    // Two signs is not a valid %d.
    same("double_sign", "--5");
    same("plus_minus", "+-5");
}

#[test]
fn non_numeric_input_leaves_x_untouched() {
    for s in ["abc", "x", ".", ",", "/", ":", "e5", "nan", "inf", "\0", "\0\0 3"] {
        same(&format!("nonnumeric_{}", s.escape_debug()), s);
    }
}

#[test]
fn hex_and_float_forms_are_not_special_to_percent_d() {
    // %d stops at 'x' — "0x10" reads as 0.
    same("hex", "0x10");
    // %d stops at '.' — "12.9" reads as 12.
    same("float", "12.9");
    same("float_leading_dot", ".5");
    same("exp", "3e2");
}

#[test]
fn digits_followed_by_trailing_garbage() {
    same("digits_then_alpha", "3abc");
    same("digits_then_space_digits", "3 4");
    same("digits_then_newline_digits", "3\n4\n");
    same("digits_then_punct", "2;;;");
    same("digits_then_sign", "2-3");
    same("no_trailing_newline", "6");
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    same("zeros_5", "0000005");
    same("zeros_7", "007");
    same("zeros_10", "010");
    same("many_zeros", "00000000000000000000000000003");
    same("zeros_only", "00000");
}

// ---------------------------------------------------------------------------
// Phase C: overflow, truncation and signedness exactly as the C performs it.
// glibc accumulates %d into a `long`, saturates at LONG_MIN/LONG_MAX, then
// narrows to `int`. Each of these lands on a different int after narrowing.
// ---------------------------------------------------------------------------

#[test]
fn values_that_narrow_to_a_nonpositive_int() {
    // 2^31: narrows to INT_MIN -> loop skipped.
    same("int_max_plus_1", "2147483648");
    // 2^32: narrows to 0 -> loop skipped.
    same("two_pow_32", "4294967296");
    // Saturates at LONG_MAX (0x7fff_ffff_ffff_ffff) -> narrows to -1.
    same("long_saturate_pos", "99999999999999999999");
    same("long_saturate_pos_long", "1234567890123456789012345678901234567890");
    // Exactly LONG_MAX -> narrows to -1.
    same("long_max", "9223372036854775807");
    // LONG_MAX + 1 -> saturates to LONG_MAX -> -1.
    same("long_max_plus_1", "9223372036854775808");
    // 2^32 + 2^31: a positive long whose low 32 bits are INT_MIN.
    same("high_bits_negative", "6442450944");
}

#[test]
fn values_that_narrow_to_a_small_positive_int() {
    // 2^32 + 1 narrows to 1.
    same("two_pow_32_plus_1", "4294967297");
    // 2^32 + 3 narrows to 3.
    same("two_pow_32_plus_3", "4294967299");
    // 2^34 + 5 narrows to 5.
    same("two_pow_34_plus_5", "17179869189");
    // A negative long whose low 32 bits are a small positive int:
    // -(2^32 - 4) = -4294967292 -> low 32 bits = 4.
    same("negative_narrows_positive", "-4294967292");
}

#[test]
fn largest_count_that_is_observable() {
    // The loop bound is `i < x`, so the true maximum the code handles is
    // INT_MAX, which would print ~2^31 lines in *both* programs and is not
    // testable in finite time. The largest count exercised here is 100000
    // (see `output_spans_column_widths_and_a_large_buffer`); this test pins the
    // adjacent parse boundaries instead, where behavior is observable.
    //
    // INT_MIN via a decimal literal, and INT_MIN reached by narrowing 2^31.
    same("int_min_literal", "-2147483648");
    same("int_min_via_2_pow_31", "2147483648");
}

// ---------------------------------------------------------------------------
// Phase C: printf formatting and buffering over a large amount of output.
// ---------------------------------------------------------------------------

#[test]
fn output_spans_column_widths_and_a_large_buffer() {
    // Crosses 1-, 2-, 3- and 4-digit i and j, i.e. every %d width change.
    same("width_100", "100");
    same("width_1000", "1000");
    same("width_10000", "10000");
    same("width_100000", "100000");
}

#[test]
fn value_split_over_many_lines_of_whitespace_before_it() {
    let mut s = String::new();
    for _ in 0..2000 {
        s.push('\n');
    }
    s.push_str("9");
    same("lots_of_leading_newlines", &s);
}

#[test]
fn very_long_digit_run_does_not_diverge() {
    // 5000 digits: both must saturate the same way (-> LONG_MAX -> -1).
    let s = "9".repeat(5000);
    same("5000_nines", &s);
    let mut z = "0".repeat(5000);
    z.push('4');
    same("5000_zeros_then_4", &z);
}

#[test]
fn binary_and_non_utf8_stdin() {
    assert_same("invalid_utf8", &[0xff, 0xfe, b'5']);
    assert_same("utf8_after_value", "5\u{00e9}\u{4e2d}".as_bytes());
    assert_same("nul_then_value", &[0x00, b'3']);
    assert_same("value_then_nul", &[b'3', 0x00, b'9']);
    assert_same("high_bytes_only", &[0x80, 0x81, 0x82]);
}

#[test]
fn stdin_closed_immediately_and_whitespace_only_variants() {
    same("only_whitespace_long", &" ".repeat(4096));
    // Whitespace longer than a stdio buffer, then a value.
    let mut s = " ".repeat(9000);
    s.push_str("8\n");
    same("long_whitespace_then_value", &s);
}

// ---------------------------------------------------------------------------
// Phase C: the reader of stdout goes away. The C program is killed by SIGPIPE;
// the Rust runtime installs SIG_IGN for SIGPIPE before main, so without an
// explicit fix the Rust program exits 0 instead of dying (see ERRORS.md).
// ---------------------------------------------------------------------------

/// Spawn `program`, close the read end of its stdout *before* it can write
/// anything (it blocks in the `%d` conversion until stdin arrives), then feed
/// stdin and collect stderr and the exit status.
///
/// Ordering makes this deterministic: the first write the program attempts is
/// already to a pipe with no reader, so both programs fail on their first
/// write attempt rather than at a buffer-size-dependent point.
fn run_with_stdout_reader_closed(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    // Close the read end of stdout now, while the child is still blocked
    // waiting for stdin.
    drop(child.stdout.take().expect("stdout was piped"));

    {
        let mut sin = child.stdin.take().expect("stdin was piped");
        let bytes = stdin_bytes.to_vec();
        std::thread::spawn(move || {
            let _ = sin.write_all(&bytes);
            let _ = sin.flush();
        });
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", program.display()));
    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => Err(signal_of(&out.status)),
    };
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

#[track_caller]
fn assert_same_with_stdout_closed(label: &str, input: &[u8]) {
    let expected = run_with_stdout_reader_closed(&c_bin(), input);
    let actual = run_with_stdout_reader_closed(Path::new(RUST_BIN), input);
    assert_eq!(
        expected.status, actual.status,
        "[{label}] exit status differs when the stdout reader is gone\n  C:    {:?}\n  Rust: {:?}",
        expected.status, actual.status
    );
    assert_eq!(
        expected.stderr,
        actual.stderr,
        "[{label}] stderr differs when the stdout reader is gone\n  C:    {}\n  Rust: {}",
        show(&expected.stderr),
        show(&actual.stderr)
    );
    // stdout is unobservable here by construction: its read end is closed
    // before either program writes, so both yield no readable bytes.
    assert!(expected.stdout.is_empty() && actual.stdout.is_empty());
}

#[test]
fn dead_stdout_reader_kills_both_programs_alike() {
    // Enough output to attempt a write at all.
    assert_same_with_stdout_closed("epipe_1", b"1\n");
    assert_same_with_stdout_closed("epipe_3", b"3\n");
    assert_same_with_stdout_closed("epipe_100000", b"100000\n");
    // No output at all: neither program writes, so neither is signalled.
    assert_same_with_stdout_closed("epipe_zero", b"0\n");
    assert_same_with_stdout_closed("epipe_empty", b"");
    assert_same_with_stdout_closed("epipe_negative", b"-5\n");
}

#[test]
fn stdout_reader_closes_partway_through_a_long_run() {
    // A reader that takes a few bytes and leaves. Both programs must end up
    // signalled rather than exiting 0.
    let c = c_bin();
    let expected = run_partial_reader(&c);
    let actual = run_partial_reader(Path::new(RUST_BIN));
    assert_eq!(
        expected, actual,
        "exit status differs when the stdout reader stops reading early\n  C:    {expected:?}\n  Rust: {actual:?}"
    );
}

fn run_partial_reader(program: &Path) -> Result<i32, Option<i32>> {
    use std::io::Read;
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));
    {
        let mut sin = child.stdin.take().expect("stdin was piped");
        std::thread::spawn(move || {
            let _ = sin.write_all(b"5000000\n");
        });
    }
    {
        let mut sout = child.stdout.take().expect("stdout was piped");
        let mut buf = [0u8; 16];
        let _ = sout.read(&mut buf);
        // `sout` drops here: the reader is gone while the child still has
        // millions of lines left to write.
    }
    let status = child.wait().expect("failed to wait");
    match status.code() {
        Some(code) => Ok(code),
        None => Err(signal_of(&status)),
    }
}
