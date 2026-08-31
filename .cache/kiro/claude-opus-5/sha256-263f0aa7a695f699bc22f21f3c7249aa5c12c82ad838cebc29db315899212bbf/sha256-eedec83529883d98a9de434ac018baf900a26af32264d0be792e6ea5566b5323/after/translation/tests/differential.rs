//! Differential tests: run the original C program and the Rust translation as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust program is never linked as a library here; it is driven exactly the
//! way a shell drives it, because that is how the two are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test. Cargo builds it for us and exports the
/// path for integration tests, so this is always the binary matching the
/// current profile.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if it is missing.
/// Built once per test process even though tests run in parallel.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build_dir = c_src.join("build");
        let exe = build_dir.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build_dir).expect("create c_src/build");
            let conf = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("cmake must be available to build the C reference program");
            assert!(
                conf.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&conf.stdout),
                String::from_utf8_lossy(&conf.stderr)
            );
            let build = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("cmake --build");
            assert!(
                build.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The programs may exit before consuming all of stdin; a broken pipe
        // here is not a test failure.
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x0b => s.push_str("\\v"),
            0x0c => s.push_str("\\f"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Assert the two programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_binary(), stdin_bytes);
    let r = run(&rust_binary(), stdin_bytes);

    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "stdout mismatch for {label}\ninput: \"{}\"",
        show(stdin_bytes)
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout byte mismatch for {label}\ninput: \"{}\"",
        show(stdin_bytes)
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "stderr mismatch for {label}\ninput: \"{}\"",
        show(stdin_bytes)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr byte mismatch for {label}\ninput: \"{}\"",
        show(stdin_bytes)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label}\ninput: \"{}\"\nC stdout: \"{}\"\nRust stdout: \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
}

#[track_caller]
fn assert_same_str(label: &str, stdin_text: &str) {
    assert_same(label, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// Sanity: the reference outputs really are what the C source says they are.
// These pin the observable contract so a future regression in *both* programs
// cannot silently pass the differential checks.
// ---------------------------------------------------------------------------

#[test]
fn reference_outputs_are_pinned() {
    let cases: &[(&str, &str)] = &[
        ("1 2 3", "Ok!\nResult: 0\n"),
        (
            "0 2 3",
            "Error: x != 1\nOperation failed\nResult: 1\n",
        ),
        (
            "1 9 3",
            "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n",
        ),
        (
            "1 2 9",
            "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n",
        ),
    ];
    for (input, expected) in cases {
        let c = run(c_binary(), input.as_bytes());
        let r = run(&rust_binary(), input.as_bytes());
        assert_eq!(
            show(&c.stdout),
            show(expected.as_bytes()),
            "C stdout for \"{input}\""
        );
        assert_eq!(c.stdout, expected.as_bytes(), "C stdout bytes for \"{input}\"");
        assert_eq!(
            show(&r.stdout),
            show(expected.as_bytes()),
            "Rust stdout for \"{input}\""
        );
        assert_eq!(r.stdout, expected.as_bytes(), "Rust stdout bytes for \"{input}\"");
        assert!(c.stderr.is_empty() && r.stderr.is_empty());
        assert_eq!(c.status, Some(0));
        assert_eq!(r.status, Some(0));
    }
}

// ---------------------------------------------------------------------------
// multi_stage(): each of the four exits, including the three `goto fail` paths.
// ---------------------------------------------------------------------------

#[test]
fn stage_success_path() {
    assert_same_str("all three match", "1 2 3");
    assert_same_str("all three match, trailing newline", "1 2 3\n");
}

#[test]
fn stage_x_not_one() {
    for input in ["0 2 3", "2 2 3", "-1 2 3", "1000000 2 3", "0 0 0"] {
        assert_same_str("x != 1", input);
    }
}

#[test]
fn stage_y_not_two() {
    // y is a static global seeded to 123; scanf overwrites it.
    for input in ["1 0 3", "1 1 3", "1 3 3", "1 -2 3", "1 123 3"] {
        assert_same_str("x == 1, y != 2", input);
    }
}

#[test]
fn stage_z_not_three() {
    for input in ["1 2 0", "1 2 2", "1 2 4", "1 2 -3", "1 2 300"] {
        assert_same_str("x == 1, y == 2, z != 3", input);
    }
}

// ---------------------------------------------------------------------------
// scanf(): how many of the three conversions actually happen.
// Unassigned variables keep their initializers: x = 0, z = 0, y = 123.
// ---------------------------------------------------------------------------

#[test]
fn no_input_at_all() {
    assert_same_str("empty input", "");
}

#[test]
fn whitespace_only_input() {
    for input in [" ", "\n", "\t", "   \n\t  \n", "\r\n", "\x0b\x0c", &" ".repeat(100_000)] {
        assert_same_str("whitespace only", input);
    }
}

#[test]
fn single_item_only() {
    // Only x is assigned; y stays 123 and z stays 0.
    for input in ["1", "1\n", "0", "5", "-1", "1   ", "1\n\n\n"] {
        assert_same_str("one field", input);
    }
}

#[test]
fn two_items_only() {
    // x and y assigned; z stays 0.
    for input in ["1 2", "1 2\n", "1 2 ", "0 0", "1 9", "1 2\t"] {
        assert_same_str("two fields", input);
    }
}

#[test]
fn more_than_three_items() {
    // scanf stops after three conversions; the rest of stdin is never read.
    for input in ["1 2 3 4", "1 2 3 4 5 6 7", "1 2 3 999999999999999999999"] {
        assert_same_str("extra fields", input);
    }
}

#[test]
fn scanf_reads_across_newlines() {
    // The `%d` directives skip any whitespace, newlines included, so these are
    // all equivalent to "1 2 3".
    for input in [
        "1\n2\n3",
        "1\n2\n3\n",
        "\n\n\n1\n\n2\n\n3",
        "1\t2\t3",
        "1  \t \n\r 2 \n\n 3",
        "\r\n1 2 3",
        "1\x0b2\x0c3",
        " 1 2 3 ",
    ] {
        assert_same_str("newline-crossing read", input);
    }
    // Leading whitespace runs of pathological length.
    let mut input = " ".repeat(100_000);
    input.push_str("1 2 3");
    assert_same_str("100k leading spaces", &input);
}

#[test]
fn optional_sign_forms() {
    for input in [
        "+1 +2 +3",
        "+1 2 3",
        "1 +2 3",
        "1 2 +3",
        "-1 -2 -3",
        "+0 +0 +0",
        "-0 -0 -0",
    ] {
        assert_same_str("signed fields", input);
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    for input in [
        "000001 000002 000003",
        "01 02 03",
        "0000000000000000000000000000001 2 3",
        "1 0000000000000000000000000000002 0000000000000000000000000000003",
    ] {
        assert_same_str("leading zeros", input);
    }
}

// ---------------------------------------------------------------------------
// scanf() matching failures: conversion stops, later variables keep their
// initializers, and the offending byte is pushed back.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_on_first_field() {
    for input in ["abc", "abc 1 2", "x", ".", "-", "+", "e", "-a", "+a", "-.5"] {
        assert_same_str("first field fails to match", input);
    }
}

#[test]
fn matching_failure_on_second_field() {
    for input in ["1 abc", "1 -", "1 +", "1 - 3", "1 . 3", "1 x 3", "1 e3 3"] {
        assert_same_str("second field fails to match", input);
    }
}

#[test]
fn matching_failure_on_third_field() {
    for input in ["1 2 abc", "1 2 -", "1 2 +", "1 2 .", "1 2 x", "1 2 -x"] {
        assert_same_str("third field fails to match", input);
    }
}

#[test]
fn digits_followed_immediately_by_letters() {
    // "%d" stops at the first non-digit without consuming it as part of the
    // number, so "1abc" assigns 1 and then fails on "abc".
    for input in [
        "1abc",
        "1abc 2 3",
        "0x1 2 3",
        "1 2x 3",
        "1 2 3extra",
        "1 2 3 abc",
        "1.5 2 3",
        "1 2.5 3",
        "1 2 3.5",
        "1e3 2 3",
    ] {
        assert_same_str("digits then non-digits", input);
    }
}

#[test]
fn sign_at_end_of_input() {
    // A sign with nothing after it: no assignment for that field.
    for input in ["-", "+", "1 -", "1 2 -", "1 2 3 -"] {
        assert_same_str("dangling sign", input);
    }
}

// ---------------------------------------------------------------------------
// Integer conversion edge cases: glibc's %d collects the digits, saturates at
// LONG_MAX/LONG_MIN on overflow, then truncates the result into an `int`.
// ---------------------------------------------------------------------------

#[test]
fn int_range_boundaries() {
    for input in [
        "2147483647 2 3",
        "-2147483648 2 3",
        "1 2147483647 3",
        "1 -2147483648 3",
        "1 2 2147483647",
        "1 2 -2147483648",
    ] {
        assert_same_str("int boundary", input);
    }
}

#[test]
fn truncation_past_int_range() {
    // 2^32 + n truncates to n, so this input actually reaches the "Ok!" path.
    assert_same_str("2^32+1 truncates to 1", "4294967297 4294967298 4294967299");
    assert_same_str("signed 2^32+n", "+4294967297 +4294967298 +4294967299");
    for input in [
        "2147483648 2 3",
        "-2147483649 2 3",
        "4294967296 2 3",
        "-4294967295 2 3",
        "1 -4294967294 3",
        "99999999999 2 3",
        "1 99999999999 3",
        "1 2 99999999999",
    ] {
        assert_same_str("truncated conversion", input);
    }
}

#[test]
fn long_range_boundaries() {
    for input in [
        "9223372036854775807 2 3",
        "-9223372036854775808 2 3",
        "1 9223372036854775807 3",
        "1 2 9223372036854775807",
    ] {
        assert_same_str("long boundary", input);
    }
}

#[test]
fn overflow_saturates_then_truncates() {
    // Under wrap-around semantics 2^64 + 1 would truncate to 1 and print
    // "Ok!"; glibc saturates to LONG_MAX first, so it does not.
    assert_same_str(
        "2^64+n does not wrap",
        "18446744073709551617 18446744073709551618 18446744073709551619",
    );
    for input in [
        "9223372036854775808 2 3",
        "-9223372036854775809 2 3",
        "-18446744073709551615 2 3",
        "99999999999999999999999 2 3",
        "1 99999999999999999999999 3",
        "1 2 99999999999999999999999",
        "-99999999999999999999999 -99999999999999999999999 -99999999999999999999999",
        "99999999999999999999999999999999999999999999999999999999999999999999999 2 3",
    ] {
        assert_same_str("overflowing conversion", input);
    }
    // A digit run far longer than any internal scratch buffer.
    let mut input = "9".repeat(100_000);
    input.push_str(" 2 3");
    assert_same_str("100k digits", &input);
}

// ---------------------------------------------------------------------------
// Bytes that are not text at all.
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_and_nul_bytes() {
    let cases: &[(&str, &[u8])] = &[
        ("NUL first", b"\x00 2 3"),
        ("NUL after digit", b"1\x002 3"),
        ("NUL between fields", b"1 \x00 3"),
        ("invalid utf8 first", b"\xff\xfe 2 3"),
        ("invalid utf8 second", b"1 \xff 3"),
        ("invalid utf8 trailing", b"1 2 3\xff"),
        ("utf8 multibyte", b"1 \xc3\xa9 3"),
        ("all high bytes", b"\x80\x81\x82\x83"),
    ];
    for (label, bytes) in cases {
        assert_same(label, bytes);
    }
}

// ---------------------------------------------------------------------------
// Deterministic randomized sweep over the same input alphabet, as a net for
// combinations the hand-written cases above do not name.
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        // Numerical Recipes LCG; deterministic across platforms.
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 16
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

#[test]
fn randomized_sweep_over_mixed_alphabet() {
    let alphabet: &[u8] = b"0123456789    \n\t+-abcxX.eE\r\x0b\x0c";
    let mut rng = Lcg(0x5eed_1234);
    for _ in 0..400 {
        let len = rng.below(26) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| *rng.pick(alphabet)).collect();
        assert_same("randomized mixed input", &bytes);
    }
}

#[test]
fn randomized_sweep_over_numeric_fields() {
    let signs = ["", "", "+", "-"];
    let lengths: [u64; 9] = [1, 1, 2, 5, 10, 19, 20, 21, 30];
    let seps = [" ", "\n", "\t", "  ", "\r\n", " \n "];
    let mut rng = Lcg(0xc0ff_ee01);
    for _ in 0..400 {
        let field_count = rng.below(5);
        let mut fields: Vec<String> = Vec::new();
        for _ in 0..field_count {
            let sign = *rng.pick(&signs);
            let len = *rng.pick(&lengths);
            let digits: String = (0..len)
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect();
            fields.push(format!("{sign}{digits}"));
        }
        let sep = *rng.pick(&seps);
        assert_same_str("randomized numeric input", &fields.join(sep));
    }
}

// ---------------------------------------------------------------------------
// Both programs must be silent on stderr and always exit 0, on every input
// above. Checked explicitly here so a change in either is caught directly.
// ---------------------------------------------------------------------------

#[test]
fn stderr_always_empty_and_status_always_zero() {
    for input in ["", "1", "1 2", "1 2 3", "abc", "-", "99999999999999999999999 2 3"] {
        let c = run(c_binary(), input.as_bytes());
        let r = run(&rust_binary(), input.as_bytes());
        assert!(c.stderr.is_empty(), "C wrote to stderr for \"{input}\"");
        assert!(r.stderr.is_empty(), "Rust wrote to stderr for \"{input}\"");
        assert_eq!(c.status, Some(0), "C exit status for \"{input}\"");
        assert_eq!(r.status, Some(0), "Rust exit status for \"{input}\"");
    }
}

#[test]
fn stdin_closed_immediately() {
    // Equivalent to `prog < /dev/null`: scanf hits an input failure at once.
    let c = Command::new(c_binary())
        .stdin(Stdio::null())
        .output()
        .expect("run C with /dev/null stdin");
    let r = Command::new(rust_binary())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust with /dev/null stdin");
    assert_eq!(show(&c.stdout), show(&r.stdout), "stdout with closed stdin");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr, "stderr with closed stdin");
    assert_eq!(c.status.code(), r.status.code(), "status with closed stdin");
}
