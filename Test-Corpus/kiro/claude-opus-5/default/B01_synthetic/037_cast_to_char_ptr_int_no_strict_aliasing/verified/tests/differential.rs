//! Differential tests: run the original C executable and the Rust executable
//! as subprocesses with identical stdin and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use if needed.
fn c_bin() -> PathBuf {
    let c_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if !bin.exists() {
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cmake = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .status()
            .expect("cmake must be installed to run the differential tests");
        assert!(cmake.success(), "cmake configure of c_src failed");
        let make = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .status()
            .expect("cmake --build failed to start");
        assert!(make.success(), "cmake --build of c_src failed");
    }
    assert!(bin.exists(), "C binary missing at {}", bin.display());
    bin
}

/// What one program produced for one input.
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<Option<i32>, Option<i32>>,
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The program may exit before consuming all of stdin; a write failure
        // here (EPIPE) is not a test failure.
        let _ = stdin.write_all(stdin_bytes);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");

    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        match out.status.signal() {
            Some(sig) => Err(Some(sig)),
            None => Ok(out.status.code()),
        }
    };
    #[cfg(not(unix))]
    let status = Ok(out.status.code());

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
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Assert C and Rust agree on stdout, stderr and exit status for one input.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(&c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (stdin = \"{}\")\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (stdin = \"{}\")\n  C:    \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (stdin = \"{}\"): C = {:?}, Rust = {:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Input classes the C program branches on.
//
// The C is: `int x = 0; scanf("%d", &x); driver(x);`  The only branch points
// are inside the `%d` conversion (input failure / matching failure / success,
// plus strtol's overflow saturation) and the fixed 4-iteration loop in
// print_hex.  Each test below pins one of those classes.
// ---------------------------------------------------------------------------

/// Input failure: nothing at all to read, so scanf returns EOF and x stays 0.
#[test]
fn empty_and_whitespace_only_input() {
    check_all(&[
        ("empty stdin", b""),
        ("single newline", b"\n"),
        ("spaces only", b"   "),
        ("all C whitespace only", b" \t\n\x0b\x0c\r"),
        ("many newlines", b"\n\n\n\n"),
    ]);
}

/// A single well-formed value: the happy path.
#[test]
fn single_plain_value() {
    check_all(&[
        ("zero", b"0"),
        ("one", b"1"),
        ("negative one", b"-1"),
        ("42", b"42"),
        ("explicit plus", b"+5"),
        ("negative zero", b"-0"),
        ("plus zero", b"+0"),
        ("trailing newline", b"7\n"),
        ("no trailing newline", b"7"),
        ("byte 0x100 boundary", b"256"),
        ("all four bytes non-zero", b"-2023406815"), // 0x87654321
        ("0x01020304", b"16909060"),
    ]);
}

/// scanf skips leading whitespace, crossing newlines (unlike fgets).
#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    check_all(&[
        ("leading spaces", b"    9"),
        ("leading newlines", b"\n\n9"),
        ("mixed leading whitespace", b"  \t\n \r\n 7"),
        ("vt and ff and cr", b"\x0b\x0c\r42"),
        ("newline before negative", b"\n-13"),
        ("whitespace before sign then digits", b"  \n  +99"),
    ]);
}

/// Only the first conversion happens; the rest of stdin is never read.
#[test]
fn only_first_token_is_consumed() {
    check_all(&[
        ("two numbers", b"1 2"),
        ("number then newline then number", b"1\n2\n"),
        ("number then letters", b"12abc"),
        ("decimal point", b"1.5"),
        ("thousands separator", b"1,234"),
        ("number then NUL", b"5\x00"),
        ("number then huge tail", b"3 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ("hex-looking input stops at x", b"0x10"),
        ("exponent-looking input", b"1e9"),
    ]);
}

/// Matching failure: scanf returns 0 without storing, so x keeps its
/// initialiser and print_hex prints four zero bytes.
#[test]
fn matching_failure_leaves_x_at_zero() {
    check_all(&[
        ("letters", b"abc"),
        ("lone minus", b"-"),
        ("lone plus", b"+"),
        ("double minus", b"--5"),
        ("sign then space then digits", b"- 5"),
        ("plus then space then digits", b"+ 5"),
        ("sign then letter", b"-a"),
        ("leading NUL bytes", b"\x00\x00 5"),
        ("period first", b".5"),
        ("word inf", b"inf"),
        ("word nan", b"nan"),
        ("escape sequence", b"\x1b[5"),
        ("non-utf8 bytes", b"\xff\xfe 9"),
        ("utf8 nbsp is not C whitespace", b"\xc2\xa0 5"),
        ("underscore", b"_1"),
        ("newline then letters", b"\n\nzzz"),
    ]);
}

/// The extremes an `int` can hold, and the truncation just past them.
#[test]
fn int_boundaries_and_truncation() {
    check_all(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MAX + 1", b"2147483648"),
        ("INT_MIN - 1", b"-2147483649"),
        ("UINT_MAX", b"4294967295"),
        ("2^32", b"4294967296"),
        ("2^32 + 1", b"4294967297"),
        ("-2^32", b"-4294967296"),
        ("2^33", b"8589934592"),
    ]);
}

/// strtol saturates at LONG_MAX / LONG_MIN before the store into `int*`
/// truncates, so everything past long's range folds onto the same two values.
#[test]
fn long_range_saturation_then_truncation() {
    check_all(&[
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MAX + 1", b"9223372036854775808"),
        ("LONG_MIN - 1", b"-9223372036854775809"),
        ("ULONG_MAX", b"18446744073709551615"),
        ("2^64", b"18446744073709551616"),
        ("20 nines", b"99999999999999999999"),
        ("negative 26 nines", b"-99999999999999999999999999"),
    ]);
}

/// Leading zeros are digits, not an octal prefix, and do not count toward
/// overflow.
#[test]
fn leading_zeros() {
    check_all(&[
        ("007", b"007"),
        ("many zeros then 5", b"000000000000000000005"),
        ("zeros then INT_MAX+1", b"0000000000000000000000002147483648"),
        ("negative with leading zeros", b"-000042"),
        ("zeros only", b"00000000"),
    ]);
}

/// Very long digit runs: the conversion must not choke or change answer.
#[test]
fn very_long_digit_runs() {
    let mut zeros = vec![b'0'; 10_000];
    zeros.push(b'5');
    let nines = vec![b'9'; 5_000];
    let mut neg_nines = vec![b'-'];
    neg_nines.extend(std::iter::repeat(b'9').take(5_000));
    let big_whitespace = {
        let mut v = vec![b' '; 8192];
        v.extend_from_slice(b"\n\t 123");
        v
    };

    check_all(&[
        ("10k zeros then 5", &zeros),
        ("5k nines", &nines),
        ("negative 5k nines", &neg_nines),
        ("8k spaces then a number", &big_whitespace),
    ]);
}

/// Command-line arguments: `main()` takes none, so they must be ignored
/// identically by both programs.
#[test]
fn arguments_are_ignored() {
    let c = c_bin();
    let r = rust_bin();
    for args in [vec!["99"], vec!["-x", "--help"], vec![""]] {
        let run_with = |bin: &Path| {
            let mut child = Command::new(bin)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            {
                let mut stdin = child.stdin.take().unwrap();
                let _ = stdin.write_all(b"5\n");
            }
            let out = child.wait_with_output().unwrap();
            (out.stdout, out.stderr, out.status.code())
        };
        let a = run_with(&c);
        let b = run_with(&r);
        assert_eq!(a, b, "argument handling differs for {args:?}");
    }
}

/// stdin at EOF from a real file rather than a pipe (/dev/null).
#[test]
fn stdin_from_dev_null() {
    #[cfg(unix)]
    {
        let mut results = Vec::new();
        for bin in [c_bin(), rust_bin()] {
            let out = Command::new(&bin)
                .stdin(Stdio::from(std::fs::File::open("/dev/null").unwrap()))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("output");
            results.push((out.stdout, out.stderr, out.status.code()));
        }
        assert_eq!(results[0], results[1], "behavior differs with stdin=/dev/null");
    }
}
