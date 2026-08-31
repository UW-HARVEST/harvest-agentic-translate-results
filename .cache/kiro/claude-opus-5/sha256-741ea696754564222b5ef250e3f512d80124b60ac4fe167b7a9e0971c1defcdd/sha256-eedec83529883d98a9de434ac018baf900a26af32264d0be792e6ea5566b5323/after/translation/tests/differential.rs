//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and identical exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven the
//! way a shell would drive them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test. Cargo builds it for integration tests
/// and hands us the path through this env var.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root, i.e. the directory holding both `c_src/` and `translation/`.
fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
}

/// Path to the C reference binary, building it with CMake on first use.
///
/// Only ever *builds* in `c_src/build/`; no file under `c_src/` is modified.
fn c_bin() -> &'static Path {
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
                .expect("cmake must be installed to build the C reference");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .output()
                .expect("cmake --build must run");
            assert!(
                bld.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(bin.exists(), "C reference binary missing at {}", bin.display());
        bin
    })
}

/// Everything an observer of a process run can see.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Exit code, or `None` if the process died from a signal.
    code: Option<i32>,
    /// Terminating signal number on unix, `None` otherwise.
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

/// Spawn `bin`, write `input` to its stdin, and collect stdout/stderr/status.
fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // Both programs read at most 99 bytes, so a short write cannot block:
        // any unread remainder is discarded when the child exits.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
        // Dropping closes the pipe, signalling EOF.
    }

    let out = child.wait_with_output().expect("child must be waitable");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

/// Assert the two programs are indistinguishable for `input`.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        Escaped(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {:?})\n  C:    {:?}\n  Rust: {:?}",
        Escaped(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for {label} (input {:?})",
        Escaped(input)
    );
}

/// Byte-string formatter that stays readable for non-UTF-8 input.
struct Escaped<'a>(&'a [u8]);
impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "b\"")?;
        for &b in self.0.iter().take(120) {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if self.0.len() > 120 {
            write!(f, "...({} bytes)", self.0.len())?;
        }
        write!(f, "\"")
    }
}

// ---------------------------------------------------------------------------
// Absolute anchor: the exact bytes the C program emits for the happy path.
//
// Derived by hand from c_src/src/main.c. `the_house` is a global mutated
// across both run() calls, so the second run() starts from where the first
// left off: floors 2->3->4, bathrooms 2.5->3.5->4.5, bedrooms 5->10->15.
// ---------------------------------------------------------------------------

#[test]
fn golden_happy_path_exact_bytes() {
    const EXPECTED: &str = concat!(
        "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n",
        "The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n",
        "The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n",
        "The house has 3 floors, 10 bedrooms, and 3.5 bathrooms\n",
        "The house has 3 floors, 10 bedrooms, and 3.5 bathrooms\n",
        "The house has 4 floors, 10 bedrooms, and 3.5 bathrooms\n",
        "The house has 4 floors, 10 bedrooms, and 4.5 bathrooms\n",
        "The house has 4 floors, 15 bedrooms, and 4.5 bathrooms\n",
    );
    let c = run(c_bin(), b"5\n");
    let r = run(rust_bin(), b"5\n");
    assert_eq!(String::from_utf8_lossy(&c.stdout), EXPECTED, "C reference drifted");
    assert_eq!(String::from_utf8_lossy(&r.stdout), EXPECTED);
    assert_eq!(c.stdout, r.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!((c.code, c.signal), (Some(0), None));
    assert_eq!((r.code, r.signal), (Some(0), None));
}

#[test]
fn golden_error_path_exact_bytes() {
    const EXPECTED: &str = "An error occurred\n";
    let c = run(c_bin(), b"abc\n");
    let r = run(rust_bin(), b"abc\n");
    assert_eq!(String::from_utf8_lossy(&c.stdout), EXPECTED, "C reference drifted");
    assert_eq!(String::from_utf8_lossy(&r.stdout), EXPECTED);
    assert_eq!((c.code, c.signal), (r.code, r.signal));
    // The C program returns 0 even on the error path; make that explicit.
    assert_eq!(c.code, Some(0));
}

// ---------------------------------------------------------------------------
// fgets() branches: NULL return, newline-terminated, EOF-terminated, and the
// 99-byte truncation boundary of `char in[100]`.
// ---------------------------------------------------------------------------

#[test]
fn fgets_returns_null_on_immediate_eof() {
    // `char in[100] = ""` is left untouched, so strtol sees the empty string.
    assert_same("empty stdin", b"");
}

#[test]
fn fgets_single_newline_only() {
    assert_same("bare newline", b"\n");
}

#[test]
fn fgets_stops_at_first_newline_ignoring_the_rest() {
    assert_same("two lines", b"5\n9\n");
    assert_same("many lines", b"7\n8\n9\n10\n");
    assert_same("first line empty, second numeric", b"\n5\n");
    assert_same("trailing newlines", b"5\n\n\n\n\n");
}

#[test]
fn fgets_eof_without_trailing_newline() {
    assert_same("no trailing newline", b"7");
    assert_same("negative, no trailing newline", b"-7");
}

#[test]
fn fgets_truncates_at_99_bytes() {
    // 98 zeros + '5' == exactly 99 bytes, entirely consumed.
    let mut v = vec![b'0'; 98];
    v.push(b'5');
    assert_same("99 bytes of digits, value 5", &v);

    // 99 zeros then a '9' that fgets never reads: the value is 0, not 9.
    let mut v = vec![b'0'; 99];
    v.extend_from_slice(b"9\n");
    assert_same("truncation drops the significant digit", &v);

    // 98 digits + newline: newline is the 99th byte and is kept.
    let mut v = vec![b'1'; 98];
    v.push(b'\n');
    assert_same("98 digits then newline", &v);

    // Far longer than the buffer: truncated to 99 digits, which overflows long.
    assert_same("200 ones", &vec![b'1'; 200]);

    // 99 leading spaces then a digit that is cut off -> no conversion.
    let mut v = vec![b' '; 99];
    v.extend_from_slice(b"5\n");
    assert_same("99 spaces then cut-off digit", &v);

    // Boundary sweep around the 99/100-byte cutoff.
    for n in 95..=105 {
        let mut v = vec![b'0'; n];
        v.extend_from_slice(b"7\n");
        assert_same(&format!("{n} zeros then 7"), &v);
    }
}

// ---------------------------------------------------------------------------
// parse_val branch: `endp == str` (strtol performed no conversion) -> false.
// ---------------------------------------------------------------------------

#[test]
fn no_conversion_paths_report_error() {
    for (label, input) in [
        ("alphabetic", &b"abc\n"[..]),
        ("whitespace only", b"   \n"),
        ("tabs and spaces only", b" \t \t \n"),
        ("vertical tab and form feed only", b"\x0b\x0c\n"),
        ("carriage return only", b"\r\n"),
        ("lone plus", b"+"),
        ("lone minus", b"-"),
        ("lone plus with newline", b"+\n"),
        ("lone minus with newline", b"-\n"),
        ("sign then space", b"- 5\n"),
        ("double sign", b"--5\n"),
        ("plus minus", b"+-5\n"),
        ("leading dot", b".5\n"),
        ("leading comma", b",5\n"),
        ("NUL first byte", b"\x005\n"),
        ("high byte first", b"\xff5\n"),
        ("non-UTF-8 prefix", b"\x80\x815\n"),
        ("escape sequence prefix", b"\x1b[0m5\n"),
        ("underscore", b"_5\n"),
        ("letter x", b"x10\n"),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// parse_val branch: strtol succeeded and the value fits in int -> run twice.
// ---------------------------------------------------------------------------

#[test]
fn successful_parses_run_twice() {
    for (label, input) in [
        ("zero", &b"0\n"[..]),
        ("one", b"1\n"),
        ("minus one", b"-1\n"),
        ("five", b"5\n"),
        ("minus three", b"-3\n"),
        ("explicit plus", b"+7\n"),
        ("minus zero", b"-0\n"),
        ("leading zeros", b"007\n"),
        ("leading spaces", b"  42\n"),
        ("leading tab", b"\t42\n"),
        ("leading vertical tab and form feed", b"\x0b\x0c 5\n"),
        ("leading carriage return", b"\r5\n"),
        ("surrounding whitespace", b"  \t 123  \n"),
        ("trailing carriage return", b"5\r\n"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn partial_parses_succeed_with_the_leading_digits() {
    // strtol stops at the first non-digit; endp != str, so parse_val succeeds.
    for (label, input) in [
        ("digits then letters", &b"12abc\n"[..]),
        ("hex literal parses as 0", b"0x1f\n"),
        ("decimal point truncates", b"2.5\n"),
        ("exponent truncates", b"1e5\n"),
        ("negative decimal truncates", b"-0.5\n"),
        ("thousands separator truncates", b"1_000\n"),
        ("comma separated", b"12,34\n"),
        ("space separated", b" 12 34\n"),
        ("digits then NUL", b"5\x00\n"),
        ("digits then embedded NUL and more", b"12\x0034\n"),
        ("digits then high byte", b"9\xff\n"),
        ("INT_MAX then junk", b" +2147483647junk\n"),
        ("digits then trailing spaces", b"5                 \n"),
    ] {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// parse_val branches: `tmp < INT_MIN` and `tmp > INT_MAX` -> false, and
// `errno == ERANGE` from strtol -> false. Also int overflow inside
// add_bedrooms, which the C performs twice on the same global.
// ---------------------------------------------------------------------------

#[test]
fn int_range_boundaries() {
    for (label, input) in [
        ("INT_MAX", &b"2147483647\n"[..]),
        ("INT_MAX - 1", b"2147483646\n"),
        ("INT_MAX + 1 rejected", b"2147483648\n"),
        ("INT_MIN", b"-2147483648\n"),
        ("INT_MIN + 1", b"-2147483647\n"),
        ("INT_MIN - 1 rejected", b"-2147483649\n"),
        ("2^32 rejected", b"4294967296\n"),
        ("-2^32 rejected", b"-4294967296\n"),
        ("INT_MAX padded with zeros", b"0000000002147483647\n"),
        ("INT_MIN with whitespace", b"\t-2147483648\t\n"),
        // Values that make bedrooms (starting at 5) overflow across two runs.
        ("third of INT_MAX", b"715827882\n"),
        ("negative third of INT_MIN", b"-715827883\n"),
        ("half of INT_MAX", b"1073741824\n"),
    ] {
        assert_same(label, input);
    }
}

#[test]
fn long_range_errors_set_errno() {
    for (label, input) in [
        ("LONG_MAX (out of int range)", &b"9223372036854775807\n"[..]),
        ("LONG_MAX + 1 -> ERANGE", b"9223372036854775808\n"),
        ("LONG_MIN (out of int range)", b"-9223372036854775808\n"),
        ("LONG_MIN - 1 -> ERANGE", b"-9223372036854775809\n"),
        ("18 nines", b"999999999999999999\n"),
        ("19 nines", b"9999999999999999999\n"),
        ("20 nines", b"99999999999999999999\n"),
        ("negative 19 nines", b"-9999999999999999999\n"),
        ("29 nines", b"99999999999999999999999999999\n"),
        ("negative 29 nines", b"-99999999999999999999999999999\n"),
        ("2^64", b"18446744073709551616\n"),
    ] {
        assert_same(label, input);
    }
    // 99 digits: fills the buffer and overflows long.
    assert_same("99 nines", &vec![b'9'; 99]);
    let mut v = vec![b'-'];
    v.extend_from_slice(&vec![b'9'; 98]);
    assert_same("minus then 98 nines", &v);

    // Digit-count sweep across the long boundary.
    for n in 1..=25 {
        let digits = vec![b'9'; n];
        let mut pos = digits.clone();
        pos.push(b'\n');
        assert_same(&format!("{n} nines"), &pos);
        let mut neg = vec![b'-'];
        neg.extend_from_slice(&digits);
        neg.push(b'\n');
        assert_same(&format!("minus {n} nines"), &neg);
    }
}

// ---------------------------------------------------------------------------
// Process-level behaviour that is not driven by stdin content.
// ---------------------------------------------------------------------------

#[test]
fn extra_argv_is_ignored() {
    // main() takes no parameters, so arguments must not change anything.
    let c = Command::new(c_bin())
        .args(["foo", "bar"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let r = Command::new(rust_bin())
        .args(["foo", "bar"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn stdin_from_dev_null() {
    let c = Command::new(c_bin()).stdin(Stdio::null()).output().unwrap();
    let r = Command::new(rust_bin()).stdin(Stdio::null()).output().unwrap();
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

// ---------------------------------------------------------------------------
// Bulk coverage: every single byte, and exhaustive short strings over an
// alphabet chosen to hit each strtol state transition.
// ---------------------------------------------------------------------------

#[test]
fn every_single_byte_input() {
    for b in 0u8..=255 {
        assert_same(&format!("single byte {b:#04x}"), &[b]);
    }
}

#[test]
fn exhaustive_short_inputs() {
    // sign / digit / zero / whitespace / newline / NUL / letter / dot.
    const ALPHABET: &[u8] = b"0-+9 \n\x00a.";
    for a in ALPHABET {
        for b in ALPHABET {
            assert_same("pair", &[*a, *b]);
            for c in ALPHABET {
                assert_same("triple", &[*a, *b, *c]);
            }
        }
    }
}

#[test]
fn pseudorandom_inputs() {
    // Deterministic xorshift64* so failures are reproducible without adding a
    // dependency on an RNG crate.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    const CHARS: &[u8] = b"0123456789+- \t\n\x00abcxX.eE\xff\x0b\x0c\r";
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);

    for _ in 0..600 {
        let len = rng.below(140);
        let input: Vec<u8> = (0..len).map(|_| CHARS[rng.below(CHARS.len())]).collect();
        assert_same("random bytes", &input);
    }

    // Numeric-shaped inputs, which concentrate on the accept/reject boundary.
    for _ in 0..400 {
        let mut input = Vec::new();
        match rng.below(3) {
            0 => input.push(b'-'),
            1 => input.push(b'+'),
            _ => {}
        }
        let digits = rng.below(24);
        for _ in 0..digits {
            input.push(b'0' + (rng.below(10) as u8));
        }
        match rng.below(3) {
            0 => input.push(b'\n'),
            1 => input.extend_from_slice(b" \n"),
            _ => {}
        }
        assert_same("random number", &input);
    }
}
