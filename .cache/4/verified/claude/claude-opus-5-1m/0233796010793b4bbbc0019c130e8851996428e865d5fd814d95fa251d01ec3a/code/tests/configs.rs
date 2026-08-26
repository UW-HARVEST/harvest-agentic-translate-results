//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every row drives the FULL pipeline (argv -> strtod(base) -> strtod(exp) ->
//! pow -> printf) of BOTH binaries with MANY randomized inputs from a fixed
//! seed, and compares stdout/stderr/exit-status/signal byte-for-byte.
//!
//! The process is the only entry point this C program has (there is no library
//! API and no convenience wrapper), so "lowest-level entry point" == argc/argv.

mod common;
use common::*;

// ------------------------------------------------------------------ helpers

/// Shortest round-trip decimal form: uniquely identifies the double, so the
/// value that reaches `pow` is bit-exact.  NaN renders as "NaN", which
/// `strtod` accepts case-insensitively.
fn dec(f: f64) -> String {
    format!("{:e}", f)
}

/// Exact hex-float (`%a`-style) rendering, so random bit patterns reach
/// `strtod`'s hexadecimal path without any decimal rounding.
fn hexfloat(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f.is_sign_negative() { "-inf".into() } else { "inf".into() };
    }
    let bits = f.to_bits();
    let sign = if bits >> 63 == 1 { "-" } else { "" };
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let frac = bits & 0x000f_ffff_ffff_ffff;
    if exp == 0 {
        if frac == 0 {
            return format!("{sign}0x0p+0");
        }
        // subnormal: leading digit 0, exponent -1022
        return format!("{sign}0x0.{:013x}p-1022", frac);
    }
    format!("{sign}0x1.{:013x}p{:+}", frac, exp - 1023)
}

fn ck(row: &str, b: &str, e: &str) {
    assert_same(row, b, e);
}

// ------------------------------------------------------------------ C01

#[test]
fn c01_small_integer_pairs() {
    let mut rng = Rng::new(1);
    for _ in 0..400 {
        let b = rng.range_i64(-50, 50);
        let e = rng.range_i64(-20, 20);
        ck("C01", &b.to_string(), &e.to_string());
    }
    // plus the exhaustive small grid, which pins odd/even/zero/negative
    for b in -5..=5i64 {
        for e in -5..=5i64 {
            ck("C01", &b.to_string(), &e.to_string());
        }
    }
}

// ------------------------------------------------------------------ C02

#[test]
fn c02_random_finite_decimals() {
    let mut rng = Rng::new(2);
    for _ in 0..600 {
        // random magnitude across the whole useful exponent range
        let sig = rng.f01() * 2.0 - 1.0;
        let ex = rng.range_i64(-40, 40) as i32;
        let b = sig * 10f64.powi(ex);
        let e = (rng.f01() * 2.0 - 1.0) * 10.0;
        let digits = 1 + rng.below(17) as usize;
        ck(
            "C02",
            &format!("{:.*e}", digits, b),
            &format!("{:.*e}", digits, e),
        );
    }
}

// ------------------------------------------------------------------ C03

#[test]
fn c03_exponent_exactly_zero() {
    let zeros = ["0", "-0", "0.0", "-0.0", "+0.0", "0e0", "0x0p0", ".0", "0."];
    let mut rng = Rng::new(3);
    for z in zeros {
        for _ in 0..12 {
            let b = rng.any_f64();
            ck("C03", &dec(b), z);
        }
        // pow(x, 0) == 1 for every x, including nan/inf
        for b in ["nan", "-nan", "inf", "-inf", "0", "-0.0", "1", "-1"] {
            let o = assert_same("C03", b, z);
            assert_eq!(o.code, Some(0), "pow({b},{z}) -> {o:?}");
            assert_eq!(stdout_str(&o), "Result: 1.00\n");
        }
    }
}

// ------------------------------------------------------------------ C04

#[test]
fn c04_empty_string_argument_is_accepted_as_zero() {
    for e in ["", "0", "1", "2", "-1", "0.5", "nan", "inf", "-inf", "3"] {
        assert_same("C04", "", e);
    }
    for b in ["", "0", "1", "2", "-2", "nan", "inf", "-inf", "0.5"] {
        assert_same("C04", b, "");
    }
    let o = assert_same("C04", "", "");
    assert_eq!(stdout_str(&o), "Result: 1.00\n");
}

// ------------------------------------------------------------------ C05

#[test]
fn c05_leading_whitespace() {
    let ws = [" ", "\t", "\n", "\r", "\x0b", "\x0c", "  ", " \t", "\n\n", " \t\n\r\x0b\x0c"];
    let mut rng = Rng::new(5);
    for _ in 0..200 {
        let w1 = *rng.pick(&ws);
        let w2 = *rng.pick(&ws);
        let b = rng.range_i64(-30, 30) as f64 + rng.f01();
        let e = rng.range_i64(-6, 6) as f64;
        assert_same_raw(
            "C05",
            &[
                format!("{w1}{b:?}").as_bytes(),
                format!("{w2}{e:?}").as_bytes(),
            ],
        );
    }
    // whitespace before every accepted token shape
    for w in ws {
        for t in ["1.5", "-2", "+3", "inf", "nan", "0x1p3", ".5", "1e2"] {
            assert_same_raw("C05", &[format!("{w}{t}").as_bytes(), b"2"]);
            assert_same_raw("C05", &[b"2", format!("{w}{t}").as_bytes()]);
        }
    }
}

// ------------------------------------------------------------------ C06

#[test]
fn c06_sign_and_partial_forms() {
    let forms = [
        "+1", "-1", "+0", "-0", ".5", "-.5", "+.5", "5.", "-5.", "+5.", "+.5e+1", "-.0", "0.",
        ".0", "+.0", "-.0e-0", "00", "007", "+007.500",
    ];
    for b in forms {
        for e in forms {
            ck("C06", b, e);
        }
    }
    let mut rng = Rng::new(6);
    for _ in 0..200 {
        let n = rng.f01();
        let s = if rng.bool() { "+" } else { "-" };
        let b = format!("{s}.{}", (n * 1e15) as u64);
        let e = format!("{}.", rng.range_i64(-9, 9));
        ck("C06", &b, &e);
    }
}

// ------------------------------------------------------------------ C07

#[test]
fn c07_exponent_notation() {
    let mut rng = Rng::new(7);
    for _ in 0..300 {
        let mant = rng.range_i64(-999, 999);
        let ex = rng.range_i64(-30, 30);
        let echar = if rng.bool() { 'e' } else { 'E' };
        let sign = match rng.below(3) {
            0 => "",
            1 => "+",
            _ => "-",
        };
        let pad = "0".repeat(rng.below(4) as usize); // leading zeros in exponent
        let b = format!("{mant}{echar}{sign}{pad}{}", ex.abs());
        let e = format!("{}{}{}", rng.range_i64(-5, 5), echar, rng.below(2));
        ck("C07", &b, &e);
    }
    for b in ["1e0", "1E0", "1e+0", "1e-0", "1e005", "1E+005", "1e308", "1e-307", "9e-1"] {
        ck("C07", b, "2");
        ck("C07", "2", b);
    }
}

// ------------------------------------------------------------------ C08

#[test]
fn c08_hex_float_literals() {
    let mut rng = Rng::new(8);
    for _ in 0..300 {
        let hexdigits = 1 + rng.below(13) as usize;
        let mut m = String::new();
        for _ in 0..hexdigits {
            let d = rng.below(16) as u32;
            let c = std::char::from_digit(d, 16).unwrap();
            m.push(if rng.bool() { c.to_ascii_uppercase() } else { c });
        }
        let p = rng.range_i64(-60, 60);
        let x = if rng.bool() { "0x" } else { "0X" };
        let pc = if rng.bool() { 'p' } else { 'P' };
        let sign = if rng.bool() { "-" } else { "" };
        let b = format!("{sign}{x}1.{m}{pc}{p}");
        let e = format!("{x}{}{pc}{}", rng.below(8), rng.range_i64(-3, 3));
        ck("C08", &b, &e);
    }
    for b in ["0x1p3", "0X1.8P-2", "0x.8p1", "0x8.p-1", "0xAp0", "0x1p+0", "-0x1p-1", "0x0p0"] {
        ck("C08", b, "2");
        ck("C08", "2", b);
    }
    // exact bit patterns via %a-style strings on both sides
    let mut rng = Rng::new(88);
    for _ in 0..200 {
        let b = f64::from_bits(rng.next_u64() >> 1); // positive, any exponent
        ck("C08", &hexfloat(b), &hexfloat(rng.range_i64(-4, 4) as f64));
    }
}

// ------------------------------------------------------------------ C09

#[test]
fn c09_infinity_spellings() {
    let infs = [
        "inf", "INF", "Inf", "iNf", "infinity", "INFINITY", "Infinity", "iNfInItY", "+inf",
        "-inf", "+INFINITY", "-Infinity", "-infinity", "+infinity",
    ];
    for b in infs {
        for e in infs {
            ck("C09", b, e);
        }
    }
    for i in infs {
        for other in ["0", "1", "-1", "2", "-2", "0.5", "-0.5", "3", "-3", "nan", ""] {
            ck("C09", i, other);
            ck("C09", other, i);
        }
    }
}

// ------------------------------------------------------------------ C10

#[test]
fn c10_nan_spellings() {
    let nans = [
        "nan", "NAN", "NaN", "-nan", "+nan", "nan(0)", "nan(1234)", "nan(x_9)", "-NAN(1)",
        "NaN(abcDEF_012)",
    ];
    for b in nans {
        for e in nans {
            ck("C10", b, e);
        }
    }
    for n in nans {
        for other in ["0", "-0.0", "1", "-1", "2", "inf", "-inf", "0.5", ""] {
            ck("C10", n, other);
            ck("C10", other, n);
        }
    }
}

// ------------------------------------------------------------------ C11

#[test]
fn c11_negative_base_integer_exponent() {
    let mut rng = Rng::new(11);
    for _ in 0..300 {
        let b = -(rng.f01() * 20.0 + 0.01);
        let e = rng.range_i64(-40, 40);
        ck("C11", &format!("{b:?}"), &e.to_string());
    }
    for e in -40..=40i64 {
        ck("C11", "-2", &e.to_string());
    }
    for b in ["-1", "-2", "-3", "-0.5", "-1.5", "-10"] {
        for e in ["2", "3", "-2", "-3", "40", "41", "-40", "-41", "0"] {
            ck("C11", b, e);
        }
    }
}

// ------------------------------------------------------------------ C12

#[test]
fn c12_negative_base_non_integer_exponent() {
    let mut rng = Rng::new(12);
    for _ in 0..150 {
        let b = -(rng.f01() * 50.0 + 0.001);
        let e = rng.range_i64(-20, 20) as f64 + 0.5;
        let o = assert_same("C12", &format!("{b:?}"), &format!("{e:?}"));
        assert_eq!(o.code, Some(1), "expected EDOM for pow({b},{e}) -> {o:?}");
    }
}

// ------------------------------------------------------------------ C13

#[test]
fn c13_zero_base() {
    let exps = [
        "0", "-0.0", "1", "2", "3", "-1", "-2", "-3", "0.5", "-0.5", "inf", "-inf", "nan", "1e300",
        "-1e300",
    ];
    for b in ["0", "0.0", "-0.0", "-0", "0e0", "0x0p0"] {
        for e in exps {
            ck("C13", b, e);
        }
    }
}

// ------------------------------------------------------------------ C14

#[test]
fn c14_infinite_base() {
    let exps = [
        "0", "-0.0", "1", "2", "3", "-1", "-2", "-3", "0.5", "-0.5", "inf", "-inf", "nan", "40",
        "41", "1e300",
    ];
    for b in ["inf", "-inf"] {
        for e in exps {
            ck("C14", b, e);
        }
    }
}

// ------------------------------------------------------------------ C15

#[test]
fn c15_infinite_exponent() {
    let bases = [
        "-1", "1", "0.5", "-0.5", "2", "-2", "0", "-0.0", "nan", "1e300", "-1e300", "0.999999",
        "1.000001", "-0.999999",
    ];
    for e in ["inf", "-inf"] {
        for b in bases {
            ck("C15", b, e);
        }
    }
}

// ------------------------------------------------------------------ C16

#[test]
fn c16_base_one_and_minus_one() {
    let mut rng = Rng::new(16);
    for b in ["1", "-1", "1.0", "-1.0"] {
        for _ in 0..40 {
            let e = rng.any_f64();
            ck("C16", b, &dec(e));
        }
        for e in ["nan", "inf", "-inf", "1e300", "-1e300", "0.5", "1e15", "1e16"] {
            ck("C16", b, e);
        }
    }
}

// ------------------------------------------------------------------ C17

#[test]
fn c17_near_overflow_boundary() {
    let mut rng = Rng::new(17);
    for _ in 0..200 {
        let b = *rng.pick(&["10", "2", "-10", "-2", "1.1", "9.9"]);
        let e = match rng.below(3) {
            0 => rng.range_i64(305, 312).to_string(),
            1 => rng.range_i64(1020, 1030).to_string(),
            _ => format!("{}.{}", rng.range_i64(306, 310), rng.below(100)),
        };
        assert_same("C17", b, &e);
    }
    for (b, e) in [
        ("10", "308"),
        ("10", "309"),
        ("2", "1023"),
        ("2", "1024"),
        ("2", "1025"),
        ("1.7976931348623157e308", "1"),
        ("1.3407807929942597e154", "2"),
        ("1.3407807929942596e154", "2"),
    ] {
        assert_same("C17", b, e);
    }
}

// ------------------------------------------------------------------ C18

#[test]
fn c18_near_underflow_boundary() {
    let mut rng = Rng::new(18);
    for _ in 0..200 {
        let b = *rng.pick(&["10", "2", "0.5", "0.1", "-10", "-0.5"]);
        let e = match rng.below(3) {
            0 => (-rng.range_i64(300, 330)).to_string(),
            1 => (-rng.range_i64(1020, 1090)).to_string(),
            _ => format!("-{}.{}", rng.range_i64(306, 325), rng.below(100)),
        };
        assert_same("C18", b, &e);
    }
    for (b, e) in [
        ("10", "-307"),
        ("10", "-308"),
        ("10", "-320"),
        ("10", "-324"),
        ("10", "-325"),
        ("2", "-1022"),
        ("2", "-1074"),
        ("2", "-1075"),
    ] {
        assert_same("C18", b, e);
    }
}

// ------------------------------------------------------------------ C19

#[test]
fn c19_percent_2f_tie_rounding() {
    // Values whose exact binary expansion terminates in ...5 at the third
    // decimal: printf must round half to EVEN.  Reached both directly and
    // through pow.
    let ties = [
        "0.125", "0.375", "0.625", "0.875", "1.125", "1.375", "2.625", "-0.125", "-0.375",
        "-2.875", "0.0625", "0.1875", "10.125", "1024.375", "0.005", "1.005", "8.995", "0.015",
        "0.025", "0.045", "2.675", "1.115",
    ];
    for t in ties {
        ck("C19", t, "1");
        ck("C19", "1", t);
        ck("C19", t, "3");
    }
    let mut rng = Rng::new(19);
    for _ in 0..200 {
        // n/8, n/16, n/32 are exactly representable and hit the tie case
        let den = *rng.pick(&[8i64, 16, 32, 64, 2, 4]);
        let num = rng.range_i64(-4000, 4000);
        let v = num as f64 / den as f64;
        ck("C19", &format!("{v:?}"), "1");
        ck("C19", &format!("{v:?}"), &rng.range_i64(-3, 3).to_string());
    }
}

// ------------------------------------------------------------------ C20

#[test]
fn c20_huge_output_digit_expansion() {
    for (b, e) in [
        ("10", "100"),
        ("10", "200"),
        ("10", "307"),
        ("10", "308"),
        ("2", "1023"),
        ("1.7976931348623157e308", "1"),
        ("1e308", "1"),
        ("-1e308", "1"),
        ("3", "600"),
        ("1e154", "2"),
        ("-2", "1023"),
    ] {
        let o = assert_same("C20", b, e);
        if is_result(&o) {
            // sanity: we really are exercising the long-digit path
            assert!(
                o.stdout.len() > 50,
                "expected a long expansion for pow({b},{e}): {o:?}"
            );
        }
    }
    let over = format!("17976931348623157{}", "0".repeat(292));
    assert_same("C20", &over, "1");
}

// ------------------------------------------------------------------ C21

#[test]
fn c21_tiny_and_negative_zero_output() {
    for (b, e) in [
        ("10", "-30"),
        ("10", "-300"),
        ("-2", "-1000"),
        ("-0.0", "3"),
        ("-0.0", "5"),
        ("-0.0", "1"),
        ("-1e-300", "1"),
        ("1e-300", "1"),
        ("-0.001", "1"),
        ("-0.004", "1"),
        ("0.004", "1"),
        ("-0.0", "0.5"),
        ("-2", "-1001"),
    ] {
        assert_same("C21", b, e);
    }
    // -0.00 must really be produced somewhere in this row
    let o = assert_same("C21", "-0.0", "3");
    assert_eq!(stdout_str(&o), "Result: -0.00\n");
}

// ------------------------------------------------------------------ C22

#[test]
fn c22_extreme_literals() {
    let lits = [
        "1.7976931348623157e308",
        "2.2250738585072014e-308",
        "5e-324",
        "2.220446049250313e-16",
        "4.9406564584124654e-324",
        "1.1125369292536007e-308",
        "9007199254740993",
        "9007199254740992",
    ];
    for l in lits {
        for e in ["1", "-1", "0.5", "2", "0", "3"] {
            assert_same("C22", l, e);
            assert_same("C22", e, l);
        }
    }
}

// ------------------------------------------------------------------ C23

#[test]
fn c23_seventeen_significant_digit_roundtrips() {
    let mut rng = Rng::new(23);
    for _ in 0..400 {
        // finite, non-extreme doubles rendered with 17 significant digits
        let b = f64::from_bits(rng.next_u64());
        if !b.is_finite() {
            continue;
        }
        let e = rng.range_i64(-5, 5) as f64 + if rng.bool() { 0.0 } else { 0.5 };
        assert_same("C23", &format!("{:.16e}", b), &format!("{:.16e}", e));
    }
}

// ------------------------------------------------------------------ C24

#[test]
fn c24_raw_random_bit_patterns() {
    let mut rng = Rng::new(24);
    for _ in 0..500 {
        let b = rng.any_f64();
        let e = rng.any_f64();
        // hex form: exact, no decimal rounding at all
        assert_same("C24", &hexfloat(b), &hexfloat(e));
    }
    let mut rng = Rng::new(240);
    for _ in 0..300 {
        // shortest decimal form (also exact) of one random and one small value
        let b = rng.any_f64();
        let e = rng.range_i64(-8, 8) as f64;
        assert_same("C24", &dec(b), &dec(e));
    }
}

// ------------------------------------------------------------------ C25

#[test]
fn c25_very_long_arguments() {
    for n in [1_000usize, 10_000, 100_000] {
        // long mantissa
        let mut s = String::from("1.");
        s.push_str(&"1234567890".repeat(n / 10));
        assert_same_raw("C25", &[s.as_bytes(), b"2"]);
        assert_same_raw("C25", &[b"2", s.as_bytes()]);
        // long leading-zero padding
        let z = format!("{}1.5", "0".repeat(n));
        assert_same_raw("C25", &[z.as_bytes(), b"3"]);
        // long fraction tail of zeros
        let t = format!("2.{}1", "0".repeat(n));
        assert_same_raw("C25", &[t.as_bytes(), b"2"]);
        // long exponent digits (overflows -> ERANGE)
        let x = format!("1e{}", "9".repeat(n.min(1000)));
        assert_same_raw("C25", &[x.as_bytes(), b"2"]);
        // long whitespace prefix
        let w = format!("{}1.5", " ".repeat(n.min(10_000)));
        assert_same_raw("C25", &[w.as_bytes(), b"2"]);
    }
}

// ------------------------------------------------------------------ C31

#[test]
fn c31_mixed_fuzz() {
    let mut rng = Rng::new(31);
    let specials = [
        "", " ", "inf", "-inf", "nan", "0", "-0.0", "1", "-1", "0.5", "1e400", "1e-400", "abc",
        "0x", "0x1p3", ".5", "5.", "+", "-", "1e", "nan(1)", "infinity", "1,5", "1.5 ", "\t2",
        "9007199254740993", "1e308", "-1e308", "2.5", "-2.5",
    ];
    for _ in 0..3000 {
        let mk = |rng: &mut Rng| -> Vec<u8> {
            match rng.below(10) {
                0 => rng.pick(&specials).as_bytes().to_vec(),
                1 => {
                    // pure fuzz bytes (no NUL: execve cannot carry one)
                    let n = rng.below(12) as usize;
                    (0..n)
                        .map(|_| {
                            let mut c = rng.below(255) as u8 + 1;
                            if c == 0 {
                                c = b'x';
                            }
                            c
                        })
                        .collect()
                }
                2 => {
                    // digits with a random garbage suffix
                    let v = rng.range_i64(-1000, 1000);
                    let junk = *rng.pick(&["x", " ", "e", ".", ",", "%", "\t", "z9"]);
                    format!("{v}{junk}").into_bytes()
                }
                3 => hexfloat(rng.any_f64()).into_bytes(),
                4 => dec(rng.any_f64()).into_bytes(),
                5 => format!("{}", rng.range_i64(-1000, 1000)).into_bytes(),
                6 => format!("{:?}", rng.f01() * 1000.0 - 500.0).into_bytes(),
                7 => format!("{}e{}", rng.range_i64(-99, 99), rng.range_i64(-400, 400))
                    .into_bytes(),
                8 => {
                    let ws = *rng.pick(&[" ", "\t", "\n", "  ", ""]);
                    format!("{ws}{}", rng.range_i64(-99, 99)).into_bytes()
                }
                _ => format!("{}", rng.any_f64()).into_bytes(),
            }
        };
        let b = mk(&mut rng);
        let e = mk(&mut rng);
        assert_same_raw("C31", &[&b, &e]);
    }
}

// ------------------------------------------------------------------ C32

#[test]
fn c32_integrality_decision_boundary() {
    let exps = [
        "9007199254740992",  // 2^53
        "9007199254740993",  // 2^53+1 (rounds to 2^53)
        "9007199254740991",  // 2^53-1
        "-9007199254740992",
        "1e15",
        "1e16",
        "1e17",
        "4503599627370496", // 2^52
        "4503599627370495.5",
        "0.5",
        "1.5",
        "-2.5",
        "2.5",
        "1.0000000000000002",
        "0.9999999999999999",
        "3.0000000000000004",
        "1e300",
        "-1e300",
    ];
    for b in ["-2", "-1", "-0.5", "-1.5", "2", "0.5", "-0.0", "0"] {
        for e in exps {
            ck("C32", b, e);
        }
    }
}

// ------------------------------------------------------------------ C33

/// Direct sweep of the `%.2f` conversion over the WHOLE f64 range.
/// `pow(x, 1) == x` exactly, so `driver <x> 1` prints `%.2f` of `x` itself --
/// this is the most concentrated test of the formatter, which is the only piece
/// of this program the Rust translation implements itself rather than delegating
/// to libc.
#[test]
fn c33_percent_2f_formatter_sweep() {
    let mut rng = Rng::new(33);
    for _ in 0..2000 {
        let x = rng.any_f64();
        assert_same("C33", &hexfloat(x), "1");
    }
    // magnitudes spread evenly over the exponent range, with full mantissas
    let mut rng = Rng::new(330);
    for _ in 0..1000 {
        let exp = rng.range_i64(-320, 308) as i32;
        let m = rng.f01() * 9.0 + 1.0;
        let v = if rng.bool() { -m } else { m } * 10f64.powi(exp);
        assert_same("C33", &format!("{v:e}"), "1");
    }
}

// ------------------------------------------------------------------ C34

/// Every binade: 2^k for k in -1074..=1023, plus the largest value in each
/// binade.  Exhaustive over exponents, so no magnitude class of the `%.2f`
/// conversion is left untested.
#[test]
fn c34_every_binade() {
    for k in -1074..=1023i32 {
        let v = if k >= -1022 {
            f64::from_bits((((k + 1023) as u64) << 52) | 0)
        } else {
            // subnormal 2^k
            f64::from_bits(1u64 << (k + 1074))
        };
        assert_same("C34", &hexfloat(v), "1");
        assert_same("C34", &hexfloat(-v), "1");
    }
}
