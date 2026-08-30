//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses on identical stdin, and compare stdout, stderr and exit status
//! byte for byte.
//!
//! The Rust code is never called as a library; both programs are driven exactly
//! the way a shell would drive them, because that is how they are compared.
//!
//! # What the C program branches on
//!
//! ```c
//! int main() {
//!     int x = 0;
//!     scanf("%d", &x);
//!     if (x) { good(); } else { bad(); }
//!     return 0;
//! }
//! ```
//!
//! `scanf`'s return value is ignored, so a matching failure or EOF leaves `x`
//! at its initializer `0`. That collapses to exactly two observable outputs,
//! reached by three distinct input classes:
//!
//! * `good()` prints `5` — reached when the parsed value is non-zero.
//! * `bad()` prints `0` — reached when the parsed value is zero, when `scanf`
//!   fails to match, and when an overflowing value truncates to zero.
//!
//! `bad()` dereferences an uninitialized `int *`, which is undefined behavior.
//! These tests pin the reference build's observed behavior rather than assuming
//! any particular value.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Path to the Rust binary under test, built by cargo for this test target.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C reference binary, building it with cmake if needed.
fn c_binary() -> PathBuf {
    let c_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .join("c_src");
    let bin = c_src.join("build").join("driver");
    if bin.is_file() {
        return bin;
    }

    // Build it, so that `cargo test` is self-sufficient. A comparison against a
    // program that did not build measures nothing, so failure here is fatal.
    let build_dir = c_src.join("build");
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build_dir)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&configure.stderr)
    );
    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .output()
        .expect("run cmake --build");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(bin.is_file(), "C binary missing after build: {}", bin.display());
    bin
}

/// Run `bin` with `input` on stdin, capturing stdout, stderr and exit status.
fn run(bin: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    // Write on a worker thread so a large input cannot deadlock against the
    // child filling its stdout pipe.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || {
        // A broken pipe is expected: the program may exit without reading all
        // of a large input, so this error is deliberately not fatal.
        let _ = stdin.write_all(&payload);
        let _ = stdin.flush();
    });

    let output = child.wait_with_output().expect("collect child output");
    writer.join().expect("stdin writer thread");
    output
}

/// Render a byte string for assertion messages, escaping non-printables and
/// eliding very long inputs.
fn show(bytes: &[u8]) -> String {
    const LIMIT: usize = 96;
    let mut s = String::new();
    for &b in bytes.iter().take(LIMIT) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > LIMIT {
        s.push_str(&format!("...({} bytes total)", bytes.len()));
    }
    s
}

/// Assert the C and Rust binaries agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch [{label}] input=\"{}\"\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch [{label}] input=\"{}\"\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit code mismatch [{label}] input=\"{}\": C={:?} Rust={:?}",
        show(input),
        c.status,
        r.status
    );
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            c.status.signal(),
            r.status.signal(),
            "terminating signal mismatch [{label}] input=\"{}\": C={:?} Rust={:?}",
            show(input),
            c.status,
            r.status
        );
    }
}

// ---------------------------------------------------------------------------
// Class 1: scanf parses a non-zero value -> good() -> prints 5
// ---------------------------------------------------------------------------

#[test]
fn nonzero_values_take_the_good_branch() {
    for v in [
        "1", "5", "7", "42", "-1", "-5", "-42", "+1", "+42", "000001",
        "0000000005", "-000001", "9", "10", "100", "12345",
    ] {
        assert_same(&format!("nonzero {v}"), v.as_bytes());
    }
}

#[test]
fn integer_bounds() {
    for v in [
        "2147483647",           // INT_MAX
        "-2147483648",          // INT_MIN
        "9223372036854775807",  // LONG_MAX exactly
        "-9223372036854775808", // LONG_MIN exactly
    ] {
        assert_same(&format!("bound {v}"), v.as_bytes());
    }
}

/// glibc reads `%d` into a `long`, then the assignment to `int` truncates.
/// These values are non-zero only *after* that truncation.
#[test]
fn overflow_truncates_to_nonzero() {
    for v in [
        "2147483648",           // INT_MAX+1 -> INT_MIN
        "-2147483649",          // INT_MIN-1 -> INT_MAX
        "4294967297",           // 2^32+1 -> 1
        "-4294967297",          // -(2^32+1) -> -1
        "99999999999999999999", // saturates to LONG_MAX -> -1
    ] {
        assert_same(&format!("overflow-nonzero {v}"), v.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Class 2: scanf parses zero -> bad()
// ---------------------------------------------------------------------------

#[test]
fn explicit_zero_takes_the_bad_branch() {
    for v in ["0", "-0", "+0", "00", "0000000000000000000000000"] {
        assert_same(&format!("zero {v}"), v.as_bytes());
    }
}

/// Truncation to `int` can turn a large non-zero value into zero, which flips
/// the branch to `bad()`.
#[test]
fn overflow_truncates_to_zero() {
    for v in [
        "4294967296",            // 2^32 -> 0
        "-4294967296",           // -2^32 -> 0
        "8589934592",            // 2^33 -> 0
        "-99999999999999999999", // saturates to LONG_MIN -> 0
    ] {
        assert_same(&format!("overflow-zero {v}"), v.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Class 3: scanf fails to match or hits EOF -> x keeps its initializer 0
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_at_zero() {
    assert_same("empty", b"");
}

#[test]
fn matching_failure_leaves_x_at_zero() {
    for v in [
        "abc", "-", "+", "-x", "+x", ".", "0x10", "3.9", "e5", "--1", "++1",
        "/", ":", "one",
    ] {
        assert_same(&format!("nomatch {v}"), v.as_bytes());
    }
}

#[test]
fn whitespace_only_input_hits_eof() {
    for v in [" ", "     ", "\n", "\n\n\n", "\t", "\r", "  \t\n\r\x0b\x0c  "] {
        assert_same(&format!("ws-only {}", show(v.as_bytes())), v.as_bytes());
    }
}

#[test]
fn non_ascii_and_control_bytes() {
    for v in [
        &b"\x00"[..],
        &b"\xff"[..],
        &b"\x80\x81"[..],
        &b"\x00\x00\x00"[..],
        "é".as_bytes(),
    ] {
        assert_same(&format!("bytes {}", show(v)), v);
    }
}

// ---------------------------------------------------------------------------
// scanf reading behavior: %d skips leading whitespace, including newlines,
// and stops at the first non-digit without consuming the rest of the line.
// ---------------------------------------------------------------------------

#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    for v in [
        "\n1", "\n\n\n\n5", "\t\t9", "   1", "   \n  0", "\r\n3", " \t 42",
        "\n\n0",
    ] {
        assert_same(&format!("lead-ws {}", show(v.as_bytes())), v.as_bytes());
    }
}

#[test]
fn trailing_input_after_the_number_is_ignored() {
    for v in [
        "1\n", "1abc", "0abc", "1 2", "0\n1\n", "0\nabc", "1\n\n\n", "5 junk",
        "-7xyz", "0 0 0",
    ] {
        assert_same(&format!("trailing {}", show(v.as_bytes())), v.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Large and pathological inputs. Long digit runs push glibc's `%d` conversion
// onto its wide-input path, which changes the stack that `bad()` reads from.
// ---------------------------------------------------------------------------

#[test]
fn very_long_digit_runs() {
    let cases: Vec<(String, Vec<u8>)> = vec![
        ("4096 zeros".into(), vec![b'0'; 4096]),
        ("200k zeros".into(), vec![b'0'; 200_000]),
        ("100k nines".into(), vec![b'9'; 100_000]),
        ("-100k nines".into(), {
            let mut v = vec![b'-'];
            v.extend(std::iter::repeat(b'9').take(100_000));
            v
        }),
        ("100k leading zeros then 2^32".into(), {
            let mut v = vec![b'0'; 100_000];
            v.extend_from_slice(b"4294967296");
            v
        }),
        ("5000 leading zeros then 7".into(), {
            let mut v = vec![b'0'; 5000];
            v.push(b'7');
            v
        }),
    ];
    for (label, input) in cases {
        assert_same(&label, &input);
    }
}

#[test]
fn very_long_whitespace_and_junk_runs() {
    let cases: Vec<(String, Vec<u8>)> = vec![
        ("8192 spaces".into(), vec![b' '; 8192]),
        ("8192 newlines".into(), vec![b'\n'; 8192]),
        ("100k spaces".into(), vec![b' '; 100_000]),
        ("100k letters".into(), vec![b'z'; 100_000]),
        ("5000 spaces then 1".into(), {
            let mut v = vec![b' '; 5000];
            v.push(b'1');
            v
        }),
        ("64k newlines then 0".into(), {
            let mut v = vec![b'\n'; 65_536];
            v.push(b'0');
            v
        }),
    ];
    for (label, input) in cases {
        assert_same(&label, &input);
    }
}

// ---------------------------------------------------------------------------
// Randomized differential sweep, deterministic so failures reproduce.
// ---------------------------------------------------------------------------

/// Small xorshift PRNG; a fixed seed keeps this test reproducible.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[test]
fn randomized_numeric_sweep() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..150 {
        let v = rng.next_u64() as i64 as i32;
        assert_same("random i32", v.to_string().as_bytes());
    }
    // Values near the branch boundary and the truncation boundaries.
    for base in [0i64, 1, -1, 2147483647, -2147483648, 4294967296, -4294967296] {
        for delta in -2i64..=2 {
            let v = base.wrapping_add(delta);
            assert_same("boundary", v.to_string().as_bytes());
        }
    }
}

#[test]
fn randomized_digit_strings_of_varying_length() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for len in [1usize, 2, 5, 9, 10, 11, 18, 19, 20, 21, 25, 40] {
        for _ in 0..4 {
            let digits: Vec<u8> = (0..len)
                .map(|_| b'0' + rng.below(10) as u8)
                .collect();
            assert_same("random digits", &digits);
            let mut negative = vec![b'-'];
            negative.extend_from_slice(&digits);
            assert_same("random negative digits", &negative);
        }
    }
}

#[test]
fn randomized_junk_sweep() {
    const ALPHABET: &[u8] = b"0123456789+-. \n\tabcxyzXYZ\x00\xff/:";
    let mut rng = Rng(0xDEAD_BEEF_CAFE_D00D);
    for _ in 0..120 {
        let len = rng.below(14) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len() as u64) as usize])
            .collect();
        assert_same("random junk", &input);
    }
}

// ---------------------------------------------------------------------------
// Environment-level behavior that the input sweep cannot reach.
// ---------------------------------------------------------------------------

/// A write to a closed pipe must kill both programs with `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, which C does
/// not; without restoring the default disposition, the Rust binary exits 0
/// where the C binary dies from signal 13.
///
/// The read end is closed *before* the child is spawned, so the child's first
/// write always fails: there is no race.
#[test]
fn broken_stdout_pipe_kills_both_with_sigpipe() {
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::ExitStatusExt;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    fn status_on_broken_pipe(bin: &Path) -> std::process::ExitStatus {
        let mut fds = [0i32; 2];
        // SAFETY: `fds` is a valid 2-element array, as pipe(2) requires.
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe(2) failed");
        let (read_fd, write_fd) = (fds[0], fds[1]);
        // Close the read end up front so any write to `write_fd` raises SIGPIPE.
        // SAFETY: `read_fd` is a live fd we own and never use again.
        unsafe { close(read_fd) };

        // SAFETY: `write_fd` is a live fd we own; Stdio takes ownership of it.
        let stdout = unsafe { Stdio::from_raw_fd(write_fd) };
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
        {
            let mut stdin = child.stdin.take().expect("piped stdin");
            let _ = stdin.write_all(b"1");
        }
        child.wait().expect("wait for child")
    }

    let c = status_on_broken_pipe(&c_binary());
    let r = status_on_broken_pipe(&rust_binary());
    assert_eq!(
        c.signal(),
        r.signal(),
        "SIGPIPE signal mismatch: C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c.code(),
        r.code(),
        "exit code mismatch on broken pipe: C={c:?} Rust={r:?}"
    );
    assert_eq!(c.signal(), Some(13), "expected the C binary to die on SIGPIPE");
}

/// With stdin at EOF immediately, `scanf` fails and `x` stays 0.
#[test]
fn stdin_at_eof_from_dev_null() {
    let c = Command::new(c_binary())
        .stdin(Stdio::null())
        .output()
        .expect("run C binary");
    let r = Command::new(rust_binary())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust binary");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with stdin=/dev/null");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with stdin=/dev/null");
    assert_eq!(c.status.code(), r.status.code(), "exit code mismatch");
}

/// Command-line arguments are ignored by `main`, which takes no parameters.
#[test]
fn extra_arguments_are_ignored() {
    for args in [vec!["ignored"], vec!["-1", "2"], vec!["--help"]] {
        let c = Command::new(c_binary())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run C binary");
        let r = Command::new(rust_binary())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run Rust binary");
        assert_eq!(c.stdout, r.stdout, "stdout mismatch with args {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr mismatch with args {args:?}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit code mismatch with args {args:?}"
        );
    }
}

/// `bad()` reads an uninitialized stack slot, so its output could in principle
/// depend on the surrounding environment. Pin that it does not.
#[test]
fn bad_branch_is_stable_across_environment_size() {
    for pad in [0usize, 1, 100, 1000, 5000] {
        let value = "A".repeat(pad);
        let c = Command::new(c_binary())
            .env("DIFFTEST_PADDING", &value)
            .stdin(Stdio::null())
            .output()
            .expect("run C binary");
        let r = Command::new(rust_binary())
            .env("DIFFTEST_PADDING", &value)
            .stdin(Stdio::null())
            .output()
            .expect("run Rust binary");
        assert_eq!(c.stdout, r.stdout, "stdout mismatch with env padding {pad}");
        assert_eq!(c.status.code(), r.status.code(), "exit code mismatch");
    }
}

/// Repeated runs must be deterministic, since `bad()`'s value is undefined
/// behavior that only happens to be stable in the reference build.
#[test]
fn repeated_runs_are_deterministic() {
    for _ in 0..20 {
        assert_same("repeat bad branch", b"0");
        assert_same("repeat good branch", b"1");
    }
}
