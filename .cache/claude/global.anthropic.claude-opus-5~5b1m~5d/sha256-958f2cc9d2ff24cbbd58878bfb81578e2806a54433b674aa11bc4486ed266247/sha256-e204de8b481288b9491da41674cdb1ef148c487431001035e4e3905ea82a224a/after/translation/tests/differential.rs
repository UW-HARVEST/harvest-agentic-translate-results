//! Differential tests: run the ORIGINAL C binary and the Rust binary as
//! subprocesses on identical stdin, and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! The Rust code is never called as a library — only the built executable is
//! driven, exactly the way the grader drives it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust executable under test (provided by cargo).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // translation/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C executable, building it with cmake on first use so that a
/// bare `cargo test` is self-contained.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
}

/// What a run of one of the programs produced.
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` if killed by a signal.
    status: Result<i32, i32>,
}

fn run(program: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sin = child.stdin.take().expect("stdin piped");
        // The child may exit without draining stdin; a broken pipe here is not
        // a test failure.
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");

    let status = match out.status.code() {
        Some(c) => Ok(c),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().unwrap_or(-1))
            }
            #[cfg(not(unix))]
            {
                Err(-1)
            }
        }
    };

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Core assertion: identical stdout, stderr and exit status for this stdin.
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(&rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (stdin={:?})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (stdin={:?})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {label} (stdin={:?}): C={:?} Rust={:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
}

fn check_all(cases: &[(&str, &[u8])]) {
    let mut failures = Vec::new();
    for (label, input) in cases {
        if let Err(e) = std::panic::catch_unwind(|| assert_same(label, input)) {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_string());
            failures.push(msg);
        }
    }
    if !failures.is_empty() {
        panic!("{} case(s) differed:\n\n{}", failures.len(), failures.join("\n\n"));
    }
}

// ---------------------------------------------------------------------------
// The C program:
//   int x = 0; scanf("%d", &x); driver(x);
//   driver: y = 2*x; y += 300; printf("%d\n", y);
// Input classes therefore are: what `scanf("%d")` does with the byte stream,
// and how `2*x + 300` behaves at the edges of `int`.
// ---------------------------------------------------------------------------

/// No input at all: scanf returns EOF, x keeps its initializer 0 -> "300".
#[test]
fn empty_input() {
    check_all(&[
        ("empty stdin", b""),
        ("stdin is only a newline", b"\n"),
        ("stdin is only spaces", b"     "),
        ("stdin is only mixed whitespace", b" \t\n\r\x0b\x0c"),
    ]);
}

/// A single well-formed integer — the happy path.
#[test]
fn single_integer() {
    check_all(&[
        ("zero", b"0"),
        ("one", b"1"),
        ("small positive", b"21"),
        ("negative", b"-5"),
        ("explicit plus", b"+7"),
        ("negative zero", b"-0"),
        ("leading zeros", b"00000000005"),
        ("trailing newline", b"12\n"),
        ("many trailing newlines", b"12\n\n\n"),
    ]);
}

/// `scanf` skips leading whitespace, including across newlines (unlike fgets).
#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    check_all(&[
        ("leading spaces", b"   42"),
        ("leading tabs", b"\t\t9"),
        ("leading newlines", b"\n\n\n8"),
        ("leading vertical tab", b"\x0b5"),
        ("leading form feed", b"\x0c6"),
        ("leading carriage return", b"\r7"),
        ("blank lines then number", b"\n   \n\t\n  -13\n"),
        ("number on the second line", b"\n123"),
    ]);
}

/// Only the first conversion happens; the rest of stdin is ignored.
#[test]
fn only_first_item_is_consumed() {
    check_all(&[
        ("two integers", b"3 4"),
        ("two integers on separate lines", b"3\n4\n"),
        ("integer then garbage", b" +12abc"),
        ("many integers", b"1 2 3 4 5 6 7 8 9 10"),
        ("integer then huge tail", b"7 999999999999999999999999999999"),
    ]);
}

/// Digits stop the conversion at the first non-digit: no hex, no exponent, no
/// decimal point.
#[test]
fn conversion_stops_at_first_non_digit() {
    check_all(&[
        ("hex-looking input reads 0", b"0x10"),
        ("float reads integral part", b"3.9"),
        ("exponent notation reads mantissa", b"1e5"),
        ("underscore separator", b"1_000"),
        ("comma separator", b"1,234"),
        ("digit then letter", b"5z"),
    ]);
}

/// Matching failures: scanf assigns nothing, so x stays 0 and output is "300".
#[test]
fn matching_failure_leaves_x_at_zero() {
    check_all(&[
        ("letters only", b"abc"),
        ("sign with no digits", b"-"),
        ("plus with no digits", b"+"),
        ("sign then space then digits", b"- 5"),
        ("double sign", b"--5"),
        ("plus then minus", b"+-5"),
        ("leading NUL byte", b"\x005"),
        ("punctuation only", b".,;!"),
        ("period then digits", b".5"),
        ("non-ASCII byte then digit", b"\xc3\xa95"),
        ("high byte only", b"\xff"),
    ]);
}

/// `int` boundaries, and the signed overflow of `2*x + 300` that the C
/// performs with wrapping two's-complement arithmetic.
#[test]
fn int_boundaries_and_arithmetic_overflow() {
    check_all(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MAX-1", b"2147483646"),
        ("INT_MIN+1", b"-2147483647"),
        // 2*x overflows int for anything at/above 2^30.
        ("2^30", b"1073741824"),
        ("2^30-1", b"1073741823"),
        ("2^30 negative", b"-1073741824"),
        // y += 300 overflows for x just under 2^30.
        ("just under half of INT_MAX", b"1073741673"),
        ("1073741674", b"1073741674"),
    ]);
}

/// Values outside `int`: glibc's %d saturates at LONG_MIN/LONG_MAX and the
/// result is stored truncated through an `int *`.
#[test]
fn values_wider_than_int_are_truncated() {
    check_all(&[
        ("INT_MAX+1", b"2147483648"),
        ("INT_MIN-1", b"-2147483649"),
        ("2^32", b"4294967296"),
        ("-2^32", b"-4294967296"),
        ("2^32+1", b"4294967297"),
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX+1 saturates", b"9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN-1 saturates", b"-9223372036854775809"),
        ("UINT64_MAX+1", b"18446744073709551617"),
        ("leading zeros then LONG_MAX+1", b"0000000000000000009223372036854775808"),
        ("10^30", b"1000000000000000000000000000000"),
        ("-10^30", b"-1000000000000000000000000000000"),
    ]);
}

/// The largest inputs the code will look at: very long digit runs.
#[test]
fn very_long_digit_runs() {
    let nines_400 = vec![b'9'; 400];
    let zeros_then_one = {
        let mut v = vec![b'0'; 500];
        v.push(b'1');
        v
    };
    let neg_nines = {
        let mut v = vec![b'-'];
        v.extend(std::iter::repeat(b'9').take(400));
        v
    };
    let huge = vec![b'7'; 5000];
    check_all(&[
        ("400 nines", &nines_400),
        ("500 zeros then 1", &zeros_then_one),
        ("negative 400 nines", &neg_nines),
        ("5000 sevens", &huge),
    ]);
}

/// Randomized differential sweep over the whole interesting numeric range and
/// over raw byte garbage. Deterministic seed, so failures are reproducible.
#[test]
fn randomized_differential_sweep() {
    // xorshift64* — no external crates.
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
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut owned: Vec<(String, Vec<u8>)> = Vec::new();

    // Numbers around and far beyond the int boundaries, with random padding.
    for i in 0..300 {
        let v = (rng.next() % (1u64 << 35)) as i64 - (1i64 << 34);
        let pre: &[u8] = match rng.below(5) {
            0 => b"",
            1 => b" ",
            2 => b"\t",
            3 => b"\n",
            _ => b"  \n\t ",
        };
        let post: &[u8] = match rng.below(6) {
            0 => b"",
            1 => b"\n",
            2 => b" x",
            3 => b"abc",
            4 => b"\n\n",
            _ => b".5",
        };
        let mut buf = pre.to_vec();
        if v >= 0 && rng.below(2) == 0 {
            buf.push(b'+');
        }
        buf.extend_from_slice(v.to_string().as_bytes());
        buf.extend_from_slice(post);
        owned.push((format!("random int #{i}"), buf));
    }

    // Random digit strings of random length (exercises overflow saturation).
    for i in 0..120 {
        let n = 1 + rng.below(60) as usize;
        let mut buf = Vec::new();
        if rng.below(10) < 4 {
            buf.push(b'-');
        }
        for _ in 0..n {
            buf.push(b'0' + rng.below(10) as u8);
        }
        owned.push((format!("random digits #{i}"), buf));
    }

    // Random raw bytes drawn from a scanf-relevant alphabet.
    const ALPHA: &[u8] = b"0123456789 +-\t\n\r\x00abcXZ.eE\xff";
    for i in 0..300 {
        let n = rng.below(13) as usize;
        let buf: Vec<u8> = (0..n).map(|_| ALPHA[rng.below(ALPHA.len() as u64) as usize]).collect();
        owned.push((format!("random bytes #{i}"), buf));
    }

    let cases: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(l, b)| (l.as_str(), b.as_slice()))
        .collect();
    check_all(&cases);
}

/// stdin closed immediately (not just empty) behaves like EOF.
#[test]
fn stdin_at_eof_immediately() {
    // /dev/null-equivalent: an empty pipe that is closed before the child reads.
    let c = run(c_bin(), b"");
    let r = run(&rust_bin(), b"");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status, r.status);
    // Sanity: the documented behavior is x==0 -> 300.
    assert_eq!(c.stdout, b"300\n");
}

/// Command-line arguments are ignored by `int main()` — both must agree.
#[test]
fn arguments_are_ignored() {
    for args in [vec!["999"], vec!["-x", "--help"], vec![""]] {
        let mut co = Command::new(c_bin());
        let mut ro = Command::new(rust_bin());
        let c = co
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run C with args");
        let r = ro
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("run Rust with args");
        assert_eq!(c.stdout, r.stdout, "stdout differs for args {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr differs for args {args:?}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit status differs for args {args:?}"
        );
    }
}
