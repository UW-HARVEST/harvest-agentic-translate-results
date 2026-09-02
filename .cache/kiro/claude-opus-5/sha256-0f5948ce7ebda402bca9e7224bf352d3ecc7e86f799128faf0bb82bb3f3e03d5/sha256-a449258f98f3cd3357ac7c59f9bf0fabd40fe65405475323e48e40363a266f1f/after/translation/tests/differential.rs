//! Differential tests: run the C reference program and the Rust translation as
//! subprocesses over the same stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links against the translation as a library; both programs are
//! driven exactly the way a shell drives them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating and running the two programs
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust program under test: the binary cargo just built for this crate.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C reference program. Prefers the CMake build described in
/// `c_src/CMakeLists.txt`; if that has not been run, compiles `c_src/src/main.c`
/// into this crate's `target/` directory with `cc`. `c_src/` itself is only ever
/// read from.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = manifest_dir().parent().expect("workspace root").to_path_buf();
        let c_src = root.join("c_src");

        let cmake_built = c_src.join("build").join("driver");
        if cmake_built.is_file() {
            return cmake_built;
        }

        let out_dir = manifest_dir().join("target").join("c_reference");
        std::fs::create_dir_all(&out_dir).expect("create target/c_reference");
        let out = out_dir.join("driver");
        if out.is_file() {
            return out;
        }

        let main_c = c_src.join("src").join("main.c");
        assert!(main_c.is_file(), "missing C source at {}", main_c.display());

        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let status = Command::new(&cc)
            .arg("-O2")
            .arg("-o")
            .arg(&out)
            .arg(&main_c)
            .status()
            .unwrap_or_else(|e| panic!("failed to spawn C compiler {cc:?}: {e}"));
        assert!(status.success(), "compiling {} failed", main_c.display());
        out
    })
}

fn run(bin: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let owned = input.to_vec();
    let mut stdin = child.stdin.take().expect("piped stdin");
    // Write on a helper thread: the child may exit without draining stdin.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
    });

    let out = child.wait_with_output().expect("wait_with_output");
    writer.join().expect("stdin writer thread");
    out
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

fn label(input: &[u8]) -> String {
    if input.len() <= 96 {
        show(input)
    } else {
        format!("{}...[{} bytes]", show(&input[..96]), input.len())
    }
}

/// Assert stdout, stderr and exit status all agree for one input.
#[track_caller]
fn assert_identical(input: &[u8]) {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);

    let ctx = label(input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input \"{ctx}\"\n  C   : {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input \"{ctx}\"\n  C   : {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs for input \"{ctx}\": C {:?} vs Rust {:?}",
        c.status,
        r.status
    );
}

fn check_all<I, T>(inputs: I)
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    for input in inputs {
        assert_identical(input.as_ref());
    }
}

/// Every sign / leading-whitespace prefix the scanf skip can see. A leading `-`
/// is what makes a matching failure observable: on failure the C program keeps
/// its initial `+0.0f` (`00000000`), while a successful conversion of a
/// zero-valued token stores `-0.0f` (`00000080`).
const PREFIXES: [&str; 7] = ["", "-", "+", "  ", "  -", "\n-", "-  "];

fn with_all_prefixes(bodies: &[&str]) -> Vec<Vec<u8>> {
    let mut out = Vec::with_capacity(bodies.len() * PREFIXES.len());
    for body in bodies {
        for p in PREFIXES {
            out.push(format!("{p}{body}").into_bytes());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs are runnable and agree on the trivial input
// ---------------------------------------------------------------------------

#[test]
fn both_programs_are_runnable() {
    assert!(c_binary().is_file(), "C binary not built");
    assert!(rust_binary().is_file(), "Rust binary not built");
    let c = run(c_binary(), b"1\n");
    assert_eq!(c.stdout, b"0000803f\n");
    assert_eq!(c.status.code(), Some(0));
    assert_identical(b"1\n");
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on
// ---------------------------------------------------------------------------

/// `scanf` returns EOF and never touches `x`, so `driver` prints the initial
/// `0.0f`. This is the "empty input" class.
#[test]
fn empty_and_whitespace_only_input() {
    check_all([
        &b""[..],
        b"\n",
        b"\n\n\n",
        b" ",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"\t\r\x0b\x0c\n   ",
        b"                                        ",
    ]);
}

/// `scanf`'s whitespace skip crosses newlines, and only the first conversion
/// happens: trailing lines are never read.
#[test]
fn leading_whitespace_and_trailing_input_are_ignored() {
    check_all([
        &b"   42"[..],
        b"\t42",
        b"\n\n42\n\n",
        b"  \n\t\n  1.5",
        b"1\n2\n",
        b"  \r\n-3.5e2extra",
        b"7 garbage that is never read",
        b"7\n8\n9\n10\n",
        b"0.5\0trailing-nul",
    ]);
}

/// Signed zeros: the sign is applied even when the significand is zero, so the
/// object representation differs from `+0.0f`.
#[test]
fn signed_zeros() {
    check_all(with_all_prefixes(&[
        "0", "00", "0.", ".0", "0.0", "0e0", "0e999", "0e-999", "0.000", "0x0", "0x0.", "0x.0",
        "0x0p0", "0x0p9999", "000000000000000000000",
    ]));
}

/// A matching failure leaves `x` at its initialized `0.0f`. Each of these is a
/// distinct reason the conversion cannot start or cannot complete.
#[test]
fn matching_failures_leave_the_initial_value() {
    check_all(with_all_prefixes(&[
        "", "abc", "xyz", ".", ".e5", "e", "e5", "E", "x", "X", "p1", "-", "+", "--1", "+-1",
        "$", "\u{7f}", "/", ":", "@", "`", "z", "i", "in", "n", "na",
    ]));
    // A bare sign or a lone dot, with nothing that can follow it.
    check_all([&b"-"[..], b"+", b".", b"-.", b"+.", b"-.e", b"-e", b"- 1", b"+ 1"]);
}

/// The `0x` prefix has its own acceptance rule inside `vfscanf`, separate from
/// `strtof`'s: with neither a hex digit nor a `.` after the prefix the whole
/// conversion fails, so `x` keeps `+0.0f` even though a `-` was read.
#[test]
fn hex_prefix_without_digits() {
    check_all(with_all_prefixes(&[
        // no hex digit and no '.' -> matching failure
        "0x", "0X", "0xg", "0xz9", "0xp", "0xp1", "0xP-3", "0x+1", "0x 1", "0xx", "0Xt",
        // a '.' is collected -> conversion succeeds, converting only the leading '0'
        "0x.", "0X.", "0x..", "0x.x", "0x.p", "0x.p1", "0x.g", "0x.e5",
    ]));
    check_all([&b"-0x\n1"[..], b"-0x\t1", b"-0X"]);
}

/// `inf` / `infinity`: `vfscanf` accepts only a 3- or 8-character match, so a
/// partial word such as `infi` is a matching failure rather than infinity.
#[test]
fn infinity_forms_including_partial_matches() {
    check_all(with_all_prefixes(&[
        "inf", "INF", "Inf", "iNf", "infinity", "INFINITY", "InFiNiTy", "infx", "inf5", "inf.",
        "infinityy", "infinity5", "i", "in", "infi", "infin", "infini", "infinit", "INFI",
        "Infinit",
    ]));
}

/// `nan`: matched three letters only. The optional `(n-char-sequence)` payload
/// is never collected, so the payload cannot reach the result.
#[test]
fn nan_forms_including_payloads() {
    check_all(with_all_prefixes(&[
        "nan", "NAN", "NaN", "nAn", "nanx", "nan5", "nan(", "nan()", "nan(1", "nan(123)",
        "NAN(0x7f)", "nan(_", "na", "n", "nn", "nan.",
    ]));
}

/// Ordinary decimal conversions, including exponent forms and the incomplete
/// exponents that `strtod` backs out of (`"1e"` converts as `1.0`).
#[test]
fn decimal_forms_and_incomplete_exponents() {
    check_all(with_all_prefixes(&[
        "1", "2.5", "3.14", "0.1", ".5", "5.", "1.", "1e0", "1e1", "1e-1", "1E5", "1e+5", "1.e5",
        "1e", "1e+", "1e-", "1E", "1E+", "1.5e", "1.5.5", "1-2", "1 2", "0000000000000001",
        "123456789012345678901234567890", "9.99999999999999999999",
    ]));
}

/// Hexadecimal floating forms, including incomplete `p` exponents.
#[test]
fn hex_float_forms() {
    check_all(with_all_prefixes(&[
        "0x1", "0x1p1", "0x1p-1", "0X1P+3", "0x.8p1", "0x1.8p2", "0x1.fffffep127",
        "0x1.ffffffp127", "0x1p128", "0x1p-149", "0x1p-150", "0x1.8p-149", "0x1p", "0x1p+",
        "0x1p-", "0x1.8p", "0xdeadbeef", "0xABCDEF", "0x1x", "0x1.p4", "0x.fp-2",
    ]));
}

/// Overflow to infinity, underflow to zero, and the subnormal range: the
/// rounding path `driver` exposes byte-for-byte.
#[test]
fn overflow_underflow_and_subnormals() {
    check_all(with_all_prefixes(&[
        "1e38", "1e39", "3.4028235e38", "3.4028236e38", "3.402823669209384e38",
        "3.402823669209385e38", "3.4028236692093846e38", "1e-38", "1e-45", "1e-46", "1e-40",
        "7e-46", "1.4e-45", "0.7e-45", "1e300", "1e-300",
    ]));
}

/// Exponent fields far beyond `int` range, in both decimal and hex forms.
#[test]
fn absurd_exponents() {
    let nines = "9".repeat(40);
    let zeros = "0".repeat(40);
    let bodies: Vec<String> = vec![
        "1e999999999".into(),
        "1e-999999999".into(),
        "1e2147483647".into(),
        "1e2147483648".into(),
        "1e-2147483649".into(),
        format!("1e{nines}"),
        format!("1e-{nines}"),
        format!("1e+{nines}"),
        format!("1e{zeros}5"),
        format!("0x1p{nines}"),
        format!("0x1p-{nines}"),
        format!("0.0e{nines}"),
        format!("0.0e-{nines}"),
    ];
    let refs: Vec<&str> = bodies.iter().map(String::as_str).collect();
    check_all(with_all_prefixes(&refs));
}

/// Round-to-nearest-ties-to-even at exact midpoints between adjacent floats,
/// where a sloppy conversion rounds the wrong way.
#[test]
fn exact_ties_between_adjacent_floats() {
    // Exact decimal midpoints between consecutive binary32 values, produced with
    // Python's `decimal` module and pasted here so the test needs no dependency.
    const MIDPOINTS: [&str; 12] = [
        // between +0 and the smallest subnormal (2^-150): ties to even -> +0
        "0.000000000000000000000000000000000000000000000701",
        "0.0000000000000000000000000000000000000000000007006492321624085354618288102188397672364827"
        , // exact 2^-150
        // between the two smallest subnormals (1.5 * 2^-149)
        "0.0000000000000000000000000000000000000000000021019476964872256063854864306565193017094482",
        // between the largest subnormal and the smallest normal
        "0.0000000000000000000000000000000000000117549432", 
        // 1.0 and its successor
        "1.000000059604644775390625",
        "1.000000059604644775390625",
        "1.0000000596046447753906250001",
        // 2^24 boundary where consecutive integers stop being representable
        "16777216", "16777217", "16777219", "16777218",
        "33554435",
    ];
    check_all(with_all_prefixes(&MIDPOINTS));
}

/// Very long tokens, longer than any internal scanning buffer, including
/// significands whose leading zeros must not consume precision.
#[test]
fn very_long_tokens() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for len in [50usize, 100, 500, 1000, 4096, 9000, 20000] {
        let digits: String = std::iter::repeat('7').take(len).collect();
        let zeros: String = std::iter::repeat('0').take(len).collect();
        cases.push(digits.clone().into_bytes());
        cases.push(format!("-{digits}e-{len}").into_bytes());
        cases.push(format!("0.{zeros}1").into_bytes());
        cases.push(format!("{zeros}1.5").into_bytes());
        cases.push(format!("0x{}p-20", "f".repeat(len.min(4096))).into_bytes());
        cases.push(format!("-0x0.{}1p900", zeros).into_bytes());
    }
    check_all(cases);
}

// ---------------------------------------------------------------------------
// Phase C: deterministic randomized sweeps over the same input space
// ---------------------------------------------------------------------------

/// xorshift64*, so the sweeps are reproducible without a dev-dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Random soup drawn from the alphabet the float grammar reacts to. This is the
/// sweep that first exposed the `0x`-prefix matching failure.
#[test]
fn random_token_soup() {
    const ALPHA: &[u8] = b"0123456789.eExXpP+-abcdefABCDEFinftyNA() \t\n";
    let mut rng = Rng::new(0x5EED_1234_5678_9ABC);
    let mut cases: Vec<Vec<u8>> = Vec::with_capacity(2400);
    for _ in 0..1600 {
        let len = rng.below(11);
        cases.push((0..len).map(|_| ALPHA[rng.below(ALPHA.len())]).collect());
    }
    // Bias hard toward a leading '-', which is what makes failure paths visible.
    for _ in 0..800 {
        let len = rng.below(7);
        let mut v = vec![b'-'];
        v.extend((0..len).map(|_| ALPHA[rng.below(ALPHA.len())]));
        cases.push(v);
    }
    check_all(cases);
}

/// Every corner of the float space, round-tripped through the textual forms a
/// C program would print, so each bit pattern must come back exactly.
#[test]
fn random_float_bit_patterns_round_trip() {
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for _ in 0..500 {
        let bits = rng.next_u64() as u32;
        let f = f32::from_bits(bits);
        // Rust's shortest round-trip form and a long fixed form; both must parse
        // back to the identical bit pattern in the C program.
        cases.push(format!("{f:?}").into_bytes());
        if f.is_finite() {
            cases.push(format!("{f:.30e}").into_bytes());
            cases.push(format!("{f:.60}").into_bytes());
        }
    }
    check_all(cases);
}

/// Dense exponent sweeps in both bases, where the scale of the accumulator and
/// the subnormal clamp change behavior.
#[test]
fn exponent_sweeps() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for e in -60i32..=60 {
        cases.push(format!("1e{e}").into_bytes());
        cases.push(format!("-9.99999999e{e}").into_bytes());
        cases.push(format!("0.00000000001e{e}").into_bytes());
    }
    for e in (-200i32..=200).step_by(3) {
        cases.push(format!("0x1p{e}").into_bytes());
        cases.push(format!("0x1.7ffffffffp{e}").into_bytes());
        cases.push(format!("-0x1.fffffffp{e}").into_bytes());
    }
    check_all(cases);
}

/// Randomized decimal and hexadecimal significands with random exponents.
#[test]
fn random_numeric_forms() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for _ in 0..400 {
        let len = 1 + rng.below(60);
        let s: String = (0..len)
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let e = rng.below(121) as i32 - 60;
        cases.push(format!("{s}e{e}").into_bytes());
        let (a, b) = s.split_at(len / 2);
        cases.push(format!("{a}.{b}e{e}").into_bytes());
    }
    const HEX: &[u8] = b"0123456789abcdefABCDEF";
    for _ in 0..400 {
        let len = 1 + rng.below(20);
        let s: String = (0..len).map(|_| HEX[rng.below(HEX.len())] as char).collect();
        let e = rng.below(321) as i32 - 160;
        cases.push(format!("0x{s}p{e}").into_bytes());
        let (a, b) = s.split_at(len / 2);
        cases.push(format!("-0x{a}.{b}p{e}").into_bytes());
    }
    check_all(cases);
}

/// Every possible byte, in the positions where the conversion inspects it:
/// alone, after a sign, before a digit, after a digit. Covers NUL and bytes
/// that are not valid UTF-8, which the C program treats as ordinary `char`s.
#[test]
fn every_byte_value_in_every_position() {
    let mut cases: Vec<Vec<u8>> = Vec::with_capacity(256 * 5);
    for b in 0u8..=255 {
        cases.push(vec![b]);
        cases.push(vec![b'-', b]);
        cases.push(vec![b, b'1']);
        cases.push(vec![b'1', b]);
        cases.push(vec![b' ', b' ', b, b'5']);
    }
    check_all(cases);
}

/// Arbitrary binary garbage, including invalid UTF-8 sequences.
#[test]
fn random_binary_garbage() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0001);
    let mut cases: Vec<Vec<u8>> = Vec::with_capacity(1200);
    for _ in 0..800 {
        let len = rng.below(13);
        cases.push((0..len).map(|_| rng.next_u64() as u8).collect());
    }
    for _ in 0..400 {
        let len = rng.below(9);
        let mut v = vec![b'-'];
        v.extend((0..len).map(|_| rng.next_u64() as u8));
        cases.push(v);
    }
    check_all(cases);
}

/// Inputs far larger than any stdio buffer. The C program stops reading at the
/// first character that cannot extend the token, so everything after it is
/// irrelevant to the output; this pins that down.
#[test]
fn very_large_inputs() {
    let mut cases: Vec<Vec<u8>> = Vec::new();

    let mut v = b"1.5".to_vec();
    v.extend(std::iter::repeat(0u8).take(100_000));
    cases.push(v);

    let mut v: Vec<u8> = std::iter::repeat(b'\n').take(50_000).collect();
    v.extend_from_slice(b"2.5");
    cases.push(v);

    let mut v: Vec<u8> = std::iter::repeat(b'9').take(200_000).collect();
    v.extend_from_slice(b"e-200000");
    cases.push(v);

    let mut rng = Rng::new(0x0BAD_F00D_0000_0001);
    cases.push((0..200_000).map(|_| rng.next_u64() as u8).collect());

    check_all(cases);
}

/// `main()` takes no parameters, so `argv` cannot change the result.
#[test]
fn command_line_arguments_are_ignored() {
    const ARGS: [&str; 4] = ["a", "-b", "--help", "2.5"];

    let outs: Vec<Output> = [c_binary(), rust_binary()]
        .into_iter()
        .map(|bin| {
            let mut child = Command::new(bin)
                .args(ARGS)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            child
                .stdin
                .as_mut()
                .expect("piped stdin")
                .write_all(b"2.5")
                .expect("write stdin");
            child.wait_with_output().expect("wait_with_output")
        })
        .collect();

    assert_eq!(outs[0].stdout, outs[1].stdout);
    assert_eq!(outs[0].stderr, outs[1].stderr);
    assert_eq!(outs[0].status.code(), outs[1].status.code());
    assert_eq!(outs[0].stdout, b"00002040\n");
}

/// stdin at immediate EOF from something other than a drained pipe: `scanf`
/// returns EOF without touching `x`.
#[test]
fn stdin_from_dev_null() {
    let outs: Vec<Output> = [c_binary(), rust_binary()]
        .into_iter()
        .map(|bin| {
            Command::new(bin)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("output")
        })
        .collect();

    assert_eq!(outs[0].stdout, outs[1].stdout);
    assert_eq!(outs[0].stderr, outs[1].stderr);
    assert_eq!(outs[0].status.code(), outs[1].status.code());
    assert_eq!(outs[0].stdout, b"00000000\n");
}
