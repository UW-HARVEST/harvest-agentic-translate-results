//! Differential tests: run the original C program and the Rust translation as
//! subprocesses, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status match exactly.
//!
//! The Rust code is never used as a library here — only the built binary is
//! driven, the same way a shell would drive it, because that is how the two
//! programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust binary under test (provided by cargo for integration tests).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn c_src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .join("c_src")
}

static BUILD_C: Once = Once::new();

/// Path to the C reference binary, building it with cmake on first use.
fn c_bin() -> PathBuf {
    let build = c_src_dir().join("build");
    let exe = build.join("driver");
    BUILD_C.call_once(|| {
        if exe.exists() {
            return;
        }
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&bld.stderr)
        );
    });
    assert!(
        exe.exists(),
        "C reference binary missing at {}; build it with: \
         cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        exe.display()
    );
    exe
}

#[derive(PartialEq, Eq, Debug)]
struct Status {
    code: Option<i32>,
    /// Terminating signal, if any. Compared explicitly because a process killed
    /// by a signal reports `code() == None`, so comparing only exit codes would
    /// silently treat "died from SIGPIPE" and "died from SIGSEGV" as equal.
    signal: Option<i32>,
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Status,
}

fn status_of(s: std::process::ExitStatus) -> Status {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        Status { code: s.code(), signal: s.signal() }
    }
    #[cfg(not(unix))]
    {
        Status { code: s.code(), signal: None }
    }
}

fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The child may legitimately exit without draining stdin (it reads at
        // most one number), so a broken pipe here is not a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }
    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: status_of(out.status),
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

fn label(input: &[u8]) -> String {
    if input.len() <= 80 {
        format!("{:?}", String::from_utf8_lossy(input))
    } else {
        format!(
            "<{} bytes starting {:?}>",
            input.len(),
            String::from_utf8_lossy(&input[..40])
        )
    }
}

/// Assert the two programs agree on stdout, stderr and exit status.
fn assert_same(input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {}\n  C:    \"{}\"\n  Rust: \"{}\"",
        label(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {}\n  C:    \"{}\"\n  Rust: \"{}\"",
        label(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status differs for input {}: C={:?} Rust={:?}",
        label(input),
        c.status,
        r.status
    );
}

/// Run both programs with a stdout pipe whose read end is already closed, and
/// require they agree. The C program is killed by SIGPIPE; Rust must not quietly
/// swallow EPIPE and exit 0 instead.
#[test]
fn broken_stdout_pipe_matches() {
    fn run_with_closed_stdout(exe: &Path) -> Status {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        // Close the read end before the child can produce any output. The child
        // is blocked reading stdin, so it cannot have written yet.
        drop(child.stdout.take());
        {
            let mut stdin = child.stdin.take().expect("stdin");
            let _ = stdin.write_all(b"1.5");
        }
        status_of(child.wait().expect("wait"))
    }
    let c = run_with_closed_stdout(&c_bin());
    let r = run_with_closed_stdout(&rust_bin());
    assert_eq!(c, r, "broken-pipe exit status differs: C={c:?} Rust={r:?}");
}

fn all(cases: &[&str]) {
    for s in cases {
        assert_same(s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Baseline: no input at all, and a single value.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_at_zero() {
    // scanf hits EOF immediately and returns EOF without assigning, so the
    // initial +0.0f is printed.
    assert_same(b"");
}

#[test]
fn whitespace_only_input() {
    // %f skips leading whitespace, then hits EOF: still a matching failure.
    all(&[" ", "   ", "\n", "\t", "\r", "\x0b", "\x0c", " \t\n\r\x0b\x0c", "\n\n\n"]);
}

#[test]
fn single_simple_values() {
    all(&[
        "0", "1", "-1", "+1", "2", "0.0", "-0.0", "+0.0", "1.0", "0.5", "-0.5", "3.14159",
        "2.718281828459045", "100", "-100", "65536", "16777216", "16777217",
    ]);
}

// ---------------------------------------------------------------------------
// scanf reads across newlines and stops at the first non-numeric character.
// ---------------------------------------------------------------------------

#[test]
fn leading_whitespace_is_skipped_across_lines() {
    all(&[
        "   42.5",
        "\n\n\n   \t 1.5",
        "\n\n\n   \t 1.5\n2.5",
        "  \t\r\n\x0b\x0c-2.5",
        "\r\n\r\n7",
        "1.5\n",
        "1.5\n\n",
    ]);
}

#[test]
fn trailing_junk_is_left_unread() {
    all(&[
        "1.5abc", "1.5 extra", "1.5\n2.5", "1.5,2.5", "1.5;", "1.5-", "1.5+", "1.5e", "1.5e+",
        "1.5e-", "1.5.5", "1.5\x00junk", "12 34", "1.5x", "5.", "5.e3", "5.x",
    ]);
}

// ---------------------------------------------------------------------------
// Matching failures: scanf assigns nothing, so x keeps its initial +0.0.
// These are the cases where checking only stdout is not enough — a wrong
// translation can produce -0.0 (00000080) instead of +0.0 (00000000).
// ---------------------------------------------------------------------------

#[test]
fn matching_failures_keep_positive_zero() {
    all(&[
        "abc", "x", "z", "+", "-", "++1", "--1", "+-1", ".", "-.", "+.", ".e5", "-.e5", "-.x",
        "e5", "-e5", "E", "e", "-e", "+e", "\x001.5", "\u{ff}", "/", ":", "@",
    ]);
}

#[test]
fn sign_only_and_sign_with_garbage() {
    all(&["+", "-", "+x", "-x", "+ 1", "- 1", "+\n1", "-\n1", "+.x", "-.x"]);
}

// ---------------------------------------------------------------------------
// Hexadecimal forms, including the "0x" corner cases where glibc's rules are
// surprising: a bare "0x" is a matching failure, but "0x." converts to a
// signed zero.
// ---------------------------------------------------------------------------

#[test]
fn hex_basic() {
    all(&[
        "0x1", "-0x1", "0X1", "0x1p3", "0X1P3", "-0x1p-3", "0x1.8p1", "0x1.8", "0xABCDEFp0",
        "0xabcdef", "0x.8p1", "0x.8", "0x10", "0xff", "0xFF", "0x1p+3", "0x1P+3", "0x0", "-0x0",
        "0x0p0", "-0x0p0", "-0x00.00p5",
    ]);
}

#[test]
fn hex_prefix_without_digits_is_a_matching_failure() {
    // Regression: glibc requires a hex digit *or* a '.' after "0x". Without
    // either it assigns nothing, so even "-0x" must print +0.0, not -0.0.
    all(&[
        "0x", "-0x", "+0x", "0X", "-0X", "0xg", "-0xg", "0xp3", "-0xp3", "0Xp3", "-0XP-2",
        "0x ", "-0x 1", "0x\n", "-0xz",
    ]);
}

#[test]
fn hex_prefix_with_only_a_dot_converts_to_signed_zero() {
    // Regression: "0x." *does* match and yields a signed zero, and the 'p'
    // exponent is not examined.
    all(&[
        "0x.", "-0x.", "+0x.", "0X.", "-0X.", "0x.g", "-0x.g", "0x.p3", "-0x.p3", "0x.P+9",
        "-0x.P+9",
    ]);
}

#[test]
fn hex_dangling_exponent() {
    all(&[
        "0x1p", "-0x1p", "0x1p+", "0x1p-", "0x1pz", "0x1.8p", "0x1.8p+", "0xfp", "-0xfp-",
    ]);
}

#[test]
fn hex_precision_and_rounding() {
    all(&[
        // ties and sticky bits around the f32 significand
        "0x1.fffffep127",
        "-0x1.fffffep127",
        "0x1.ffffffp127",
        "0x1p128",
        "0x1.000001p0",
        "0x1.0000010000001p0",
        "0x1.000003p0",
        "0x1.000002p0",
        // subnormal boundary: 2^-149 is the smallest subnormal
        "0x1p-149",
        "-0x1p-149",
        "0x1p-150",
        "-0x1p-150",
        "0x1.000001p-150",
        "0x1p-151",
        "0x3p-151",
        "0x1p-125",
        "0x0.0000001p-140",
        // long significands that overflow the accumulator into the sticky bit
        "0x1.fffffffffffffffffffffffffffffffffffffp0",
        "0x123456789abcdef0123456789abcdef0123456789p0",
        "0xfffffffffffffffffffffffffffffffffffffffffp-200",
    ]);
}

// ---------------------------------------------------------------------------
// inf / nan, all spellings, plus the truncated spellings that fail to match.
// ---------------------------------------------------------------------------

#[test]
fn infinity_spellings() {
    all(&[
        "inf", "INF", "Inf", "iNf", "-inf", "+inf", "-INF", "infinity", "INFINITY", "Infinity",
        "InFiNiTy", "-infinity", "+infinity", "-INFINITY",
    ]);
}

#[test]
fn infinity_trailing_characters() {
    all(&["inf1", "infx", "inf.", "inf ", "inf\n", "infinity1", "infinityx", "inf(", "infn"]);
}

#[test]
fn truncated_infinity_and_nan_fail_to_match() {
    // "i", "in", "infi", ... are all matching failures: glibc commits to the
    // full "infinity" spelling once an 'i' follows "inf".
    all(&[
        "i", "in", "-i", "-in", "infi", "infin", "infini", "infinit", "INFI", "-infi", "-infinit",
        "n", "na", "-n", "-na", "NA", "nam",
    ]);
}

#[test]
fn nan_spellings() {
    all(&["nan", "NAN", "NaN", "nAn", "-nan", "+nan", "-NAN"]);
}

#[test]
fn nan_parenthesised_sequence_is_not_consumed() {
    // glibc's scanf("%f") matches only the bare "nan" and never sets a payload,
    // so all of these produce the default quiet NaN.
    all(&[
        "nan(", "nan()", "nan(1)", "nan(0x1)", "nan(abc)", "nan(x", "nan(1", "-nan(1)", "nan(_)",
        "nan(123456789)",
    ]);
}

// ---------------------------------------------------------------------------
// Decimal magnitude: overflow to infinity, underflow to zero, subnormals.
// ---------------------------------------------------------------------------

#[test]
fn overflow_and_underflow() {
    all(&[
        "1e38", "1e39", "-1e39", "1e40", "1e300", "1e1000", "3.4028235e38", "3.4028236e38",
        "3.402823466e38", "3.402823467e38", "-3.4028236e38", "1e-38", "1e-45", "1.4e-45",
        "7.0064923e-46", "7.0064922e-46", "1e-46", "-1e-46", "1e-100", "-1e-300", "1e-1000",
        "0.0000000000001",
    ]);
}

#[test]
fn overflow_tie_rounds_to_infinity() {
    // Exact midpoint between FLT_MAX and 2^128 ties to even, i.e. to infinity.
    all(&[
        "340282356779733661637539395458142568448",
        "-340282356779733661637539395458142568448",
        "340282356779733661637539395458142568447",
        "340282356779733661637539395458142568449",
        "340282346638528859811704183484516925440", // FLT_MAX exactly
        "340282366920938463463374607431768211456", // 2^128
    ]);
}

#[test]
fn subnormal_ties() {
    all(&[
        // 2^-150, the exact midpoint between 0 and the smallest subnormal: ties to even -> 0
        "0.000000000000000000000000000000000000000000000700649232162408535461864791644958065640130970938257885878534141944895541342930300743319094181060791015625",
        // just above that midpoint -> smallest subnormal
        "0.0000000000000000000000000000000000000000000007006492321624085354618647916449580656401309709382578858785341419448955413429303007433190941810607910156251",
        "1.4012984643248171e-45",
        "2.8025969286496341e-45",
        "4.2038953929744512e-45",
    ]);
}

#[test]
fn exponent_forms() {
    all(&[
        "1e10", "1E10", "1e+10", "1e-10", "1E+10", "1E-10", "1e00010", "1e0", "1e-0", "1e+0",
        "1.5e3", ".5e3", "5.e3", "1e007",
    ]);
}

#[test]
fn many_digits_are_still_rounded_correctly() {
    all(&[
        "123456789012345678901234567890",
        "0.123456789012345678901234567890",
        "1.00000000000000000000000000000001",
        "0.99999999999999999999999999999999",
        "16777215.5",
        "16777216.5",
        "16777217.5",
        "8388608.5",
        "1.000000059604644775390625",  // exactly 1 + 2^-24, a tie
        "1.0000000596046447753906251", // just above the tie
        "1.0000001788139343261718750", // 1 + 3*2^-24, ties up
    ]);
}

// ---------------------------------------------------------------------------
// Regressions for the exponent-clamping bugs: a very long significand combined
// with a large explicit exponent must not be flushed to zero/infinity, because
// the digit count cancels most of the exponent.
// ---------------------------------------------------------------------------

#[test]
fn huge_significand_cancels_huge_decimal_exponent() {
    // 1 followed by 1000060 zeros, times 10^-1000060, is exactly 1.0.
    let mut s = Vec::new();
    s.push(b'1');
    s.extend(std::iter::repeat(b'0').take(1_000_060));
    s.extend_from_slice(b"e-1000060");
    assert_same(&s);

    // Same significand with a slightly smaller negative exponent: 10^55, which
    // overflows f32 to infinity.
    let mut s2 = Vec::new();
    s2.push(b'1');
    s2.extend(std::iter::repeat(b'0').take(1_000_060));
    s2.extend_from_slice(b"e-1000005");
    assert_same(&s2);

    // A long run of leading fractional zeros against a large positive exponent.
    let mut s3 = Vec::from(&b"0."[..]);
    s3.extend(std::iter::repeat(b'0').take(1_000_060));
    s3.extend_from_slice(b"1e1000015");
    assert_same(&s3);
}

#[test]
fn huge_significand_cancels_huge_hex_exponent() {
    // 0x1 followed by 250010 hex zeros is 2^1000040; with p-1000041 that is 2^-1.
    for (zeros, exp) in [(250_010usize, "p-1000041"), (250_010, "p-1000040")] {
        let mut s = Vec::from(&b"0x1"[..]);
        s.extend(std::iter::repeat(b'0').take(zeros));
        s.extend_from_slice(exp.as_bytes());
        assert_same(&s);
    }

    let mut s = Vec::from(&b"0x0."[..]);
    s.extend(std::iter::repeat(b'0').take(250_010));
    s.extend_from_slice(b"1p1000045");
    assert_same(&s);
}

#[test]
fn floods_of_exponent_digits_saturate_the_same_way() {
    for n in [10usize, 100, 400] {
        let nines: String = "9".repeat(n);
        let zeros: String = "0".repeat(n);
        all(&[
            &format!("1e{nines}"),
            &format!("1e-{nines}"),
            &format!("1e+{zeros}5"),
            &format!("0x1p{nines}"),
            &format!("0x1p-{nines}"),
            &format!("0x1p+{zeros}3"),
        ]);
    }
}

#[test]
fn long_significands() {
    for n in [100usize, 1000, 5000, 20000] {
        let z = "0".repeat(n);
        all(&[
            &format!("1{z}"),
            &format!("0.{z}1"),
            &format!("3.{}", "1".repeat(n)),
            &format!("-1.{}e-45", "4".repeat(n)),
            &format!("{z}1.5"),
            &format!("0x1.{}p0", "f".repeat(n)),
            &format!("0x{}", "f".repeat(n)),
        ]);
    }
}

// ---------------------------------------------------------------------------
// Non-numeric and binary input.
// ---------------------------------------------------------------------------

#[test]
fn every_single_byte_prefix() {
    // Covers each possible first byte, both alone and followed by a valid
    // number, exercising the whitespace / sign / digit / letter dispatch.
    for b in 0u8..=255 {
        assert_same(&[b]);
        assert_same(&[b, b'1', b'.', b'5']);
    }
}

#[test]
fn embedded_nul_and_high_bytes() {
    assert_same(b"\x001.5");
    assert_same(b"1.5\x00");
    assert_same(b"\xff\xfe");
    assert_same(b"\xc3\xa93.5");
    assert_same(b"1.5\xff");
    assert_same(b"\x80");
}

// ---------------------------------------------------------------------------
// Deterministic fuzzing over the float grammar's alphabet.
// ---------------------------------------------------------------------------

/// Tiny deterministic PRNG so the corpus is identical on every run.
struct Rng(u64);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 32) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

#[test]
fn fuzz_token_soup() {
    const ALPHA: &[u8] = b"0123456789abcdefxXpPeE+-. \t\n\r\x0bnNaAiIfFtTyY()gz";
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for _ in 0..2500 {
        let len = rng.below(14) as usize;
        let s: Vec<u8> = (0..len).map(|_| ALPHA[rng.below(ALPHA.len() as u32) as usize]).collect();
        assert_same(&s);
    }
}

#[test]
fn fuzz_random_bytes() {
    let mut rng = Rng(0x0fed_cba9_8765_4321);
    for _ in 0..1500 {
        let len = rng.below(20) as usize;
        let s: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        assert_same(&s);
    }
}

#[test]
fn fuzz_structured_decimals() {
    let mut rng = Rng(0xdead_beef_cafe_0001);
    for _ in 0..1200 {
        let sign = ["", "-", "+"][rng.below(3) as usize];
        let ndig = rng.below(22) as usize;
        let nfrac = rng.below(20) as usize;
        let int: String = (0..ndig).map(|_| (b'0' + rng.below(10) as u8) as char).collect();
        let frac: String = (0..nfrac).map(|_| (b'0' + rng.below(10) as u8) as char).collect();
        if int.is_empty() && frac.is_empty() {
            continue;
        }
        let e = match rng.below(4) {
            0 => String::new(),
            1 => format!("e{}", rng.below(120) as i32 - 60),
            2 => format!("E{:+}", rng.below(100) as i32 - 50),
            _ => format!("e{}", rng.below(800) as i32 - 400),
        };
        assert_same(format!("{sign}{int}.{frac}{e}").as_bytes());
        assert_same(format!("{sign}{int}{e}").as_bytes());
    }
}

#[test]
fn fuzz_structured_hex() {
    const HEX: &[u8] = b"0123456789abcdefABCDEF";
    let mut rng = Rng(0xfeed_face_1234_5678);
    for _ in 0..1200 {
        let sign = ["", "-", "+"][rng.below(3) as usize];
        let pfx = ["0x", "0X"][rng.below(2) as usize];
        let ni = [0usize, 1, 3, 8, 20, 40][rng.below(6) as usize];
        let nf = [0usize, 1, 3, 8, 20, 40][rng.below(6) as usize];
        let ip: String = (0..ni).map(|_| HEX[rng.below(HEX.len() as u32) as usize] as char).collect();
        let fp: String = (0..nf).map(|_| HEX[rng.below(HEX.len() as u32) as usize] as char).collect();
        let body = match rng.below(4) {
            0 if !ip.is_empty() => ip.clone(),
            1 => format!("{ip}.{fp}"),
            2 if !fp.is_empty() => format!(".{fp}"),
            _ => format!("{ip}."),
        };
        let p = match rng.below(4) {
            0 => String::new(),
            1 => format!("p{}", rng.below(320) as i32 - 160),
            2 => format!("P{:+}", rng.below(80) as i32 - 40),
            _ => format!("p{}", rng.below(2400) as i32 - 1200),
        };
        assert_same(format!("{sign}{pfx}{body}{p}").as_bytes());
    }
}

#[test]
fn fuzz_subnormal_and_overflow_boundaries() {
    let mut rng = Rng(0x5555_aaaa_3333_9999);
    for _ in 0..900 {
        let sign = ["", "-"][rng.below(2) as usize];
        let d: String = (0..1 + rng.below(11) as usize)
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let head = &d[..1];
        let tail = if d.len() > 1 { &d[1..] } else { "0" };
        // decimal exponents straddling both f32 limits
        let e = rng.below(14) as i32 - 50; // -50 ..= -37
        assert_same(format!("{sign}{head}.{tail}e{e}").as_bytes());
        let e2 = 37 + rng.below(4) as i32; // 37 ..= 40
        assert_same(format!("{sign}{head}.{tail}e{e2}").as_bytes());
        // hex exponents straddling the subnormal boundary at 2^-149
        let hp = rng.below(41) as i32 - 160; // -160 ..= -120
        assert_same(format!("{sign}0x1.{tail}p{hp}").as_bytes());
        let hp2 = 120 + rng.below(21) as i32;
        assert_same(format!("{sign}0x1.{tail}p{hp2}").as_bytes());
    }
}
