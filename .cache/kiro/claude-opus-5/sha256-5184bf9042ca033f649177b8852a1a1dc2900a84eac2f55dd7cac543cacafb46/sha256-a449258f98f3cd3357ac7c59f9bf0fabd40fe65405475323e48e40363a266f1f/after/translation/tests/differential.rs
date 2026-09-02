//! Differential tests: run the original C `driver` and the translated Rust
//! `driver` as subprocesses with identical argv, and require byte-identical
//! stdout, byte-identical stderr and an identical exit status.
//!
//! The Rust code is never linked in as a library — the built binary is driven
//! exactly the way a shell drives it, because that is what the C program is
//! being compared against.
//!
//! `argv[0]` matters: `main.c` does
//! `fprintf(stderr, "%s requires 4 inputs\n", argv[0])`, so the two binaries
//! would always differ on the error path simply because they live at different
//! paths. Both children are therefore launched with the same `argv[0]` via
//! `CommandExt::arg0`, which is the only way to compare the error path at all.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The `argv[0]` both children see.
const ARG0: &str = "driver";

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // .../<root>/translation/Cargo.toml -> .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C program. Built on demand if it is not there yet.
///
/// `c_src/` is treated as read-only: when a build is needed it is configured
/// out-of-tree into `translation/target/c_build` so nothing is written under
/// `c_src/`. An existing `c_src/build/driver` is reused if present.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        if let Some(p) = std::env::var_os("C_DRIVER") {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "C_DRIVER={} is not a file", p.display());
            return p;
        }

        let root = repo_root();
        let prebuilt = root.join("c_src/build/driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let src = root.join("c_src");
        let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/c_build");
        std::fs::create_dir_all(&build).expect("create out-of-tree cmake build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("cmake must be available to build the C reference program");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("cmake --build must run");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let out = build.join("driver");
        assert!(out.is_file(), "C driver not produced at {}", out.display());
        out
    })
}

/// Path to the compiled Rust program. Cargo builds it for us and hands over the
/// path through this environment variable.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Output {
    /// `None` when the child was killed by a signal instead of exiting.
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} stdout={:?} stderr={:?}",
            self.code,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn run(bin: &Path, args: &[OsString]) -> Output {
    let out = Command::new(bin)
        .arg0(ARG0)
        .args(args)
        // A fixed, minimal environment so neither child can be influenced by
        // the ambient one. (The C program never calls setlocale, so it always
        // runs in the "C" locale regardless -- pinning it just removes doubt.)
        .env_clear()
        .env("LC_ALL", "C")
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));

    Output {
        code: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn show(args: &[OsString]) -> String {
    let parts: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a.as_bytes())))
        .collect();
    format!("[{}]", parts.join(", "))
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(args: &[OsString]) {
    let c = run(c_binary(), args);
    let r = run(rust_binary(), args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for argv {}\n  C   : {:?}\n  Rust: {:?}",
        show(args),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for argv {}\n  C   : {:?}\n  Rust: {:?}",
        show(args),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.code,
        r.code,
        "exit status differs for argv {}\n  C   : {:?}\n  Rust: {:?}",
        show(args),
        c.code,
        r.code
    );
}

fn oss(s: &str) -> OsString {
    OsString::from(s)
}

fn bytes(b: &[u8]) -> OsString {
    OsStr::from_bytes(b).to_os_string()
}

/// Check a batch of `&str` argument lists.
#[track_caller]
fn check_all(cases: &[&[&str]]) {
    for case in cases {
        let args: Vec<OsString> = case.iter().map(|s| oss(s)).collect();
        assert_same(&args);
    }
}

/// Check a batch of 3-argument vectors (the `argc == 4` path).
#[track_caller]
fn check_triples(triples: &[[&str; 3]]) {
    for t in triples {
        let args: Vec<OsString> = t.iter().map(|s| oss(s)).collect();
        assert_same(&args);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(r.is_file(), "Rust binary missing at {}", r.display());

    // The happy path produces the same three formatted floats. Note these are
    // *not* the exactly normalized values (0.267261, ...) -- `Q_rsqrt` is a
    // one-Newton-iteration approximation, and its error is visible at %f's
    // default precision of 6.
    let args = [oss("1"), oss("2"), oss("3")];
    let out = run(c, &args);
    assert_eq!(out.code, Some(0));
    assert_eq!(out.stdout, b"0.267214 0.534428 0.801642\n");
    assert!(out.stderr.is_empty());
    assert_same(&args);
}

// ---------------------------------------------------------------------------
// Phase B: the branches main.c actually takes
// ---------------------------------------------------------------------------

/// `if (argc != 4)` — every arity other than exactly three operands must go to
/// the error path: `argv[0] requires 4 inputs` on stderr and `exit(1)`.
///
/// This is the case where checking stdout alone would pass while the exit
/// status silently differed, so all three channels are asserted.
#[test]
fn wrong_arity_takes_the_error_path() {
    check_all(&[
        &[],                                // argc == 1
        &["1"],                             // argc == 2
        &["1", "2"],                        // argc == 3
        &["1", "2", "3", "4"],              // argc == 5
        &["1", "2", "3", "4", "5"],         // argc == 6
        &["1", "2", "3", "4", "5", "6"],    // argc == 7
        &["", "", ""],                      // argc == 4, but empty operands
        &["", ""],                          // argc == 3 with empty operands
    ]);

    // And pin the exact bytes of the error path, so the shape of the message is
    // covered and not just "C and Rust agree".
    let out = run(rust_binary(), &[oss("1")]);
    assert_eq!(out.code, Some(1));
    assert!(out.stdout.is_empty(), "error path must not write to stdout");
    assert_eq!(out.stderr, b"driver requires 4 inputs\n");
}

/// Exactly three operands: normalize and print `"%f %f %f\n"`.
#[test]
fn happy_path_vectors() {
    check_triples(&[
        ["1", "2", "3"],
        ["3", "4", "0"],
        ["1", "0", "0"],
        ["0", "1", "0"],
        ["0", "0", "1"],
        ["-1", "-2", "-3"],
        ["100", "200", "300"],
        ["0.1", "0.2", "0.3"],
        ["1000000", "2000000", "3000000"],
        ["123456789", "987654321", "1"],
        ["1e20", "1e20", "1e20"],
        ["1e-20", "1e-20", "1e-20"],
        ["0.0000001", "0", "0"],
        ["1.5", "-2.25", "3.125"],
    ]);
}

/// The zero vector: `DotProduct` is 0, so `Q_rsqrt(0)` reinterprets 0 as
/// `0x5f3759df` and returns a huge value, which then multiplies back to zero.
/// Signed zeros are included because the sign survives into the output.
#[test]
fn zero_and_signed_zero_vectors() {
    check_triples(&[
        ["0", "0", "0"],
        ["-0", "-0", "-0"],
        ["0.0", "-0.0", "0.0"],
        ["-0.0", "-0.0", "-0.0"],
        ["-0", "0", "0"],
        ["0", "-0", "0"],
        ["0", "0", "-0"],
        // Nothing parses, so every component becomes 0.0 -- the zero vector
        // reached through the atof failure path instead of literal zeros.
        ["abc", "def", "ghi"],
    ]);
}

/// `atof` never reports an error: unparsable text silently becomes `0.0`, and a
/// valid prefix is used even when trailing garbage follows.
#[test]
fn atof_returns_zero_or_a_prefix_instead_of_failing() {
    check_triples(&[
        ["abc", "1", "1"],
        ["", "1", "1"],
        [" ", "1", "1"],
        ["+", "1", "1"],
        ["-", "1", "1"],
        [".", "1", "1"],
        ["+.", "1", "1"],
        ["-.", "1", "1"],
        ["e", "1", "1"],
        ["e5", "1", "1"],
        [".e5", "1", "1"],
        ["7abc", "1", "1"],
        ["1_000", "1", "1"],
        ["0b101", "1", "1"],
        ["--1", "1", "1"],
        ["++1", "1", "1"],
        ["- 1", "1", "1"],
        ["1d5", "1", "1"],
        ["1..2", "1", "1"],
        ["..1", "1", "1"],
        ["1,5", "1", "1"],
        // A valid mantissa with a malformed exponent: strtod backs the exponent
        // out and keeps only the mantissa.
        ["1e", "1", "1"],
        ["1e+", "1", "1"],
        ["1e-", "1", "1"],
        ["1e++5", "1", "1"],
        ["1e1e1", "1", "1"],
        ["5e", "1", "1"],
        // Leading whitespace is skipped; a sign may follow it.
        ["  12", "+3", "-4"],
        ["\t7", "1", "1"],
        ["\n7", "1", "1"],
        ["\u{b}\u{c}\r7", "1", "1"],
        ["  \t\n 7 ", "1", "1"],
        [" -\t1", "1", "1"],
        // Fraction-only and integer-only spellings.
        [".5", ".5", ".5"],
        ["1.", "2.", "3."],
        ["00000.00000", "1", "1"],
        ["000000000000000000001", "1", "1"],
    ]);
}

/// `strtod` also accepts hexadecimal floats, `inf`/`infinity` and `nan`, in any
/// case, with an optional sign.
#[test]
fn atof_hex_infinity_and_nan_spellings() {
    check_triples(&[
        ["0x10", "0x1p4", "0X1.8P1"],
        ["0x", "0x", "0x"],
        ["0X", "1", "1"],
        ["0xg", "1", "1"],
        ["0x.8", "1", "1"],
        ["0x8.", "1", "1"],
        ["0x.p1", "1", "1"],
        ["0x1p", "1", "1"],
        ["0x1p+", "1", "1"],
        ["-0x0", "0x0p0", "1"],
        ["0x1.fffffffffffffp+1023", "1", "1"],
        ["0x1p1024", "1", "1"],
        ["0x1p-1074", "1", "1"],
        ["0x1p-1075", "1", "1"],
        ["-0x1p-1075", "1", "1"],
        ["0x7fffffffffffffff", "1", "1"],
        ["inf", "1", "1"],
        ["-inf", "1", "1"],
        ["INF", "1", "1"],
        ["+inf", "1", "1"],
        ["  inf", "1", "1"],
        ["inf   ", "1", "1"],
        ["infinity", "1", "1"],
        ["-INFINITY", "1", "1"],
        ["infin", "1", "1"],
        ["infinit", "1", "1"],
        ["nan", "1", "1"],
        ["-nan", "1", "1"],
        ["+nan", "1", "1"],
        ["NAN", "1", "1"],
        ["NaN(x)", "1", "1"],
        ["nan(123)", "1", "1"],
        ["nan()", "1", "1"],
        ["nan(abc", "1", "1"],
        ["-nan(0xdeadbeef)", "1", "1"],
    ]);
}

/// Overflow to infinity and underflow to zero, both in the `double` returned by
/// `atof` and in the narrowing to `float` on assignment to `vec3_t`.
#[test]
fn overflow_and_underflow_in_conversion() {
    check_triples(&[
        // Overflows double -> HUGE_VAL.
        ["1e400", "1", "1"],
        ["-1e400", "1", "1"],
        ["1e999999999999999999", "1", "1"],
        ["1e+309", "1", "1"],
        [".0e999", "1", "1"],
        // Underflows double -> 0.
        ["1e-400", "1", "1"],
        ["1e-999999999999999999", "1", "1"],
        // Fits in double but overflows the narrowing to float.
        ["1e39", "1", "1"],
        ["-1e39", "1", "1"],
        ["1e308", "1", "1"],
        ["1.7976931348623157e308", "1", "1"],
        // Fits in double but underflows the narrowing to float.
        ["1e-46", "1", "1"],
        ["-1e-46", "1", "1"],
        ["5e-324", "5e-324", "5e-324"],
        // float boundaries: max finite, min normal, min subnormal.
        ["3.4028234663852886e38", "1", "1"],
        ["3.4028235e38", "3.4028235e38", "3.4028235e38"],
        ["1.1754943508222875e-38", "1", "1"],
        ["1.401298464324817e-45", "1", "1"],
        ["1e-45", "1e-45", "1e-45"],
        ["1e-38", "1e-38", "1e-38"],
        ["1e38", "1e38", "1e38"],
        // Long digit strings that must still round identically.
        ["9999999999999999999999999999999999999999", "1", "1"],
        ["0.000000000000000000000000000000000000000001", "1", "1"],
    ]);
}

/// Non-finite inputs, where `Q_rsqrt` and the final multiplies go through the
/// IEEE special cases. `printf` spells these `inf`/`-inf`/`nan`/`-nan`, and the
/// sign of a NaN is observable, so every sign combination is exercised.
#[test]
fn infinities_and_nans_through_the_math() {
    let specials = [
        "0", "-0", "1", "-1", "inf", "-inf", "nan", "-nan", "1e39", "-1e39", "1e-45", "-1e-45",
    ];
    for a in specials.iter() {
        for b in specials.iter() {
            for c in specials.iter() {
                assert_same(&[oss(a), oss(b), oss(c)]);
            }
        }
    }
}

/// `inf * 0` and `inf + -inf` are invalid operations. On x86 they yield the
/// "QNaN floating-point indefinite", whose sign bit is *set*, so the C program
/// prints `-nan` rather than `nan` for these.
#[test]
fn invalid_operations_print_negative_nan() {
    check_triples(&[
        ["inf", "0", "0"],
        ["-inf", "0", "0"],
        ["0", "inf", "0"],
        ["0", "0", "inf"],
        ["inf", "0", "1"],
        ["1e39", "0", "0"],
        ["inf", "-inf", "0"],
        ["1e39", "-1e39", "0"],
        ["inf", "inf", "0"],
    ]);

    // Pin the actual bytes: this is the case that a "NaN is NaN" comparison
    // would wave through.
    let args = [oss("inf"), oss("0"), oss("0")];
    let c = run(c_binary(), &args);
    assert_eq!(c.stdout, b"-inf -nan -nan\n");
    assert_same(&args);
}

/// Arguments are raw bytes, not text: `argv` need not be valid UTF-8.
#[test]
fn non_utf8_arguments() {
    let cases: Vec<Vec<Vec<u8>>> = vec![
        vec![b"\x80\xff".to_vec(), b"1".to_vec(), b"1".to_vec()],
        vec![b"1".to_vec(), b"\xc3".to_vec(), b"1".to_vec()],
        vec![b"1".to_vec(), b"1".to_vec(), b"\xfe\xfd\xfc".to_vec()],
        vec![b"\xff5".to_vec(), b"5\xff".to_vec(), b"5".to_vec()],
        // Valid UTF-8 but not ASCII digits: full-width "123".
        vec!["１２３".as_bytes().to_vec(), b"1".to_vec(), b"1".to_vec()],
        // Wrong arity with non-UTF-8 bytes, i.e. the error path.
        vec![b"\x80\xff".to_vec(), b"1".to_vec()],
    ];
    for case in &cases {
        let args: Vec<OsString> = case.iter().map(|b| bytes(b)).collect();
        assert_same(&args);
    }
}

/// Very long arguments: `atof` reads the whole token, however long.
#[test]
fn very_long_arguments() {
    let long_int = "9".repeat(5000);
    let long_frac = format!("0.{}1", "0".repeat(5000));
    let long_hex = format!("0x{}p-8000", "f".repeat(2000));
    let long_exp = format!("1e{}", "9".repeat(400));
    let long_ws = format!("{}5", " ".repeat(4000));
    let long_zeros = format!("{}1", "0".repeat(5000));
    let cases = [
        [long_int.as_str(), "1", "1"],
        [long_frac.as_str(), "1", "1"],
        [long_hex.as_str(), "1", "1"],
        [long_exp.as_str(), "1", "1"],
        [long_ws.as_str(), "1", "1"],
        [long_zeros.as_str(), "1", "1"],
    ];
    check_triples(&cases);
}

// ---------------------------------------------------------------------------
// Phase C: broader sweeps, to catch paths the hand-written cases miss
// ---------------------------------------------------------------------------

/// Deterministic 64-bit xorshift, so the sweeps below are reproducible.
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

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Render an `f32` as a decimal string that `atof` converts back to exactly the
/// same `f32`. `f32 -> f64` is exact and 17 significant digits round-trip an
/// `f64`, so the conversion is lossless.
fn exact_arg(f: f32) -> String {
    if f.is_nan() {
        // `atof` cannot spell an arbitrary NaN payload, and the payload is not
        // observable in `%f` output -- only the sign is.
        return if f.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if f.is_infinite() {
        return if f < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    format!("{:.17e}", f as f64)
}

/// Notable `f32` bit patterns: zeros, subnormals, the normal boundary, the max
/// finite value, infinities, quiet NaNs of both signs, and the magic constants
/// from `Q_rsqrt` itself.
const SPECIAL_BITS: [u32; 28] = [
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // smallest +subnormal
    0x8000_0001, // smallest -subnormal
    0x007f_ffff, // largest +subnormal
    0x807f_ffff, // largest -subnormal
    0x0080_0000, // smallest +normal
    0x8080_0000, // smallest -normal
    0x3f80_0000, // 1.0
    0xbf80_0000, // -1.0
    0x3fc0_0000, // 1.5 (threehalfs)
    0x7f7f_ffff, // max finite
    0xff7f_ffff, // -max finite
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x7fc0_0000, // +qNaN
    0xffc0_0000, // -qNaN (the x86 indefinite)
    0x7fc0_0001, // +qNaN, other payload
    0xffc0_0001, // -qNaN, other payload
    0x7f80_0001, // +sNaN
    0xff80_0001, // -sNaN
    0x7fff_ffff, // +NaN, all payload bits
    0xffff_ffff, // -NaN, all payload bits
    0x4b00_0000, // 2^23, where floats stop being contiguous integers
    0x5f37_59df, // the Q_rsqrt magic constant read as a float
    0x1f77_59df, // Q_rsqrt(inf)'s intermediate
    0x7e80_0000, // large enough that squaring overflows to inf
    0xfe80_0000, // negative counterpart
];

/// Every ordered pair of notable bit patterns in the first two components.
#[test]
fn sweep_special_bit_patterns() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for &a in SPECIAL_BITS.iter() {
        for &b in SPECIAL_BITS.iter() {
            let c = SPECIAL_BITS[rng.below(SPECIAL_BITS.len())];
            assert_same(&[
                oss(&exact_arg(f32::from_bits(a))),
                oss(&exact_arg(f32::from_bits(b))),
                oss(&exact_arg(f32::from_bits(c))),
            ]);
        }
    }
}

/// Uniformly random `f32` bit patterns, covering the whole value space
/// including subnormals, infinities and NaNs.
#[test]
fn sweep_random_float_bit_patterns() {
    let mut rng = Rng(0xdead_beef_cafe_f00d);
    for _ in 0..1200 {
        let a = f32::from_bits(rng.next_u32());
        let b = f32::from_bits(rng.next_u32());
        let c = f32::from_bits(rng.next_u32());
        assert_same(&[
            oss(&exact_arg(a)),
            oss(&exact_arg(b)),
            oss(&exact_arg(c)),
        ]);
    }
}

/// Random decimal magnitudes spanning the whole exponent range, so the sweep
/// includes values that overflow, underflow and round on the way in, plus
/// dot products that overflow to infinity.
#[test]
fn sweep_random_decimal_magnitudes() {
    let mut rng = Rng(0x0bad_f00d_1234_5677);
    for _ in 0..1200 {
        let mut args: Vec<OsString> = Vec::with_capacity(3);
        for _ in 0..3 {
            let sign = if rng.next_u64() & 1 == 0 { "-" } else { "" };
            let mant = rng.next_u64() % 10_000_000;
            let exp = (rng.next_u64() % 700) as i64 - 350;
            args.push(oss(&format!("{sign}{mant}.{mant}e{exp}")));
        }
        assert_same(&args);
    }
}

/// Random hexadecimal floats, including binary exponents far outside the
/// representable range in both directions.
#[test]
fn sweep_random_hex_floats() {
    const HEX: &[u8] = b"0123456789abcdefABCDEF";
    let mut rng = Rng(0x5151_5151_2323_2323);
    for _ in 0..900 {
        let mut args: Vec<OsString> = Vec::with_capacity(3);
        for _ in 0..3 {
            let sign = if rng.next_u64() & 1 == 0 { "-" } else { "" };
            let ndig = 1 + rng.below(18);
            let nfrac = rng.below(15);
            let int: String = (0..ndig)
                .map(|_| HEX[rng.below(HEX.len())] as char)
                .collect();
            let frac: String = (0..nfrac)
                .map(|_| HEX[rng.below(HEX.len())] as char)
                .collect();
            let exp = (rng.next_u64() % 2200) as i64 - 1100;
            args.push(oss(&format!("{sign}0x{int}.{frac}p{exp}")));
        }
        assert_same(&args);
    }
}

/// Random strings assembled from numeric-looking fragments: mostly invalid,
/// which is exactly the point -- `atof` must fail the same way in both.
#[test]
fn sweep_adversarial_numeric_tokens() {
    const PIECES: &[&str] = &[
        "0x", "0X", "", "0", "00", "1", ".", "..", "e", "E", "p", "P", "+", "-", "+-", "f", "F",
        "inf", "nan", "infinity", "999999999999999999999999999999", "000000000000000000000000",
        "11111111111111111111", "ffff", "p1024", "e400", "e-400", "p-1100", "p1100", ".0", "(",
        ")", " ", "\t", "\n", "_", ",", "'", "*", "/", "#", "\u{b}", "\u{c}", "\r",
    ];
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..1500 {
        let mut args: Vec<OsString> = Vec::with_capacity(3);
        for _ in 0..3 {
            let n = 1 + rng.below(6);
            let s: String = (0..n).map(|_| PIECES[rng.below(PIECES.len())]).collect();
            args.push(oss(&s));
        }
        assert_same(&args);
    }
}

/// Random raw byte strings, valid UTF-8 or not, of random length including
/// empty. Covers the arity error path too by varying the argument count.
#[test]
fn sweep_random_raw_bytes_and_arity() {
    let mut rng = Rng(0xfeed_face_dead_10cc);
    for _ in 0..1200 {
        let argc = rng.below(6); // 0..=5 operands: exercises argc != 4 as well
        let mut args: Vec<OsString> = Vec::with_capacity(argc);
        for _ in 0..argc {
            let n = rng.below(11);
            // 1..=255: a NUL byte cannot appear in an argv entry.
            let b: Vec<u8> = (0..n).map(|_| 1 + (rng.next_u32() % 255) as u8).collect();
            args.push(bytes(&b));
        }
        assert_same(&args);
    }
}
