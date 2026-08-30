//! Differential tests: run the original C program and the Rust translation as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here calls the Rust code as a library; both programs are driven
//! exactly the way a shell would drive them, which is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary under test, supplied by cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with cmake on first use if necessary.
/// `c_src/` itself is never modified; only the (git-ignored) build directory
/// is created.
fn c_bin() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.exists() {
        return bin;
    }
    std::fs::create_dir_all(&build).expect("cannot create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("failed to run cmake (is cmake installed?)");
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
        .expect("failed to run cmake --build");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(bin.exists(), "C binary missing after build: {}", bin.display());
    bin
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(stdin_bytes)
        .or_else(|e| {
            // The program may exit before consuming all input; that is fine.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("failed writing to child stdin");
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("failed waiting for child");
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

/// Compare the two programs on one stdin payload: stdout, stderr, exit status.
fn assert_same(stdin_bytes: &[u8]) {
    let c = run(&c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);
    let shown = String::from_utf8_lossy(stdin_bytes).to_string();

    assert_eq!(
        c.stdout, r.stdout,
        "stdout mismatch for input {shown:?} ({stdin_bytes:?})\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr mismatch for input {shown:?} ({stdin_bytes:?})\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "exit code mismatch for input {shown:?}: C={:?} Rust={:?}",
        c.code, r.code
    );
    assert_eq!(
        c.signal, r.signal,
        "termination signal mismatch for input {shown:?}: C={:?} Rust={:?}",
        c.signal, r.signal
    );
}

fn assert_all(cases: &[&str]) {
    for case in cases {
        assert_same(case.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Sanity: the expected happy-path output, spelled out, so a wholesale change
// in formatting cannot pass unnoticed just because both programs changed.
// ---------------------------------------------------------------------------

#[test]
fn happy_path_exact_bytes() {
    let c = run(&c_bin(), b"1 2 3 4\n");
    assert_eq!(c.stdout, b"1 2 1 4\n");
    let r = run(&rust_bin(), b"1 2 3 4\n");
    assert_eq!(r.stdout, b"1 2 1 4\n");
    assert_eq!(c.code, Some(0));
    assert_eq!(r.code, Some(0));
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Input-count classes: the four scanf calls each may or may not run out of
// input. Unconsumed variables keep their initial 0.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same(b"");
}

#[test]
fn whitespace_only_input() {
    assert_all(&[" ", "\n", "\t", "\r", "\x0b", "\x0c", "   \n\t \r\n ", "\n\n\n\n\n"]);
}

#[test]
fn partial_input_one_through_four_items() {
    assert_all(&["1", "1 2", "1 2 3", "1 2 3 4", "1 2 3 4 5", "1 2 3 4 5 6 7 8"]);
}

#[test]
fn missing_trailing_newline_variants() {
    assert_all(&["1 2 3 4", "1 2 3 4\n", "1 2 3 4\n\n", "1 2 3 4 ", "1 2 3 4\t"]);
}

// ---------------------------------------------------------------------------
// scanf reads across newlines (unlike fgets): every separator layout must give
// the same answer as a single space.
// ---------------------------------------------------------------------------

#[test]
fn scanf_reads_across_newlines_and_any_whitespace() {
    assert_all(&[
        "1 2 3 4",
        "1\n2\n3\n4",
        "1\n2\n3\n4\n",
        "1\t2\t3\t4",
        "1\r\n2\r\n3\r\n4",
        "\n\n1  \t\n 2\n\n\n3\r4\x0b\x0c",
        "   1\n2 3\n\n4   \n",
        "1\x0b2\x0c3\r4",
    ]);
}

// ---------------------------------------------------------------------------
// Bit-field truncation: x is 2 bits (mod 4), y is 3 bits (mod 8),
// b is a 1-bit bool fed !!b, z is a full int.
// ---------------------------------------------------------------------------

#[test]
fn bitfield_truncation_x_two_bits() {
    assert_all(&[
        "0 0 0 0", "1 0 0 0", "2 0 0 0", "3 0 0 0", "4 0 0 0", "5 0 0 0", "6 0 0 0", "7 0 0 0",
        "8 0 0 0", "100 0 0 0", "255 0 0 0", "256 0 0 0",
    ]);
}

#[test]
fn bitfield_truncation_y_three_bits() {
    assert_all(&[
        "0 0 0 0", "0 1 0 0", "0 5 0 0", "0 7 0 0", "0 8 0 0", "0 9 0 0", "0 15 0 0", "0 16 0 0",
        "0 100 0 0", "0 255 0 0", "0 256 0 0",
    ]);
}

#[test]
fn bool_bitfield_is_normalized_by_double_negation() {
    // Any nonzero b prints 1; only 0 prints 0.
    assert_all(&[
        "0 0 0 0",
        "0 0 1 0",
        "0 0 2 0",
        "0 0 -1 0",
        "0 0 256 0",
        "0 0 2147483647 0",
        "0 0 -2147483648 0",
        "0 0 -0 0",
        "0 0 +0 0",
        "0 0 0000 0",
    ]);
}

#[test]
fn z_is_printed_as_a_full_signed_int() {
    assert_all(&[
        "1 2 3 0",
        "1 2 3 1",
        "1 2 3 -1",
        "1 2 3 2147483647",
        "1 2 3 -2147483648",
        "1 2 3 1000000",
        "1 2 3 -1000000",
    ]);
}

// ---------------------------------------------------------------------------
// %u accepts a sign, so negative input wraps modulo 2^32 before truncation.
// ---------------------------------------------------------------------------

#[test]
fn unsigned_conversion_accepts_minus_sign() {
    assert_all(&[
        "-1 -1 -1 -1",
        "-2 -3 1 1",
        "-4 -8 1 1",
        "-4294967295 -4294967295 1 1",
        "-4294967296 -4294967296 1 1",
        "-0 -0 1 1",
    ]);
}

#[test]
fn plus_sign_is_accepted_everywhere() {
    assert_all(&["+1 +2 +3 +4", "+0 +0 +0 +0", "+4 +8 +1 +2147483647"]);
}

// ---------------------------------------------------------------------------
// Integer overflow / truncation exactly as the C library performs it.
// ---------------------------------------------------------------------------

#[test]
fn values_at_and_beyond_32_bit_limits() {
    assert_all(&[
        "4294967295 4294967295 2147483647 -2147483648",
        "4294967296 4294967296 1 1",
        "4294967297 4294967299 1 1",
        "2147483647 2147483647 1 2147483647",
        "2147483648 2147483648 1 2147483648",
        "1 1 1 2147483648",
        "1 1 1 -2147483649",
        "1 1 2147483648 1",
        "1 1 -2147483649 1",
    ]);
}

#[test]
fn values_at_and_beyond_64_bit_limits() {
    assert_all(&[
        "9223372036854775807 1 1 1",
        "9223372036854775808 1 1 1",
        "18446744073709551614 1 1 1",
        "18446744073709551615 1 1 1",
        "18446744073709551616 1 1 1",
        "18446744073709551617 1 1 1",
        // A negative magnitude that is exactly ULONG_MAX is negated, not
        // saturated: this distinguishes "overflowed" from "equals ULONG_MAX".
        "1 -18446744073709551615 1 1",
        "-18446744073709551615 1 1 1",
        "1 -18446744073709551616 1 1",
        "1 1 9223372036854775807 1",
        "1 1 9223372036854775808 1",
        "1 1 -9223372036854775808 1",
        "1 1 -9223372036854775809 1",
        "1 1 1 9223372036854775807",
        "1 1 1 9223372036854775808",
        "1 1 1 -9223372036854775808",
        "1 1 1 -9223372036854775809",
        "1 1 1 18446744073709551616",
    ]);
}

#[test]
fn absurdly_long_digit_runs_overflow_the_same_way() {
    let nines = "9".repeat(400);
    let neg_nines = format!("-{nines}");
    let zeros = format!("{}5", "0".repeat(400));
    let cases = [
        format!("{nines} 1 1 1"),
        format!("1 {nines} 1 1"),
        format!("1 1 {nines} 1"),
        format!("1 1 1 {nines}"),
        format!("{neg_nines} 1 1 1"),
        format!("1 1 1 {neg_nines}"),
        format!("{zeros} {zeros} {zeros} {zeros}"),
        format!("{nines} {neg_nines} {nines} {neg_nines}"),
    ];
    for c in &cases {
        assert_same(c.as_bytes());
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    assert_all(&["007 010 011 012", "0000000001 0000000008 00 000000012", "08 09 08 09"]);
}

// ---------------------------------------------------------------------------
// Matching-failure paths: scanf leaves the destination untouched (so it keeps
// its initializer 0) and leaves the offending character in the stream, so
// every later conversion fails too.
// ---------------------------------------------------------------------------

#[test]
fn non_numeric_input_fails_all_conversions() {
    assert_all(&["abc", "abc def", "x", "!", "?", "hello world", "NaN", "inf"]);
}

#[test]
fn failure_at_each_of_the_four_positions() {
    assert_all(&[
        "abc 2 3 4",
        "1 abc 3 4",
        "1 2 abc 4",
        "1 2 3 abc",
        "1 2 3 4abc",
        "1abc 2 3 4",
    ]);
}

#[test]
fn sign_without_digits_is_a_matching_failure() {
    assert_all(&[
        "-", "+", "- 1 2 3", "+ 1 2 3", "1 - 2 3", "1 2 - 3", "1 2 3 -", "1 2 3 +", "--1 2 3 4",
        "+-1 2 3 4", "-+1 2 3 4", "1 2 -x 7", "-a 1 2 3", "1 2 3 -a",
    ]);
}

#[test]
fn floating_point_and_hex_style_input() {
    assert_all(&[
        "1.5 2.5 3.5 4.5",
        ".5 .5 .5 .5",
        "0x10 0x10 0x10 0x10",
        "1e5 1e5 1e5 1e5",
        "1 2 3 4.9",
        "1,2,3,4",
        "1_2 3 4 5",
    ]);
}

#[test]
fn non_ascii_and_control_bytes() {
    // Driven as raw bytes; the C program reads bytes, not characters.
    let cases: [&[u8]; 8] = [
        b"\xff\xfe\xfd",
        b"1 2 \xff 4",
        b"\xc3\xa9 1 2 3",
        b"1 2 3 \xc3\xa9",
        b"\x00",
        b"1 2 3 4\x00",
        b"1\x002 3 4",
        b"\x01\x02\x03",
    ];
    for c in cases {
        assert_same(c);
    }
}

// ---------------------------------------------------------------------------
// A broad sweep so no single hand-written case is load-bearing.
// ---------------------------------------------------------------------------

#[test]
fn sweep_all_small_value_combinations() {
    for x in 0u32..=9 {
        for y in 0u32..=9 {
            assert_same(format!("{x} {y} 0 7").as_bytes());
            assert_same(format!("{x} {y} 1 -7").as_bytes());
        }
    }
}

#[test]
fn sweep_token_in_every_position() {
    const TOKENS: &[&str] = &[
        "0",
        "1",
        "3",
        "4",
        "7",
        "8",
        "-1",
        "-8",
        "+9",
        "abc",
        "-",
        "+",
        "0x1",
        "1.5",
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775808",
        "18446744073709551615",
        "-18446744073709551615",
        "18446744073709551616",
        "00000000009",
    ];
    for pos in 0..4 {
        for tok in TOKENS {
            let mut parts = ["1", "2", "3", "4"];
            parts[pos] = tok;
            assert_same(parts.join(" ").as_bytes());
        }
    }
}

#[test]
fn sweep_pseudorandom_token_streams() {
    // Deterministic xorshift so failures reproduce exactly.
    const TOKENS: &[&str] = &[
        "0", "1", "2", "3", "7", "8", "-1", "-0", "+5", "abc", "", "-", "+", "--1", "0x10", "1.5",
        "007", "4294967295", "4294967296", "2147483647", "-2147483648", "-2147483649",
        "99999999999999999999", "-99999999999999999999", "18446744073709551615",
        "-18446744073709551615", "18446744073709551616", "1a", "a1",
    ];
    const SEPS: &[&str] = &[" ", "\n", "\t", "  ", "\r\n", ""];
    let mut state: u64 = 0x2545F4914F6CDD1D;
    let mut next = |m: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as usize % m
    };
    for _ in 0..400 {
        let n = next(7);
        let mut input = String::new();
        for _ in 0..n {
            input.push_str(TOKENS[next(TOKENS.len())]);
            input.push_str(SEPS[next(SEPS.len())]);
        }
        assert_same(input.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Environment-level behaviours: closed stdin, and extra argv (main ignores it).
// ---------------------------------------------------------------------------

#[test]
fn closed_stdin_behaves_identically() {
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run C with /dev/null stdin");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust with /dev/null stdin");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn extra_arguments_are_ignored_identically() {
    let mut c = Command::new(c_bin());
    let mut r = Command::new(rust_bin());
    for cmd in [&mut c, &mut r] {
        cmd.args(["--help", "junk", "-1"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }
    let feed = |mut cmd: Command| {
        let mut child = cmd.spawn().expect("spawn");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"5 9 1 -3\n")
            .unwrap();
        drop(child.stdin.take());
        child.wait_with_output().expect("wait")
    };
    let co = feed(c);
    let ro = feed(r);
    assert_eq!(co.stdout, ro.stdout);
    assert_eq!(co.stderr, ro.stderr);
    assert_eq!(co.status.code(), ro.status.code());
}
