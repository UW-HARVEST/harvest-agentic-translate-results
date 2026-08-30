//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status are byte-for-byte / value identical.
//!
//! The Rust program is NEVER loaded as a library here -- it is driven exactly
//! the way a shell drives it, because that is how it is compared against the C.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two executables
// ---------------------------------------------------------------------------

/// Repository root: the parent of the `translation/` crate directory.
fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Builds `c_src` with CMake (once per test binary) and returns the executable.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("create c_src/build");

            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` -- is cmake installed?");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr),
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
                String::from_utf8_lossy(&compile.stderr),
            );
        }

        assert!(
            exe.is_file(),
            "the C executable was not produced at {}",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("status", &self.status)
            .finish()
    }
}

fn run(exe: &Path, stdin_bytes: &[u8], args: &[&str]) -> Outcome {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("piped stdin");
        // The child may legitimately stop reading; a write failure is not a
        // test failure, so the result is deliberately ignored.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

/// Core assertion: the C and Rust programs agree on all three observables.
#[track_caller]
fn assert_same_with_args(label: &str, stdin_bytes: &[u8], args: &[&str]) {
    let c = run(c_binary(), stdin_bytes, args);
    let r = run(rust_binary(), stdin_bytes, args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (stdin = {:?}, args = {args:?})\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(stdin_bytes),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (stdin = {:?}, args = {args:?})\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(stdin_bytes),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (stdin = {:?}, args = {args:?})",
        String::from_utf8_lossy(stdin_bytes),
    );
}

#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    assert_same_with_args(label, stdin_bytes, &[]);
}

#[track_caller]
fn assert_same_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

/// Convenience: build a two-line stdin from one token, and also exercise the
/// same token without a trailing newline (which changes what the second
/// `fgets` sees).
#[track_caller]
fn assert_token(token: &str) {
    let both = format!("{token}\n{token}\n");
    assert_same(&format!("token {token:?} twice"), both.as_bytes());

    let no_eol = format!("{token}\n{token}");
    assert_same(&format!("token {token:?} no trailing eol"), no_eol.as_bytes());

    let only_one = format!("{token}\n");
    assert_same(&format!("token {token:?} once"), only_one.as_bytes());
}

// ===========================================================================
// Phase A sanity: both binaries exist and the well-known baseline holds
// ===========================================================================

#[test]
fn both_binaries_are_runnable_and_agree_on_the_baseline() {
    // Also pin the literal expected bytes, so a regression in BOTH programs
    // (or a mis-built C binary) cannot silently make the suite vacuous.
    let expected = b"Calling good()...\n50\n20\nFinished good()\nCalling bad()...\n25\nFinished bad()\n";

    let c = run(c_binary(), b"5\n4\n", &[]);
    assert_eq!(c.stdout, expected, "C baseline output changed");
    assert_eq!(c.stderr, b"");
    assert_eq!(c.status, Some(0));

    assert_same("baseline 5/4", b"5\n4\n");
}

// ===========================================================================
// Phase B: the branches the C source actually has
// ===========================================================================

/// `fgets() != NULL` vs the `else` branch, in goodB2G and in bad().
///
/// * empty stdin  -> both fgets fail  -> "fgets() failed." twice
/// * one line     -> goodB2G succeeds, bad()'s fgets fails
/// * two lines    -> both succeed
#[test]
fn fgets_null_and_non_null_branches() {
    assert_same_all(&[
        ("empty stdin: both fgets return NULL", b""),
        ("one line: only bad()'s fgets returns NULL", b"5\n"),
        ("one line, no trailing newline", b"5"),
        ("two lines: neither fgets returns NULL", b"5\n4\n"),
        ("two lines, no trailing newline", b"5\n4"),
        ("three lines: third is never read", b"5\n4\n3\n"),
        ("four lines: extras never read", b"2\n4\n8\n16\n"),
        ("a single bare newline", b"\n"),
        ("two bare newlines", b"\n\n"),
        ("three bare newlines", b"\n\n\n"),
    ]);
}

/// goodB2G's `fabs(data) > 0.000001` -- both sides of the comparison.
#[test]
fn goodb2g_divide_by_zero_guard_both_branches() {
    assert_same_all(&[
        // false branch -> "This would result in a divide by zero"
        ("exact zero", b"0\n0\n"),
        ("negative zero", b"-0\n-0\n"),
        ("negative zero, decimal", b"-0.0\n-0.0\n"),
        ("plus zero, decimal", b"+0.0\n+0.0\n"),
        ("unparseable -> atof gives 0.0", b"abc\nxyz\n"),
        ("empty line -> atof gives 0.0", b"\n5\n"),
        ("whitespace only", b" \n \n"),
        ("tabs only", b"\t\t\t\n\t\t\t\n"),
        ("below the epsilon", b"0.0000001\n0.0000001\n"),
        ("negative, below the epsilon", b"-0.0000001\n-0.0000001\n"),
        // NaN: fabs(NaN) > eps is false, so goodB2G takes the "zero" branch,
        // while bad() divides and casts NaN to int.
        ("NaN", b"nan\nnan\n"),
        ("negative NaN", b"-nan\n-NAN\n"),
        // exactly at the epsilon: 1e-6 as a float is > 1e-6 as a double,
        // so this actually takes the TRUE branch -- pinned by the C.
        ("exactly the epsilon", b"0.000001\n0.000001\n"),
        ("just above the epsilon", b"0.0000011\n0.0000011\n"),
        ("just below the epsilon", b"9.9999994e-7\n9.9999994e-7\n"),
        ("just at/above in float", b"9.9999995e-7\n9.9999995e-7\n"),
        // true branch
        ("plain positive", b"4\n4\n"),
        ("plain negative", b"-4\n-4\n"),
        ("fraction", b"0.5\n0.5\n"),
        ("large", b"1e6\n1e6\n"),
    ]);
}

/// bad() has no guard at all: it always divides. Division by zero yields
/// +/-inf, and the `(int)` cast of inf/NaN is UB that the C binary resolves to
/// INT_MIN on x86-64. The Rust must reproduce that exact value.
#[test]
fn bad_divides_unconditionally_including_by_zero() {
    assert_same_all(&[
        ("bad() divides by +0.0", b"5\n0\n"),
        ("bad() divides by -0.0", b"5\n-0\n"),
        ("bad() divides by unparseable (0.0)", b"5\nnot a number\n"),
        ("bad() divides by an empty line", b"5\n\n"),
        ("bad() gets NaN", b"5\nnan\n"),
        ("bad() gets -NaN", b"5\n-nan\n"),
        ("bad() gets +inf", b"5\ninf\n"),
        ("bad() gets -inf", b"5\n-inf\n"),
        ("bad()'s fgets fails -> data stays 0.0", b"5\n"),
        ("both zero", b"0\n0\n"),
    ]);
}

/// The `(int)` cast: in range, out of range, and right at the boundaries.
/// 100.0/x overflows `int` once |x| < ~4.6566e-8.
#[test]
fn int_cast_truncation_and_overflow() {
    for token in [
        "100",          // -> 1
        "3",            // -> 33 (truncation toward zero)
        "-3",           // -> -33
        "7",            // -> 14
        "-7",           // -> -14
        "0.5",          // -> 200
        "1e-5",         // -> 10000000
        "1e-7",         // -> 1000000000
        "4.6566129e-8", // just inside INT_MAX
        "4.6566128e-8", // right at the boundary
        "4.6566127e-8", // just outside -> INT_MIN via UB
        "4.65661e-8",
        "-4.6566129e-8",
        "-4.6566127e-8",
        "1e-9",  // far out of range -> INT_MIN
        "1e-30", // -> INT_MIN
        "1e-45", // float subnormal -> INT_MIN
        "1e-46", // flushes to 0.0f -> inf -> INT_MIN
        "7e-46",
        "1e39",  // overflows float -> +inf -> 100/inf = 0
        "-1e39", // -> -inf -> -0.0 -> 0
        "1e300", // overflows float -> +inf
        "1e999", // strtod overflows to inf
        "-1e999",
        "1e-999", // strtod underflows to 0.0
        "-1e-999",
        "3.4028234e38", // largest finite float
        "3.4028236e38", // just past it -> inf
    ] {
        assert_token(token);
    }
}

/// `fgets` reads at most CHAR_ARRAY_SIZE-1 == 19 bytes and does NOT skip past
/// the newline, so a long first line is split: the tail feeds the second read.
/// This is the classic behaviour the task calls out, so every length around
/// the 19/20 byte boundary gets its own case.
#[test]
fn fgets_nineteen_byte_buffer_boundary() {
    for n in 0..=26usize {
        let body = "1".repeat(n);

        let with_second = format!("{body}\n2\n");
        assert_same(
            &format!("first line of {n} '1's, then a second line"),
            with_second.as_bytes(),
        );

        assert_same(&format!("a lone line of {n} '1's, no newline"), body.as_bytes());

        let with_newline = format!("{body}\n");
        assert_same(
            &format!("a lone line of {n} '1's, with a newline"),
            with_newline.as_bytes(),
        );
    }

    assert_same_all(&[
        // 24 chars: the first fgets takes "1111111111111111111" (19 ones ->
        // 1.1e18) and the second takes the "12222\n" remainder.
        ("24-char line split across both reads", b"111111111111111111112222\n"),
        ("exactly 19 chars", b"1234567890123456789\n"),
        ("exactly 20 chars", b"12345678901234567890\n"),
        ("exactly 21 chars", b"123456789012345678901\n"),
        // The split lands mid-exponent, so the two halves parse differently.
        ("split inside an exponent", b"1.000000000000001e-7\n"),
        ("split inside a mantissa", b"0.000000000000000005\n"),
        ("19 spaces then a digit", b"                  1\n"),
        ("19 spaces then nothing", b"                   \n"),
        ("20 spaces", b"                    \n"),
        ("a very long run of digits", &[b'9'; 100]),
        ("a very long run of zeros", &[b'0'; 100]),
    ]);
}

/// `printf("%d\n", ...)` formatting, and the fixed `printLine` strings, are
/// compared implicitly everywhere; this pins the exact byte layout once more
/// for the two most interesting outputs.
#[test]
fn printf_formatting_is_byte_identical() {
    let cases: [&[u8]; 4] = [b"2\n2\n", b"0\n0\n", b"", b"nan\n0\n"];
    for input in cases {
        let c = run(c_binary(), input, &[]);
        let r = run(rust_binary(), input, &[]);
        assert_eq!(c.stdout, r.stdout);
        assert_eq!(c.stderr, r.stderr);
        assert_eq!(c.status, r.status);
        // No stray whitespace, and always exactly one trailing newline.
        assert!(c.stdout.ends_with(b"Finished bad()\n"));
        assert!(!c.stdout.ends_with(b"\n\n"));
    }
}

// ===========================================================================
// Phase C: the input classes not covered above
// ===========================================================================

/// Everything `strtod`/`atof` recognises, and everything it rejects.
#[test]
fn atof_accepts_and_rejects() {
    for token in [
        // plain and signed decimals
        "5", "+3", "-4", "+3.5", "-3.5", "0", "-0", "+0", "0.1",
        // a bare/edge-placed radix point
        "5.", ".5", ".25", ".", "-.", "+.", "..", "5..5",
        // exponents, valid and truncated
        "1e2", "1E-2", "1e+2", "1e", "1e+", "1e-", "0e", "e", "E", ".e5", "e5",
        "5e5e5", "1e07", "1e00000002",
        // signs with nothing after them
        "-", "+", "--5", "++5", "- 5", "+ 5",
        // leading whitespace of every flavour strtod skips
        "   7  ", "\t8", "\r9", "\x0b7", "\x0c7", " \t\r\x0b\x0c 6",
        // trailing junk stops the conversion but does not void it
        "12abc", "34xyz", "5d", "5f", "5 6", "7,8", "9;",
        // pure junk -> 0.0
        "abc", "xyz", "hello", "#", "?", "/",
        // infinities, in the spellings strtod accepts and some it does not
        "inf", "INF", "Inf", "iNf", "infinity", "INFINITY", "Infinity",
        "-inf", "+inf", "-INFINITY", "i", "in", "infin", "infinit", "infinity1",
        "inf ",
        // NaNs, including the n-char-sequence form
        "nan", "NAN", "NaN", "-nan", "+nan", "nan(1)", "nan(123)", "nan(x)",
        "nan()", "NAN(_)", "nan(", "n", "na",
        // hexadecimal floats
        "0x10", "0X10", "0x1f", "0x1p3", "0X1P3", "0x1.8p1", "0x.8p2",
        "0x1p+3", "0x1p-3", "0x0p0", "0x0.0p0", "0x1p1p1",
        // hex prefixes with no valid hex digits -> strtod converts just "0"
        "0x", "0X", "0xg", "0x.", "0x.p0", "0xp3", "-0x", "0x1p",
        // hex exponents that overflow an int
        "0x1p1000", "-0x1p-200", "0x1p9999", "0x1p-9999", "0x1p2147483648",
        // subnormal and boundary doubles
        "5e-324", "2.5e-324", "1e-323", "1e-320", "1e308", "1e309",
        // zeros with wild exponents
        "0e999", "0e-999", "0.0e999", "-0e5",
        // digit separators are NOT a C thing
        "1_000",
        // long precise values (still inside the 19-byte fgets window)
        "3.141592653589793", "2.718281828459045", "1.7976931348e308",
    ] {
        assert_token(token);
    }
}

/// Bytes that are not text: NULs (which terminate the C string early),
/// control characters, and invalid UTF-8. The Rust translation must not choke
/// on any of them.
#[test]
fn non_utf8_and_embedded_nul_bytes() {
    assert_same_all(&[
        ("NUL truncates the C string", b"5\x006\n7\x008\n"),
        ("a leading NUL", b"\x005\n\x005\n"),
        ("a lone NUL", b"\x00\n\x00\n"),
        ("NUL with no newline at all", b"\x00"),
        ("control bytes", b"\x01\x02\x03\n\x04\x05\n"),
        ("invalid UTF-8", b"\xff\xfe\n\xfd\xfc\n"),
        ("a UTF-8 continuation byte alone", b"\x80\n\x80\n"),
        ("latin-1 text then a number", b"\xc3\xa9 5\n\xc3\xa9 5\n"),
        ("a number then invalid UTF-8", b"5\xff\n4\xfe\n"),
        ("a 0x7f byte", b"\x7f\n\x7f\n"),
        ("a NUL filling the buffer", &[0u8; 40]),
        ("high bytes filling the buffer", &[0xffu8; 40]),
    ]);
}

/// CRLF line endings: `fgets` keeps the `\r`, and `strtod` stops at it, so
/// "5\r\n" still parses as 5.
#[test]
fn carriage_returns_are_kept_by_fgets() {
    assert_same_all(&[
        ("CRLF", b"5\r\n4\r\n"),
        ("CRLF with zeros", b"0\r\n0\r\n"),
        ("a lone CR, never a line terminator for fgets", b"5\r4\r"),
        ("CR before the digits", b"\r5\n\r4\n"),
        ("CRLF only", b"\r\n\r\n"),
    ]);
}

/// `main` ignores argc/argv entirely, so extra arguments must change nothing.
#[test]
fn command_line_arguments_are_ignored() {
    assert_same_with_args("no args", b"5\n4\n", &[]);
    assert_same_with_args("one arg", b"5\n4\n", &["hello"]);
    assert_same_with_args("several args", b"5\n4\n", &["a", "b", "c"]);
    assert_same_with_args("flag-looking args", b"5\n4\n", &["-h", "--help"]);
    assert_same_with_args("args with an empty stdin", b"", &["x"]);
}

/// A large amount of input: only the first 19-byte chunk and its remainder are
/// ever consumed, and neither program may fail on the unread tail.
#[test]
fn very_large_input_is_mostly_unread() {
    let big: Vec<u8> = "1234567890\n".repeat(20_000).into_bytes();
    assert_same("20k repeated lines", &big);

    let one_huge_line: Vec<u8> = {
        let mut v = vec![b'7'; 100_000];
        v.push(b'\n');
        v
    };
    assert_same("one 100k-byte line", &one_huge_line);

    let huge_whitespace: Vec<u8> = {
        let mut v = vec![b' '; 50_000];
        v.extend_from_slice(b"5\n");
        v
    };
    assert_same("50k spaces then a digit", &huge_whitespace);
}

/// A deterministic sweep so the suite covers far more of the atof/divide/cast
/// state space than the hand-written lists alone.
#[test]
fn deterministic_sweep_over_generated_inputs() {
    // A tiny xorshift PRNG keeps this reproducible with no dev-dependencies.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let alphabet: &[u8] = b"0123456789.eExXaAbBcCdDfFpPnNiItTyY+- \t\r\x0b\x0c";
    let mut cases: Vec<Vec<u8>> = Vec::new();

    // Random byte soup drawn from the interesting alphabet.
    for _ in 0..200 {
        let mut input = Vec::new();
        for _ in 0..(next() % 4) {
            let len = (next() % 24) as usize;
            for _ in 0..len {
                input.push(alphabet[(next() as usize) % alphabet.len()]);
            }
            input.push(b'\n');
        }
        cases.push(input);
    }

    // Values swept across the epsilon guard and the int-overflow boundary.
    for i in 0..60u32 {
        let near_eps = 1e-6f64 * (0.99 + 0.0004 * f64::from(i));
        let near_ovf = (100.0f64 / 2147483647.0) * (0.995 + 0.0002 * f64::from(i));
        for value in [near_eps, -near_eps, near_ovf, -near_ovf] {
            let token = format!("{value:.9e}");
            cases.push(format!("{token}\n{token}\n").into_bytes());
        }
    }

    // Powers of ten, positive and negative, across the whole float range.
    for exponent in -50i32..=50 {
        let token = format!("1e{exponent}");
        cases.push(format!("{token}\n{token}\n").into_bytes());
        let token = format!("-1e{exponent}");
        cases.push(format!("{token}\n{token}\n").into_bytes());
    }

    // Small integers, where the truncating cast is easiest to get wrong.
    for n in -40i32..=40 {
        let token = n.to_string();
        cases.push(format!("{token}\n{token}\n").into_bytes());
    }

    for (index, input) in cases.iter().enumerate() {
        assert_same(&format!("generated case #{index}"), input);
    }
}
