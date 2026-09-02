//! Differential tests: every case runs the C program and the translated program
//! as subprocesses and compares stdout, stderr and exit status byte for byte.
//!
//! The cases are organised by the branches `c_src/src/main.c` actually takes:
//!
//! 1. `argc != 3`                        -> `Usage: ...`, exit 1
//! 2. `errno == ERANGE` after base       -> `Range error while converting base ...`
//! 3. `*endptr1 != '\0'`                 -> `Invalid numeric input for base: ...`
//! 4. `errno == ERANGE` after exponent   -> `Range error while converting exponent ...`
//! 5. `*endptr2 != '\0'`                 -> `Invalid numeric input for exponent: ...`
//! 6. `errno == EDOM` after `pow`        -> `Domain error: ...`
//! 7. `errno == ERANGE` after `pow`      -> `Range error: pow(...) caused overflow ...`
//! 8. otherwise                          -> `Result: %.2f`, exit 0
//!
//! plus the sub-branches inside `strtod` (whitespace, signs, decimal, hex,
//! `inf`, `nan`, `nan(payload)`, "no conversion performed") and inside `pow`'s
//! `errno` reporting (pole, overflow, underflow-to-zero, domain).

mod harness;

use harness::{assert_same, check, check_cross, Rng};

// ---------------------------------------------------------------------------
// 1. argc != 3
// ---------------------------------------------------------------------------

#[test]
fn wrong_argument_count() {
    // Every argc the program can be handed except the accepted one.
    assert_same(&[]);
    assert_same(&[b"1"]);
    assert_same(&[b"1", b"2", b"3"]);
    assert_same(&[b"1", b"2", b"3", b"4"]);
    assert_same(&[b"2", b"10", b""]);
    assert_same(&[b""]);
    // Arguments that would otherwise be errors are never even looked at.
    assert_same(&[b"abc"]);
    assert_same(&[b"1e400", b"abc", b"x"]);
}

// ---------------------------------------------------------------------------
// 8. the happy path
// ---------------------------------------------------------------------------

#[test]
fn happy_path() {
    check("2", "10");
    check("2", "0.5");
    check("10", "3");
    check("1.5", "2");
    check("-2", "3");
    check("-2", "4");
    check("0", "0");
    check("0", "5");
    check("1", "0");
    check("1", "99999");
    check("100", "0.5");
    check("2", "-3");
    check("7", "-1");
    check("1e100", "1");
    check("1e-100", "1");
}

#[test]
fn single_item_and_maximum_magnitudes() {
    // The largest and smallest magnitudes the code can carry through `pow`
    // without tripping a range error.
    check("1.7976931348623157e308", "1");
    check("-1.7976931348623157e308", "1");
    check("1.7976931348623157e308", "0");
    check("5e-324", "1");
    check("-5e-324", "1");
    check("2.2250738585072014e-308", "1");
    check("2", "1023");
    check("2", "1024");
    check("2", "-1074");
    check("2", "-1075");
    check("1.7976931348623157e308", "1.0000000000000002");
}

// ---------------------------------------------------------------------------
// 2 & 4. strtod reports ERANGE
// ---------------------------------------------------------------------------

#[test]
fn strtod_range_errors() {
    // Overflow.
    let over = [
        "1e309",
        "-1e309",
        "1e400",
        "1e999999999999999999999",
        "2e308",
        "1.7976931348623159e308",
        "0x1p1024",
        "0x1p99999",
        "179769313486231580000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    ];
    // Underflow: gradual underflow sets ERANGE too, which is why 1e-320 fails
    // even though it is representable as a subnormal.
    let under = [
        "1e-320",
        "-1e-320",
        "1e-400",
        "5e-324",
        "1e-999999999999",
        "0x1p-1080",
        "4.9406564584124654e-325",
    ];
    for s in over.iter().chain(under.iter()) {
        check(s, "2"); // reported against the base
        check("2", s); // reported against the exponent
    }
    // A representable subnormal reached exactly does *not* set ERANGE.
    check("0x1p-1074", "1");
    check("0x1p-1073", "1");
    check("5e-324", "1"); // inexact -> ERANGE
    check("0x1p-1023", "1"); // exact -> no ERANGE
}

#[test]
fn range_error_is_checked_before_trailing_characters() {
    // The C code tests `errno == ERANGE` first, so an input that both overflows
    // and has leftover characters reports the range error, not the invalid one.
    for s in [
        "1e400xyz",
        "1e-400xyz",
        "1e999999999zzz",
        "0x1p99999q",
        "-1e400!",
        "1e400e",
        "1e-400.5",
    ] {
        check(s, "2");
        check("2", s);
    }
}

#[test]
fn base_is_validated_before_exponent() {
    // Both arguments are bad: only the base is reported.
    check("abc", "def");
    check("1e400", "1e400");
    check("abc", "1e400");
    check("1e400", "abc");
    check("1e-400", "xyz");
    check("xyz", "1e-400");
}

// ---------------------------------------------------------------------------
// 3 & 5. `*endptr != '\0'` - trailing characters
// ---------------------------------------------------------------------------

#[test]
fn invalid_numeric_input() {
    let bad = [
        "abc", "x", "1x", "1.2.3", "1,5", "--1", "++1", "1-", "5%", "1 ", "5 ", " ", "  ", "\t",
        "\n", "\r", "\u{b}", "\u{c}", "+", "-", ".", "+.", "-.", "e", "E", "e5", ".e5", "1e",
        "1e+", "1e-", "0x", "0X", "0xg", "0x.", "0x1p", "0x1p+", "0x1p-", "0x1p2x", "0b101",
        "0o10", "1_0", "infi", "infin", "infinityx", "INFx", "nan(", "nan(x", "nan()x", "nan(a)b",
        "TRUE", "null", "--", "+-1", "-+1", "0.0.0", ". ", " .", "1d5", "1e5e5", "0x1e5p",
    ];
    for s in bad {
        check(s, "2");
        check("2", s);
    }
}

#[test]
fn empty_argument_converts_to_zero() {
    // `strtod("")` performs no conversion, leaving `endptr == nptr`; because
    // `*endptr` is then the terminator, the C program silently accepts it as 0.
    check("", "2");
    check("2", "");
    check("", "");
    check("", "0");
    check("", "-1");
}

// ---------------------------------------------------------------------------
// strtod's accepted lexical forms
// ---------------------------------------------------------------------------

#[test]
fn strtod_accepted_forms() {
    let forms = [
        // leading whitespace and signs
        " 5", "\t5", "\n5", "\u{b}5", "\u{c}5", "\r5", "   -5", " \t\n\u{b}\u{c}\r-5", "+5",
        "-5", "+0", "-0",
        // decimal shapes
        "5.", ".5", "5.0", "0.5", "00005", "0.", ".0", "0e0", "1E2", "1e+2", "1e-2", "1E-2",
        "0e999999", "0e-999999", "0.000", "-0.000",
        // hexadecimal, exponent optional in glibc
        "0x10", "0X10", "0x1.8p3", "0x1.8P+3", "0x1.8", "0xAp0", "0Xa.bP-2", "0x1.p3", "0x.8",
        "0x0p0", "-0x0p0", "0x1p-0", "-0x10", "+0x10", "0x1e5",
        // infinities
        "inf", "INF", "Inf", "iNf", "infinity", "INFINITY", "InFiNiTy", "-inf", "+inf",
        "-infinity",
        // NaNs, including the payload form glibc encodes into the mantissa
        "nan", "NAN", "nAn", "-nan", "+nan", "nan()", "nan(0)", "nan(1)", "-nan(1)", "+nan(1)",
        " nan(1)", "nan(123)", "nan(0x10)", "nan(0X10)", "nan(010)", "nan(07)", "nan(08)",
        "nan(00)", "nan(0_)", "nan(abc)", "nan(1a)", "nan(1_2)", "nan(_1)", "nan(0b101)",
        "nan(1e5)", "nan(0x)", "nan(0xg)", "nan(0xdeadbeef)", "nan(0xfffffffffffff)",
        "nan(0x8000000000000)", "nan(18446744073709551615)", "nan(18446744073709551616)",
        "nan(99999999999999999999999)", "nan(2251799813685247)", "nan(2251799813685248)",
        "nan(2251799813685249)", "nan(4503599627370496)", "NAN(1)",
    ];
    for s in forms {
        check(s, "2");
        check("2", s);
        check(s, "1");
    }
}

// ---------------------------------------------------------------------------
// 6. pow sets EDOM
// ---------------------------------------------------------------------------

#[test]
fn pow_domain_errors() {
    // A negative finite base with a finite non-integer exponent.
    check_cross(
        &["-2", "-0.5", "-1", "-1024.875", "-1e300", "-1e-300", "-3", "-0.125"],
        &["0.5", "-0.5", "0.3333333333", "1.5", "-1.5", "2.5", "0.125", "0.375", "-0.625"],
    );
    // The domain check happens before the overflow check.
    check("-2", "1e300");
    check("-1e300", "1.5");
    check("-1e-300", "-1.5");
}

#[test]
fn negative_base_integer_exponent_is_not_a_domain_error() {
    // Integral exponents are fine, including ones far beyond 2^53 where every
    // double is an integer.
    for e in [
        "0", "-0", "1", "-1", "2", "-2", "3", "-3", "10", "-10", "4503599627370496",
        "4503599627370497", "9007199254740992", "1e300", "-1e300", "1e16",
    ] {
        check("-2", e);
        check("-0.5", e);
        check("-1", e);
    }
}

// ---------------------------------------------------------------------------
// 7. pow sets ERANGE
// ---------------------------------------------------------------------------

#[test]
fn pow_pole_errors() {
    // pow(+-0, y) with y < 0 is a pole error.
    for e in ["-1", "-2", "-3", "-0.5", "-1e300", "-1e-300", "-1e10"] {
        check("0", e);
        check("-0", e);
        check("0.0", e);
        check("-0.0", e);
    }
    // ... but a non-negative exponent is not.
    for e in ["0", "-0", "1", "2", "0.5", "1e300"] {
        check("0", e);
        check("-0", e);
    }
}

#[test]
fn pow_overflow_and_underflow() {
    // Overflow to infinity.
    check_cross(
        &["2", "10", "-2", "1.1", "1e300", "-1e300", "0.5", "0.1"],
        &["400", "1024", "1025", "10000", "1e300", "-400", "-1024", "-10000", "-1e300"],
    );
    // The underflow cliff: glibc reports ERANGE only once the result reaches
    // zero, so a merely subnormal result is silent.
    for e in [
        "-1021", "-1022", "-1023", "-1030", "-1050", "-1070", "-1073", "-1074", "-1075", "-1076",
        "-1080",
    ] {
        check("2", e);
        check("-2", e);
    }
    for e in ["1073", "1074", "1075", "1076"] {
        check("0.5", e);
        check("-0.5", e);
    }
    check("3", "-700");
    check("1.1", "-3600");
    check("1.1", "-3700");
}

#[test]
fn pow_infinities_and_nans_never_set_errno() {
    let special = ["inf", "-inf", "nan", "-nan", "1", "-1", "0", "-0", "2", "-2", "0.5"];
    check_cross(&special, &special);
}

// ---------------------------------------------------------------------------
// printf("%.2f", ...) formatting
// ---------------------------------------------------------------------------

#[test]
fn format_ties_and_signed_zero() {
    // Values whose exact binary expansion ends in a 5 at the third decimal
    // place, so `%.2f` has to break a true tie (glibc rounds half to even).
    let ties = [
        "0.125", "0.375", "0.625", "0.875", "1.125", "1.375", "2.625", "-0.125", "-0.375",
        "-0.625", "-0.875", "100.125", "1024.875", "-1024.875", "4.125", "8.375", "-2.625",
    ];
    for s in ties {
        check(s, "1");
        check(s, "3");
        check(s, "0.5");
    }
    // Results that are themselves exact ties.
    check("2", "-3");
    check("0.5", "3");
    check("8", "-1");
    check("-2", "-3");
    check("-0.5", "3");
    check("4", "-1.5");
    // Signed zero, and tiny values that round to +-0.00.
    check("-0", "3");
    check("-0", "2");
    check("-0.0", "1");
    check("-2", "-1073");
    check("2", "-1074");
    check("-1e-200", "3");
    check("1e-200", "3");
    // Infinities and NaNs reach the formatter through both operands and the
    // result; glibc prints `inf`, `-inf`, `nan` and `-nan`.
    check("inf", "2");
    check("-inf", "3");
    check("-inf", "2");
    check("nan", "2");
    check("-nan", "2");
    check("2", "nan");
    check("2", "inf");
    check("2", "-inf");
    // A finite result far too large for a short decimal rendering.
    check("1e300", "1");
    check("-1e300", "1");
    check("1e150", "2");
}

// ---------------------------------------------------------------------------
// argument bytes
// ---------------------------------------------------------------------------

#[test]
fn non_utf8_arguments() {
    // C's `argv` is bytes and the diagnostics echo it back with `%s`, so the
    // translation must not lose or replace invalid UTF-8.
    let bad: [&[u8]; 10] = [
        b"\xff\xfe",
        b"\x80",
        b"5\xc3",
        b"\xed\xa0\x80",
        b"2\xff",
        b"\xff",
        b"1e400\xff",
        b"\xc0\x80",
        b"nan\xff",
        b"-\xff",
    ];
    for a in bad {
        assert_same(&[a, b"2"]);
        assert_same(&[b"2", a]);
        assert_same(&[a, a]);
    }
}

#[test]
fn long_arguments() {
    let nines = "9".repeat(100_000);
    let deep = format!("0.{}1", "0".repeat(100_000));
    let wide = format!("1{}", "0".repeat(100_000));
    let hex = format!("0x{}p-80000", "f".repeat(20_000));
    let padded = format!("{}2", " ".repeat(10_000));
    let precise = format!("1.{}1e-308", "0".repeat(50_000));
    for s in [&nines, &deep, &wide, &hex, &padded, &precise] {
        check(s, "1");
        check("2", s);
    }
}

// ---------------------------------------------------------------------------
// generated sweeps
// ---------------------------------------------------------------------------

#[test]
fn generated_lexical_sweep() {
    // Random short strings over the alphabet strtod cares about, which lands on
    // every "no conversion" / "partial conversion" branch far more thoroughly
    // than hand-written cases can.
    const ALPHA: &[u8] = b"0123456789abcdefABCDEF.eEpPxX+- \tinIfNnaA()_";
    let mut rng = Rng::new(0x5EED_1234);
    for _ in 0..1500 {
        let n = rng.below(10) as usize;
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHA)).collect();
        assert_same(&[&s, b"2"]);
    }
}

#[test]
fn generated_numeric_sweep() {
    // Well-formed decimals spanning the whole exponent range, so the sweep
    // straddles the overflow, underflow and success outcomes.
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..1200 {
        let sign = if rng.below(2) == 0 { "-" } else { "" };
        let digits: String = (0..1 + rng.below(18))
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let frac: String = (0..rng.below(18))
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let base = format!("{}{}.{}e{}", sign, digits, frac, rng.range(-340, 340));
        let exponent = format!("{}", rng.range(-1200, 1200));
        check(&base, &exponent);
    }
}

#[test]
fn generated_hex_sweep() {
    let mut rng = Rng::new(0xBEEF_5678);
    for _ in 0..900 {
        let digits: String = (0..1 + rng.below(16))
            .map(|_| char::from_digit(rng.below(16) as u32, 16).unwrap())
            .collect();
        let s = format!("0x{}p{}", digits, rng.range(-1150, 1100));
        check(&s, "1");
        check(&s, "2");
    }
}

#[test]
fn generated_pow_sweep() {
    // Random operand pairs, biased towards the exponents where `pow` changes
    // its `errno` behaviour.
    const BASES: &[&str] = &[
        "2", "-2", "0.5", "-0.5", "10", "-10", "1.1", "0.9", "3", "-3", "1e300", "-1e300",
        "1e-300", "-1e-300", "1", "-1", "0", "-0", "inf", "-inf", "nan", "-nan", "7", "-7",
        "1.0000000000000002", "0.9999999999999999",
    ];
    let mut rng = Rng::new(0xDEAD_BEEF);
    for _ in 0..2000 {
        let b = *rng.pick(BASES);
        let e = match rng.below(4) {
            0 => format!("{}", rng.range(-1200, 1200)),
            1 => format!("{}.5", rng.range(-1200, 1200)),
            2 => format!("{}e{}", rng.range(-99, 99), rng.range(-10, 10)),
            _ => format!("{}", rng.range(-5, 5)),
        };
        check(b, &e);
    }
}
