//! Differential tests: run the original C `driver` and the Rust `driver` as
//! subprocesses, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status match exactly.
//!
//! The Rust code is never called as a library here. Both programs are driven
//! the way a shell would drive them (`echo ... | driver`), because that is how
//! the translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // tests/ lives in the crate root (`translation/`); the C tree is its sibling.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C program, building it with CMake on first use.
///
/// Never silently skips: if the C program cannot be built the tests fail loudly,
/// because a comparison against a program that did not build measures nothing.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "expected the C sources at {}",
            c_src.display()
        );
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.is_file() {
            std::fs::create_dir_all(&build).expect("could not create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake` -- it must be on PATH to build the C reference");
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
        }
        assert!(
            bin.is_file(),
            "the C program was not produced at {}",
            bin.display()
        );
        bin
    })
}

/// Every Rust binary under test.
///
/// Always includes the one Cargo just built for this test run. The optimised
/// `target/release/driver` is also included when it exists, so the artifact that
/// ships is covered by the same assertions.
fn rust_bins() -> &'static [PathBuf] {
    static RUST_BINS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    RUST_BINS.get_or_init(|| {
        let mut bins = vec![PathBuf::from(env!("CARGO_BIN_EXE_driver"))];
        let release = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("driver");
        if release.is_file() && !bins.contains(&release) {
            bins.push(release);
        }
        bins
    })
}

// ---------------------------------------------------------------------------
// Running a program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<Option<i32>, Option<i32>>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={:?} stderr={:?}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", bin.display()));

    // Feed stdin from a helper thread. The programs read only as far as one
    // `%d` conversion needs, so the write side may see EPIPE once the child
    // exits; that is expected and not a failure.
    let mut stdin = child.stdin.take().expect("piped stdin");
    let owned = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed waiting for {}: {e}", bin.display()));
    let _ = writer.join();

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
        status: match signal {
            Some(_) => Err(signal),
            None => Ok(out.status.code()),
        },
    }
}

/// Assert stdout, stderr and exit status are byte-for-byte identical.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let expected = run(c_bin(), input);
    for rust in rust_bins() {
        let actual = run(rust, input);
        assert_eq!(
            expected.stdout,
            actual.stdout,
            "stdout differs for {label} (input {:?}, rust bin {})\n  C:    {:?}\n  Rust: {:?}",
            Preview(input),
            rust.display(),
            String::from_utf8_lossy(&expected.stdout),
            String::from_utf8_lossy(&actual.stdout)
        );
        assert_eq!(
            expected.stderr,
            actual.stderr,
            "stderr differs for {label} (input {:?}, rust bin {})",
            Preview(input),
            rust.display()
        );
        assert_eq!(
            expected.status,
            actual.status,
            "exit status differs for {label} (input {:?}, rust bin {})",
            Preview(input),
            rust.display()
        );
    }
}

/// Shortens huge inputs in failure messages.
struct Preview<'a>(&'a [u8]);
impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(&self.0[..self.0.len().min(48)]);
        if self.0.len() > 48 {
            write!(f, "{s}...({} bytes)", self.0.len())
        } else {
            write!(f, "{s}")
        }
    }
}

// ---------------------------------------------------------------------------
// The two reachable outputs, pinned as literals
// ---------------------------------------------------------------------------

/// `bad()`: data = CHAR_MAX = 127, `data * 2` == 254 in `int`, narrowed to
/// `char` it wraps to -2; `printf("%02x", (char)-2)` promotes to `int` and
/// prints the sign-extended 32-bit pattern.
const BAD_OUT: &[u8] = b"fffffffe\n";

/// `good()`: `goodG2B` prints 2*2 == 4 as `04`; `goodB2G` has data == 127,
/// which is not `< CHAR_MAX/2` (63), so it takes the else branch.
const GOOD_OUT: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";

/// Guards the two branch outputs against the C program directly, so a
/// regression in *both* programs at once still fails.
#[test]
fn golden_outputs_match_c() {
    let bad = run(c_bin(), b"0");
    assert_eq!(bad.stdout, BAD_OUT, "C bad() output changed");
    assert_eq!(bad.stderr, b"", "C writes nothing to stderr");
    assert_eq!(bad.status, Ok(Some(0)), "C always returns 0");

    let good = run(c_bin(), b"1");
    assert_eq!(good.stdout, GOOD_OUT, "C good() output changed");
    assert_eq!(good.stderr, b"", "C writes nothing to stderr");
    assert_eq!(good.status, Ok(Some(0)), "C always returns 0");

    for rust in rust_bins() {
        assert_eq!(run(rust, b"0").stdout, BAD_OUT);
        assert_eq!(run(rust, b"1").stdout, GOOD_OUT);
    }
}

// ---------------------------------------------------------------------------
// Phase B: the inputs `main` branches on
// ---------------------------------------------------------------------------

/// `if (x)` is false: `bad()` runs. Includes the case where `scanf` performs no
/// conversion at all and therefore leaves `x` at its initialiser of 0.
#[test]
fn zero_and_no_conversion_take_the_bad_branch() {
    for (label, input) in [
        ("empty input / immediate EOF", &b""[..]),
        ("plain zero", b"0"),
        ("zero with newline", b"0\n"),
        ("negative zero", b"-0"),
        ("plus zero", b"+0"),
        ("many zeros", b"0000000000000000000000000000"),
        ("negative many zeros", b"-0000000000000000000000000"),
    ] {
        assert_same(label, input);
        assert_eq!(run(c_bin(), input).stdout, BAD_OUT, "{label} should be bad()");
    }
}

/// `if (x)` is true: `good()` runs.
#[test]
fn nonzero_takes_the_good_branch() {
    for (label, input) in [
        ("one", &b"1"[..]),
        ("two", b"2"),
        ("minus one", b"-1"),
        ("explicit plus", b"+5"),
        ("leading zeros", b"007"),
        ("large positive", b"123456"),
        ("with trailing newline", b"42\n"),
    ] {
        assert_same(label, input);
        assert_eq!(
            run(c_bin(), input).stdout,
            GOOD_OUT,
            "{label} should be good()"
        );
    }
}

/// `%d` skips leading whitespace and reads straight across newlines.
#[test]
fn whitespace_handling() {
    for (label, input) in [
        ("spaces then digit", &b"   7"[..]),
        ("tab then digit", b"\t7"),
        ("newline then digit", b"\n7"),
        ("crlf then digit", b"\r\n9"),
        ("vertical tab and form feed", b"\x0b\x0c 5"),
        ("many newlines then digit", b"\n\n\n5"),
        ("newlines then zero", b"\n\n\n0"),
        ("mixed whitespace then negative", b"  \n\t-7"),
        ("trailing whitespace", b"   7   "),
        ("whitespace only", b"   \t\n "),
        ("single space", b" "),
        ("single newline", b"\n"),
    ] {
        assert_same(label, input);
    }
}

/// Matching failures: `%d` stores nothing, so `x` keeps its initial 0.
#[test]
fn matching_failures_leave_x_at_zero() {
    for (label, input) in [
        ("alphabetic", &b"abc"[..]),
        ("leading dot", b".5"),
        ("sign only, minus", b"-"),
        ("sign only, plus", b"+"),
        ("sign then newline", b"-\n"),
        ("sign then space", b"- 5"),
        ("sign then tab", b"-\t5"),
        ("double minus", b"--5"),
        ("plus then minus", b"+-5"),
        ("minus then plus", b"-+5"),
        ("sign then letter", b"-a"),
        ("comma", b","),
        ("underscore then digit", b"_1"),
        ("high byte", b"\xff\xff5"),
        ("nul byte then digit", b"\x005"),
        ("utf-8 BOM then digit", b"\xef\xbb\xbf5"),
    ] {
        assert_same(label, input);
        assert_eq!(
            run(c_bin(), input).stdout,
            BAD_OUT,
            "{label} is a matching failure, so x stays 0"
        );
    }
}

/// `%d` stops at the first non-digit; anything after it is never read.
#[test]
fn conversion_stops_at_first_non_digit() {
    for (label, input) in [
        ("digits then letters", &b"1abc"[..]),
        ("digits then dot", b"1.9"),
        ("digits then space then digits", b"1 2"),
        ("hex-looking input converts as 0", b"0x10"),
        ("octal-looking input is decimal", b"010"),
        ("exponent form", b"1e3"),
        ("zero then letters", b"0zzz"),
        ("digits then nul", b"3\x00\x00"),
    ] {
        assert_same(label, input);
    }
    // "0x10" converts the leading 0 only, so it is the bad() branch.
    assert_eq!(run(c_bin(), b"0x10").stdout, BAD_OUT);
    // "010" converts to decimal 10, which is non-zero.
    assert_eq!(run(c_bin(), b"010").stdout, GOOD_OUT);
}

/// Overflow, truncation and signedness exactly as the C runtime performs them:
/// the value is converted at `long` width, saturated there, then truncated into
/// an `int`. Truncation can land on 0 for a huge input, which flips the branch.
#[test]
fn out_of_range_values_truncate_into_int() {
    for (label, input) in [
        ("INT_MAX", &b"2147483647"[..]),
        ("INT_MAX + 1", b"2147483648"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MIN - 1", b"-2147483649"),
        ("2^32 truncates to 0", b"4294967296"),
        ("2^32 - 1 truncates to -1", b"4294967295"),
        ("-2^32 truncates to 0", b"-4294967296"),
        ("2^33 truncates to 0", b"8589934592"),
        ("3 * 2^32 truncates to 0", b"12884901888"),
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX + 1 saturates", b"9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN - 1 saturates", b"-9223372036854775809"),
        ("far above LONG_MAX", b"99999999999999999999"),
        ("far below LONG_MIN", b"-99999999999999999999"),
        ("2^64", b"18446744073709551616"),
        ("2^64 negated", b"-18446744073709551616"),
    ] {
        assert_same(label, input);
    }
    // Spot-check the branch flips that truncation causes.
    assert_eq!(run(c_bin(), b"4294967296").stdout, BAD_OUT);
    assert_eq!(run(c_bin(), b"4294967295").stdout, GOOD_OUT);
    assert_eq!(run(c_bin(), b"-9223372036854775808").stdout, BAD_OUT);
    assert_eq!(run(c_bin(), b"9223372036854775807").stdout, GOOD_OUT);
}

/// Every power-of-two boundary up to 2^71, in both signs, plus neighbours.
#[test]
fn power_of_two_boundary_sweep() {
    let mut values: Vec<i128> = Vec::new();
    for bit in 0..72u32 {
        let base = 1i128 << bit;
        for delta in [-2i128, -1, 0, 1, 2] {
            values.push(base + delta);
            values.push(-(base + delta));
        }
    }
    values.sort_unstable();
    values.dedup();
    for v in values {
        assert_same("power-of-two boundary", v.to_string().as_bytes());
    }
}

/// The maximum the conversion has to cope with: digit runs far longer than any
/// integer width, where the C library saturates and then truncates.
#[test]
fn very_long_digit_runs() {
    for len in [18usize, 19, 20, 21, 25, 40, 100, 1000, 10_000] {
        for sign in ["", "-", "+"] {
            assert_same(
                "run of nines",
                format!("{sign}{}", "9".repeat(len)).as_bytes(),
            );
            assert_same(
                "power of ten",
                format!("{sign}1{}", "0".repeat(len)).as_bytes(),
            );
            assert_same(
                "leading zeros then one",
                format!("{sign}{}1", "0".repeat(len)).as_bytes(),
            );
            assert_same("run of zeros", format!("{sign}{}", "0".repeat(len)).as_bytes());
        }
    }
}

/// Bulk input that never yields a conversion, and bulk whitespace that does.
#[test]
fn bulk_input() {
    assert_same("4096 nul bytes", &[0u8; 4096]);
    assert_same("4096 spaces", &[b' '; 4096]);
    assert_same("2048 letters", &[b'z'; 2048]);
    assert_same(
        "5000 newlines then zero",
        format!("{}0", "\n".repeat(5000)).as_bytes(),
    );
    assert_same(
        "5000 newlines then one",
        format!("{}1", "\n".repeat(5000)).as_bytes(),
    );
}

// ---------------------------------------------------------------------------
// Phase C: deterministic fuzzing over the conversion alphabet
// ---------------------------------------------------------------------------

/// Small deterministic PRNG so the fuzz corpus is reproducible and needs no
/// external crates.
struct XorShift(u64);
impl XorShift {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// Random byte strings drawn from the characters `%d` actually reacts to:
/// digits, signs, every whitespace character, and bytes that force a matching
/// failure.
#[test]
fn fuzz_conversion_alphabet() {
    const ALPHABET: &[u8] = b"0123456789+-  \t\n\r\x0b\x0c.eExX\x00\xff\xfeaz";
    let mut rng = XorShift(0x9E3779B97F4A7C15);
    for _ in 0..1500 {
        let len = rng.below(15) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len() as u32) as usize])
            .collect();
        assert_same("fuzz", &input);
    }
}

/// Random decimal numerals of random length and sign, which concentrates on the
/// saturate-then-truncate path.
#[test]
fn fuzz_numerals() {
    let mut rng = XorShift(0xDEADBEEFCAFEF00D);
    for _ in 0..600 {
        let digits = 1 + rng.below(25) as usize;
        let mut s = String::new();
        match rng.below(3) {
            0 => s.push('-'),
            1 => s.push('+'),
            _ => {}
        }
        for _ in 0..digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        assert_same("fuzz numeral", s.as_bytes());
    }
}
