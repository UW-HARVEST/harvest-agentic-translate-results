//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on identical stdin and require byte-identical stdout, byte-identical
//! stderr and an identical exit status.
//!
//! Nothing here links the Rust code as a library; both sides are driven exactly the
//! way a shell would drive them.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Builds `c_src` with cmake once per test process, then returns the C executable.
fn c_binary() -> PathBuf {
    static BUILD: Once = Once::new();
    let root = workspace_root();
    let c_src = root.join("c_src");
    let build_dir = c_src.join("build");
    let exe = build_dir.join("driver");

    BUILD.call_once(|| {
        if exe.exists() {
            return;
        }
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let status = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .status()
            .expect("run cmake");
        assert!(status.success(), "cmake configure failed");
        let status = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .status()
            .expect("run cmake --build");
        assert!(status.success(), "cmake build failed");
    });

    assert!(
        exe.exists(),
        "C executable missing at {} -- build c_src first",
        exe.display()
    );
    exe
}

/// Every Rust build of the program that is available to test.
///
/// Always includes the binary cargo built for this test invocation, and also the
/// `--release` binary when it exists. Testing both matters here: the C relies on
/// signed wrap-around, so a debug build with overflow checks must not panic where
/// the release build wraps.
fn rust_binaries() -> Vec<PathBuf> {
    let mut bins = vec![PathBuf::from(env!("CARGO_BIN_EXE_driver"))];
    let release = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("driver");
    if release.exists() && release != bins[0] {
        bins.push(release);
    }
    bins
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn exit_signal(status: &std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

/// Feeds `input` to `bin` on stdin and collects the complete outcome.
fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin piped");
        // The program may exit without consuming stdin; a broken pipe is not an error.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: exit_signal(&out.status),
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts the C program and every Rust build agree on stdout, stderr and status.
fn assert_same(input: &str) {
    assert_same_bytes(input.as_bytes())
}

fn assert_same_bytes(input: &[u8]) {
    let c = c_binary();
    let expected = run(&c, input);

    for rust in rust_binaries() {
        let actual = run(&rust, input);
        let label = format!("input {:?} (rust: {})", show(input), rust.display());

        assert_eq!(
            show(&expected.stdout),
            show(&actual.stdout),
            "stdout mismatch for {label}"
        );
        assert_eq!(
            expected.stdout, actual.stdout,
            "stdout byte mismatch for {label}"
        );
        assert_eq!(
            show(&expected.stderr),
            show(&actual.stderr),
            "stderr mismatch for {label}"
        );
        assert_eq!(
            expected.stderr, actual.stderr,
            "stderr byte mismatch for {label}"
        );
        assert_eq!(
            expected.code, actual.code,
            "exit code mismatch for {label}"
        );
        assert_eq!(
            expected.signal, actual.signal,
            "termination signal mismatch for {label}"
        );
    }
}

// ---------------------------------------------------------------------------
// Prefix comparison, for inputs whose output is astronomically long
// ---------------------------------------------------------------------------

/// Reads up to `limit` bytes of stdout, then kills the child.
///
/// `foo()` iterates once per unit of `x`/`y`, so inputs near `INT_MAX` emit
/// multiple gigabytes. Those still have to be checked, so compare a long prefix
/// and stop rather than materialising the whole stream.
fn stdout_prefix(bin: &Path, input: &[u8], limit: usize) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin piped");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let mut buf = vec![0u8; limit];
    let mut filled = 0;
    {
        let stdout = child.stdout.as_mut().expect("stdout piped");
        while filled < limit {
            match stdout.read(&mut buf[filled..]) {
                Ok(0) => break, // real EOF: the program finished on its own
                Ok(n) => filled += n,
                Err(_) => break,
            }
        }
    }
    buf.truncate(filled);

    let _ = child.kill();
    let _ = child.wait();
    buf
}

fn assert_same_prefix(input: &str, limit: usize) {
    let c = c_binary();
    let expected = stdout_prefix(&c, input.as_bytes(), limit);

    for rust in rust_binaries() {
        let actual = stdout_prefix(&rust, input.as_bytes(), limit);
        assert_eq!(
            expected.len(),
            actual.len(),
            "prefix length mismatch for input {input:?} (rust: {})",
            rust.display()
        );
        if expected != actual {
            let at = expected
                .iter()
                .zip(actual.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            let lo = at.saturating_sub(60);
            panic!(
                "prefix mismatch for input {input:?} (rust: {}) at byte {at}\n C: {:?}\n R: {:?}",
                rust.display(),
                show(&expected[lo..(at + 60).min(expected.len())]),
                show(&actual[lo..(at + 60).min(actual.len())]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Phase A: both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_programs_build_and_run() {
    let c = c_binary();
    assert!(c.exists(), "C binary should exist at {}", c.display());
    for rust in rust_binaries() {
        assert!(rust.exists(), "rust binary {} should exist", rust.display());
    }
    // Simplest possible invocation: empty stdin. Neither program may hang or crash.
    assert_same("");
}

// ---------------------------------------------------------------------------
// Phase B: the inputs the C branches on
// ---------------------------------------------------------------------------

/// `scanf` returns EOF / does not assign; `x` and `y` keep their initialisers of 0,
/// so `foo`'s `while (x > 0 || y > 0)` is false immediately and nothing is printed.
#[test]
fn empty_and_whitespace_only_input() {
    for input in ["", " ", "\n", "\t\n", "   \n\n  \t ", "\r\n"] {
        assert_same(input);
    }
}

/// Only the first `%d` converts: `y` stays 0.
#[test]
fn single_item_input() {
    for input in ["1", "1 ", "1\n", "0", "4", "7", "-1", "-4", "1 abc", "1 -", "1 +", "1 ."] {
        assert_same(input);
    }
}

/// The loop never runs: both operands non-positive.
#[test]
fn loop_never_entered() {
    for input in ["0 0", "-1 -1", "0 -1", "-1 0", "-5 -5", "-0 -0"] {
        assert_same(input);
    }
}

/// The `x == 1 && y == 4` guard that jumps straight to `label2`, skipping the
/// `x` decrement on the first pass, plus its immediate neighbours.
#[test]
fn goto_label2_guard() {
    for input in ["1 4", "1 3", "1 5", "0 4", "2 4", "1 0", "2 3", "2 5"] {
        assert_same(input);
    }
}

/// `if (y == 0) continue;` — reaches the `while` condition without the `y` block.
#[test]
fn continue_path_y_zero() {
    for input in ["1 0", "2 0", "3 0", "5 0", "10 0"] {
        assert_same(input);
    }
}

/// `if (x < 3) goto label1;` — the backward jump that re-runs the body without
/// reprinting "loop", versus falling through to the `while` condition when x >= 3.
#[test]
fn goto_label1_backward_jump() {
    for input in ["0 1", "0 5", "1 1", "2 2", "3 3", "4 4", "5 5", "3 1", "4 1", "6 2", "3 9"] {
        assert_same(input);
    }
}

/// Exhaustive sweep of the small operand grid, which is where all the label
/// interactions live. Combinations of x > 0 with y < 0 are covered separately by
/// prefix comparison: the C wraps `y` down through INT_MIN and emits gigabytes.
#[test]
fn exhaustive_small_grid() {
    for x in -3i32..=12 {
        for y in -3i32..=12 {
            if x > 0 && y < 0 {
                continue;
            }
            assert_same(&format!("{x} {y}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: scanf parsing behaviour
// ---------------------------------------------------------------------------

/// `scanf` skips arbitrary whitespace, including newlines, between conversions —
/// unlike `fgets`, it happily reads the second integer off a later line.
#[test]
fn scanf_reads_across_whitespace_and_newlines() {
    for input in [
        "1 4", "1\n4", "1\n\n\n4", "1\t4", "  1   4  ", "\n\n1\r\n\t 4\n", "1\r4", " 3 3 ",
    ] {
        assert_same(input);
    }
}

/// Matching failure on the first or second conversion leaves that variable at 0.
#[test]
fn matching_failure_leaves_variables_at_zero() {
    for input in [
        "abc", "abc 4", "0x10 4", "1e5 4", ".5 4", "--1 4", "++1 4", "- 4", "+ 4", "-", "+",
        "x", "1abc 2", "12abc 4", "/4 4", ": 4",
    ] {
        assert_same(input);
    }
}

/// Trailing input past the two conversions is simply left unread.
#[test]
fn trailing_input_is_ignored() {
    for input in ["1 4 9", "1 4 garbage", "5 5\n", "5 5 ", "3 3 3 3 3", "2 2\nextra\n"] {
        assert_same(input);
    }
}

/// Signs and redundant leading zeros.
#[test]
fn signs_and_leading_zeros() {
    for input in [
        "+3 +4", "+1 +4", "-3 -4", "007 008", "0000000001 0000000004", "+0 -0",
        "000000000000000000000000000000005 4",
        "-000000000000000000000000000000005 4",
    ] {
        assert_same(input);
    }
}

/// glibc's `%d` clamps to LONG_MAX / LONG_MIN and then truncates that `long` to
/// `int`. Positive overflow therefore yields -1 while negative overflow yields 0.
/// These are the cases that make the two directions asymmetric.
#[test]
fn scanf_integer_overflow_truncation() {
    for input in [
        // positive overflow -> LONG_MAX -> (int)-1
        "99999999999999999999 4",
        "9223372036854775808 4",
        "18446744073709551616 4",
        "99999999999999999999999999999999999999 4",
        // negative overflow -> LONG_MIN -> (int)0
        "-99999999999999999999 4",
        "-9223372036854775809 4",
        "-18446744073709551616 4",
        "-99999999999999999999999999999999999999 4",
        // exactly LONG_MIN / LONG_MAX
        "-9223372036854775808 4",
        "9223372036854775807 4",
        // in-range long, truncated to int
        "4294967296 4",
        "-4294967297 4",
        "4294967297 4",
        "2147483648 4",
        "68719476736 4",
        // overflow in the second conversion
        // (`1 99999999999999999999` truncates y to -1, which with x > 0 sends the
        //  C into the wrap-around run; it is covered in huge_output_prefixes_match)
        "0 99999999999999999999",
        "1 -99999999999999999999",
        "0 4294967296",
    ] {
        assert_same(input);
    }
}

/// int boundary values that do not send the loop into a multi-gigabyte run.
#[test]
fn int_boundaries_bounded_output() {
    for input in [
        "2147483648 0",           // -> INT_MIN, loop not entered
        "2147483648 4",           // x = INT_MIN, y = 4
        "-2147483648 -2147483648",// both INT_MIN
        "0 -2147483648",          // y = INT_MIN, loop not entered
        "-2147483648 0",
        "99999999999999999999 4", // x = -1
        "99999999999999999999 0", // x = -1, loop not entered
    ] {
        assert_same(input);
    }
}

/// Input arriving without a trailing newline, and input containing NUL bytes.
#[test]
fn odd_byte_level_inputs() {
    assert_same_bytes(b"1 4");
    assert_same_bytes(b"1 4\0 9");
    assert_same_bytes(b"\0 1 4");
    assert_same_bytes(b"1\x0b4");
    assert_same_bytes(b"1\x0c4");
    assert_same_bytes(&[0xff, 0xfe, b' ', b'4']);
}

// ---------------------------------------------------------------------------
// Phase C: the enormous-output classes, compared by prefix
// ---------------------------------------------------------------------------

/// Operands near INT_MAX drive ~2^31 iterations. The full streams are many
/// gigabytes, so compare a two-megabyte prefix, which still covers hundreds of
/// thousands of iterations of every branch involved.
#[test]
fn huge_output_prefixes_match() {
    const LIMIT: usize = 2 * 1024 * 1024;
    for input in [
        "2147483647 0",            // x = INT_MAX, y = 0: loop/x forever
        "-2147483649 4",           // x truncates to INT_MAX
        "0 2147483647",            // y = INT_MAX: the goto label1 inner loop
        "1000000 1000000",
        "100000 0",
        "0 100000",
        // x > 0 with y < 0: the C decrements y below zero and relies on wrap-around
        "1 -1",
        "5 -3",
        "3 -1",
        "1 -2147483648",
        "-2147483649 -2147483648",
        // y truncates to -1 via LONG_MAX, with x > 0
        "1 99999999999999999999",
        "4 9223372036854775807",
    ] {
        assert_same_prefix(input, LIMIT);
    }
}

/// Mid-sized operands: still fully compared, but large enough to exercise many
/// thousands of loop iterations end to end, including final termination.
#[test]
fn medium_sized_operands_complete() {
    for input in ["100 100", "500 3", "3 500", "1000 0", "0 1000", "250 250", "77 4", "4 77"] {
        assert_same(input);
    }
}
