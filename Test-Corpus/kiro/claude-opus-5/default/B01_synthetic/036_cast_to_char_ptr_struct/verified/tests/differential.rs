//! Differential tests: run the original C executable and the Rust executable as
//! subprocesses, feed both the same bytes on stdin, and require byte-identical
//! stdout, byte-identical stderr and an identical exit status.
//!
//! The Rust code is never linked or called as a library here; both programs are
//! driven exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

/// `translation/` (the crate root).
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory that holds both `c_src/` and `translation/`.
fn workspace_dir() -> PathBuf {
    crate_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust executable built by cargo for this integration test.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Configure + build `c_src` with CMake (once per test binary) and return the
/// path to the resulting `driver` executable.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_dir().join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "expected {} to exist",
            c_src.join("CMakeLists.txt").display()
        );

        // Build out-of-tree so nothing in c_src/src is touched.
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");

        let exe = build_dir.join("driver");
        if !exe.is_file() {
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build_dir)
                .output()
                .expect("failed to run `cmake ..` (is cmake installed?)");
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
                .expect("failed to run `cmake --build .`");
            assert!(
                build.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
        }

        assert!(
            exe.is_file(),
            "C executable not found at {}",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
    /// Signal number, when the process was terminated by one.
    signal: Option<i32>,
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Run {
    use std::io::Write;

    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("piped stdin");
        // The child may legitimately stop reading; a short write error is not a
        // test failure, so ignore it and let the exit status speak.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
        // dropping `sink` closes the pipe -> the child observes EOF
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", exe.display()));

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

/// Human-readable rendering of a byte string for assertion messages.
fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:02x?}"),
    }
}

/// Core assertion: both programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{label}` (stdin = {})\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{label}` (stdin = {})\n  C   : {}\n  Rust: {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for case `{label}` (stdin = {})\n  C   : code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
        show(stdin_bytes),
        c.code,
        c.signal,
        r.code,
        r.signal
    );
}

fn check(label: &str, stdin_text: &str) {
    assert_same(label, stdin_text.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A — both programs build and run
// ---------------------------------------------------------------------------

#[test]
fn a_both_programs_build_and_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(&rust_bin(), b"1\n");
    assert_eq!(c.code, Some(0), "C program should exit 0 on valid input");
    assert_eq!(r.code, Some(0), "Rust program should exit 0 on valid input");
    assert!(!c.stdout.is_empty(), "C program produced no stdout");
    assert_eq!(c.stdout, r.stdout);
}

/// Pins down the actual byte image so a silently-changed struct layout is
/// caught even if both programs were to drift together.
#[test]
fn a_known_good_output_shape() {
    let r = run(&rust_bin(), b"1\n");
    assert_eq!(
        r.stdout,
        b"01000000030000000000000000000040\n".to_vec(),
        "floors=1, bedrooms=3, bathrooms=2.0 little-endian hex dump + newline"
    );
    let c = run(c_bin(), b"1\n");
    assert_eq!(c.stdout, r.stdout);
}

// ---------------------------------------------------------------------------
// Phase B — the input classes the C program branches on
//
// `main` is `scanf("%d", &x); driver(x);`. Every branch lives inside the
// library conversion, so the input classes are the states of a `%d` match:
//   * EOF before any non-whitespace     -> no assignment, x stays 0
//   * matching failure (no digits)      -> no assignment, x stays 0
//   * successful conversion             -> x assigned
//   * conversion that overflows `long`  -> saturates, then truncates to int
//   * conversion that fits `long` but not `int` -> truncates to int
// ---------------------------------------------------------------------------

#[test]
fn b_empty_input_no_assignment() {
    check("empty", "");
}

#[test]
fn b_single_item() {
    check("single digit 1", "1");
    check("single digit 1 + newline", "1\n");
    check("zero", "0");
    check("three", "3");
}

#[test]
fn b_negative_values() {
    check("-1", "-1");
    check("-0", "-0");
    check("-7\\n", "-7\n");
}

#[test]
fn b_explicit_plus_sign() {
    check("+5", "+5");
    check("+0", "+0");
    check("+0000012", "+0000012");
}

#[test]
fn b_int_extremes() {
    check("INT_MAX", "2147483647");
    check("INT_MAX-1", "2147483646");
    check("INT_MIN", "-2147483648");
}

#[test]
fn b_values_that_truncate_to_int() {
    check("INT_MAX+1", "2147483648");
    check("2^32", "4294967296");
    check("2^32-1", "4294967295");
    check("2^33", "8589934592");
    check("3e9", "3000000000");
    check("INT_MIN-1", "-2147483649");
}

#[test]
fn b_values_that_overflow_long() {
    check("LONG_MAX", "9223372036854775807");
    check("LONG_MAX+1", "9223372036854775808");
    check("LONG_MIN", "-9223372036854775808");
    check("LONG_MIN-1", "-9223372036854775809");
    check("2^64", "18446744073709551616");
    check("26 nines", "99999999999999999999999999");
    check("negative 20 nines", "-99999999999999999999");
}

#[test]
fn b_matching_failure_paths() {
    check("letters", "abc");
    check("lone minus", "-");
    check("lone plus", "+");
    check("double minus", "--5");
    check("minus space digit", "- 5");
    check("ws minus ws digit", "  -  5");
    check("leading dot", ".5");
    check("exponent-looking", "e5");
    check("comma", ",1");
    check("non-ascii digit U+0663", "\u{0663}");
}

#[test]
fn b_whitespace_only_reaches_eof() {
    check("single space", " ");
    check("three spaces", "   ");
    check("single newline", "\n");
    check("tabs", "\t\t\t");
    check("mixed ws + newlines", "  \n  \n");
    check("all C isspace chars", " \t\n\u{0b}\u{0c}\r");
}

#[test]
fn b_scanf_skips_leading_whitespace_across_newlines() {
    check("newlines then 42", "\n\n\n42\n");
    check("spaces tabs newline then 7", "   \n\t 7");
    check("vt ff cr then 9", "\u{0b}\u{0c}\r 9");
    check("ws then INT_MIN-1", " \n\t\u{0b}\u{0c}\r-2147483649");
}

#[test]
fn b_conversion_stops_at_first_non_digit() {
    check("5abc", "5abc");
    check("0x10 is base 10", "0x10");
    check("12.7 stops at dot", "12.7");
    check("only first field read", "5 9");
    check("only first line read", "1\n2");
    check("trailing junk lines", "8\nignored\nalso ignored\n");
}

#[test]
fn b_leading_zeros_are_decimal_not_octal() {
    check("007", "007");
    check("0000000009", "0000000009");
}

// ---------------------------------------------------------------------------
// Phase C — paths not covered above
// ---------------------------------------------------------------------------

/// A NUL byte is not whitespace and not a digit, so it is a matching failure;
/// after a digit run it simply terminates the conversion.
#[test]
fn c_embedded_nul_bytes() {
    assert_same("NUL then digit", b"\0 5");
    assert_same("digit then NUL", b"5\0");
    assert_same("only NUL", b"\0");
}

/// Digit runs long enough to exercise glibc's internal work-buffer growth.
#[test]
fn c_very_long_digit_runs() {
    let nines = "9".repeat(5000);
    check("5000 nines", &nines);

    let neg_nines = format!("-{}", "9".repeat(5000));
    check("negative 5000 nines", &neg_nines);

    let padded = format!("{}7", "0".repeat(5000));
    check("5000 zeros then 7", &padded);

    let padded_neg = format!("-{}42", "0".repeat(4096));
    check("negative 4096 zeros then 42", &padded_neg);
}

/// Whitespace run long enough to cross any internal buffer boundary before the
/// first significant character appears.
#[test]
fn c_long_whitespace_prefix() {
    let ws = format!("{}123", " ".repeat(9000));
    check("9000 spaces then 123", &ws);

    let only_ws = "\n".repeat(9000);
    check("9000 newlines only", &only_ws);
}

/// Arbitrary non-text input must not diverge (and must not panic in Rust).
#[test]
fn c_binary_input() {
    let bytes: Vec<u8> = (0u8..=255).collect();
    assert_same("all 256 byte values", &bytes);

    // High-bit bytes only: no whitespace, no digits -> matching failure.
    let high: Vec<u8> = (0x80u8..=0xff).collect();
    assert_same("high-bit bytes", &high);

    // Invalid UTF-8 sandwiching a digit.
    assert_same("invalid utf8 then digit", &[0xff, 0xfe, b'4']);
    assert_same("digit then invalid utf8", &[b'4', 0xff, 0xfe]);
}

/// Every single-byte input, to sweep the whitespace / sign / digit / other
/// classification of the first character exhaustively.
#[test]
fn c_every_single_byte_input() {
    for b in 0u8..=255 {
        assert_same(&format!("single byte 0x{b:02x}"), &[b]);
    }
}

/// Every two-byte input of the form <sign-or-space><byte>, sweeping what may
/// follow a sign or a whitespace skip.
#[test]
fn c_sign_or_space_followed_by_every_byte() {
    for lead in [b'-', b'+', b' ', b'\n'] {
        for b in 0u8..=255 {
            assert_same(&format!("0x{lead:02x} then 0x{b:02x}"), &[lead, b]);
        }
    }
}

/// Each individual digit as the whole input, and each digit after a sign.
#[test]
fn c_each_digit_value() {
    for d in b'0'..=b'9' {
        assert_same(&format!("digit {}", d as char), &[d]);
        assert_same(&format!("minus digit {}", d as char), &[b'-', d]);
        assert_same(&format!("plus digit {}", d as char), &[b'+', d]);
    }
}

/// Powers of two and their neighbours around the 32-bit boundary, where the
/// `(int)long` truncation in the C library is observable.
#[test]
fn c_truncation_boundary_sweep() {
    for shift in 28..=40u32 {
        let base: i64 = 1i64 << shift;
        for delta in [-1i64, 0, 1] {
            let v = base + delta;
            check(&format!("{v}"), &v.to_string());
            check(&format!("{}", -v), &(-v).to_string());
        }
    }
}

/// Values chosen so that the low 32 bits are zero, sign bit set, etc.
#[test]
fn c_bit_pattern_cases() {
    for v in [
        "2147483649",
        "6442450944",
        "4294967297",
        "-4294967296",
        "-4294967295",
        "-2147483647",
        "1000000",
        "65536",
        "255",
        "256",
    ] {
        check(v, v);
    }
}

/// stdin closed / unreadable, and a very large stdin that is mostly ignored.
#[test]
fn c_stdin_edge_shapes() {
    // A megabyte of input where only the first token matters.
    let mut big = String::from("11\n");
    big.push_str(&"0123456789\n".repeat(90_000));
    check("large trailing input after token", &big);

    // Token appears only after a lot of unmatched junk -> matching failure,
    // the rest is never consumed.
    let mut junk = String::from("junk");
    junk.push_str(&"x".repeat(100_000));
    junk.push_str("42");
    check("junk prefix then digits", &junk);
}
