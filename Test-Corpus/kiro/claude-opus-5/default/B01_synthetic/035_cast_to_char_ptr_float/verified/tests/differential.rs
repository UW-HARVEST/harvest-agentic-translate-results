//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses on the same stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links the Rust code as a library. Both programs are driven the
//! way a shell drives them, because that is how the translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two executables
// ---------------------------------------------------------------------------

/// `<workspace>/translation` -> `<workspace>`
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Build `c_src` with cmake once per test process and return the executable.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.exists() {
            return exe;
        }
        std::fs::create_dir_all(&build).expect("cannot create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("cmake not found; install cmake to run the differential tests");
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
            .expect("failed to invoke cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );
        assert!(exe.exists(), "cmake did not produce {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} status={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.status
        )
    }
}

fn run(exe: &Path, stdin_bytes: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sin = child.stdin.take().expect("piped stdin");
        let bytes = stdin_bytes.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock the test on a full pipe.
        let writer = std::thread::spawn(move || {
            let _ = sin.write_all(&bytes);
            let _ = sin.flush();
            drop(sin);
        });
        let out = child.wait_with_output().expect("wait_with_output");
        let _ = writer.join();
        let status = decode_status(&out.status);
        return Run {
            stdout: out.stdout,
            stderr: out.stderr,
            status,
        };
    }
}

#[cfg(unix)]
fn decode_status(s: &std::process::ExitStatus) -> Result<i32, i32> {
    use std::os::unix::process::ExitStatusExt;
    match s.code() {
        Some(c) => Ok(c),
        None => Err(s.signal().unwrap_or(-1)),
    }
}

#[cfg(not(unix))]
fn decode_status(s: &std::process::ExitStatus) -> Result<i32, i32> {
    Ok(s.code().unwrap_or(-1))
}

/// Assert the two programs agree on stdout, stderr and exit status.
fn assert_same(stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for stdin {:?}\n  C  : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(stdin_bytes),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for stdin {:?}\n  C  : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(stdin_bytes),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status differs for stdin {:?}: C={:?} Rust={:?}",
        String::from_utf8_lossy(stdin_bytes),
        c.status,
        r.status
    );
}

fn assert_all(cases: &[&str]) {
    for c in cases {
        assert_same(c.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_programs_run_and_agree_on_a_trivial_input() {
    let c = run(c_bin(), b"1.5");
    let r = run(rust_bin(), b"1.5");
    // Sanity-check the C side really produced the object representation of
    // 1.5f so a silently broken build cannot make the whole suite vacuous.
    assert_eq!(c.stdout, 1.5f32.to_ne_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into_bytes()
        .iter()
        .copied()
        .chain(std::iter::once(b'\n'))
        .collect::<Vec<u8>>());
    assert_eq!(c, r, "C and Rust disagree on the simplest possible input");
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on
// ---------------------------------------------------------------------------

/// No conversion happens, so `x` keeps its initializer of 0.f.
#[test]
fn empty_and_whitespace_only_input_leaves_x_at_zero() {
    assert_all(&[
        "", " ", "  ", "\n", "\n\n\n", "\t", "\r", "\x0b", "\x0c",
        " \t\n\x0b\x0c\r", "                                        ",
    ]);
}

/// scanf skips leading whitespace, including newlines, before the number.
#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    assert_all(&[
        " 3.25", "\t3.25", "\n3.25", "\r\n3.25", "\n\n\n  \t 3.25",
        "\x0b3.25", "\x0c3.25", "   \n\t  -2.5", " \n inf", " \n nan",
        " \n 0x1p4",
    ]);
}

/// A single well-formed item, and only the first item is consumed.
#[test]
fn single_item_and_trailing_junk() {
    assert_all(&[
        "0", "1", "2", "9", "1.5", "-1.5", "+1.5", "0.1", "3.14159265358979",
        "1 2", "1\n2", "1.5abc", "1.5 ", "1.5\n", "2.5xyz", "7.0\t8.0",
    ]);
}

#[test]
fn signs_and_signed_zero() {
    assert_all(&[
        "0", "-0", "+0", "0.0", "-0.0", "+0.0", "0e0", "-0e0", "-0e999999",
        "-0.000", "-1", "+1", "--1", "++1", "+-1", "- 1", "+ 1", "-", "+",
        "-\n1",
    ]);
}

/// Digits, a radix point, and the forms with digits missing on one side.
#[test]
fn decimal_grammar_edges() {
    assert_all(&[
        ".5", "-.5", "+.5", "5.", "-5.", "+5.", ".", "-.", "+.", "..", ".e5",
        "0.", ".0", "-.0", "1.2.3", "1..2", "000000000000001", "0000.0000",
    ]);
}

/// An `e` only starts an exponent when at least one digit follows.
#[test]
fn decimal_exponent_edges() {
    assert_all(&[
        "1e5", "1E5", "1e+5", "1e-5", "1e05", "1e", "1E", "1e+", "1e-",
        "1e+x", "1e-x", "1ex", "1.e5", "1.5e", "1.5e+", "1.5e+x", "1.5e3",
        ".5e2", "5.e2", "e", "e5", "E5", "-e5", "1e5e5", "1e-5e", "1e 5",
        "1e\n5",
    ]);
}

/// Nothing that can start a float: matching failure, no assignment.
#[test]
fn matching_failure_inputs() {
    assert_all(&[
        "abc", "x", "z", "q1", "/", ",", "*", "%", "$", "(", ")", "_", "#",
        "@", "!", "&", "^", "~", "'", "\"", "[", "]", "{", "}", ":", ";",
        "<", ">", "?", "|", "\\", "`", "=",
    ]);
    // Every single non-whitespace ASCII byte on its own.
    for b in 1u8..128 {
        assert_same(&[b]);
    }
}

/// Bytes that are not valid ASCII text at all.
#[test]
fn non_ascii_and_nul_bytes() {
    let cases: &[&[u8]] = &[
        b"\x00", b"\x001", b"1\x002", b"1.5\x00", b"\xff", b"\x80",
        b"\x801.5", b"1.5\xff", b"\xc3\xa9", b"\xef\xbb\xbf1.5",
        b"\xff\xfe", b"+\x00", b"0x\x00", b"-\xff",
    ];
    for c in cases {
        assert_same(c);
    }
}

/// `inf` / `infinity`, case-insensitive, and the partial-word failure path.
#[test]
fn infinity_forms_including_partial_word_failure() {
    assert_all(&[
        "inf", "INF", "Inf", "iNf", "inF", "-inf", "+inf", "-INF",
        "infinity", "INFINITY", "Infinity", "InFiNiTy", "-infinity",
        "+infinity", "inf5", "inf.", "inf ", "inf\n", "infx",
        // partial words: glibc commits to "infinity" once it sees a 4th 'i'
        "i", "in", "infi", "infin", "infini", "infinit", "infinit ",
        "-infin", "+infi", "infinityx", "infinity5", "infinit8",
        "INFIN", "iNfInI",
    ]);
    // Every prefix of "infinity", alone and followed by assorted bytes.
    let w = "infinity";
    for i in 0..=w.len() {
        for suffix in ["", "x", "0", ".", " ", "\n", "y", ")"] {
            assert_same(format!("{}{}", &w[..i], suffix).as_bytes());
        }
    }
}

/// `nan`, `nan(...)`, including an unterminated payload.
#[test]
fn nan_forms_including_payload() {
    assert_all(&[
        "nan", "NAN", "Nan", "nAn", "-nan", "+nan", "-NAN", "nan ", "nan\n",
        "nanx", "nan5", "na", "n", "nA", "nan(", "nan()", "nan(1)",
        "nan(123)", "nan(abc)", "nan(abc_1)", "nan(_)", "nan(ABC123_)",
        "nan(abc", "nan(abc)x", "nan(0x1)", "nan(-1)", "nan( )", "nan(a b)",
        "nan(4194303)", "-nan(1)", "nan(0x7fffff)", "nan(2)",
    ]);
}

/// Hexadecimal floats, including the two `0x`-with-no-digits behaviours.
#[test]
fn hex_float_forms() {
    assert_all(&[
        "0x1", "0X1", "0x1p3", "0x1P3", "0X1p3", "0x1p+3", "0x1p-3",
        "0x1.8p1", "-0x1.8p1", "+0x1.8p1", "0x.8p1", "0x8.p1", "0xa", "0xA",
        "0xabcdef", "0xABCDEF", "0x0", "-0x0", "0x0p0", "-0x0p0",
        // no hex digits at all: "0x" is rejected outright, but "0x." is
        // handed to the converter and yields a signed zero
        "0x", "-0x", "+0x", "0X", "-0X", "0x.", "-0x.", "+0x.", "0X.",
        // exponent marker without digits
        "0x1p", "0x1p+", "0x1p-", "0x1px", "0x1p+x", "0x1p 3", "0x1p\n3",
        // not hex at all / partial
        "0xg", "0x1g", "0xx", "00x1", "0", "0.5", "0e1", "0x1.8", "0x1.",
        "0x1.8p", "0x1.8px",
    ]);
}

/// Overflow to infinity, underflow to zero, and the subnormal range.
#[test]
fn overflow_underflow_and_subnormals() {
    assert_all(&[
        "1e38", "1e39", "3.4028235e38", "3.4028236e38", "1e400", "-1e400",
        "1e2147483647", "1e2147483648", "1e99999999999999999999",
        "340282346638528859811704183484516925440",
        "340282356779733661637539395458142568448",
        "1e-38", "1e-45", "1e-46", "1e-100", "1e-400", "-1e-400",
        "1e-2147483647", "1e-2147483648", "1e-99999999999999999999",
        "1.1754943508222875e-38", "1.1754942106924411e-38",
        "7.0064923216240854e-46", "1.4012984643248171e-45",
        "0.000000000000000000000000000000000000000000001",
        "0x1p127", "0x1.fffffep127", "0x1.ffffffp127", "0x1p128", "0x1p1000",
        "0x1p-126", "0x1p-149", "0x1p-150", "0x1p-151", "0x1p-1000",
        "0x1p2147483647", "0x1p-2147483647", "0x1p99999999999999999999",
        "0x1p-99999999999999999999",
    ]);
}

/// Round-to-nearest-even at the 24-bit significand boundary.
#[test]
fn rounding_ties_to_even() {
    assert_all(&[
        "16777216", "16777217", "16777218", "16777219", "16777220",
        "8388609", "33554435", "33554433",
        "1.00000005960464477539062", "1.00000011920928955078125",
        "1.000000059604644775390625", "1.0000000596046447753906251",
        "0x1.0000001p0", "0x1.0000003p0", "0x1.0000005p0", "0x1.0000007p0",
        "0x1.000001p0", "0x1.000003p0",
        // subnormal rounding boundary: half of 2^-149
        "0x1p-150", "0x1.8p-150", "0x1.0000001p-150", "0x0.8p-149",
        "0x1p-149", "0x3p-150",
    ]);
}

/// Very long tokens, the maximum the accumulator handles, and long whitespace.
#[test]
fn long_inputs() {
    let long_digits = "9".repeat(200_000);
    let long_zeros = format!("0.{}1", "0".repeat(200_000));
    let long_hex = format!("0x{}p-400000", "f".repeat(100_000));
    let long_ws = format!("{}1.5", " ".repeat(500_000));
    let many_leading_zeros = format!("{}1.5", "0".repeat(5_000));
    let hex_over_128_bits = format!("0x{}p0", "1".repeat(64));
    let long_exp_zeros = format!("1e{}5", "0".repeat(1_000));
    for s in [
        long_digits,
        long_zeros,
        long_hex,
        long_ws,
        many_leading_zeros,
        hex_over_128_bits,
        long_exp_zeros,
        "0x123456789abcdef123456789abcdef123456789abcdefp0".to_string(),
        "1".repeat(1_000),
        format!("{}e-3000", "1".repeat(1_000)),
    ] {
        assert_same(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase C: systematic coverage of short token space
// ---------------------------------------------------------------------------

/// Exhaustive two-byte combinations over the alphabet the scanner branches on.
#[test]
fn exhaustive_two_byte_tokens() {
    const CHARS: &[u8] = b"0189abcdefxXpPeE+-. inN()_";
    let mut buf = [0u8; 2];
    for &a in CHARS {
        for &b in CHARS {
            buf[0] = a;
            buf[1] = b;
            assert_same(&buf);
        }
    }
}

/// Deterministic pseudo-random tokens over the same alphabet, lengths 3..=7.
#[test]
fn pseudorandom_short_tokens() {
    const CHARS: &[u8] = b"0123456789abcdefxXpPeE+-. \tinNfF()_";
    // xorshift64* so the corpus is reproducible without a dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };
    for _ in 0..1200 {
        let len = 3 + (next() % 5) as usize;
        let token: Vec<u8> = (0..len).map(|_| CHARS[(next() as usize) % CHARS.len()]).collect();
        assert_same(&token);
    }
}

/// Word-level fuzzing: concatenations of the fragments the parser reacts to.
#[test]
fn pseudorandom_fragment_concatenations() {
    const FRAGMENTS: &[&str] = &[
        "inf", "INF", "infinity", "Infinity", "nan", "NAN", "infin", "infi",
        "nan(", "nan()", "nan(x)", "0x", "0X", "0x.", "0x0", "0x1", "p", "P",
        "e", "E", ".", "-", "+", "0", "1", "9", "f", "a", " ", "\t", "\n",
        "(", ")", "_", "z", "%",
    ];
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };
    for _ in 0..1200 {
        let n = 1 + (next() % 4) as usize;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(FRAGMENTS[(next() as usize) % FRAGMENTS.len()]);
        }
        assert_same(s.as_bytes());
    }
}

/// Round-trip every bit pattern drawn pseudo-randomly from f32 space, via the
/// shortest decimal Rust prints for it, plus f64-precision decimals.
#[test]
fn roundtrip_float_bit_patterns() {
    let mut state: u64 = 0x1234_5678_9ABC_DEF0;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };
    for _ in 0..500 {
        let bits = next() as u32;
        let f = f32::from_bits(bits);
        assert_same(format!("{f:?}").as_bytes());
        assert_same(format!("{f:e}").as_bytes());
    }
    for _ in 0..500 {
        let d = f64::from_bits(next());
        assert_same(format!("{d:?}").as_bytes());
    }
}

/// Every prefix of a few well-formed tokens, with assorted following bytes:
/// each prefix is a distinct point at which the scanner can bail out.
#[test]
fn prefixes_of_wellformed_tokens() {
    for word in [
        "infinity", "INFINITY", "nan(abcd)", "0x1.8p+12", "-0x0.0p0",
        "1.25e-11", "-3.5e+7", "0x.8p-3",
    ] {
        for i in 0..=word.len() {
            for suffix in ["", "x", "0", ".", "p", " ", "\n", "e", ")", "-"] {
                assert_same(format!("{}{}", &word[..i], suffix).as_bytes());
            }
        }
    }
}
