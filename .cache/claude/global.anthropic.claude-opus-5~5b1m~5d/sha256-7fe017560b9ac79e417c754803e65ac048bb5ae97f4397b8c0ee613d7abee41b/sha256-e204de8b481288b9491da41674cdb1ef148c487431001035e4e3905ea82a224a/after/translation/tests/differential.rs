//! Differential tests: run the C `driver` and the Rust `driver` as *subprocesses*
//! and compare stdout, stderr and the exit status byte for byte.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell would drive them.
//!
//! `argv[0]` appears in the usage message, so both binaries are exec'd with a
//! fixed `argv[0]` ("driver") via `CommandExt::arg0`; otherwise the two paths
//! would make the usage message differ for reasons that have nothing to do with
//! the translation.

#![cfg(unix)]

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .expect("translation/ must have a parent")
            .to_path_buf()
    })
}

/// The Rust binary under test, built by cargo for this test run.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C binary. Uses `c_src/build/driver` when it is already there, otherwise
/// configures an out-of-source cmake build under `target/` so that nothing in
/// `c_src/` is touched.
fn c_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let prebuilt = repo_root().join("c_src/build/driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let src = repo_root().join("c_src");
        let build = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/c_build");
        std::fs::create_dir_all(&build).expect("create cmake build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("cmake must be installed to run the differential tests");
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
            .expect("run cmake --build");
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

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Output {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` when killed by a signal.
    code: Option<i32>,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={:?} stderr={:?}",
            self.code,
            show(&self.stdout),
            show(&self.stderr)
        )
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes.iter().take(400) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    if bytes.len() > 400 {
        s.push_str("...(truncated)");
    }
    s
}

fn run(bin: &Path, args: &[&[u8]]) -> Output {
    let mut cmd = Command::new(bin);
    cmd.arg0("driver");
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));
    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(args: &[&[u8]]) {
    let c = run(c_bin(), args);
    let r = run(rust_bin(), args);
    if c != r {
        let pretty: Vec<String> = args.iter().map(|a| show(a)).collect();
        panic!(
            "output mismatch for argv {:?}\n  C   : {:?}\n  Rust: {:?}",
            pretty, c, r
        );
    }
    // Guard against a vacuous comparison: this program always says something.
    assert!(
        !(c.stdout.is_empty() && c.stderr.is_empty()),
        "expected the C program to produce output for argv {:?}",
        args
    );
}

fn check_all(cases: &[&[&[u8]]]) {
    for case in cases {
        assert_same(case);
    }
}

/// Convenience: one base and one exponent, as byte strings.
fn pair<'a>(b: &'a str, e: &'a str) -> [&'a [u8]; 2] {
    [b.as_bytes(), e.as_bytes()]
}

fn check_pairs(pairs: &[(&str, &str)]) {
    for (b, e) in pairs {
        let args = pair(b, e);
        assert_same(&args);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let args = pair("2", "10");
    let c = run(c_bin(), &args);
    let r = run(rust_bin(), &args);
    assert_eq!(c.stdout, b"Result: 1024.00\n".to_vec(), "C baseline output");
    assert_eq!(c, r);
}

// ---------------------------------------------------------------------------
// argc branch: `if (argc != 3)`
// ---------------------------------------------------------------------------

#[test]
fn arity_errors() {
    check_all(&[
        &[],                                      // no arguments at all
        &[b"2"],                                  // one argument
        &[b"2", b"3", b"4"],                      // one too many
        &[b"2", b"3", b"4", b"5"],                // several too many
        &[b""],                                   // single empty argument
        &[b"", b"", b""],                         // three empty arguments
    ]);
}

// ---------------------------------------------------------------------------
// Happy paths
// ---------------------------------------------------------------------------

#[test]
fn happy_paths() {
    check_pairs(&[
        ("2", "10"),
        ("2.5", "3"),
        ("-2", "3"),
        ("-2", "2"),
        ("0", "0"),
        ("0", "5"),
        ("10", "2"),
        ("2", "0.5"),
        ("1e10", "2"),
        ("1", "1e308"),
        ("-1", "1e308"),
        ("3", "-1"),
        ("2", "-1"),
        ("1e100", "3"),
        ("1e300", "1"),
        ("0.5", "-3"),
        ("2", "1024"),
        ("2", "-1074"),
        ("1e308", "1"),
        ("1e-308", "1"),
        ("4503599627370495.5", "1"),
        ("1e16", "1"),
        ("1e17", "1"),
    ]);
}

// ---------------------------------------------------------------------------
// Base conversion: `*endptr1 != '\0'`
// ---------------------------------------------------------------------------

#[test]
fn invalid_base() {
    check_pairs(&[
        ("abc", "2"),
        (" ", "2"),
        ("  ", "2"),
        ("1.2.3", "2"),
        ("12x", "3"),
        ("+", "2"),
        ("-", "2"),
        ("--1", "2"),
        (".", "2"),
        ("+.", "2"),
        ("-.", "2"),
        ("e5", "2"),
        ("1 2", "2"),
        ("%", "2"),
        ("5\n", "2"),
        (" 5 ", "2"),
        ("12  ", "3"),
        ("1_000", "2"),
        ("1,5", "2"),
        ("0x", "2"),
        ("0X", "2"),
        ("-0x", "2"),
        ("0x.", "2"),
        ("0xg", "2"),
        ("0x1g", "2"),
        ("00x1", "2"),
        ("0x1p", "2"),
        ("0x1p+", "2"),
        ("0x1p-", "2"),
        ("infinit", "2"),
        ("infinityx", "2"),
        ("nan(", "2"),
        ("nan(abc", "2"),
        ("nan(ab)c", "2"),
        ("1e", "2"),
        ("1e+", "2"),
        ("0e", "2"),
    ]);
}

/// The empty string is *accepted* by this program: `strtod("")` performs no
/// conversion, leaves `endptr` at the terminating NUL, so `*endptr == '\0'`
/// and the base silently becomes 0.0. Replicated, not fixed.
#[test]
fn empty_argument_is_accepted_as_zero() {
    check_pairs(&[("", "2"), ("2", ""), ("", ""), ("", "0"), ("", "-1")]);
}

// ---------------------------------------------------------------------------
// Exponent conversion: same checks, second argument
// ---------------------------------------------------------------------------

#[test]
fn invalid_exponent() {
    check_pairs(&[
        ("2", "abc"),
        ("2", " "),
        ("2", "1.2.3"),
        ("2", "3x"),
        ("2", "+"),
        ("2", "-"),
        ("2", "."),
        ("2", "e5"),
        ("2", "1e"),
        ("2", "0x"),
        ("2", "nan("),
        ("2", "1 2"),
        ("2", "\t"),
    ]);
}

/// The base is validated before the exponent, so a bad base wins even when the
/// exponent is bad too.
#[test]
fn base_is_checked_before_exponent() {
    check_pairs(&[
        ("abc", "def"),
        ("abc", "1e999"),
        ("1e999", "abc"),
        ("1e999", "1e999"),
        ("1e-999", "abc"),
    ]);
}

// ---------------------------------------------------------------------------
// Leading whitespace and sign forms accepted by strtod
// ---------------------------------------------------------------------------

#[test]
fn whitespace_and_sign_forms() {
    check_all(&[
        &[b"  12", b"3"],
        &[b"\t12", b"3"],
        &[b"\n12", b"3"],
        &[b"\r12", b"3"],
        &[b"\x0b12", b"3"],
        &[b"\x0c12", b"3"],
        &[b"\n\t 12", b"3"],
        &[b" +5", b"3"],
        &[b"+.5", b"2"],
        &[b"-.5", b"2"],
        &[b"5.", b"2"],
        &[b".5", b"2"],
        &[b"+2", b"+3"],
        &[b"00", b"2"],
        &[b"000.000", b"2"],
        &[b"12\n", b"3"],
    ]);
}

// ---------------------------------------------------------------------------
// inf / nan spellings
// ---------------------------------------------------------------------------

#[test]
fn infinity_and_nan_forms() {
    check_pairs(&[
        ("inf", "2"),
        ("-inf", "2"),
        ("+inf", "+2"),
        ("INF", "inf"),
        ("infinity", "2"),
        ("INFINITY", "2"),
        ("iNfInItY", "2"),
        ("nan", "2"),
        ("NaN", "2"),
        ("-nan", "3"),
        ("nan", "-3"),
        ("nan", "0"),
        ("nan", "nan"),
        ("-nan(1)", "2"),
        ("nan(123)", "2"),
        ("nan()", "2"),
        ("1", "nan"),
        ("0", "nan"),
        ("inf", "0"),
        ("-1", "inf"),
        ("-1", "-inf"),
        ("1", "inf"),
        ("0.5", "-inf"),
        ("2", "inf"),
        ("2", "-inf"),
        ("-2", "inf"),
        ("0", "inf"),
        ("0", "-inf"),
        ("inf", "-1"),
        ("-inf", "3"),
        ("-inf", "0.5"),
        ("-inf", "-3"),
    ]);
}

// ---------------------------------------------------------------------------
// Hexadecimal literals
// ---------------------------------------------------------------------------

#[test]
fn hex_forms() {
    check_pairs(&[
        ("0x1p3", "2"),
        ("0X1P4", "2"),
        ("0x10", "2"),
        ("0x0", "2"),
        ("-0x0", "2"),
        ("0x0p999", "2"),
        ("0x0p-999", "2"),
        ("0x0.0p0", "1"),
        ("0x.8p1", "1"),
        ("0x8.p-3", "1"),
        ("0x.1", "1"),
        ("0x1.", "1"),
        ("0x1.8p3", "1"),
        ("0x0000000000000000000001p0", "1"),
        ("0x1p1023", "1"),
        ("0x1p1024", "1"),
        ("0x1p-1022", "1"),
        ("0x1p-1074", "1"),
        ("0x1p-1075", "1"),
        ("0x1p-1076", "1"),
        ("0x1.fffffffffffffp1023", "1"),
        ("0x1.fffffffffffff8p1023", "1"),
        ("0x1.fffffffffffffp-1022", "1"),
        ("0x0.0000000000001p-1022", "1"),
        ("0x1p99999999999999999999", "2"),
        ("0x1p-99999999999999999999", "2"),
    ]);
}

/// Hexadecimal mantissas that are exact ties when rounded to 53 bits, scaled up
/// far enough that one ulp is visible through `%.2f`. glibc rounds to nearest
/// with ties to even.
#[test]
fn hex_rounding_ties() {
    check_pairs(&[
        ("0x20000000000001p10", "1"),  // 2^53+1, tie, rounds down
        ("0x20000000000003p10", "1"),  // 2^53+3, tie, rounds up
        ("0x20000000000001p1", "1"),
        ("0x40000000000002p10", "1"),
        ("0x1.00000000000008p60", "1"),
        ("0x1.00000000000018p60", "1"),
        ("0x1.0000000000000fp60", "1"),
        ("0x3ffffffffffffffp10", "1"),
    ]);
}

/// Hex values just below DBL_MIN whose plain 53-bit rounding already reaches
/// DBL_MIN: glibc raises nothing, even though the value is "tiny".
#[test]
fn hex_tiny_values_at_dbl_min() {
    check_pairs(&[
        ("0x1.fffffffffffffffp-1023", "1"),
        ("0x1.ffffffffffffff8p-1023", "1"),
        ("0x1.fffffffffffffffffffp-1023", "1"),
        ("0x1.0000000000000001p-1022", "1"),
        ("0x1.fp-1023", "1"),
        ("0x1.8p-1023", "1"),
        ("0x1p-1023", "1"),
    ]);
}

/// A long mantissa combined with a saturating exponent: the decimal exponent
/// must stay large enough (relative to the digit count) that the result really
/// collapses to zero / infinity.
#[test]
fn long_mantissa_with_saturating_exponent() {
    let nines: Vec<u8> = std::iter::repeat(b'9').take(700).collect();

    let mut huge_neg = nines.clone();
    huge_neg.extend_from_slice(b"e-99999999999999");
    let mut moderate_neg = nines.clone();
    moderate_neg.extend_from_slice(b"e-1000");
    let mut huge_pos = nines.clone();
    huge_pos.extend_from_slice(b"e99999999999999");

    let mut long_frac = b"0.".to_vec();
    long_frac.extend(std::iter::repeat(b'0').take(700));
    long_frac.extend(std::iter::repeat(b'9').take(20));
    long_frac.extend_from_slice(b"e1000");

    let mut one_zeros = b"1".to_vec();
    one_zeros.extend(std::iter::repeat(b'0').take(700));
    one_zeros.extend_from_slice(b"e-99999999999999");

    for lit in [&huge_neg, &moderate_neg, &huge_pos, &long_frac, &one_zeros] {
        assert_same(&[lit.as_slice(), b"1"]);
        assert_same(&[b"2", lit.as_slice()]);
    }

    // 10 000 significant digits: the mantissa length now dominates any
    // saturated exponent, so the exponent must not be clamped too early.
    let many: Vec<u8> = std::iter::repeat(b'9').take(10_000).collect();
    let mut a = many.clone();
    a.extend_from_slice(b"e-99999999999999");
    let mut b = many.clone();
    b.extend_from_slice(b"e-10000"); // ~1.0, well inside range
    let mut c = many.clone();
    c.extend_from_slice(b"e99999999999999");
    let mut d = b"1".to_vec();
    d.extend(std::iter::repeat(b'0').take(20_000));
    d.extend_from_slice(b"e-99999999999999");
    let mut e = b"0.".to_vec();
    e.extend(std::iter::repeat(b'0').take(10_000));
    e.extend_from_slice(b"1e99999999999999");

    for lit in [&a, &b, &c, &d, &e] {
        assert_same(&[lit.as_slice(), b"1"]);
    }
}

// ---------------------------------------------------------------------------
// ERANGE from strtod (`errno == ERANGE` branches), base and exponent
// ---------------------------------------------------------------------------

#[test]
fn conversion_range_errors() {
    check_pairs(&[
        ("1e999", "2"),
        ("-1e999", "2"),
        ("2", "1e999"),
        ("1e-999", "2"),
        ("2", "1e-999"),
        ("1e-320", "2"),
        ("1e-323", "2"),
        ("2.5e-324", "2"),
        ("1e99999999999999999999", "2"),
        ("1e-99999999999999999999", "2"),
        ("2", "1e99999999999999999999"),
        ("2", "1e-99999999999999999999"),
        ("1e+00000000000000000000000309", "1"),
        ("1e-00000000000000000000000309", "1"),
        ("1.7976931348623159e308", "1"),
        ("179769313486231580793728971405303", "1"),
    ]);
}

/// Values that look tiny but must *not* raise ERANGE (exact subnormals, and
/// tiny values whose plain 53-bit rounding already reaches DBL_MIN).
#[test]
fn conversion_range_non_errors() {
    check_pairs(&[
        ("0", "2"),
        ("-0", "2"),
        ("0.0", "2"),
        ("0.00000", "2"),
        ("0e999", "2"),
        ("0e-999", "2"),
        ("0e0", "2"),
        ("1e-310", "2"),
        ("1e-308", "2"),
        ("4.9e-324", "2"),
        ("4.9406564584124654e-324", "1"),
        ("2.2250738585072014e-308", "1"),
        ("2.2250738585072011e-308", "1"),
        ("1.7976931348623157e308", "1"),
        ("1e00000000000000000000000000", "1"),
    ]);
}

/// The DBL_MIN / 2^-1074 neighbourhood, where glibc's underflow rule is subtle:
/// ERANGE is raised only when the value is below the smallest normal *and* the
/// conversion is inexact.
#[test]
fn subnormal_boundary_decimals() {
    check_pairs(&[
        // exactly 2^-1074, and exact multiples of it
        ("4.94065645841246544176568792868e-324", "1"),
        ("9.88131291682493088353137585736e-324", "1"),
        ("1.48219693752373963253e-323", "1"),
        // exactly half of 2^-1074 (ties to even -> 0, inexact -> ERANGE)
        ("2.470328229206232720821e-324", "1"),
        ("2.4703282292062327208828439643412e-324", "1"),
        ("2.4703282292062327208828439643413e-324", "1"),
        // just above / below the smallest normal
        ("2.2250738585072013e-308", "1"),
        ("2.2250738585072012e-308", "1"),
        ("2.225073858507201e-308", "1"),
        ("1.1125369292536007e-308", "1"),
        // long exact decimal expansions
        ("0.00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000049406564584124654417656879286822137236505980261432476442558568250067550727020875186529983636163599237979656469544571773092665671035593979639877479601078187812630071319031140452784581716784898210368871863605699873072305000638559800615601280866424491365387286253406737302909118588009302768840009779402197832221472823427199930582966127178654626630376547771078584121127309910606074580796227218665356633148558252612497965159917568412906403997100299382825016323491785180435243380488478289181366208485508946362219722344619774486339183167450818013216969944071834431497514591962908368404901295012963348023550402029121417262495963951439591780370118534696384861859587869303518254988612360563723272874521613813125116925897304700347869908422105934571032372857982086219578610623154405425442830348509843833540064057930188304122154254656126336801636122040400128005893085578015580654903405640394747802879307904539567555886695129501182146917000e-4000", "1"),
    ]);
}

// ---------------------------------------------------------------------------
// Exactly representable subnormals, and the exact glibc underflow threshold.
//
// `2^-n` is exactly `5^n * 10^-n`, so writing `k*5^n` followed by `e-n` gives
// the *exact* decimal expansion of `k * 2^-n`. That is the only way to reach
// the branches where a value below DBL_MIN does **not** raise ERANGE, and the
// only way to sit exactly on the `2^-1022 - 2^-1076` threshold.
// ---------------------------------------------------------------------------

/// Decimal digits of `k * 5^n`.
fn k_times_pow5(k: u128, n: u32) -> Vec<u8> {
    // little-endian decimal digits
    let mut d: Vec<u8> = Vec::new();
    let mut k = k;
    if k == 0 {
        d.push(0);
    }
    while k > 0 {
        d.push((k % 10) as u8);
        k /= 10;
    }
    for _ in 0..n {
        let mut carry = 0u8;
        for x in d.iter_mut() {
            let v = *x * 5 + carry;
            *x = v % 10;
            carry = v / 10;
        }
        while carry > 0 {
            d.push(carry % 10);
            carry /= 10;
        }
    }
    while d.len() > 1 && *d.last().unwrap() == 0 {
        d.pop();
    }
    d.iter().rev().map(|&x| b'0' + x).collect()
}

/// `digits - 1`, for a digit string that does not end in 0.
fn dec_minus_one(digits: &[u8]) -> Vec<u8> {
    let mut v = digits.to_vec();
    let last = v.last_mut().unwrap();
    assert!(*last > b'0', "helper only handles a non-zero last digit");
    *last -= 1;
    v
}

fn literal(digits: &[u8], exp: i32) -> Vec<u8> {
    let mut v = digits.to_vec();
    v.extend_from_slice(format!("e{}", exp).as_bytes());
    v
}

#[test]
fn exact_subnormals_do_not_raise_erange() {
    // k * 2^-1074 written exactly: representable, so no ERANGE.
    for k in [1u128, 2, 3, 4, 7, 1000, (1u128 << 52) - 1] {
        let lit = literal(&k_times_pow5(k, 1074), -1074);
        assert_same(&[lit.as_slice(), b"1"]);
        assert_same(&[b"2", lit.as_slice()]);
    }

    // (2k+1) * 2^-1075, i.e. exactly half way between two subnormals:
    // inexact, so ERANGE.
    for k in [0u128, 1, 2, 5] {
        let lit = literal(&k_times_pow5(2 * k + 1, 1075), -1075);
        assert_same(&[lit.as_slice(), b"1"]);
    }
}

#[test]
fn underflow_threshold_is_exact() {
    // (2^54 - 1) * 2^-1076 == 2^-1022 - 2^-1076, the exact point at which glibc
    // stops reporting underflow.
    let thr = k_times_pow5((1u128 << 54) - 1, 1076);
    assert_same(&[literal(&thr, -1076).as_slice(), b"1"]); // on it: no ERANGE
    assert_same(&[literal(&dec_minus_one(&thr), -1076).as_slice(), b"1"]); // below: ERANGE

    let mut above = thr.clone();
    *above.last_mut().unwrap() += 1; // last digit of 5^n is 5, so this is safe
    assert_same(&[literal(&above, -1076).as_slice(), b"1"]);

    // exactly 2^-1022 (DBL_MIN) and 2^-1022 - 2^-1075
    assert_same(&[literal(&k_times_pow5(1u128 << 54, 1076), -1076).as_slice(), b"1"]);
    assert_same(&[
        literal(&k_times_pow5((1u128 << 54) - 2, 1076), -1076).as_slice(),
        b"1",
    ]);
}

// ---------------------------------------------------------------------------
// pow: EDOM branch
// ---------------------------------------------------------------------------

#[test]
fn pow_domain_errors() {
    check_pairs(&[
        ("-2", "0.5"),
        ("-2", "-0.5"),
        ("-8", "0.3333333"),
        ("-1e300", "0.5"),
        ("-0.0001", "1.5"),
        ("-2", "1e-40"),
        ("-2", "0.3333333333"),
        ("-1.5", "2.5"),
        ("-1", "0.5"),
    ]);
}

// ---------------------------------------------------------------------------
// pow: ERANGE branch (overflow, underflow to zero, pole error)
// ---------------------------------------------------------------------------

#[test]
fn pow_range_errors() {
    check_pairs(&[
        ("10", "400"),
        ("10", "-400"),
        ("0", "-1"),
        ("-0", "-1"),
        ("0", "-2"),
        ("0", "-0.5"),
        ("2", "-1075"),
        ("2", "1100"),
        ("0.5", "1075"),
        ("3", "-700"),
        ("1e-200", "2"),
        ("1e308", "2"),
        ("-10", "401"),
        ("-0.5", "1075"),
        ("1e-300", "3"),
    ]);
}

/// Results that stay finite / subnormal without raising anything.
#[test]
fn pow_no_errno() {
    check_pairs(&[
        ("2", "-1050"),
        ("1e-300", "1.05"),
        ("1e-160", "2"),
        ("2", "-1073"),
        ("2", "-1074"),
        ("0.5", "1074"),
        ("1e-100", "3"),
        ("1", "1e300"),
        ("1", "-1e300"),
        ("-1", "1e300"),
        ("0", "1e300"),
    ]);
}

// ---------------------------------------------------------------------------
// printf("%.2f") formatting: ties, negative zero, huge magnitudes
// ---------------------------------------------------------------------------

#[test]
fn printf_formatting() {
    check_pairs(&[
        // exact ties at the second decimal (glibc rounds half to even)
        ("0.125", "1"),
        ("0.375", "1"),
        ("0.625", "1"),
        ("0.875", "1"),
        ("1.125", "1"),
        ("-0.125", "1"),
        ("8.5", "-1"),
        ("0.5", "3"),
        // decimal literals that are not exact ties
        ("2.675", "1"),
        ("0.005", "1"),
        ("0.015", "1"),
        ("0.025", "1"),
        ("0.045", "1"),
        ("1.005", "1"),
        ("-0.005", "1"),
        ("123456789.555", "1"),
        // negative zero keeps its sign
        ("-0", "3"),
        ("-0", "1"),
        ("-0.0", "5"),
        ("-0.0000001", "1"),
        ("0.0000001", "1"),
        // very large / very small finite results
        ("1e300", "1"),
        ("1e150", "2"),
        ("1e-300", "1"),
        ("1.7976931348623157e308", "1"),
    ]);
}

// ---------------------------------------------------------------------------
// Bytes that are not valid UTF-8 must reach strtod unchanged
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_arguments() {
    check_all(&[
        &[b"\xff\xfe", b"2"],
        &[b"1\xff", b"2"],
        &[b"2", b"\xc3"],
        &[b"\x80\x80", b"\x80"],
        &[b"\xff", b"\xff"],
        &[b"12\xc3\xa9", b"2"],
        &[b"\xe2\x82\xac", b"2"],
    ]);
}

// ---------------------------------------------------------------------------
// Long inputs
// ---------------------------------------------------------------------------

#[test]
fn long_inputs() {
    let mut cases: Vec<Vec<u8>> = Vec::new();

    // "1e000...0001": exponent digits far beyond any useful range
    let mut v = b"1e".to_vec();
    v.extend(std::iter::repeat(b'0').take(5000));
    v.push(b'1');
    cases.push(v);

    // 1 followed by 400 zeros -> overflow
    let mut v = b"1".to_vec();
    v.extend(std::iter::repeat(b'0').take(400));
    cases.push(v);

    // 0.000...01 with 400 fractional zeros -> underflow
    let mut v = b"0.".to_vec();
    v.extend(std::iter::repeat(b'0').take(400));
    v.push(b'1');
    cases.push(v);

    // exactly 2^-1074 written out in full
    let mut v = b"0.".to_vec();
    v.extend(std::iter::repeat(b'0').take(323));
    v.extend_from_slice(b"49406564584124654417656879286822");
    cases.push(v);

    // 800 nines, with and without a compensating exponent
    cases.push(std::iter::repeat(b'9').take(800).collect());
    let mut v: Vec<u8> = std::iter::repeat(b'9').take(400).collect();
    v.extend_from_slice(b"e-400");
    cases.push(v);

    // 400 hex digits
    let mut v = b"0x".to_vec();
    v.extend(std::iter::repeat(b'f').take(400));
    cases.push(v);

    // long digit run that is still an ordinary number
    let mut v: Vec<u8> = std::iter::repeat(b'1').take(30).collect();
    v.extend_from_slice(b".");
    v.extend(std::iter::repeat(b'7').take(60));
    cases.push(v);

    for base in &cases {
        for exp in [&b"1"[..], &b"2"[..], &b"-1"[..], &b"0.5"[..]] {
            assert_same(&[base.as_slice(), exp]);
            assert_same(&[exp, base.as_slice()]);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic randomised sweep over the interesting shapes
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

/// Build one pseudo-random argument covering a mix of shapes: plain integers,
/// decimals with exponents around the overflow / underflow cliffs, hex
/// literals, inf / nan spellings, junk, and random character soup.
fn gen_arg(rng: &mut Rng) -> Vec<u8> {
    const FIXED: [&str; 34] = [
        "", " ", "abc", "12x", "1.2.3", "+", "-", ".", "e5", "1e", "1e+", "0x", "0x.", "--1",
        "1 2", "%", "\t5", "5\n", " 5 ", "inf", "-inf", "nan", "-nan", "infinity", "INF", "NaN",
        "nan(x)", "0", "-0", "0.0", "0e999", "0e-999", "1", "-1",
    ];
    const NOTABLE: [&str; 12] = [
        "1e308",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "2.2250738585072014e-308",
        "2.2250738585072011e-308",
        "4.9406564584124654e-324",
        "2.4703282292062327e-324",
        "2.4703282292062328e-324",
        "5e-324",
        "1e-323",
        "1075",
        "-1075",
    ];

    match rng.below(10) {
        0 => format!("{}", rng.below(2001) as i64 - 1000).into_bytes(),
        1 => format!(
            "{}e{}",
            rng.below(1_000_000) + 1,
            rng.below(681) as i64 - 340
        )
        .into_bytes(),
        2 => format!(
            "{}e{}",
            rng.below(1_000_000) + 1,
            rng.below(2201) as i64 - 1100
        )
        .into_bytes(),
        3 => {
            // digits, optional fraction, optional exponent
            let n = 1 + rng.below(40);
            let mut s: Vec<u8> = (0..n).map(|_| b'0' + rng.below(10) as u8).collect();
            if rng.below(2) == 0 {
                s.push(b'.');
                let m = 1 + rng.below(30);
                s.extend((0..m).map(|_| b'0' + rng.below(10) as u8));
            }
            if rng.below(2) == 0 {
                s.extend_from_slice(format!("e{}", rng.below(801) as i64 - 400).as_bytes());
            }
            s
        }
        4 => {
            let sign = *rng.pick(&["", "-", "+"]);
            let tail = *rng.pick(&["", ".8", "p+3", "P-2", ".fffp1"]);
            let p = if rng.below(2) == 0 {
                format!("p{}", rng.below(2201) as i64 - 1100)
            } else {
                String::new()
            };
            format!("{sign}0x{:x}{tail}{p}", rng.next_u64() >> rng.below(60)).into_bytes()
        }
        5 => rng.pick(&FIXED).as_bytes().to_vec(),
        6 => rng.pick(&NOTABLE).as_bytes().to_vec(),
        7 => {
            // near the subnormal cliff: 17 significant digits, tiny exponent
            let mant: Vec<u8> = (0..17).map(|_| b'0' + rng.below(10) as u8).collect();
            let mut s = vec![mant[0], b'.'];
            s.extend_from_slice(&mant[1..]);
            s.extend_from_slice(format!("e-{}", 300 + rng.below(30)).as_bytes());
            s
        }
        8 => {
            const AL: &[u8] = b"0123456789.eE+-xXpPaAfFnNiItyY \t\r\n\x0b\x0c()_\xff";
            (0..rng.below(14)).map(|_| *rng.pick(AL)).collect()
        }
        _ => format!(
            "{}.{}",
            rng.below(4000) as i64 - 2000,
            rng.next_u64() % 100_000_000
        )
        .into_bytes(),
    }
}

#[test]
fn randomised_sweep() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for _ in 0..700 {
        let a = gen_arg(&mut rng);
        let b = gen_arg(&mut rng);
        match rng.below(8) {
            0 => assert_same(&[a.as_slice()]),
            1 => assert_same(&[a.as_slice(), b.as_slice(), b"9"]),
            _ => assert_same(&[a.as_slice(), b.as_slice()]),
        }
    }
}
