//! Differential tests: run the original C `driver` and the translated Rust
//! `driver` as subprocesses over the same stdin bytes and require byte-identical
//! stdout, byte-identical stderr and the same exit status.
//!
//! Nothing here links against the translation as a library — the binary is
//! driven exactly the way a shell drives it, because that is how the two
//! programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two executables
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/  ->  <root>/
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust executable under test (built by cargo for this test run).
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C executable, configuring/building it with CMake on first use.
fn c_bin() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let src = workspace_root().join("c_src");
        let build = src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake must be installed to run the differential tests");
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
                .expect("cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C executable missing at {}", exe.display());
        exe
    })
    .as_path()
}

// ---------------------------------------------------------------------------
// running and comparing
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let mut stdin = child.stdin.take().expect("piped stdin");
    let owned = input.to_vec();
    // A separate thread avoids deadlocking on large inputs.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait_with_output");
    writer.join().expect("stdin writer thread");

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

/// Assert stdout, stderr and exit status all match for one input.
#[track_caller]
fn assert_same(input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for input {:?}: C {:?} vs Rust {:?}",
        String::from_utf8_lossy(input),
        c,
        r
    );
}

#[track_caller]
fn assert_all(inputs: &[&str]) {
    for s in inputs {
        assert_same(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A — both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn phase_a_both_binaries_run() {
    let c = run(c_bin(), b"1.5\n");
    let r = run(rust_bin(), b"1.5\n");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert_eq!(c.code, Some(0), "C exit status");
    assert_eq!(c, r, "baseline run differs");
}

// ---------------------------------------------------------------------------
// Phase B — the input classes main() / driver() branch on
// ---------------------------------------------------------------------------

/// `scanf` returns EOF: `f` keeps its initialiser `0.0`.
#[test]
fn phase_b_empty_and_whitespace_only_input() {
    assert_all(&[
        "",
        " ",
        "\n",
        "\t",
        "\r",
        "\x0b",
        "\x0c",
        "   \n\n\t \r\x0b\x0c   ",
        "\n\n\n\n",
    ]);
}

/// A single well-formed item, positive and negative, integral and fractional.
#[test]
fn phase_b_single_plain_value() {
    assert_all(&[
        "0", "-0", "+0", "0.0", "-0.0", "1", "-1", "+1", "1.5", "-1.5", "2", "10", "100",
        "3.14159265358979", "-3.14159265358979", "0.5", "-0.5", "42", "1234567890",
    ]);
}

/// `scanf` skips leading whitespace, including across newlines — `fgets` would
/// not. This pins that behaviour down.
#[test]
fn phase_b_scanf_reads_across_newlines() {
    assert_all(&[
        "\n\n1.25",
        "   \t\n  1.25",
        "\n\n\n\n\n\n\n-7.5\n",
        " \r\n\x0b\x0c\t 0x1.8p3",
        "\n \n \n inf",
        "\n\t\n nan",
    ]);
}

/// Only the first item is consumed; trailing input is irrelevant to the output.
#[test]
fn phase_b_trailing_input_is_ignored() {
    assert_all(&[
        "1 2",
        "1\n2",
        "1.5 junk",
        "1.5abc",
        "2.5\n\n\n\n",
        "3 4 5 6 7",
        "8.25zzzzzzzz",
        "9.5\0trailing",
    ]);
}

/// Every `%.4f` decision: rounding down, rounding up and exact decimal ties
/// (values of the form odd/32 terminate at the fifth fractional digit).
#[test]
fn phase_b_printf_precision_and_rounding() {
    assert_all(&[
        "0.00004", "0.00005", "0.00006", "-0.00005", "0.000049999999999999996",
        "0.00005000000001", "0.03125", "0.09375", "0.15625", "0.21875", "0.28125", "0.34375",
        "0.53125", "1.03125", "2.03125", "0.99995", "0.99999", "9.99995", "1.00005",
        "123456789.123456789", "0.9999999999999999",
    ]);
}

/// Non-finite values: `%llx` shows the pattern, `%a`/`%.4f` print inf/nan with
/// the sign taken from the sign bit.
#[test]
fn phase_b_infinity_and_nan_spellings() {
    assert_all(&[
        "inf", "INF", "Inf", "-inf", "+inf", "infinity", "INFINITY", "Infinity", "iNfInItY",
        "-infinity", "+infinity", "nan", "NAN", "NaN", "-nan", "+nan", "nan()", "nan(1)",
        "nan(1234)", "nan(0x1)", "nan(abcXYZ_098)", "-nan(1)",
    ]);
}

/// Values that overflow to infinity or underflow to zero / a subnormal.
#[test]
fn phase_b_overflow_underflow_and_subnormals() {
    assert_all(&[
        "1e308", "1e309", "-1e309", "1e400", "1e-320", "1e-323", "1e-324", "1e-325", "1e-400",
        "5e-324", "4.9e-324", "2.4703282292062327e-324", "2.4703282292062328e-324",
        "2.2250738585072009e-308", "2.2250738585072014e-308", "1.7976931348623157e308",
        "1.7976931348623159e308", "-1.7976931348623159e308",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C — the paths nothing above reaches
// ---------------------------------------------------------------------------

/// Matching failure: `scanf` converts nothing, so `driver` still sees `0.0`.
#[test]
fn phase_c_matching_failure_leaves_f_at_zero() {
    assert_all(&[
        "abc", ".", "-", "+", "-.", "+.", "e", "e5", "E5", "p5", "-e5", "+e5", "x", "0y", "--1",
        "++1", "+-1", "-+1", ".e5", "-.e5", "/", ":", "z9", "\0", "\0 1", "()", "_",
    ]);
}

/// A leading `.` or a trailing `.` is still a valid conversion.
#[test]
fn phase_c_radix_point_edges() {
    assert_all(&[
        ".5", "-.5", "+.5", ".25", "1.", "-1.", "1.e5", "1.E5", "1.2.3", "1..2", ".0", "-.0",
        "0.", "5.", ".00000",
    ]);
}

/// Truncated / malformed exponents: the digits are consumed but `strtod` may
/// still convert only the mantissa.
#[test]
fn phase_c_exponent_edges() {
    assert_all(&[
        "1e", "1e+", "1e-", "1E", "1E+", "1E-", "1.5e", "1e5e5", "1e+5+5", "1e-x", "1ee5",
        "1e5.5", "1e0", "1e00000", "1e+0", "1e-0", "1e" , "1e999999999999999999999",
        "1e-999999999999999999999", "0e999999999999999999999", "1e" ,
        "1e00000000000000000000005", "1e-00000000000000000000005",
    ]);
}

/// The hexadecimal form, including the bare `0x` prefix error path.
#[test]
fn phase_c_hex_float_forms() {
    assert_all(&[
        "0x", "0X", "-0x", "+0x", "0xp0", "0xP0", "0xz", "0x.", "0X.", "0x.p5", "0X.P5", "0x.5",
        "0x5.", "0x0", "0x0p0", "0x0p99999", "-0x0p0", "-0x0.0p0", "0x1", "0x1p0", "0x1P0",
        "0x1.8p1", "0X1P-1074", "0x1p+", "0x1p-", "0x1p", "0x1pz", "0xap1", "0xAp1", "0x1e5",
        "0x1e5p2", "0xdeadbeef", "0xDEADBEEFp-30", "0x.8p0", "0x0.0000000000001p-1022",
    ]);
}

/// Hex values that land exactly on a rounding boundary, including the carry
/// that pushes the rounded significand up to 2^53.
#[test]
fn phase_c_hex_rounding_boundaries() {
    assert_all(&[
        "0x1.fffffffffffff8p0",
        "0x1.fffffffffffff8p1",
        "0x1.fffffffffffff8p-1",
        "0x1.fffffffffffffcp0",
        "0x1.ffffffffffffffp0",
        "0x1.fffffffffffff8p1023",
        "0x1.fffffffffffffp1023",
        "0x1.fffffffffffff8p-1023",
        "0x0.fffffffffffff8p-1022",
        "0x1.ffffffffffffffp-1074",
        "0x1p1024",
        "0x1p-1074",
        "0x1p-1075",
        "0x1.0000000000001p-1074",
        "0x1.8p-1075",
        "0x1p-1073",
        "0x0.00000000000008p-1022",
        "0x0.0000000000000fp-1022",
        "0x1p16384",
        "0x1p-16384",
        "0xfffffffffffffffffffffffffffffffffp-100",
        "0x1111111111111111111111111111111111111111.p0",
        "0x.0000000000000000000000000000001p0",
    ]);
}

/// Decimal values right at the 53-bit integer boundary.
#[test]
fn phase_c_integer_significand_boundary() {
    assert_all(&[
        "9007199254740992",
        "9007199254740993",
        "9007199254740994",
        "9007199254740995",
        "9007199254740996",
        "18014398509481984",
        "18014398509481985",
        "-9007199254740993",
    ]);
}

/// Words that only partially match `inf` / `infinity` / `nan`: glibc has
/// already consumed the bytes, so these are matching failures.
#[test]
fn phase_c_partial_infinity_nan_words() {
    let words = [
        "n", "na", "nax", "nan(", "nan(abc", "nan(()", "i", "in", "inx", "inft", "infi",
        "infin", "infini", "infinit", "infinit9", "infinityy", "infinity5", "iNf", "nAn",
    ];
    for w in words {
        for pre in ["", "+", "-", "  ", "\n\t", "  -"] {
            for suf in ["", "x", "1", ".5", "\n", " ", "\n5"] {
                assert_same(format!("{pre}{w}{suf}").as_bytes());
            }
        }
    }
}

/// Inputs far longer than any stdio buffer, and inputs made only of padding.
#[test]
fn phase_c_very_long_inputs() {
    let long_digits = "5".repeat(5000);
    let long_frac = format!("0.{}", "1".repeat(5000));
    let long_pad = format!("{}3.5", " ".repeat(5000));
    let long_int = format!("1{}", "0".repeat(400));
    let tiny = format!("0.{}1", "0".repeat(400));
    let long_ws = "\n".repeat(20000);
    let long_junk = "z".repeat(10000);
    let long_exp = format!("1e{}5", "0".repeat(100));
    let long_nexp = format!("1e-{}5", "0".repeat(100));
    let huge_mantissa = format!("{}.{}", "9".repeat(1000), "9".repeat(1000));
    for s in [
        long_digits,
        long_frac,
        long_pad,
        long_int,
        tiny,
        long_ws,
        long_junk,
        long_exp,
        long_nexp,
        huge_mantissa,
    ] {
        assert_same(s.as_bytes());
    }
}

/// Raw bytes that are not valid UTF-8 or contain NULs.
#[test]
fn phase_c_non_utf8_and_nul_bytes() {
    let cases: &[&[u8]] = &[
        b"\xff\xfe\xfd",
        b"\x801.5",
        b"1.5\xff",
        b"\x00\x00\x00",
        b"\xc3\x28",
        b"\xe2\x82\xac",
        b"-\xff",
        b"\t\xff1",
        b"1\x002",
    ];
    for c in cases {
        assert_same(c);
    }
}

/// Every binade of the exponent range, exercised through exact hex literals so
/// each `%a` shape (normal, subnormal, zero) and each `%llx` pattern is hit.
/// Split into shards so the harness can run them in parallel.
fn exponent_range_shard(mantissas: &[&str]) {
    for e in -1080i32..=1024 {
        for m in mantissas {
            assert_same(format!("0x{m}p{e}").as_bytes());
            assert_same(format!("-0x{m}p{e}").as_bytes());
        }
    }
}

#[test]
fn phase_c_exponent_range_sweep_shard_1() {
    exponent_range_shard(&["1", "1.8"]);
}

#[test]
fn phase_c_exponent_range_sweep_shard_2() {
    exponent_range_shard(&["1.4", "1.c"]);
}

#[test]
fn phase_c_exponent_range_sweep_shard_3() {
    exponent_range_shard(&["1.0000000000001", "1.fffffffffffff"]);
}

#[test]
fn phase_c_exponent_range_sweep_shard_4() {
    // The carry-to-2^53 rounding case at every exponent.
    exponent_range_shard(&["1.fffffffffffff8", "1.ffffffffffffff"]);
}

/// A deterministic pseudo-random sweep of bit patterns, decimal strings and
/// junk, so the comparison is not limited to hand-picked cases.
#[test]
fn phase_c_deterministic_random_sweep() {
    // xorshift64* — no external crates, fully reproducible.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    // 1. Random doubles, fed back in as exact hex literals.
    for _ in 0..400 {
        let bits = next();
        let f = f64::from_bits(bits);
        if !f.is_finite() {
            continue;
        }
        assert_same(hex_literal(f).as_bytes());
    }

    // 2. Random decimal strings.
    const DIGITS: &[u8] = b"0123456789";
    for _ in 0..400 {
        let r = next();
        let n = 1 + (r % 22) as usize;
        let mut s = String::new();
        if r & (1 << 60) != 0 {
            s.push(if r & (1 << 61) != 0 { '-' } else { '+' });
        }
        let mut w = next();
        for _ in 0..n {
            s.push(DIGITS[(w % 10) as usize] as char);
            w /= 10;
            if w == 0 {
                w = next();
            }
        }
        if r & (1 << 55) != 0 {
            let pos = 1 + (next() % s.len() as u64) as usize;
            let pos = s.floor_char_boundary_compat(pos);
            s.insert(pos, '.');
        }
        if r & (1 << 56) != 0 {
            s.push('e');
            if r & (1 << 57) != 0 {
                s.push('-');
            }
            s.push_str(&(next() % 400).to_string());
        }
        assert_same(s.as_bytes());
    }

    // 3. Random hex-float strings.
    const XDIGITS: &[u8] = b"0123456789abcdefABCDEF";
    for _ in 0..400 {
        let r = next();
        let n = (r % 20) as usize;
        let mut s = String::new();
        if r & (1 << 60) != 0 {
            s.push('-');
        }
        s.push_str(if r & (1 << 59) != 0 { "0X" } else { "0x" });
        let mut w = next();
        for _ in 0..n {
            s.push(XDIGITS[(w % 22) as usize] as char);
            w /= 22;
            if w == 0 {
                w = next();
            }
        }
        if r & (1 << 55) != 0 {
            s.push('.');
            let mut w = next();
            for _ in 0..(next() % 6) {
                s.push(XDIGITS[(w % 22) as usize] as char);
                w /= 22;
                if w == 0 {
                    w = next();
                }
            }
        }
        if r & (1 << 56) != 0 {
            s.push('p');
            if r & (1 << 57) != 0 {
                s.push('-');
            }
            s.push_str(&(next() % 1500).to_string());
        }
        assert_same(s.as_bytes());
    }

    // 4. Random byte soup drawn from the alphabet the scanner branches on.
    const SOUP: &[u8] = b" \t\n\r\x0b\x0c+-.0123456789eExXpPaAbBcCdDfFiInNyYtTgG()_\x00\xff";
    for _ in 0..600 {
        let r = next();
        let n = (r % 13) as usize;
        let mut v = Vec::with_capacity(n);
        let mut w = next();
        for _ in 0..n {
            v.push(SOUP[(w % SOUP.len() as u64) as usize]);
            w /= SOUP.len() as u64;
            if w == 0 {
                w = next();
            }
        }
        assert_same(&v);
    }
}

/// Exact hex literal for a finite double, in the shape `strtod` accepts.
fn hex_literal(f: f64) -> String {
    let bits = f.to_bits();
    let neg = bits >> 63 != 0;
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let mant = bits & 0x000f_ffff_ffff_ffff;
    let sign = if neg { "-" } else { "" };
    if exp == 0 {
        format!("{sign}0x0.{:013x}p-1022", mant)
    } else {
        format!("{sign}0x1.{:013x}p{}", mant, exp - 1023)
    }
}

/// `String::floor_char_boundary` is unstable; these strings are ASCII anyway.
trait FloorCharBoundaryCompat {
    fn floor_char_boundary_compat(&self, i: usize) -> usize;
}
impl FloorCharBoundaryCompat for String {
    fn floor_char_boundary_compat(&self, i: usize) -> usize {
        let mut i = i.min(self.len());
        while !self.is_char_boundary(i) {
            i -= 1;
        }
        i
    }
}
