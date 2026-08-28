//! Differential tests: run the C reference program and the Rust port as
//! subprocesses with identical arguments and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Branch inventory of the C program (`c_src/src/main.c`, the inline helpers in
//! `c_src/inc/q_shared.h` and `Q_rsqrt` in `c_src/src/q_math.c`):
//!
//! * `main`:
//!   - `argc != 4`  -> `fprintf(stderr, "%s requires 4 inputs\n", argv[0]); exit(1);`
//!   - `argc == 4`  -> three `atof()` calls, `VectorNormalizeFast()`,
//!                     `printf("%f %f %f\n", ...)`, `return 0`.
//! * `atof()` (= `strtod`): leading whitespace, optional sign, the decimal
//!   form, the hexadecimal (`0x`) form, `inf`/`infinity`, `nan`/`nan(chars)`,
//!   an empty subject sequence (-> `0.0`), overflow (-> `HUGE_VAL`) and
//!   underflow (-> subnormal / zero). Each result is then narrowed to a C
//!   `float`, which can overflow to infinity or flush to (signed) zero.
//! * `DotProduct` + `Q_rsqrt`: single-precision `x*x + y*y + z*z` (which can
//!   overflow to `+inf` or underflow to `0`), the integer bit hack
//!   `0x5f3759df - (i >> 1)` (unsigned wrap-around), and one Newton step. A
//!   zero dot product yields the huge magic constant, an infinite one yields
//!   `-inf`, a NaN one propagates NaN.
//! * `printf("%f")`: six fractional digits, round-half-to-even at the tie,
//!   `-0.000000`, `inf`, `-inf`, `nan`, `-nan`, a single trailing newline.

mod common;

use common::{
    assert_same, assert_same3, assert_same_with_env, c_bin, run_stdout_to, run_with_arg0, rust_bin,
    Rng,
};

// ---------------------------------------------------------------------------
// argc branch: `argc != 4`
// ---------------------------------------------------------------------------

#[test]
fn usage_error_for_wrong_argument_counts() {
    // Every argument count except exactly three reaches the error path.
    assert_same(&[]);
    assert_same(&[b"1"]);
    assert_same(&[b"1", b"2"]);
    assert_same(&[b"1", b"2", b"3", b"4"]);
    assert_same(&[b"1", b"2", b"3", b"4", b"5"]);
    assert_same(&[b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10"]);
    // Empty arguments still count towards argc.
    assert_same(&[b""]);
    assert_same(&[b"", b""]);
    assert_same(&[b"", b"", b"", b""]);
}

#[test]
fn usage_error_prints_argv0_verbatim() {
    // `fprintf(stderr, "%s requires 4 inputs\n", argv[0])` echoes argv[0] as
    // raw bytes, whatever they are.
    for arg0 in [
        &b"driver"[..],
        b"",
        b"./some/path/driver",
        b"a b\tc",
        b"weird\xff\xfe-name",
        b"%s %f %d",
        b"very-long-argv0-very-long-argv0-very-long-argv0-very-long-argv0",
    ] {
        let c = run_with_arg0(c_bin(), arg0, &[b"1", b"2"]);
        let r = run_with_arg0(rust_bin(), arg0, &[b"1", b"2"]);
        assert_eq!(
            c.stderr,
            r.stderr,
            "stderr differs for argv[0] = {:?}\n C: {:?}\n R: {:?}",
            String::from_utf8_lossy(arg0),
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr)
        );
        assert_eq!(c.stdout, r.stdout);
        assert_eq!(c.code, r.code, "exit code differs for argv[0] {arg0:?}");
        assert_eq!(c.signal, r.signal);
        assert_eq!(c.code, Some(1), "the C program must exit(1) here");
    }
}

// ---------------------------------------------------------------------------
// Ordinary vectors
// ---------------------------------------------------------------------------

#[test]
fn plain_vectors() {
    let cases: &[[&str; 3]] = &[
        ["3", "4", "5"],
        ["1", "0", "0"],
        ["0", "1", "0"],
        ["0", "0", "1"],
        ["-1", "0", "0"],
        ["1", "1", "1"],
        ["-1", "-1", "-1"],
        ["1.5", "-2.25", "3.125"],
        ["0.1", "0.2", "0.3"],
        ["100", "200", "300"],
        ["1e-3", "2e-3", "-4e-3"],
        ["123456", "654321", "-999999"],
        ["3.14159265358979", "2.71828182845905", "1.41421356237309"],
        ["7", "0", "0"],
        ["0.0001", "0", "0"],
        ["1000000", "1000000", "1000000"],
    ];
    for [a, b, c] in cases {
        assert_same3(a, b, c);
    }
}

#[test]
fn zero_and_signed_zero_vectors() {
    // A zero dot product makes Q_rsqrt return the magic constant unchanged by
    // the Newton step, and the sign of each zero survives the multiplication.
    for a in ["0", "-0", "+0", "0.0", "-0.0", "0e10", "-0e-10"] {
        for b in ["0", "-0"] {
            for c in ["0", "-0"] {
                assert_same3(a, b, c);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Special values: infinities, NaNs (including sign and payload) and the
// magnitudes that overflow / underflow the float conversion or the dot product
// ---------------------------------------------------------------------------

#[test]
fn special_value_combinations() {
    const VALUES: &[&str] = &[
        "nan", "-nan", "nan(1)", "inf", "-inf", "0", "-0", "1", "-1", "1e40", "1e20", "1e-45",
    ];
    for a in VALUES {
        for b in VALUES {
            for c in VALUES {
                assert_same3(a, b, c);
            }
        }
    }
}

#[test]
fn nan_payloads_and_spellings() {
    for v in [
        "nan",
        "NAN",
        "NaN",
        "-nan",
        "+nan",
        "nan()",
        "nan(0)",
        "nan(1)",
        "-nan(1)",
        "nan(0x7ffff)",
        "nan(0x7ffffffffffff)",
        "nan(4194304)",
        "nan(123456789012345678901234567890)",
        "nan(_abc123)",
        "nan(abc",
        "nan(",
        "nanx",
        "na",
    ] {
        assert_same3(v, "1", "2");
        assert_same3("1", v, "2");
        assert_same3("2", "1", v);
        assert_same3(v, v, v);
    }
}

#[test]
fn infinity_spellings() {
    for v in [
        "inf",
        "INF",
        "Inf",
        "-inf",
        "+inf",
        "infinity",
        "INFINITY",
        "Infinity",
        "-infinity",
        "infinityx",
        "infx",
        "in",
        "i",
    ] {
        assert_same3(v, "1", "2");
        assert_same3("1", v, "2");
        assert_same3("1", "2", v);
        assert_same3(v, v, v);
    }
}

#[test]
fn magnitudes_that_overflow_or_underflow() {
    let cases: &[[&str; 3]] = &[
        // double -> float overflow to +/-inf
        ["1e39", "0", "0"],
        ["-1e39", "0", "0"],
        ["3.4028235e38", "0", "0"],   // largest finite float
        ["3.4028236e38", "0", "0"],   // rounds to inf
        ["3.402823466e38", "1", "1"], // just below the boundary
        ["1e308", "1e308", "1e308"],
        ["1e309", "0", "0"], // already inf as a double
        // dot product overflows to +inf even though the components are finite
        ["1e20", "1e20", "1e20"],
        ["1e19", "1e19", "1e19"],
        ["2e19", "0", "0"],
        ["1.9e19", "0", "0"],
        ["-2e19", "3e19", "0"],
        // subnormal floats, and dot products that underflow to zero
        ["1e-45", "1e-45", "1e-45"],
        ["1.4e-45", "0", "0"], // smallest positive subnormal float
        ["7e-46", "0", "0"],   // rounds to the subnormal or to zero
        ["1e-46", "0", "0"],   // flushes to zero
        ["1.17549435e-38", "0", "0"], // smallest normal float
        ["1.1754942e-38", "0", "0"],  // largest subnormal float
        ["1e-20", "1e-20", "1e-20"],
        ["1e-25", "0", "0"],
        ["1e-30", "1e-30", "0"],
        ["1e-400", "0", "0"], // underflows to zero already as a double
        ["-1e-400", "0", "0"],
        // mixed extremes
        ["1e30", "1e-30", "0"],
        ["1e38", "1e-38", "1"],
        ["1e20", "1", "1e-20"],
    ];
    for [a, b, c] in cases {
        assert_same3(a, b, c);
    }
}

// ---------------------------------------------------------------------------
// atof() / strtod() subject-sequence forms
// ---------------------------------------------------------------------------

/// Every string here is fed in each of the three argument positions, and as all
/// three arguments at once.
const ATOF_FORMS: &[&str] = &[
    // nothing convertible -> 0.0
    "",
    " ",
    "\t",
    "\n",
    "\r",
    "\x0b",
    "\x0c",
    "abc",
    "+",
    "-",
    ".",
    "-.",
    "+.",
    "e5",
    "E-5",
    ",5",
    "1,5",
    "--1",
    "++1",
    " - 5",
    "x1",
    "d5",
    // leading whitespace is skipped
    "   1.5",
    "\t\t-2.5",
    "\n\r\x0b\x0c 3.5",
    " +4.5",
    // signs
    "+1.5",
    "-1.5",
    "+.5",
    "-.5",
    // decimal forms
    "1",
    "1.",
    ".5",
    "5.",
    "5.e2",
    "0.5",
    "00000000000000000001.5",
    "1.5000000000",
    // exponents, complete and incomplete
    "1e",
    "1e+",
    "1e-",
    "1e5",
    "1E5",
    "1e05",
    "1e+5",
    "1e-5",
    "1.5e2",
    "1.5E-2",
    "1e00000000000000000000005",
    "1e2147483647",
    "1e2147483648",
    "1e-2147483648",
    "1e99999999999999999999",
    "1e-99999999999999999999",
    // trailing garbage stops the conversion but keeps the prefix
    "1.5abc",
    "1.5e",
    "1.5e+x",
    "2.5xyz",
    "3junk",
    "0.5%",
    // long digit strings
    "999999999999999999999999999999999999999999999999999",
    "0.000000000000000000000000000000000000000000000000001",
    "12345678901234567890.12345678901234567890",
];

#[test]
fn atof_decimal_and_garbage_forms() {
    for s in ATOF_FORMS {
        let b = s.as_bytes();
        assert_same(&[b, b"1", b"2"]);
        assert_same(&[b"1", b, b"2"]);
        assert_same(&[b"1", b"2", b]);
        assert_same(&[b, b, b]);
    }
}

#[test]
fn atof_hexadecimal_forms() {
    const HEX: &[&str] = &[
        "0x",
        "0X",
        "-0x",
        "0x0",
        "0x1",
        "0X1",
        "0x10",
        "0xff",
        "0xFF",
        "0x1p",
        "0x1p+",
        "0x1p-",
        "0x1p0",
        "0x1p3",
        "0X1P-3",
        "0x.8p1",
        "0x8.p-1",
        "0x1.8p-2",
        "0x1.fffffep127",  // largest finite float
        "0x1.ffffffp127",  // rounds to inf as a float
        "0x1p128",         // inf as a float
        "0x1p-149",        // smallest subnormal float
        "0x1p-150",        // ties to zero
        "0x1.8p-150",      // rounds up to the subnormal
        "0x1p1024",        // inf already as a double
        "0x1p-1074",       // smallest subnormal double
        "0x1p-1075",       // ties to zero as a double
        "0x1.8p-1074",
        "0x1.fffffffffffffp1023",
        "0x1.fffffffffffff8p1023",
        "0x1.00000000000008p0",
        "0x1.0000000000000fp0",
        "0x10000000000000080000000000000000p0",
        "0x1p3abc",
        "0xzz",
        "0x.p1",
        "-0x1.8p1",
        "+0x2p2",
        "  0x1.4p3",
    ];
    for s in HEX {
        let b = s.as_bytes();
        assert_same(&[b, b"1", b"2"]);
        assert_same(&[b"1", b, b"2"]);
        assert_same(&[b"1", b"2", b]);
        assert_same(&[b, b, b]);
    }
}

#[test]
fn atof_rounding_boundaries() {
    // Values that force correct rounding in strtod and/or in the narrowing to
    // float (halfway cases, subnormal boundaries, glibc's classic hard cases).
    const CASES: &[&str] = &[
        "2.2250738585072011e-308",
        "2.2250738585072012e-308",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "4.9e-324",
        "2.4703282292062327e-324",
        "2.4703282292062328e-324",
        "1.0000000596046447753906250",  // exactly halfway between two floats
        "1.00000005960464477539062501", // just above that
        "1.00000005960464477539062499", // just below that
        "0.5000000000000000277555756156289135105907917022705078125",
        "16777215",  // 2^24 - 1, exactly representable
        "16777216",  // 2^24
        "16777217",  // rounds to 16777216
        "16777219",  // rounds to 16777220
        "-16777217", // same, negative
        "8388608.5",
        "0.1",
        "0.2",
        "0.3",
        "1e-7",
        "1e-8",
    ];
    for s in CASES {
        assert_same3(s, "1", "2");
        assert_same3(s, s, s);
    }
}

#[test]
fn non_utf8_arguments() {
    // argv is arbitrary bytes; C's atof() just stops at the first byte it
    // cannot use.
    let cases: &[&[u8]] = &[
        b"\xff",
        b"\xff1.5",
        b"1.5\xff",
        b"1.\xff5",
        b"\x801",
        b"\xc3\x28",
        b"2\xe2\x82\xac",
        b"\x00" as &[u8], // a real NUL cannot appear in argv, but 0x00-free
        b"\x01\x02\x03",
        b"-\xff2",
        b"\xef\xbb\xbf1.5", // UTF-8 BOM in front of a number
    ];
    for c in cases {
        // `\x00` is not allowed in an argument; skip that single byte string by
        // replacing it with something equivalent and still non-UTF8.
        let arg: &[u8] = if *c == b"\x00" { b"\xfe" } else { c };
        assert_same(&[arg, b"1", b"2"]);
        assert_same(&[b"1", arg, b"2"]);
        assert_same(&[b"1", b"2", arg]);
        assert_same(&[arg, arg, arg]);
    }
}

#[test]
fn very_long_arguments() {
    let long_digits = "9".repeat(100_000);
    let long_zeros = format!("0.{}123456", "0".repeat(100_000));
    let long_hex = format!("0x{}p-3", "a".repeat(50_000));
    let long_garbage = "z".repeat(100_000);
    let long_with_prefix = format!("1.5{}", "x".repeat(100_000));
    let long_exp = format!("1e{}", "9".repeat(10_000));

    for s in [
        &long_digits,
        &long_zeros,
        &long_hex,
        &long_garbage,
        &long_with_prefix,
        &long_exp,
    ] {
        assert_same(&[s.as_bytes(), b"1", b"2"]);
    }
}

// ---------------------------------------------------------------------------
// printf("%f") formatting
// ---------------------------------------------------------------------------

#[test]
fn printf_rounding_ties() {
    // Inputs whose normalized components land exactly on an odd multiple of
    // 2^-7, i.e. exactly halfway between two 6-decimal outputs. glibc rounds
    // half to even there.
    let cases: &[[&str; 3]] = &[
        [
            "4.8422939461560759",
            "0.81374671068945759",
            "-7.5198306388523033",
        ],
        [
            "6.2208943911988399",
            "-8.5829116659679254",
            "0.2225272214902887",
        ],
        [
            "4.9303453465485703",
            "-6.1052787186584201",
            "4.8243625082255992",
        ],
        [
            "1.6907954601629154",
            "-2.1109307959691641",
            "-8.2235348304049865",
        ],
        [
            "4.8112731991258748",
            "-8.1469413622620053",
            "0.27708575369314303",
        ],
        [
            "7.2852769773324475",
            "-7.709444416495483",
            "9.8307597428891178",
        ],
    ];
    for [a, b, c] in cases {
        assert_same3(a, b, c);
    }
}

#[test]
fn locale_does_not_change_formatting() {
    // The C program never calls setlocale(), so it must keep using '.' as the
    // decimal separator no matter what the environment says.
    for locale in ["C", "de_DE.UTF-8", "fr_FR.UTF-8", "en_US.UTF-8", "POSIX"] {
        assert_same_with_env(
            &[b"1.5", b"-2.25", b"3"],
            &[("LC_ALL", locale), ("LANG", locale), ("LC_NUMERIC", locale)],
        );
        assert_same_with_env(&[b"1,5", b"2", b"3"], &[("LC_ALL", locale)]);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn failing_stdout_write_is_ignored_identically() {
    // Writing to /dev/full always fails with ENOSPC; neither program reports it
    // and both still exit successfully.
    let args: &[&[u8]] = &[b"1", b"2", b"3"];
    let c = run_stdout_to(c_bin(), args, "/dev/full");
    let r = run_stdout_to(rust_bin(), args, "/dev/full");
    assert_eq!(c.stderr, r.stderr, "stderr differs when stdout fails");
    assert_eq!(c.code, r.code, "exit code differs when stdout fails");
    assert_eq!(c.signal, r.signal);

    // /dev/null accepts everything, so this is just the mirror case.
    let c = run_stdout_to(c_bin(), args, "/dev/null");
    let r = run_stdout_to(rust_bin(), args, "/dev/null");
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.code, r.code);
}

// ---------------------------------------------------------------------------
// Boundaries inside DotProduct / Q_rsqrt
// ---------------------------------------------------------------------------

/// Print a float with enough digits that `strtod` reproduces it exactly.
fn exact(f: f32) -> String {
    format!("{:.9e}", f)
}

#[test]
fn dot_product_overflow_boundary() {
    // Around sqrt(FLT_MAX) ~= 1.8446743e19 the dot product flips from a finite
    // value to +inf, which makes Q_rsqrt return -inf and turns the output into
    // "-inf" / "-nan" (0 * -inf).
    let base = 1.8446743e19f32.to_bits();
    for d in -24i32..=24 {
        let f = f32::from_bits(base.wrapping_add(d as u32));
        let s = exact(f);
        assert_same3(&s, "0", "0");
        assert_same3(&s, &s, "0");
        let neg = exact(-f);
        assert_same3(&neg, "0", "0");
    }
}

#[test]
fn dot_product_underflow_boundary() {
    // Around sqrt(FLT_TRUE_MIN) ~= 3.74e-23 the squares stop being
    // representable: the dot product becomes a subnormal and then exactly zero,
    // in which case Q_rsqrt returns the raw magic constant (~1.98e19) and the
    // components are scaled by it instead of being normalized.
    let base = 3.74e-23f32.to_bits();
    for d in -32i32..=32 {
        let f = f32::from_bits(base.wrapping_add(d as u32));
        let s = exact(f);
        assert_same3(&s, "0", "0");
        assert_same3(&s, &s, &s);
    }
    // A tiny component next to a normal one, and fully subnormal vectors.
    for e in -45i32..=-30 {
        let a = format!("1e{e}");
        let b = format!("1e{}", e + 1);
        let c = format!("1e{}", e + 2);
        assert_same3(&a, "1", "0");
        assert_same3(&a, &b, &c);
    }
}

#[test]
fn q_rsqrt_bit_hack_wraparound() {
    // `0x5f3759df - (i >> 1)` wraps around (unsigned) whenever the sign bit of
    // `number` is set, which only happens for a negative NaN dot product.
    for v in ["-nan", "-nan(1)", "-nan(0x7ffff)"] {
        assert_same3(v, "0", "0");
        assert_same3("1", v, "1e30");
        assert_same3(v, v, v);
    }
}

#[test]
fn flag_like_and_option_arguments() {
    // The program has no option parsing at all: these are just unparsable
    // numbers that atof() turns into 0.0.
    for a in ["-h", "--help", "-", "--", "-v", "/dev/stdin", "@file"] {
        assert_same3(a, "1", "2");
        assert_same3("1", a, "2");
        assert_same3(a, a, a);
    }
    // ... and with the wrong argument count they hit the usage error instead.
    assert_same(&[b"--help"]);
    assert_same(&[b"-h", b"1", b"2", b"3"]);
}

#[test]
fn many_arguments() {
    let args: Vec<&[u8]> = (0..200).map(|_| &b"1"[..]).collect();
    assert_same(&args);
}

// ---------------------------------------------------------------------------
// Randomized differential fuzzing (deterministic seeds)
// ---------------------------------------------------------------------------

#[test]
fn fuzz_exact_float_bit_patterns() {
    // Random 32-bit patterns printed with enough digits to round-trip exactly,
    // so both programs see the very same float, including NaNs, infinities and
    // subnormals.
    let mut rng = Rng::new(0x1234_5678_9abc_def1);
    for _ in 0..300 {
        let mut args = Vec::new();
        for _ in 0..3 {
            let f = f32::from_bits(rng.next_u32());
            args.push(if f.is_nan() {
                if f.is_sign_negative() {
                    "-nan".to_string()
                } else {
                    "nan".to_string()
                }
            } else if f.is_infinite() {
                if f.is_sign_negative() {
                    "-inf".to_string()
                } else {
                    "inf".to_string()
                }
            } else {
                format!("{:.9e}", f)
            });
        }
        assert_same3(&args[0], &args[1], &args[2]);
    }
}

#[test]
fn fuzz_random_magnitudes() {
    let mut rng = Rng::new(0x0bad_c0ff_ee00_1234);
    for _ in 0..300 {
        let mut args = Vec::new();
        for _ in 0..3 {
            // Random mantissa with an exponent spread across the whole float
            // range so dot products overflow, underflow and stay normal.
            let mantissa = (rng.next_u32() as f64) / (u32::MAX as f64) * 2.0 - 1.0;
            let exp = rng.below(90) as i32 - 45;
            args.push(format!("{}e{}", mantissa, exp));
        }
        assert_same3(&args[0], &args[1], &args[2]);
    }
}

#[test]
fn fuzz_random_strings() {
    // Random garbage from the alphabet that strtod() actually reacts to, to
    // exercise the parser's partial-match and rejection paths.
    const ALPHABET: &[u8] = b"0123456789.eE+-xXpPabcdefABCDEFnNiIfFtTyY_(), \t";
    let mut rng = Rng::new(0xfeed_face_dead_beef);
    for _ in 0..400 {
        let mut args: Vec<Vec<u8>> = Vec::new();
        for _ in 0..3 {
            let len = rng.below(14) as usize;
            let s: Vec<u8> = (0..len)
                .map(|_| ALPHABET[rng.below(ALPHABET.len() as u32) as usize])
                .collect();
            args.push(s);
        }
        assert_same(&[&args[0], &args[1], &args[2]]);
    }
}

#[test]
fn fuzz_structured_numbers() {
    let mut rng = Rng::new(0x5151_5151_2727_2727);
    for _ in 0..300 {
        let mut args: Vec<String> = Vec::new();
        for _ in 0..3 {
            let s = match rng.below(6) {
                0 => {
                    // decimal with random digit counts and exponent
                    let ip: String = (0..1 + rng.below(20))
                        .map(|_| (b'0' + rng.below(10) as u8) as char)
                        .collect();
                    let fp: String = (0..rng.below(20))
                        .map(|_| (b'0' + rng.below(10) as u8) as char)
                        .collect();
                    let sign = ["", "+", "-"][rng.below(3) as usize];
                    format!("{sign}{ip}.{fp}e{}{}", ["", "+", "-"][rng.below(3) as usize], rng.below(60))
                }
                1 => {
                    // hex float
                    let hd: String = (0..1 + rng.below(20))
                        .map(|_| b"0123456789abcdef"[rng.below(16) as usize] as char)
                        .collect();
                    let fd: String = (0..rng.below(20))
                        .map(|_| b"0123456789abcdef"[rng.below(16) as usize] as char)
                        .collect();
                    format!(
                        "{}0x{hd}.{fd}p{}{}",
                        ["", "-"][rng.below(2) as usize],
                        ["", "+", "-"][rng.below(3) as usize],
                        rng.below(200)
                    )
                }
                2 => format!("0.{}{}", "0".repeat(rng.below(50) as usize), 1 + rng.below(999999)),
                3 => format!("{}{}", 1 + rng.below(999999), "0".repeat(rng.below(50) as usize)),
                4 => format!("{}e-{}", 1 + rng.below(999999), rng.below(60)),
                _ => format!("{}", (rng.next_u32() as i32 as f64) / 1024.0),
            };
            args.push(s);
        }
        assert_same3(&args[0], &args[1], &args[2]);
    }
}
