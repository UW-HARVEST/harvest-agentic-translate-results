//! Differential tests: run the original C `driver` and the translated Rust
//! `driver` as subprocesses with identical arguments and require that stdout,
//! stderr and the exit status match byte for byte.
//!
//! The Rust code is deliberately *not* used as a library here — the program is
//! graded by execution, so it is driven exactly the way a shell would drive it.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Byte-level result of one run.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} exit={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.status,
            self.signal
        )
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/translation`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the compiled C program, building it with CMake on first use.
fn c_binary() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake not available; build c_src manually");
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
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
}

/// Path to the compiled Rust program. Cargo builds it for us.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Run one program. `argv0` is forced to the same value for both binaries so
/// that the `"%s requires 4 inputs"` message — which echoes `argv[0]` — can be
/// compared byte for byte even though the two executables live at different
/// paths.
fn run(program: &Path, argv0: &[u8], args: &[&[u8]]) -> Run {
    let mut cmd = Command::new(program);
    cmd.arg0(OsStr::from_bytes(argv0));
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    // A fixed, minimal environment: `%f` and `strtod` are locale sensitive in
    // C, so both programs must see the same locale.
    cmd.env_clear();
    cmd.env("LC_ALL", "C");
    cmd.env("LANG", "C");
    let out = cmd.output().unwrap_or_else(|e| {
        panic!("failed to run {}: {e}", program.display());
    });
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&out.status);
    #[cfg(not(unix))]
    let signal = None;
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
        signal,
    }
}

const ARGV0: &[u8] = b"driver";

/// Assert that both programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same_args(args: &[&[u8]]) {
    let c = run(c_binary(), ARGV0, args);
    let r = run(rust_binary(), ARGV0, args);

    let pretty: Vec<String> = args
        .iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a)))
        .collect();
    let label = format!("argv = [{}]", pretty.join(", "));

    assert_eq!(
        c.stdout, r.stdout,
        "stdout differs for {label}\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr differs for {label}\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status differs for {label}: C {:?} vs Rust {:?}",
        c.status, r.status
    );
    assert_eq!(
        c.signal, r.signal,
        "termination signal differs for {label}: C {:?} vs Rust {:?}",
        c.signal, r.signal
    );
}

/// Convenience wrapper for the common all-ASCII case.
#[track_caller]
fn same(args: &[&str]) {
    let bytes: Vec<&[u8]> = args.iter().map(|s| s.as_bytes()).collect();
    assert_same_args(&bytes);
}

/// Every argument slot, in turn, is set to `token` while the others stay at
/// `"1"`. The C program parses all three arguments independently, so this
/// exercises the token in each position.
#[track_caller]
fn same_in_each_slot(token: &str) {
    same(&[token, "1", "1"]);
    same(&["1", token, "1"]);
    same(&["1", "1", token]);
}

// ---------------------------------------------------------------------------
// argc: the only explicit branch in main()
// ---------------------------------------------------------------------------

/// `main` rejects anything other than `argc == 4` before touching `argv[1..]`,
/// writing to stderr and exiting 1.
#[test]
fn argc_not_four_is_an_error() {
    same(&[]);
    same(&["1"]);
    same(&["1", "2"]);
    same(&["1", "2", "3", "4"]);
    same(&["1", "2", "3", "4", "5"]);
    same(&["1", "2", "3", "4", "5", "6"]);
}

/// The error message interpolates `argv[0]`; both programs are given the same
/// `argv[0]`, including values that are empty, non-UTF-8 or `printf`-like.
#[test]
fn argc_error_echoes_argv0_verbatim() {
    for argv0 in [
        &b""[..],
        b"driver",
        b"./driver",
        b"/a/b/c/driver",
        b"%s %d %n",
        b"\xff\xfe\x80",
        b"with space",
        &[b'A'; 300][..],
    ] {
        let c = run(c_binary(), argv0, &[]);
        let r = run(rust_binary(), argv0, &[]);
        assert_eq!(
            c, r,
            "mismatch for argv0 {:?}",
            String::from_utf8_lossy(argv0)
        );
        assert_eq!(c.status, Some(1));
    }
}

/// Exactly three arguments is the success path.
#[test]
fn argc_four_is_the_success_path() {
    same(&["1", "2", "3"]);
    let c = run(c_binary(), ARGV0, &[b"1", b"2", b"3"]);
    assert_eq!(c.status, Some(0));
    assert_eq!(c.stderr, b"");
}

// ---------------------------------------------------------------------------
// VectorNormalizeFast / Q_rsqrt over ordinary values
// ---------------------------------------------------------------------------

#[test]
fn ordinary_vectors() {
    same(&["1", "2", "3"]);
    same(&["3", "4", "0"]);
    same(&["-1", "-1", "-1"]);
    same(&["1", "0", "0"]);
    same(&["0", "1", "0"]);
    same(&["0", "0", "1"]);
    same(&["0.5", "0.5", "0.5"]);
    same(&["100000", "200000", "300000"]);
    same(&["-3.5", "2.25", "-0.125"]);
    same(&["1e10", "-1e10", "1e10"]);
    same(&["123.456", "789.012", "-345.678"]);
}

/// The zero vector: `Q_rsqrt(0)` is not guarded against, so this exercises the
/// bit hack on a zero input.
#[test]
fn zero_vector() {
    same(&["0", "0", "0"]);
    same(&["0.0", "0.0", "0.0"]);
    same(&["-0", "-0", "-0"]);
    same(&["-0", "0", "-0"]);
    same(&["0", "-0", "0"]);
}

/// Negative zero has to keep its sign through `%f`, which prints `-0.000000`.
#[test]
fn negative_zero_components() {
    same(&["-0", "1", "1"]);
    same(&["1", "-0", "1"]);
    same(&["1", "1", "-0"]);
    same(&["-0.0", "-0.0", "1"]);
}

/// Squaring in `DotProduct` overflows to infinity well before `float`'s
/// maximum, and `Q_rsqrt` then produces a negative infinity.
#[test]
fn dot_product_overflows_to_infinity() {
    same(&["1e19", "1e19", "1e19"]);
    same(&["1e20", "0", "0"]);
    same(&["1e30", "1e30", "1e30"]);
    same(&["1e38", "1e38", "1e38"]);
    same(&["3.4028235e38", "0", "0"]);
    same(&["-3.4028235e38", "0", "0"]);
    same(&["1e39", "0", "0"]);
    same(&["-1e39", "0", "0"]);
    same(&["1e300", "0", "0"]);
}

/// Squaring tiny values underflows to zero, so `Q_rsqrt` sees exactly `0`.
#[test]
fn dot_product_underflows_to_zero() {
    same(&["1e-30", "1e-30", "1e-30"]);
    same(&["1e-23", "0", "0"]);
    same(&["1e-38", "0", "0"]);
    same(&["1e-45", "0", "0"]);
    same(&["1e-46", "0", "0"]);
    same(&["1.4012984643e-45", "0", "0"]);
    same(&["7.0064923217e-46", "0", "0"]);
    same(&["1e-320", "0", "0"]);
}

/// `float` subnormals, reached both directly and through the double-to-float
/// narrowing that the assignment in `main` performs.
#[test]
fn float_subnormals() {
    same(&["0x1p-149", "0", "0"]);
    same(&["0x1p-149", "0x1p-149", "0x1p-149"]);
    same(&["0x1p-126", "0x1p-140", "0x1p-149"]);
    same(&["0x0.8p-148", "0", "0"]);
    same(&["-0x1p-149", "0x1p-149", "0"]);
    same(&["0x1p-150", "0", "0"]);
    same(&["0x1.8p-150", "0", "0"]);
}

// ---------------------------------------------------------------------------
// Infinities and NaNs: the interesting part of Q_rsqrt
// ---------------------------------------------------------------------------

#[test]
fn infinities() {
    same(&["inf", "0", "0"]);
    same(&["-inf", "0", "0"]);
    same(&["inf", "inf", "inf"]);
    same(&["-inf", "-inf", "-inf"]);
    same(&["inf", "-inf", "0"]);
    same(&["-inf", "inf", "0"]);
    same(&["inf", "1", "-inf"]);
    same(&["1", "inf", "1"]);
    same(&["infinity", "0", "0"]);
    same(&["-INFINITY", "0", "0"]);
}

/// A single NaN anywhere poisons the whole result.
#[test]
fn single_nan() {
    same_in_each_slot("nan");
    same_in_each_slot("-nan");
    same_in_each_slot("NaN");
    same_in_each_slot("nan(1234)");
    same_in_each_slot("-nan(0x7f)");
}

/// Two NaNs of *different* signs is the case that pinned down the operand
/// order of `DotProduct`'s additions: x86 NaN propagation is not commutative,
/// so which NaN survives — and therefore the sign printed by `%f` — depends on
/// the left-to-right evaluation order of the C expression.
#[test]
fn mixed_sign_nans_select_the_surviving_nan() {
    let nans = ["nan", "-nan"];
    let others = ["1", "-1", "0", "-0", "inf", "-inf", "1e30", "1e-40"];

    // Every arrangement of two NaNs plus one ordinary value.
    for a in nans {
        for b in nans {
            for o in others {
                same(&[a, b, o]);
                same(&[a, o, b]);
                same(&[o, a, b]);
            }
        }
    }
    // Three NaNs, all sign combinations.
    for a in nans {
        for b in nans {
            for c in nans {
                same(&[a, b, c]);
            }
        }
    }
}

/// `inf * 0` and `inf - inf` are invalid operations; on x86 they produce the
/// "QNaN indefinite", whose sign bit is set, so `%f` prints `-nan`.
#[test]
fn invalid_operations_produce_negative_nan() {
    same(&["inf", "0", "0"]);
    same(&["-inf", "0", "0"]);
    same(&["inf", "-inf", "0"]);
    same(&["inf", "-inf", "1"]);
    same(&["0", "inf", "0"]);
    same(&["0", "0", "-inf"]);
}

/// NaN payloads and signs combined with infinities.
#[test]
fn nans_mixed_with_infinities() {
    let pool = ["nan", "-nan", "inf", "-inf", "0", "-0", "1"];
    for a in pool {
        for b in pool {
            for c in pool {
                same(&[a, b, c]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// atof / strtod: every parsing branch reachable from argv
// ---------------------------------------------------------------------------

/// `atof` performs no error reporting: an unparsable argument silently becomes
/// `0.0` and the program still exits 0.
#[test]
fn unparsable_arguments_become_zero() {
    // Note: a NUL byte cannot appear inside an `argv` entry at all, since the
    // kernel passes `argv` as NUL-terminated strings, so it is not an input
    // either program can observe.
    for t in [
        "", " ", "\t", "\n", "abc", "xyz", "+", "-", ".", "-.", "+.", "e", "E", "e5", "E5", ".e5",
        "--1", "+-1", "- 1", "1 5", "hello world", "/", "?", ",", "1,5", "(", ")",
    ] {
        same_in_each_slot(t);
    }
    same(&["abc", "def", "ghi"]);
    same(&["", "", ""]);
}

/// Leading whitespace is skipped; a trailing tail simply ends the conversion.
#[test]
fn leading_whitespace_and_trailing_garbage() {
    same_in_each_slot(" 5");
    same_in_each_slot("\t6");
    same_in_each_slot("\n7");
    same_in_each_slot("\u{b}8");
    same_in_each_slot("\u{c}9");
    same_in_each_slot("\r10");
    same_in_each_slot(" \t\n\u{b}\u{c}\r 1.5");
    same_in_each_slot("2.5 ");
    same_in_each_slot("2.5abc");
    same_in_each_slot("2.5e");
    same_in_each_slot("2.5e+");
    same_in_each_slot("2.5e-");
    same_in_each_slot("2.5ex");
    same_in_each_slot("1.2.3");
    same_in_each_slot("1e5x");
}

/// Signs, empty integer or fractional parts, and exponents.
#[test]
fn decimal_forms() {
    for t in [
        "0", "-0", "+0", "0.", ".0", "1.", ".5", "+.5", "-.5", "00", "007", "1e0", "1E0", "1e+0",
        "1e-0", "1e5", "1E5", "1e+5", "1e-5", "12345678901234567890", "0.000001", "1e22", "1e23",
        "0.1", "0.2", "0.3", "1.0000000000000002", "0.499999999999", "3.0000001",
    ] {
        same_in_each_slot(t);
    }
}

/// Hexadecimal floating point, including the `0x`-with-no-digits case where
/// `strtod` consumes only the leading `0`.
#[test]
fn hexadecimal_forms() {
    for t in [
        "0x", "0X", "-0x", "0x0", "0X0", "-0x0", "0x1", "-0x1", "0x10", "0xff", "0xFF", "0xabcdef",
        "0x1p1", "0x1P1", "0x1p+1", "0x1p-1", "0x1p", "0x1p+", "0x1p-", "0x1px", "0x.8", "0x.8p1",
        "0x8.", "0x8.p-1", "0x1.8p3", "0x.", "0x.p1", "0xzz", "0x1.fffffep+127",
        "0X1.FFFFFFFFFFFFFP+1023", "0x1p1024", "0x1p-1074", "0x1p-1075", "0x1p1000", "0x1p-1000",
        "0x1p10000", "0x1p-10000", "0x10000000000000000000", "0x1.0000000000000001p0",
    ] {
        same_in_each_slot(t);
    }
    same(&["0x10", "0x20", "0x30"]);
}

/// `inf` / `nan` spellings, including prefixes that stop short and therefore
/// parse as nothing at all.
#[test]
fn infinity_and_nan_spellings() {
    for t in [
        "inf", "INF", "Inf", "iNf", "+inf", "-inf", "infi", "infin", "infini", "infinit",
        "infinity", "INFINITY", "InFiNiTy", "infinityx", "-infinity", "in", "i", "nan", "NAN",
        "NaN", "nAn", "+nan", "-nan", "nanx", "nan(", "nan()", "nan(0)", "nan(1234)",
        "nan(0x7fffff)", "nan(abc_123)", "nan(ab!)", "-nan()", "-nan(1)", "na", "n",
    ] {
        same_in_each_slot(t);
    }
}

/// Values around `double`'s and `float`'s representable limits, where `strtod`
/// saturates or rounds and the assignment to `vec_t` narrows to `float`.
#[test]
fn range_limits_and_narrowing() {
    for t in [
        "1e308",
        "1e309",
        "-1e309",
        "1e400",
        "-1e400",
        "1e-400",
        "-1e-400",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "5e-324",
        "2.4703282292062327e-324",
        "4.9406564584124654e-324",
        "2.2250738585072011e-308",
        "2.2250738585072014e-308",
        "1e2147483647",
        "1e-2147483648",
        "1e99999999999999999999",
        "1e-99999999999999999999",
        "3.402823466e38",
        "3.402823467e38",
        "3.4028235e38",
        "3.4028236e38",
        "1.1754943508e-38",
    ] {
        same_in_each_slot(t);
    }
}

/// Very long digit strings, which take the slow paths in `strtod`.
#[test]
fn very_long_arguments() {
    let long_int = "1".repeat(1000);
    let long_nines = "9".repeat(1000);
    let long_frac = format!("0.{}1", "0".repeat(1000));
    let long_zeros_then_digits = format!("{}12345", "0".repeat(1000));
    let padded = format!("{}1.5", " ".repeat(1000));
    for t in [
        long_int.as_str(),
        long_nines.as_str(),
        long_frac.as_str(),
        long_zeros_then_digits.as_str(),
        padded.as_str(),
    ] {
        same_in_each_slot(t);
    }
}

/// Arguments that are not valid UTF-8: C sees raw bytes, so the Rust program
/// must read `argv` as bytes rather than as `String`.
#[test]
fn non_utf8_arguments() {
    for t in [
        &b"\xff\xfe"[..],
        b"\x80\x81\x82",
        b"1\xff",
        b"\xc3",
        b"\xed\xa0\x80",
        b"1.5\xff9",
        b"\xff1.5",
    ] {
        assert_same_args(&[t, b"1", b"1"]);
        assert_same_args(&[b"1", t, b"1"]);
        assert_same_args(&[b"1", b"1", t]);
    }
}

// ---------------------------------------------------------------------------
// printf("%f %f %f\n") formatting
// ---------------------------------------------------------------------------

/// `%f` uses six digits after the point, no exponent, and the values here are
/// chosen so that the seventh decimal digit is exactly 5 — an exact tie, where
/// glibc rounds half to even.
#[test]
fn percent_f_rounding_ties() {
    // Odd multiples of 1/128 are the only floats that tie at six decimals.
    for t in [
        "0x1p-7", "0x3p-7", "0x5p-7", "0x7p-7", "0x9p-7", "0x1.02p0", "0x1.06p0", "0x7fp-7",
        "-0x1p-7", "-0x3p-7", "-0x7fp-7",
    ] {
        same_in_each_slot(t);
    }
    // A spread of magnitudes, so the integer part is exercised too.
    for t in [
        "0.0078125", "0.0234375", "1.0078125", "255.9921875", "-0.0078125", "1000000.5",
        "12345678.5",
    ] {
        same_in_each_slot(t);
    }
}

/// A deterministic sweep of `float` bit patterns fed in as exact hex floats,
/// covering every exponent class in a single test.
#[test]
fn float_bit_pattern_sweep() {
    // xorshift32 keeps the sweep reproducible without pulling in a dependency.
    let mut state: u32 = 0x1234_5678;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };

    for _ in 0..150 {
        let mut args = Vec::new();
        for _ in 0..3 {
            let bits = next();
            let v = f32::from_bits(bits);
            args.push(if v.is_nan() {
                if v.is_sign_negative() {
                    "-nan".to_string()
                } else {
                    "nan".to_string()
                }
            } else if v.is_infinite() {
                if v < 0.0 { "-inf".into() } else { "inf".into() }
            } else {
                // An exact, round-trippable spelling of the float.
                format!("{:?}", f64::from(v))
            });
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        same(&refs);
    }
}

/// The same sweep restricted to exponents that keep `DotProduct` finite, so
/// the printed values are ordinary normalized components rather than
/// infinities.
#[test]
fn finite_normalized_results_sweep() {
    let mut state: u32 = 0x9e37_79b9;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };

    for _ in 0..150 {
        let mut args = Vec::new();
        for _ in 0..3 {
            // Exponent field 100..=140 keeps |v| within about 1e-9 .. 1e6.
            let exp = 100 + (next() % 41);
            let mant = next() & 0x007f_ffff;
            let sign = (next() & 1) << 31;
            let bits = sign | (exp << 23) | mant;
            args.push(format!("{:?}", f64::from(f32::from_bits(bits))));
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        same(&refs);
    }
}
