//! Differential test suite: runs the original C program and the Rust
//! translation as *subprocesses* and compares stdout, stderr and exit status
//! byte for byte.
//!
//! The Rust code is never linked as a library; both programs are driven the
//! way a shell would drive them, because that is how the translation is
//! graded.
//!
//! Input classes are derived from the C source (`c_src/src/main.c`):
//!
//! ```c
//! void driver(double f) {
//!     raw_double_t u = {.f = f};
//!     printf("%llx %a %.4f\n", u.x, f, f);
//! }
//! int main() { double f = 0.0f; scanf("%lf", &f); driver(f); return 0; }
//! ```
//!
//! `main` itself has no `if`, so every branch that matters lives inside the
//! two libc calls the program makes:
//!
//! * `scanf("%lf", &f)` — leading-whitespace skip, EOF (input failure),
//!   optional sign, the `inf`/`infinity` spellings (including the partial
//!   spellings glibc treats as errors), `nan`, the `0x`/`0X` hex form
//!   (including the bare-prefix conversion error and the `0x.` case that
//!   `strtod` stops early on), the decimal form, `p`/`e` exponents with and
//!   without digits, matching failure (no digits at all), overflow to
//!   infinity, underflow to zero and the subnormal range.  On any failure the
//!   C program leaves `f` at its initialiser, `0.0`.
//! * `printf("%llx %a %.4f\n", ...)` — the raw bit pattern read back out of
//!   the union, `%a` for zero / subnormal / normal / inf / nan with trailing
//!   hex-zero trimming, and `%.4f` for nan / inf / negative zero / values
//!   whose fixed rendering is hundreds of digits long, plus the exact
//!   halfway decimals where the rounding rule is observable.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two executables
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // tests/ -> translation/ -> <working directory>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Builds `c_src` with cmake if necessary and returns the path to `driver`.
///
/// Nothing under `c_src/` is modified; only the ignored `c_src/build/`
/// directory is created.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if bin.is_file() {
            return bin;
        }

        std::fs::create_dir_all(&build).expect("cannot create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("cmake not found - install cmake to run the differential tests");
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
            .expect("cmake --build failed to start");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        assert!(bin.is_file(), "c_src/build/driver was not produced");
        bin
    })
}

/// Every Rust `driver` executable worth checking.
///
/// `CARGO_BIN_EXE_driver` is whatever profile `cargo test` is using; when a
/// `--release` binary also exists it is checked as well, so that behaviour
/// which only differs between profiles (debug overflow checks,
/// `panic = "abort"`) cannot hide.
fn rust_binaries() -> &'static [PathBuf] {
    static RUST_BINS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    RUST_BINS.get_or_init(|| {
        let mut v = vec![PathBuf::from(env!("CARGO_BIN_EXE_driver"))];
        let release = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("driver");
        if release.is_file() && !v.contains(&release) {
            v.push(release);
        }
        v
    })
}

// ---------------------------------------------------------------------------
// running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// Exit code, or `None` when the process was killed by a signal.
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} signal={:?}\n    stdout={:?}\n    stderr={:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", bin.display()));

    let mut stdin = child.stdin.take().expect("piped stdin");
    let bytes = input.to_vec();
    // Write on a helper thread: a program that never reads its input would
    // otherwise deadlock the test on a large payload.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&bytes);
        let _ = stdin.flush();
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

/// Compares the C program and every Rust binary on one input.
fn check(input: &[u8]) {
    let expected = run(c_binary(), input);
    for rust in rust_binaries() {
        let actual = run(rust, input);
        assert!(
            expected == actual,
            "output differs for input {:?} ({} bytes)\n  C    ({}): {:?}\n  Rust ({}): {:?}",
            String::from_utf8_lossy(&input[..input.len().min(120)]),
            input.len(),
            c_binary().display(),
            expected,
            rust.display(),
            actual,
        );
    }
}

fn check_all<I, S>(inputs: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<[u8]>,
{
    for i in inputs {
        check(i.as_ref());
    }
}

// ---------------------------------------------------------------------------
// Phase A — the programs are runnable at all
// ---------------------------------------------------------------------------

#[test]
fn both_programs_build_and_run() {
    let c = run(c_binary(), b"1.5");
    assert_eq!(c.code, Some(0), "C program: {c:?}");
    assert_eq!(c.stdout, b"3ff8000000000000 0x1.8p+0 1.5000\n");
    assert!(c.stderr.is_empty());

    assert!(
        !rust_binaries().is_empty(),
        "no Rust driver binary was located"
    );
    for rust in rust_binaries() {
        let r = run(rust, b"1.5");
        assert_eq!(r.code, Some(0), "{}: {r:?}", rust.display());
    }
    check(b"1.5");
}

/// The C program always `return 0`, and never writes to stderr.
#[test]
fn exit_status_and_stderr_are_checked_not_just_stdout() {
    for input in [&b""[..], b"garbage", b"1.5", b"inf", b"infi", b"0x"] {
        let c = run(c_binary(), input);
        assert_eq!(c.code, Some(0), "input {input:?}: {c:?}");
        assert!(c.stderr.is_empty(), "input {input:?}: {c:?}");
        check(input);
    }
}

// ---------------------------------------------------------------------------
// Phase B — the input classes scanf("%lf") branches on
// ---------------------------------------------------------------------------

/// Empty input and whitespace-only input: `%lf` hits EOF, scanf reports an
/// input failure, and `f` keeps its `0.0` initialiser.
#[test]
fn empty_and_whitespace_only_input() {
    check_all([
        &b""[..],
        b" ",
        b"\n",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"   \n\n\t  ",
        b"\n\n\n",
        // whitespace is skipped, newlines included: scanf reads across them
        b"   \t\n\n  -3.5",
        b"\n\n\n42",
        b"\x0b1.5",
        b"\x0c2.5",
        b"\r3.5",
    ]);
}

/// A single item, and the trailing text scanf leaves unread.
#[test]
fn single_value_and_unread_trailing_text() {
    check_all([
        &b"0"[..],
        b"1",
        b"-1",
        b"1.5",
        b"-1.5",
        b"42",
        b"+42",
        b"-42",
        b"42abc",
        b"1a",
        b"1-",
        b"1.2.3",
        b"3.14159",
        b"2.718281828459045",
        b"0.1",
        b"0.2",
        b"0.3",
        // scanf stops at the first non-numeric byte; the rest is ignored
        b"1.5 2.5 3.5",
        b"1.5\n2.5\n",
        b"9007199254740993",
        b"18446744073709551616",
        b"123456789012345678901234567890",
    ]);
}

/// Signed and unsigned zero, and the `%.4f` / `%a` / `%llx` rendering of both.
#[test]
fn signed_zero() {
    check_all([
        &b"0"[..],
        b"-0",
        b"+0",
        b"0.0",
        b"-0.0",
        b"-0.00001",
        b"-0.0000499",
        b"-1e-400",
        b"+1e-400",
        b"1e-400",
        b"-0x0p0",
        b"0x0p0",
        b"-0x0.0p0",
        b"0x0p99999",
        b"0000.0000",
        b"000000000000000000001",
    ]);
}

/// `inf` / `infinity`, and the partial spellings glibc turns into errors:
/// once a 4th `i` shows up the scanner commits to `infinity` and anything
/// shorter is a failure, leaving `f` at 0.0.
#[test]
fn infinity_spellings_including_the_error_paths() {
    check_all([
        &b"inf"[..],
        b"INF",
        b"Inf",
        b"iNf",
        b"-inf",
        b"+inf",
        b"infinity",
        b"INFINITY",
        b"iNfInItY",
        b"-infinity",
        b"infinityy",
        b"INFINITYX",
        b"inf inity",
        b"inf(",
        b"infx",
        // 4..7 matching characters: glibc reports an error, C prints 0.0
        b"infi",
        b"infin",
        b"infini",
        b"infinit",
        b"-infinit",
        b"Infinit",
        b"infinitx",
        // shorter than "inf": also an error
        b"i",
        b"in",
        b"in5",
        b"i5",
        b"inx",
    ]);
}

/// `nan`, its case variants, its sign, and the shorter spellings that fail.
#[test]
fn nan_spellings_including_the_error_paths() {
    check_all([
        &b"nan"[..],
        b"NaN",
        b"NAN",
        b"-nan",
        b"+nan",
        // glibc's scanf does not consume a parenthesised payload; the value
        // is the default quiet NaN either way, which %llx makes visible
        b"nan(123)",
        b"nan()",
        b"NAN(123)",
        b"nanx",
        b"nan(",
        b"-NAN",
        // too short: error, so 0.0
        b"n",
        b"na",
        b"nax",
        b"nx",
    ]);
}

/// The hex form: prefix handling, the bare-prefix conversion error, and the
/// `0x.` input that `strtod` stops early on and converts as a signed zero.
#[test]
fn hex_prefix_edge_cases() {
    check_all([
        &b"0x1p+0"[..],
        b"0X1P+0",
        b"0x1P-0",
        b"0x1.8p1",
        b"-0x1.8p1",
        b"0x.8p1",
        b"0X.8P+2",
        b"0x.0p0",
        b"0x0",
        b"0x1e5",
        b"0X1E5",
        b"0x1.8.2p1",
        // bare prefix: glibc reports a conversion error => 0.0
        b"0x",
        b"0X",
        b"0xg",
        b"0X0X1",
        // "0x." is long enough to reach strtod, which converts the leading
        // "0" and stops at 'x' => signed zero, conversion *succeeds*
        b"0x.",
        b"-0x.",
        b"+0x.",
        b"0x.p1",
        b"0x.P1",
        // 'p' with no exponent digits: strtod stops before it
        b"0x1p",
        b"0x1p+",
        b"0x1p-",
        b"0x1pz",
        // '0' at EOF, and '0' followed by a non-'x'
        b"0",
        b"0a",
        b"0e5",
    ]);
}

/// Decimal-form edge cases: a lone dot, leading/trailing dots, and exponents
/// with no digits.
#[test]
fn decimal_form_edge_cases() {
    check_all([
        &b"."[..],
        b"..",
        b"...",
        b"-.",
        b"+.",
        b".e",
        b".E",
        b".p",
        b".e5",
        b"-.e5",
        b".5",
        b"5.",
        b"-.5",
        b"+.5",
        b"5e",
        b"5E",
        b"5e+",
        b"5e-",
        b"5ez",
        b"1e5",
        b"1E5",
        b"1e+5",
        b"1e-5",
        b"1e+-5",
        b"1e5.5",
        b"5.e5",
        // no digits at all: matching failure => 0.0
        b"abc",
        b"x",
        b"zzz",
        b"-",
        b"+",
        b"--1",
        b"++1",
        b"-x",
        b"/",
        b"e5",
        b"p5",
    ]);
}

/// Overflow to infinity, underflow to zero, and the subnormal range.
#[test]
fn overflow_underflow_and_subnormals() {
    check_all([
        &b"1e308"[..],
        b"1e309",
        b"-1e309",
        b"1.7976931348623157e308",
        b"1.7976931348623159e308",
        b"1e-308",
        b"1e-310",
        b"-1e-310",
        b"1e-320",
        b"-1e-320",
        b"1e-323",
        b"5e-324",
        b"4.9e-324",
        b"2.5e-324",
        b"2.4703282292062327e-324",
        b"2.4703282292062328e-324",
        b"7.4e-324",
        b"1e-400",
        // hex, at and past the representable ends
        b"0x1.fffffffffffffp1023",
        b"-0x1.fffffffffffffp1023",
        b"0x1.fffffffffffff7p1023",
        b"0x1.fffffffffffff8p1023",
        b"-0x1.fffffffffffff8p1023",
        b"0x1.ffffffffffffffp1023",
        b"0x1p1023",
        b"0x2p1023",
        b"0x1p1024",
        b"-0x1p1024",
        b"0x1.0000000000001p1023",
        b"0x1p-1022",
        b"0x1p-1023",
        b"0x1p-1074",
        b"-0x1p-1074",
        b"0x1p-1075",
        b"-0x1p-1075",
        b"0x1.4p-1074",
        b"0x1.8p-1074",
        b"0x1.8p-1075",
        b"0x0.8p-1073",
        b"0x0.0000000000001p-1022",
        b"0x0.0000000000000fp-1022",
        b"0x0.00000000000008p-1022",
        b"0x0.00000000000018p-1022",
        b"0x0.00000000000028p-1022",
        b"0x1.fffffffffffffp-1023",
        b"0x1.0000000000000fp0",
        b"0x1.00000000000008p0",
        b"0x1.00000000000018p0",
        // exponents far outside any sane range, and ones that overflow the
        // exponent accumulator itself
        b"0x1p2000",
        b"-0x1p2000",
        b"0x1p-2000",
        b"-0x1p-2000",
        b"0x1p2147483648",
        b"0x1p1099511627776",
        b"0x1p1099511627777",
        b"0x1p-1099511627776",
        b"0x1p99999999999999999999",
        b"-0x1p99999999999999999999",
        b"0x1p-99999999999999999999",
        b"-0x1p-99999999999999999999",
        b"1e2147483648",
        b"1e-2147483648",
        b"1e99999999999999999999",
        b"1e-99999999999999999999",
    ]);
}

/// `%.4f` rounding where the exact binary value sits precisely on the
/// halfway decimal: every odd `m/32` has five decimal digits ending in 5.
#[test]
fn fixed_point_exact_halfway_rounding() {
    let mut cases: Vec<String> = Vec::new();
    for m in (1..128).step_by(2) {
        let v = m as f64 / 32.0;
        cases.push(format!("{v:.10}"));
        cases.push(format!("{v}"));
    }
    for k in 0..40 {
        cases.push(format!("{}", k as f64 + 1.0 / 32.0));
        cases.push(format!("{}", k as f64 + 3.0 / 32.0));
        cases.push(format!("{}", -(k as f64) - 1.0 / 32.0));
    }
    cases.extend(
        [
            "0.03125",
            "0.09375",
            "0.0625",
            "0.5",
            "0.00005",
            "0.00015",
            "1.00005",
            "2.00005",
            "0.99995",
            "0.99994",
            "1.99995",
            "9999.99995",
            "9999.99994",
            "-9999.99995",
            "0.00004999999999999999",
            "0.000050000000000000004",
            "0.0000499999999999999999",
            "0.00005000000000000001",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    check_all(cases);
}

/// `%llx` / `%a` / `%.4f` over exact bit patterns, fed in as hex floats so
/// the value survives the round trip: zero, every subnormal shape, the
/// binade boundaries, inf and nan.
///
/// The exponent field is swept densely across the three regions where the
/// renderers change shape — the subnormal end (`0`), the normal/subnormal and
/// zero-crossing around `1023`, and the inf/nan end (`0x7ff`) — and with a
/// stride in between.
#[test]
fn every_exponent_field_and_mantissa_shape() {
    let mut exps: Vec<u64> = Vec::new();
    exps.extend(0..=24);
    exps.extend(1000..=1046);
    exps.extend(0x7ff - 24..=0x7ff);
    exps.extend((0..=0x7ffu64).step_by(29));
    exps.sort_unstable();
    exps.dedup();

    let mut cases = Vec::new();
    for exp in exps {
        for mant in [0u64, 1, 0xf_ffff_ffff_ffff, 0x8_0000_0000_0000, 0x5_5555_5555_5555] {
            for sign in [0u64, 1] {
                let bits = (sign << 63) | (exp << 52) | mant;
                cases.push(hex_float_literal(f64::from_bits(bits)));
            }
        }
    }
    check_all(cases);
}

/// Long mantissas: more significant digits than a double can hold, on both
/// sides of the point, in both bases.
#[test]
fn very_long_mantissas() {
    let mut cases: Vec<String> = Vec::new();
    cases.push(format!("{}1", "0".repeat(400)));
    cases.push(format!("1{}", "0".repeat(400)));
    cases.push(format!("0.{}1", "0".repeat(400)));
    cases.push(format!("0x{}", "f".repeat(300)));
    cases.push(format!("0x1.{}1p0", "0".repeat(300)));
    cases.push("1".repeat(800));
    cases.push(format!("0x1p{}1", "0".repeat(100)));
    cases.push("17976931348623158079372897140530341507993413271003782693617377898044496829276475094664901797758720709633028641669288791094655554785194040263065748867150582068190890200070838367627385484581771153176447573027006985557136695962284291481986083493647529271907416844436551070434271155969950809304288017790417449779".to_owned());
    // multi-megabyte inputs: the C program only reads the prefix it needs
    cases.push("1.5".repeat(400_000));
    cases.push(format!("{}2.5", " ".repeat(100_000)));
    cases.push("1".repeat(1_000_000));
    cases.push(format!("0x{}p1", "a".repeat(500_000)));
    check_all(cases);
}

// ---------------------------------------------------------------------------
// Phase C — stream-shape divergences a fixed-input diff cannot see
// ---------------------------------------------------------------------------

/// `scanf` stops at the first byte it cannot use and never waits for EOF, so
/// the C program terminates against an endless producer (`yes 1.5 | driver`).
/// A translation that slurps all of stdin up front hangs here instead.
#[test]
fn endless_stdin_terminates_like_the_c_program() {
    fn run_against_endless_producer(bin: &Path) -> Run {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", bin.display()));

        let mut stdin = child.stdin.take().expect("piped stdin");
        // Feeds "1.5\n" for ever; exits once the child stops reading.
        let feeder = std::thread::spawn(move || {
            let chunk = "1.5\n".repeat(1024).into_bytes();
            while stdin.write_all(&chunk).is_ok() {}
        });

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            match child.try_wait().expect("try_wait") {
                Some(_) => break,
                None if std::time::Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = feeder.join();
                    panic!(
                        "{} did not terminate on an endless stdin; it is reading to EOF \
                         instead of stopping where scanf stops",
                        bin.display()
                    );
                }
                None => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }

        let out = child.wait_with_output().expect("wait_with_output");
        let _ = feeder.join();

        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            out.status.signal()
        };
        #[cfg(not(unix))]
        let signal = None;

        Run {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal,
        }
    }

    let expected = run_against_endless_producer(c_binary());
    assert_eq!(expected.code, Some(0), "C program: {expected:?}");
    assert_eq!(expected.stdout, b"3ff8000000000000 0x1.8p+0 1.5000\n");
    for rust in rust_binaries() {
        let actual = run_against_endless_producer(rust);
        assert!(
            expected == actual,
            "endless stdin differs\n  C    : {expected:?}\n  Rust ({}): {actual:?}",
            rust.display(),
        );
    }
}

/// With stdout a pipe whose reader is already gone, the C program dies from
/// `SIGPIPE` (shell status 141).  The Rust runtime sets `SIGPIPE` to `SIG_IGN`
/// before `main`, so a translation that leaves it that way exits 0 instead.
#[cfg(unix)]
#[test]
fn closed_stdout_dies_from_sigpipe_like_the_c_program() {
    use std::os::unix::process::ExitStatusExt;

    /// Spawns `bin` with stdout on a pipe, then drops the read end so the
    /// child's first write fails.  Returns `(exit code, terminating signal)`.
    fn spawn_with_dead_reader(bin: &Path, input: &[u8]) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", bin.display()));

        let mut stdin = child.stdin.take().expect("piped stdin");
        // Drop the read end of the child's stdout pipe: the next write fails.
        drop(child.stdout.take());

        let bytes = input.to_vec();
        let w = std::thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
            let _ = stdin.flush();
        });
        let status = child.wait().expect("wait");
        let _ = w.join();
        (status.code(), status.signal())
    }

    for input in [&b"1.5"[..], b"", b"inf", b"garbage"] {
        let expected = spawn_with_dead_reader(c_binary(), input);
        assert_eq!(
            expected,
            (None, Some(13)),
            "C program should die from SIGPIPE for input {input:?}"
        );
        for rust in rust_binaries() {
            let actual = spawn_with_dead_reader(rust, input);
            assert_eq!(
                expected,
                actual,
                "closed-stdout exit status differs for input {input:?}\n  C: {expected:?}\n  Rust ({}): {actual:?}",
                rust.display(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C — systematic sweeps for paths no hand-written case reaches
// ---------------------------------------------------------------------------

/// A tiny deterministic PRNG, so the sweeps below are reproducible without a
/// dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// Formats `f` the way `%a` would, but always with a full mantissa, giving a
/// lossless literal to feed back through `scanf`.
fn hex_float_literal(f: f64) -> String {
    let bits = f.to_bits();
    let sign = if bits >> 63 != 0 { "-" } else { "" };
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let mant = bits & 0x000f_ffff_ffff_ffff;
    if exp == 0x7ff {
        return if mant == 0 {
            format!("{sign}inf")
        } else {
            format!("{sign}nan")
        };
    }
    let (lead, e) = if exp == 0 { (0, -1022) } else { (1, exp - 1023) };
    format!("{sign}0x{lead}.{mant:013x}p{e:+}")
}

/// Random bit patterns, round-tripped through the hex-float syntax: exercises
/// the `%llx` / `%a` / `%.4f` renderers over the whole double space.
#[test]
fn sweep_random_bit_patterns() {
    let mut rng = Rng(0x1234_5678_9abc_def0);
    let mut cases = Vec::new();
    for _ in 0..3000 {
        cases.push(hex_float_literal(f64::from_bits(rng.next_u64())));
    }
    check_all(cases);
}

/// Random decimal literals: digits, an optional point at every position, an
/// optional exponent, an optional sign.
#[test]
fn sweep_random_decimal_literals() {
    let mut rng = Rng(0xdead_beef_cafe_0001);
    let mut cases = Vec::new();
    for _ in 0..3000 {
        let nd = 1 + rng.below(25);
        let digits: String = (0..nd)
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let mut s = if rng.below(10) < 6 {
            let k = rng.below(digits.len() + 1);
            format!("{}.{}", &digits[..k], &digits[k..])
        } else {
            digits
        };
        if rng.below(2) == 0 {
            s.push(*rng.pick(&['e', 'E']));
            s.push_str(rng.pick(&["", "+", "-"]));
            s.push_str(&rng.below(400).to_string());
        }
        if rng.below(10) < 4 {
            s.insert(0, *rng.pick(&['+', '-']));
        }
        cases.push(s);
    }
    check_all(cases);
}

/// Random hex literals, including mantissas far longer than 53 bits (which
/// forces the sticky-bit path) and exponents across the whole range.
#[test]
fn sweep_random_hex_literals() {
    const HEX: &[u8] = b"0123456789abcdefABCDEF";
    let mut rng = Rng(0x0bad_f00d_0000_0002);
    let mut cases = Vec::new();
    for _ in 0..3000 {
        let nd = rng.below(46);
        let digits: String = (0..nd).map(|_| *rng.pick(HEX) as char).collect();
        let mut s = format!("{}{}", rng.pick(&["0x", "0X"]), digits);
        if rng.below(10) < 6 {
            let k = 2 + rng.below(s.len() - 1);
            s.insert(k, '.');
        }
        if rng.below(10) < 7 {
            s.push(*rng.pick(&['p', 'P']));
            s.push_str(rng.pick(&["", "+", "-"]));
            s.push_str(&rng.below(1200).to_string());
        }
        if rng.below(10) < 4 {
            s.insert(0, *rng.pick(&['+', '-']));
        }
        cases.push(s);
    }
    check_all(cases);
}

/// Hex mantissas sitting exactly on, just below and just above the 53-bit
/// rounding boundary, in the normal and the subnormal range.
#[test]
fn sweep_rounding_boundaries() {
    let mut rng = Rng(0xfeed_face_0000_0003);
    let mut cases = Vec::new();
    for _ in 0..300 {
        let e = -1090i64 + rng.below(2121) as i64;
        let m = (rng.next_u64() & ((1 << 53) - 1)) | (1 << 52);
        for tail in ["8", "80000001", "7fffffff", "0", "8000000000000000", "4", "c"] {
            cases.push(format!("0x{m:x}.{tail}p{e}"));
            cases.push(format!("-0x{m:x}.{tail}p{e}"));
        }
    }
    check_all(cases);
}

/// Arbitrary byte soup, including NUL and non-ASCII bytes, drawn from the
/// alphabet the scanner reacts to.  This is the class that catches "the Rust
/// program exits non-zero (or panics) where the C exits 0".
#[test]
fn sweep_byte_soup() {
    const ALPHA: &[u8] =
        b"0123456789abcdefABCDEFxXpPeE.,+- \t\n\r\0infINFnanNAty()/*\x80\xff\x01";
    let mut rng = Rng(0xabcd_0000_0000_0004);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for _ in 0..4000 {
        let n = rng.below(17);
        cases.push((0..n).map(|_| *rng.pick(ALPHA)).collect());
    }
    // and with a plausible prefix in front, to reach deeper into the scanner
    for _ in 0..3000 {
        let prefix: &[u8] = rng.pick(&[
            &b""[..],
            b"0x",
            b"-0x",
            b"+",
            b"-",
            b"  ",
            b"\n",
            b"0X",
            b"in",
            b"na",
            b"0x.",
            b"1e",
        ]);
        let n = rng.below(15);
        let mut v = prefix.to_vec();
        v.extend((0..n).map(|_| *rng.pick(ALPHA)));
        cases.push(v);
    }
    check_all(cases);
}

/// Every single byte on its own, and every two-byte combination over the
/// bytes the scanner treats specially.
#[test]
fn sweep_short_inputs_exhaustively() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for b in 0u16..=255 {
        cases.push(vec![b as u8]);
    }
    const INTERESTING: &[u8] = b"0123456789abcdefxXpPeEnNiIfFtTyY.+- \t\n\0(){}\x80";
    for &a in INTERESTING {
        for &b in INTERESTING {
            cases.push(vec![a, b]);
        }
    }
    check_all(cases);
}
