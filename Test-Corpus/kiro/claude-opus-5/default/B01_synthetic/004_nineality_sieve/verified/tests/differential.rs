//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical arguments and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is never called as a library. Both sides are driven exactly
//! the way a shell would drive them, because that is how they are compared.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust binary under test, provided by Cargo.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if necessary.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let root = workspace_root();
    let c_src = root.join("c_src");
    let bin = c_src.join("build").join("driver");

    BUILD.call_once(|| {
        if bin.is_file() {
            return;
        }
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
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
    });

    assert!(
        bin.is_file(),
        "C binary not found at {}; build it with: \
         cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        bin.display()
    );
    bin
}

/// The observable result of one run: stdout bytes, stderr bytes, exit code and
/// terminating signal. Both `code` and `signal` are compared so that a normal
/// exit is never confused with death by a signal.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("code", &self.code)
            .field("signal", &self.signal)
            .field("stdout_len", &self.stdout.len())
            .field("stdout", &Preview(&self.stdout))
            .field("stderr", &Preview(&self.stderr))
            .finish()
    }
}

/// Prints at most the first and last 200 bytes of a stream, lossily, so that a
/// failure on a multi-megabyte output stays readable.
struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const N: usize = 200;
        if self.0.len() <= 2 * N {
            write!(f, "{:?}", String::from_utf8_lossy(self.0))
        } else {
            write!(
                f,
                "{:?} ...[{} bytes elided]... {:?}",
                String::from_utf8_lossy(&self.0[..N]),
                self.0.len() - 2 * N,
                String::from_utf8_lossy(&self.0[self.0.len() - N..])
            )
        }
    }
}

fn run(bin: &Path, args: &[&OsStr]) -> Outcome {
    let out = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Core assertion: the two programs agree on stdout, stderr and exit status.
fn assert_same(args: &[&OsStr]) -> Outcome {
    let c = run(&c_bin(), args);
    let r = run(&rust_bin(), args);

    let shown: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a.as_bytes())))
        .collect();
    let label = format!("args = [{}]", shown.join(", "));

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label}\n  C: {:?}\n  Rust: {:?}",
        Preview(&c.stdout),
        Preview(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label}\n  C: {:?}\n  Rust: {:?}",
        Preview(&c.stderr),
        Preview(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for {label}\n  C: {c:?}\n  Rust: {r:?}"
    );
    c
}

/// Convenience wrapper for byte-string arguments (supports non-UTF-8).
fn same(args: &[&[u8]]) -> Outcome {
    let owned: Vec<OsString> = args
        .iter()
        .map(|b| OsStr::from_bytes(b).to_os_string())
        .collect();
    let refs: Vec<&OsStr> = owned.iter().map(|s| s.as_os_str()).collect();
    assert_same(&refs)
}

fn same1(arg: &[u8]) -> Outcome {
    same(&[arg])
}

// ---------------------------------------------------------------------------
// `argc != 2` branch
// ---------------------------------------------------------------------------

#[test]
fn no_arguments_is_usage_error() {
    let o = same(&[]);
    assert_eq!(o.code, Some(1));
    assert_eq!(
        o.stdout,
        b"Error: should only be a single (integer) argument!\n"
    );
    // The C program prints its errors on stdout, not stderr.
    assert!(o.stderr.is_empty());
}

#[test]
fn two_arguments_is_usage_error() {
    let o = same(&[b"5", b"7"]);
    assert_eq!(o.code, Some(1));
    assert_eq!(
        o.stdout,
        b"Error: should only be a single (integer) argument!\n"
    );
}

#[test]
fn many_arguments_is_usage_error() {
    let o = same(&[b"1", b"2", b"3", b"4", b"5"]);
    assert_eq!(o.code, Some(1));
}

// ---------------------------------------------------------------------------
// `end == argv[1]` branch: strtol performed no conversion
// ---------------------------------------------------------------------------

#[test]
fn empty_argument_is_parse_error() {
    let o = same1(b"");
    assert_eq!(o.code, Some(1));
    assert_eq!(o.stdout, b"Error: first argument must be an integer!\n");
}

#[test]
fn non_numeric_arguments_are_parse_errors() {
    for a in [
        &b"abc"[..],
        b"INF",
        b"nan",
        b".5",
        b"-",
        b"+",
        b"--5",
        b"+-5",
        b"  -  5",
        b" ",
        b"\t",
        b"\t\n\x0b\x0c\r ",
        b"_1",
        b"e5",
        b"#9",
        b"\xff\xfe",   // not valid UTF-8
        b"\xc2\xb3",   // U+00B3 SUPERSCRIPT THREE
        b"\xd9\xa3",   // U+0663 ARABIC-INDIC DIGIT THREE
        b"\xef\xbc\x99", // U+FF19 FULLWIDTH DIGIT NINE
    ] {
        let o = same1(a);
        assert_eq!(
            o.code,
            Some(1),
            "expected parse error for {:?}",
            String::from_utf8_lossy(a)
        );
        assert_eq!(o.stdout, b"Error: first argument must be an integer!\n");
    }
}

// ---------------------------------------------------------------------------
// Happy path: loop until the value ends in 9
// ---------------------------------------------------------------------------

#[test]
fn single_item_when_value_already_ends_in_nine() {
    // The loop prints once and breaks immediately.
    let o = same1(b"9");
    assert_eq!(o.code, Some(0));
    assert_eq!(o.stdout, b"9\n");
}

#[test]
fn counts_up_from_zero() {
    let o = same1(b"0");
    assert_eq!(o.code, Some(0));
    assert_eq!(o.stdout, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

#[test]
fn counts_up_from_small_positive() {
    let o = same1(b"5");
    assert_eq!(o.stdout, b"5\n6\n7\n8\n9\n");
    assert_eq!(o.code, Some(0));
}

#[test]
fn every_final_digit_zero_through_nine() {
    for d in 0..=9u8 {
        let arg = format!("{d}");
        same1(arg.as_bytes());
        let arg = format!("1{d}");
        same1(arg.as_bytes());
    }
}

#[test]
fn negative_values_never_satisfy_the_modulo_and_count_up_to_nine() {
    // C's `%` truncates toward zero, so -9 % 10 == -9, not 9: the loop runs
    // all the way up to positive 9.
    let o = same1(b"-9");
    assert_eq!(o.code, Some(0));
    assert!(o.stdout.starts_with(b"-9\n-8\n"));
    assert!(o.stdout.ends_with(b"8\n9\n"));

    for a in [&b"-1"[..], b"-3", b"-10", b"-19", b"-99", b"-100"] {
        same1(a);
    }
}

#[test]
fn negative_zero_and_explicit_plus_sign() {
    same1(b"-0");
    same1(b"+0");
    same1(b"+9");
    same1(b"+5");
}

#[test]
fn leading_whitespace_is_skipped_by_strtol() {
    for a in [
        &b" 7"[..],
        b"   7",
        b"\t7",
        b"\n7",
        b"\x0b7",
        b"\x0c7",
        b"\r7",
        b"\t\x0b\x0c\r\n 8",
    ] {
        same1(a);
    }
}

#[test]
fn trailing_garbage_is_ignored_because_end_is_not_checked() {
    // The C code only tests `end == argv[1]`; anything after the digits is
    // silently discarded.
    for a in [
        &b"5abc"[..],
        b"9 ",
        b"1e5",
        b"1_0",
        b"0x10", // base 10: parses "0", stops at 'x'
        b"  12  34",
        b"7-",
        b"3.9",
        b"8,000",
    ] {
        same1(a);
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    same1(b"007");
    same1(b"00000009");
    same1(b"0000");
}

// ---------------------------------------------------------------------------
// long -> int narrowing, and strtol saturation on overflow
// ---------------------------------------------------------------------------

#[test]
fn long_result_is_truncated_into_int() {
    // 4294967305 == 2^32 + 9, so the int keeps only 9.
    let o = same1(b"4294967305");
    assert_eq!(o.stdout, b"9\n");

    // 2^32 truncates to 0.
    let o = same1(b"4294967296");
    assert_eq!(o.stdout, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");

    same1(b"-4294967287");
    same1(b"-4294967296");
    same1(b"8589934601");
}

#[test]
fn strtol_saturates_at_long_max_then_truncates() {
    // LONG_MAX == 0x7fff_ffff_ffff_ffff; the low 32 bits are 0xffff_ffff == -1.
    let expected: &[u8] = b"-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n";
    for a in [
        &b"9223372036854775807"[..],  // exactly LONG_MAX
        b"9223372036854775808",       // LONG_MAX + 1, saturates
        b"18446744073709551616",      // 2^64
        b"99999999999999999999",
    ] {
        let o = same1(a);
        assert_eq!(o.stdout, expected, "for {:?}", String::from_utf8_lossy(a));
        assert_eq!(o.code, Some(0));
    }
}

#[test]
fn strtol_saturates_at_long_min_then_truncates() {
    // LONG_MIN == 0x8000_0000_0000_0000; the low 32 bits are 0.
    let expected: &[u8] = b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n";
    for a in [
        &b"-9223372036854775808"[..], // exactly LONG_MIN
        b"-9223372036854775809",      // LONG_MIN - 1, saturates
        b"-18446744073709551616",
        b"-99999999999999999999",
    ] {
        let o = same1(a);
        assert_eq!(o.stdout, expected, "for {:?}", String::from_utf8_lossy(a));
    }
}

#[test]
fn very_long_digit_strings() {
    let mut nines = vec![b'9'; 1000];
    same1(&nines);

    nines.insert(0, b'-');
    same1(&nines);

    let mut zeros = vec![b'0'; 1000];
    same1(&zeros);
    zeros.push(b'3');
    same1(&zeros);
}

#[test]
fn largest_value_that_terminates_without_overflowing() {
    // INT_MAX is 2147483647 and ends in 7, so any start that reaches it wraps.
    // 2147483639 is the largest input the loop handles without overflow: it
    // ends in 9, so it prints once and breaks.
    let o = same1(b"2147483639");
    assert_eq!(o.stdout, b"2147483639\n");
    assert_eq!(o.code, Some(0));
}

// ---------------------------------------------------------------------------
// Large outputs: exercise stdio buffering across many writes
// ---------------------------------------------------------------------------

#[test]
fn large_output_is_byte_identical() {
    let o = same1(b"-300000");
    assert_eq!(o.code, Some(0));
    assert_eq!(o.stdout.iter().filter(|b| **b == b'\n').count(), 300_010);
    assert!(o.stdout.starts_with(b"-300000\n-299999\n"));
    assert!(o.stdout.ends_with(b"8\n9\n"));
}

// ---------------------------------------------------------------------------
// Signed-overflow wrap region.
//
// Values whose last digit is not 9 and that sit at INT_MAX wrap to INT_MIN and
// then count all the way back up to 9 -- billions of lines. Compare a bounded
// prefix instead, which is exactly where the wrap is observable, and require
// both programs to die the same way when the reader goes away.
// ---------------------------------------------------------------------------

/// Read at most `limit` bytes of stdout, then close the pipe and wait.
fn run_truncated(bin: &Path, arg: &[u8], limit: usize) -> (Vec<u8>, Option<i32>, Option<i32>) {
    let mut child = Command::new(bin)
        .arg(OsStr::from_bytes(arg))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut buf = Vec::new();
    {
        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut chunk = [0u8; 4096];
        while buf.len() < limit {
            match stdout.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(e) => panic!("read failed: {e}"),
            }
        }
        buf.truncate(limit);
        // Dropping `stdout` closes the read end; the writer now sees EPIPE.
    }

    let status = child.wait().expect("wait for child");
    (buf, status.code(), status.signal())
}

fn assert_same_prefix(arg: &[u8], limit: usize) {
    let (c_out, c_code, c_sig) = run_truncated(&c_bin(), arg, limit);
    let (r_out, r_code, r_sig) = run_truncated(&rust_bin(), arg, limit);
    let label = String::from_utf8_lossy(arg).to_string();
    assert_eq!(
        c_out,
        r_out,
        "stdout prefix differs for {label:?}\n  C: {:?}\n  Rust: {:?}",
        Preview(&c_out),
        Preview(&r_out)
    );
    assert_eq!(
        (c_code, c_sig),
        (r_code, r_sig),
        "exit status differs for {label:?} (C code/signal vs Rust)"
    );
}

#[test]
fn int_max_wraps_to_int_min_like_the_c_build_does() {
    // The wrap is visible in the first two lines.
    let (c_out, ..) = run_truncated(&c_bin(), b"2147483647", 200);
    assert!(
        c_out.starts_with(b"2147483647\n-2147483648\n"),
        "unexpected C output: {:?}",
        Preview(&c_out)
    );
    assert_same_prefix(b"2147483647", 200);
}

#[test]
fn wrap_region_prefixes_match() {
    for a in [
        &b"2147483647"[..], // INT_MAX, wraps to INT_MIN
        b"2147483640",      // counts up to INT_MAX, then wraps
        b"2147483648",      // long -> int truncation lands on INT_MIN
        b"-2147483648",     // INT_MIN
        b"-2147483649",     // truncates to INT_MAX, then wraps
        b"-2147483640",
        b"-1000000000",
    ] {
        assert_same_prefix(a, 4096);
    }
}

#[test]
fn sigpipe_terminates_both_programs_the_same_way() {
    // A Rust program ignores SIGPIPE unless it opts out, which would make it
    // exit 0 where the C program is killed by the signal.
    let (_, c_code, c_sig) = run_truncated(&c_bin(), b"-2147483648", 64);
    let (_, r_code, r_sig) = run_truncated(&rust_bin(), b"-2147483648", 64);
    assert_eq!(
        c_sig,
        Some(13),
        "expected the C program to be killed by SIGPIPE, got code={c_code:?} signal={c_sig:?}"
    );
    assert_eq!((c_code, c_sig), (r_code, r_sig));
}
