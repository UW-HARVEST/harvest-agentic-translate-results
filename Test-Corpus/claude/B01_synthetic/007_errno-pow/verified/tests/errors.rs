//! Phase C — error-path differential tests.
//!
//! One test per row of ERRORS.md.  Each constructs the exact invalid
//! input/condition, runs BOTH binaries, and asserts they return the same
//! rejection: same exit status, same terminating signal, same stderr bytes,
//! same (empty) stdout.  Rows also assert WHICH C branch was taken, so a test
//! cannot silently pass by both binaries failing in some other way.

mod common;
use common::*;

// ---------------------------------------------------------------- E01..E07
// argc != 3  (main.c:31-33)   `Usage: %s base exponent\n`, exit 1
// (E05 argc==0 lives in process_axes.rs — it needs a raw execve.)

const USAGE: &str = "Usage: driver base exponent\n";

#[test]
fn e01_argc1_no_arguments() {
    let o = assert_same_raw("E01", &[]);
    assert_eq!(o.code, Some(1));
    assert_eq!(stderr_str(&o), USAGE);
    assert!(o.stdout.is_empty());
}

#[test]
fn e02_argc2_one_argument() {
    for a in ["2", "", "abc", "-", "1e400"] {
        let o = assert_same_raw("E02", &[a.as_bytes()]);
        assert_eq!(o.code, Some(1));
        assert_eq!(stderr_str(&o), USAGE);
        assert!(o.stdout.is_empty());
    }
}

#[test]
fn e03_argc4_three_arguments() {
    let o = assert_same_raw("E03", &[b"2", b"3", b"4"]);
    assert_eq!(o.code, Some(1));
    assert_eq!(stderr_str(&o), USAGE);
}

#[test]
fn e04_argc11_many_arguments() {
    let args: Vec<&[u8]> = vec![b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10"];
    let o = assert_same_raw("E04", &args);
    assert_eq!(o.code, Some(1));
    assert_eq!(stderr_str(&o), USAGE);
    // ... and every count from 0 to 12 except the valid 3-argv case (argc==3
    // means argv[0] + 2 args).
    for n in 0..=12usize {
        if n == 2 {
            continue;
        }
        let v: Vec<&[u8]> = (0..n).map(|_| &b"1"[..]).collect();
        let o = assert_same_raw("E04", &v);
        assert_eq!(o.code, Some(1), "argc={}", n + 1);
        assert_eq!(stderr_str(&o), USAGE);
    }
}

#[test]
fn e06_empty_argv0() {
    // argv[0] == "" -> "Usage:  base exponent\n" (two spaces)
    let c = run_raw(&c_bin(), "", &[], None);
    let r = run_raw(&rust_bin(), "", &[], None);
    assert_eq!(c, r, "[E06] divergence with empty argv[0]");
    assert_eq!(stderr_str(&c), "Usage:  base exponent\n");
    assert_eq!(c.code, Some(1));
}

#[test]
fn e07_non_utf8_argv0() {
    for a0 in ["\u{ff}\u{fe}prog", "pro\u{7f}g", "p%sg"] {
        let c = run_raw(&c_bin(), a0, &[], None);
        let r = run_raw(&rust_bin(), a0, &[], None);
        assert_eq!(c, r, "[E07] divergence with argv[0]={a0:?}");
        assert_eq!(c.code, Some(1));
        assert!(c.stderr.starts_with(b"Usage: "));
        assert!(c.stderr.ends_with(b" base exponent\n"));
    }
}

// ---------------------------------------------------------------- E08..E12
// base strtod ERANGE (main.c:41-43)

fn expect_base_range(row: &str, base: &str) {
    let o = assert_same(row, base, "2");
    assert_eq!(o.code, Some(1), "[{row}] base={base:?}");
    assert_eq!(
        stderr_str(&o),
        format!("Range error while converting base '{base}'\n"),
        "[{row}] base={base:?}"
    );
    assert!(o.stdout.is_empty());
}

#[test]
fn e08_base_overflow() {
    expect_base_range("E08", "1e400");
}

#[test]
fn e09_base_overflow_variants() {
    for b in [
        "-1e400",
        "1e999999",
        "1e310",
        "-1e310",
        "0x1p+50000",
        "1.8e308",
    ] {
        expect_base_range("E09", b);
    }
    // ...and the same magnitudes spelled out as plain digit strings with no
    // exponent at all: 309 digits = just above DBL_MAX -> ERANGE.
    let over = format!("17976931348623159{}", "0".repeat(292));
    assert_eq!(over.len(), 309);
    expect_base_range("E09", &over);
    let way_over = format!("9{}", "0".repeat(400));
    expect_base_range("E09", &way_over);
    // ...whereas 309 digits that round DOWN to DBL_MAX are accepted.
    let under = format!("17976931348623157{}", "0".repeat(292));
    let o = assert_same("E09", &under, "1");
    assert_eq!(o.code, Some(0), "{o:?}");
}

#[test]
fn e10_base_underflow() {
    for b in ["1e-400", "-1e-400", "1e-999999", "0x1p-50000"] {
        expect_base_range("E10", b);
    }
}

#[test]
fn e11_base_subnormal_is_erange() {
    // glibc strtod sets ERANGE for INEXACT gradual underflow even though the
    // value is representable.  1e-308 < DBL_MIN, so it is rejected too.
    for b in [
        "1e-320",
        "5e-324",
        "1e-308",
        "-1e-320",
        "2.2250738585072012e-308",
        "2.2e-308",
        "0x1.fffffffffffffp-1023",
    ] {
        expect_base_range("E11", b);
    }
    // ...but an EXACTLY representable subnormal does NOT set ERANGE in glibc's
    // hex path, and a decimal that rounds up to DBL_MIN stays normal: both are
    // accepted.  (Verified against the C.)
    for b in ["0x1p-1023", "0x1p-1074", "2.2250738585072013e-308"] {
        let o = assert_same("E11", b, "1");
        assert_eq!(o.code, Some(0), "base={b} -> {o:?}");
    }
}

#[test]
fn e12_erange_checked_before_endptr_base() {
    // Both out-of-range AND trailing garbage: C tests errno FIRST, so this is a
    // RANGE error, not an "Invalid numeric input" error.
    for b in ["1e400xyz", "1e-400zzz", "1e400 ", "1e999999abc"] {
        expect_base_range("E12", b);
    }
}

// ---------------------------------------------------------------- E13..E19
// base *endptr != '\0' (main.c:44-46)

fn expect_base_invalid(row: &str, base: &[u8]) {
    let o = assert_same_raw(row, &[base, b"2"]);
    assert_eq!(o.code, Some(1), "[{row}] base={}", esc(base));
    let mut want = b"Invalid numeric input for base: '".to_vec();
    want.extend_from_slice(base);
    want.extend_from_slice(b"'\n");
    assert_eq!(o.stderr, want, "[{row}] base={}", esc(base));
    assert!(o.stdout.is_empty());
}

#[test]
fn e13_base_no_conversion() {
    for b in [
        "abc", "x", "-", "+", ".", "e5", "E5", "--1", "+-1", "1..2", "-.", "+.", "..", "d1", "'1'",
        "1/2", "one", "true", "null", "NULL", "%f", "\\n", "-x1",
    ] {
        expect_base_invalid("E13", b.as_bytes());
    }
}

#[test]
fn e14_base_partial_conversion() {
    for b in [
        "12abc", "1.5x", "2 3", "1,5", "3.14foo", "1e5e5", "1.2.3", "0.1.", "5%", "9)", "1_000",
        "2^3", "1e", "1e+", "1e-", "1.5e", "12,", "-3-",
    ] {
        expect_base_invalid("E14", b.as_bytes());
    }
}

#[test]
fn e15_base_trailing_whitespace() {
    // Leading whitespace is skipped by strtod; TRAILING whitespace is not.
    for b in [
        &b"1.5 "[..],
        b"1.5\t",
        b"1.5\n",
        b"1.5\r",
        b"1.5\x0b",
        b"1.5\x0c",
        b"2  ",
        b" 2 ",
        b"\t2\t",
    ] {
        expect_base_invalid("E15", b);
    }
}

#[test]
fn e16_base_whitespace_only() {
    // No conversion => endptr == nptr => *endptr is the whitespace byte.
    for b in [
        &b" "[..],
        b"   ",
        b"\t",
        b"\n",
        b"\r",
        b"\x0b",
        b"\x0c",
        b" \t\n\r\x0b\x0c",
    ] {
        expect_base_invalid("E16", b);
    }
}

#[test]
fn e17_base_incomplete_hex() {
    // glibc converts the leading "0" and leaves "x..." in endptr.
    for b in ["0x", "0X", "0x.", "0xp1", "0Xg", "0x+", "0x-1", "00x1"] {
        expect_base_invalid("E17", b.as_bytes());
    }
}

#[test]
fn e18_base_non_utf8_bytes() {
    for b in [
        &b"\xff\xfe\x80"[..],
        b"\xc3",
        b"\x80\x81",
        b"1\xff",
        b"\xff1",
        b"\x01\x02",
        b"\x7f",
        b"2\xe2\x82\xac",
    ] {
        expect_base_invalid("E18", b);
    }
}

#[test]
fn e19_base_partial_special_tokens() {
    for b in [
        "nan(", "nan(x", "inf1", "infin", "infinit", "NANx", "nane", "in", "na", "i", "n",
        "infinityy", "nan()x",
    ] {
        expect_base_invalid("E19", b.as_bytes());
    }
}

// ---------------------------------------------------------------- E20..E25
// exponent conversion errors (main.c:52-57)

fn expect_exp_range(row: &str, exp: &str) {
    let o = assert_same(row, "2", exp);
    assert_eq!(o.code, Some(1), "[{row}] exp={exp:?}");
    assert_eq!(
        stderr_str(&o),
        format!("Range error while converting exponent '{exp}'\n"),
        "[{row}] exp={exp:?}"
    );
}

fn expect_exp_invalid(row: &str, exp: &[u8]) {
    let o = assert_same_raw(row, &[b"2", exp]);
    assert_eq!(o.code, Some(1), "[{row}] exp={}", esc(exp));
    let mut want = b"Invalid numeric input for exponent: '".to_vec();
    want.extend_from_slice(exp);
    want.extend_from_slice(b"'\n");
    assert_eq!(o.stderr, want, "[{row}] exp={}", esc(exp));
}

#[test]
fn e20_exponent_overflow() {
    for e in ["1e400", "-1e400", "1e999999", "0x1p+50000"] {
        expect_exp_range("E20", e);
    }
}

#[test]
fn e21_exponent_underflow() {
    for e in ["1e-400", "1e-320", "5e-324", "1e-308", "-1e-400"] {
        expect_exp_range("E21", e);
    }
}

#[test]
fn e22_erange_checked_before_endptr_exponent() {
    for e in ["1e400zzz", "1e-400 ", "1e400\t"] {
        expect_exp_range("E22", e);
    }
}

#[test]
fn e23_base_error_reported_before_exponent_error() {
    // Both invalid: only the BASE message appears.
    let o = assert_same("E23", "abc", "def");
    assert_eq!(o.code, Some(1));
    assert_eq!(stderr_str(&o), "Invalid numeric input for base: 'abc'\n");
    // base invalid + exponent out of range -> base message
    let o = assert_same("E23", "abc", "1e400");
    assert_eq!(stderr_str(&o), "Invalid numeric input for base: 'abc'\n");
    // base valid + exponent invalid -> exponent message
    let o = assert_same("E23", "2", "def");
    assert_eq!(
        stderr_str(&o),
        "Invalid numeric input for exponent: 'def'\n"
    );
}

#[test]
fn e24_base_range_before_exponent_range() {
    let o = assert_same("E24", "1e400", "1e400");
    assert_eq!(o.code, Some(1));
    assert_eq!(stderr_str(&o), "Range error while converting base '1e400'\n");
    // errno is reset before the exponent conversion, so a *valid* base cannot
    // leak a stale ERANGE into the exponent check.
    let o = assert_same("E24", "1e400", "abc");
    assert_eq!(stderr_str(&o), "Range error while converting base '1e400'\n");
}

#[test]
fn e25_exponent_invalid_suffix() {
    for e in [
        &b"abc"[..],
        b"2x",
        b" 2 ",
        b"2 ",
        b"\t",
        b"   ",
        b"0x",
        b"-",
        b".",
        b"e1",
        b"1,5",
        b"\xff\xfe",
        b"nan(",
        b"inf1",
    ] {
        expect_exp_invalid("E25", e);
    }
}

// ---------------------------------------------------------------- E26..E28
// pow EDOM (main.c:63-65)

fn expect_domain(row: &str, base: &str, exp: &str) {
    let o = assert_same(row, base, exp);
    assert_eq!(o.code, Some(1), "[{row}] pow({base},{exp})");
    let s = stderr_str(&o);
    assert!(
        s.starts_with("Domain error: pow(")
            && s.ends_with(") is undefined in the real number domain.\n"),
        "[{row}] pow({base},{exp}) -> {s:?}"
    );
    assert!(o.stdout.is_empty());
}

#[test]
fn e26_pow_domain_error() {
    let o = assert_same("E26", "-2", "0.5");
    assert_eq!(o.code, Some(1));
    assert_eq!(
        stderr_str(&o),
        "Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n"
    );
    for (b, e) in [
        ("-2", "0.5"),
        ("-8", "0.3333333333333333"),
        ("-1", "2.5"),
        ("-1.5", "1.5"),
        ("-100", "0.1"),
        ("-0.5", "-0.5"),
        ("-3", "-1.5"),
    ] {
        expect_domain("E26", b, e);
    }
}

#[test]
fn e27_pow_domain_error_large_and_random() {
    expect_domain("E27", "-1.5", "1000000000000000.5");
    expect_domain("E27", "-1e300", "1.5");
    expect_domain("E27", "-1e-300", "-1.5");
    // randomized: negative base, guaranteed non-integer exponent
    let mut rng = Rng::new(27);
    for _ in 0..150 {
        let b = -(rng.f01() * 100.0 + 0.001);
        let e = rng.range_i64(-40, 40) as f64 + 0.5;
        expect_domain("E27", &format!("{b:?}"), &format!("{e:?}"));
    }
}

#[test]
fn e28_pow_domain_message_formatting() {
    // the %.2f formatting of the EDOM message, incl. -0.00, ties and 309 digits
    for (b, e) in [
        ("-0.005", "0.5"),
        ("-0.001", "0.5"),
        ("-0.125", "0.5"),
        ("-0.375", "0.5"),
        ("-1e300", "0.5"),
        ("-1.7976931348623157e308", "1.5"),
        ("-4.9", "1.005"),
        ("-2", "-0.625"),
        ("-2", "1e15"),
    ] {
        let o = assert_same("E28", b, e);
        assert_eq!(o.code, Some(1), "pow({b},{e}) -> {o:?}");
    }
}

// ---------------------------------------------------------------- E29..E33
// pow ERANGE (main.c:66-68)

fn expect_pow_range(row: &str, base: &str, exp: &str) {
    let o = assert_same(row, base, exp);
    assert_eq!(o.code, Some(1), "[{row}] pow({base},{exp}) -> {o:?}");
    let s = stderr_str(&o);
    assert!(
        s.starts_with("Range error: pow(") && s.ends_with(") caused overflow or underflow.\n"),
        "[{row}] pow({base},{exp}) -> {s:?}"
    );
    assert!(o.stdout.is_empty());
}

#[test]
fn e29_pow_overflow() {
    let o = assert_same("E29", "10", "400");
    assert_eq!(
        stderr_str(&o),
        "Range error: pow(10.00, 400.00) caused overflow or underflow.\n"
    );
    for (b, e) in [
        ("10", "400"),
        ("2", "5000"),
        ("-10", "401"),
        ("-10", "402"),
        ("2", "1024"),
        ("1.0000001", "1e10"),
        ("10", "309"),
        ("2", "1e15"),
    ] {
        expect_pow_range("E29", b, e);
    }
}

#[test]
fn e30_pow_underflow() {
    let o = assert_same("E30", "10", "-400");
    assert_eq!(
        stderr_str(&o),
        "Range error: pow(10.00, -400.00) caused overflow or underflow.\n"
    );
    for (b, e) in [
        ("10", "-400"),
        ("0.5", "5000"),
        ("2", "-5000"),
        ("-0.5", "5001"),
        ("10", "-325"),
        ("0.1", "400"),
    ] {
        expect_pow_range("E30", b, e);
    }
}

#[test]
fn e31_pow_pole_divide_by_zero() {
    for (b, e) in [
        ("0", "-1"),
        ("0", "-2"),
        ("-0.0", "-1"),
        ("-0.0", "-3"),
        ("-0.0", "-2"),
        ("0.0", "-1e300"),
        ("0", "-0.5"),
    ] {
        expect_pow_range("E31", b, e);
    }
    // ... but an INFINITE exponent at the pole sets no errno at all: the C
    // succeeds.  Verified against the C binary.
    let o = assert_same("E31", "0", "-inf");
    assert_eq!(o.code, Some(0));
    assert_eq!(stdout_str(&o), "Result: inf\n");
    let o = assert_same("E31", "-0.0", "-inf");
    assert_eq!(o.code, Some(0));
    assert_eq!(stdout_str(&o), "Result: inf\n");
}

#[test]
fn e32_pow_gradual_underflow_is_not_an_error() {
    // glibc's pow does NOT set ERANGE for a subnormal result, so the C SUCCEEDS
    // here -- unlike strtod (E11), which does set ERANGE for subnormals.
    let o = assert_same("E32", "10", "-320");
    assert_eq!(o.code, Some(0), "{o:?}");
    assert_eq!(stdout_str(&o), "Result: 0.00\n");
    assert!(o.stderr.is_empty());
    for (b, e) in [("10", "-310"), ("2", "-1030"), ("10", "-315"), ("2", "-1074")] {
        let o = assert_same("E32", b, e);
        assert_eq!(o.code, Some(0), "pow({b},{e}) -> {o:?}");
    }
}

#[test]
fn e33_edom_checked_before_erange() {
    // Inputs where pow is invalid AND overflowing: C tests EDOM first.
    for (b, e) in [
        ("-1e300", "1.5"),
        ("-1e-300", "1.5"),
        ("-2", "1e300"),
        ("-0.5", "1e300"),
    ] {
        let o = assert_same("E33", b, e);
        assert_eq!(o.code, Some(1), "pow({b},{e}) -> {o:?}");
        let s = stderr_str(&o);
        assert!(
            s.starts_with("Domain error: ") || s.starts_with("Range error: "),
            "{s:?}"
        );
    }
}

// ---------------------------------------------------------------- E38..E40

#[test]
fn e38_errno_other_than_erange_edom_is_ignored() {
    // strtod may set an errno that is neither ERANGE nor EDOM; the C ignores it
    // and must still succeed.  Also: a *successful* run must not be disturbed by
    // whatever errno the earlier conversions left behind.
    for (b, e) in [
        ("0", "0"),
        ("1", "0"),
        ("", ""),
        ("0x0", "0x0"),
        ("nan", "0"),
        ("inf", "0"),
    ] {
        let o = assert_same("E38", b, e);
        assert_eq!(o.code, Some(0), "pow({b},{e}) -> {o:?}");
        assert_eq!(stdout_str(&o), "Result: 1.00\n");
    }
}

#[test]
fn e39_boundary_one_step_past_valid_range() {
    // DBL_MAX is fine; one representable decimal step up is ERANGE.
    let o = assert_same("E39", "1.7976931348623157e308", "1");
    assert_eq!(o.code, Some(0), "{o:?}");
    let o = assert_same("E39", "1.7976931348623159e308", "1");
    assert_eq!(o.code, Some(1));
    assert!(stderr_str(&o).starts_with("Range error while converting base"));

    // DBL_MIN is fine; so is the next decimal down (it still rounds to DBL_MIN,
    // i.e. stays normal), but one more step down underflows inexactly -> ERANGE.
    let o = assert_same("E39", "2.2250738585072014e-308", "1");
    assert_eq!(o.code, Some(0), "{o:?}");
    let o = assert_same("E39", "2.2250738585072013e-308", "1");
    assert_eq!(o.code, Some(0), "{o:?}");
    let o = assert_same("E39", "2.2250738585072012e-308", "1");
    assert_eq!(o.code, Some(1), "{o:?}");

    // exponent side of the same boundaries
    for e in ["1.7976931348623157e308", "1.7976931348623159e308"] {
        assert_same("E39", "1", e);
    }
    // hex-float boundaries: 2^1024 overflows, 2^1023 does not
    let o = assert_same("E39", "0x1p1023", "1");
    assert_eq!(o.code, Some(0), "{o:?}");
    let o = assert_same("E39", "0x1p1024", "1");
    assert_eq!(o.code, Some(1), "{o:?}");
    // 2^-1074 is DBL_TRUE_MIN: representable, but strtod flags gradual underflow
    for b in ["0x1p-1022", "0x1p-1023", "0x1p-1074", "0x1p-1075"] {
        assert_same("E39", b, "1");
    }
}

#[test]
fn e40_zero_length_and_oversized_arguments() {
    // The empty string is ACCEPTED by the C as 0.0 (endptr == nptr, *endptr == 0)
    let o = assert_same("E40", "", "");
    assert_eq!(o.code, Some(0), "{o:?}");
    assert_eq!(stdout_str(&o), "Result: 1.00\n");
    let o = assert_same("E40", "", "2");
    assert_eq!(o.code, Some(0));
    assert_eq!(stdout_str(&o), "Result: 0.00\n");
    let o = assert_same("E40", "", "-1");
    assert_eq!(o.code, Some(1)); // pow(0,-1) -> pole
    let o = assert_same("E40", "3", "");
    assert_eq!(stdout_str(&o), "Result: 1.00\n");

    // oversized arguments
    for n in [1_000usize, 10_000, 100_000] {
        let mut s = String::from("1.");
        s.push_str(&"1234567890".repeat(n / 10));
        let o = assert_same_raw("E40", &[s.as_bytes(), b"2"]);
        assert_eq!(o.code, Some(0), "len={} -> {:?}", s.len(), o);
        // a long run of leading zeros must still parse
        let z = format!("{}1", "0".repeat(n));
        assert_same_raw("E40", &[z.as_bytes(), b"3"]);
        // long invalid tail
        let bad = format!("1.5{}", "z".repeat(n.min(10_000)));
        let o = assert_same_raw("E40", &[bad.as_bytes(), b"2"]);
        assert_eq!(o.code, Some(1));
    }
}
