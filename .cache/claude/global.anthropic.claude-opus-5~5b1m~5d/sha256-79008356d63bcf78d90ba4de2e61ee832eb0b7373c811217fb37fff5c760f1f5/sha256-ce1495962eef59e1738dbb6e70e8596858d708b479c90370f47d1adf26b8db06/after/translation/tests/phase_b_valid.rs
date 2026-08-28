//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C01 … C35). Every generative row runs many
//! randomized inputs from a fixed-seed PRNG so that value-dependent code paths
//! (saturation, rounding, partial `strtod` consumption, out-of-range offsets)
//! are hit, not just one hand-picked value.
//!
//! Both implementations are reached exclusively through `dlopen`/`dlsym` on
//! their `.so`s.

mod common;

use common::*;

/* ================================================================= C01 ==== */

#[test]
fn c01_null_input_buffer() {
    // Deterministic single state, plus randomized `item` pre-contents.
    let mut rng = Rng::new(0xC01);
    for _ in 0..256 {
        let mut case = Case::from_str("123").buffer_null();
        case.item_type = rng.next_u64() as i32;
        case.item_valueint = rng.next_u64() as i32;
        case.item_valuedouble_bits = rng.next_u64();
        let o = diff(&case);
        assert_eq!(o.ret, 0, "NULL buffer must be rejected");
        // item preserved
        assert_eq!(o.type_, case.item_type);
        assert_eq!(o.valueint, case.item_valueint);
        assert_eq!(o.valuedouble_bits, case.item_valuedouble_bits);
    }
}

/* ================================================================= C02 ==== */

#[test]
fn c02_null_content() {
    let mut rng = Rng::new(0xC02);
    for _ in 0..500 {
        let mut case = Case::from_str("12345").content_null();
        case.length = rng.next_u64() as usize;
        case.offset = rng.next_u64() as usize;
        case.depth = rng.next_u64() as usize;
        case.item_type = rng.next_u64() as i32;
        case.item_valueint = rng.next_u64() as i32;
        case.item_valuedouble_bits = rng.next_u64();
        let o = diff(&case);
        assert_eq!(o.ret, 0, "NULL content must be rejected");
        assert_eq!(o.buf_length, case.length);
        assert_eq!(o.buf_offset, case.offset);
        assert_eq!(o.buf_depth, case.depth);
    }
    // Plus the canonical all-zero variant.
    let o = diff(&Case::from_bytes(b"").content_null().length(0).offset(0));
    assert_eq!(o.ret, 0);
}

/* ================================================================= C03 ==== */

#[test]
fn c03_zero_length() {
    let mut rng = Rng::new(0xC03);
    // length == 0 with a non-null content of many different shapes.
    for _ in 0..500 {
        let n = rng.below(12) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        let o = diff(&Case::from_bytes(&bytes).length(0).depth(rng.next_u64() as usize));
        assert_eq!(o.ret, 0, "length 0 must be rejected");
        assert_eq!(o.buf_offset, 0, "offset must not advance");
    }
    let o = diff(&Case::from_bytes(b"").length(0));
    assert_eq!(o.ret, 0);
}

/* ================================================================= C04 ==== */

#[test]
fn c04_offset_equals_length() {
    let mut rng = Rng::new(0xC04);
    for _ in 0..500 {
        let n = rng.range(1, 16) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        let o = diff(&Case::from_bytes(&bytes).length(n).offset(n));
        assert_eq!(o.ret, 0, "offset == length must be rejected");
        assert_eq!(o.buf_offset, n);
    }
}

/* ================================================================= C05 ==== */

#[test]
fn c05_offset_past_length() {
    let mut rng = Rng::new(0xC05);
    for _ in 0..800 {
        let n = rng.below(16) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        let off = n + rng.range(1, 64) as usize;
        let o = diff(&Case::from_bytes(&bytes).length(n).offset(off));
        assert_eq!(o.ret, 0, "offset > length must be rejected");
        assert_eq!(o.buf_offset, off);
    }
    // Extreme but still non-wrapping offsets.
    for off in [usize::MAX / 2, usize::MAX / 2 + 1, usize::MAX - 1] {
        let o = diff(&Case::from_str("123").length(3).offset(off));
        assert_eq!(o.ret, 0);
        assert_eq!(o.buf_offset, off);
    }
}

/* ================================================================= C06 ==== */

#[test]
fn c06_offset_size_max_wraps() {
    // `can_access_at_index` computes `offset + index`, which wraps in C.
    for length in [0usize, 1, 5, usize::MAX, usize::MAX - 1] {
        let o = diff(&Case::from_str("123").length(length).offset(usize::MAX));
        assert_eq!(o.ret, 0, "SIZE_MAX offset must be rejected (length={length})");
        assert_eq!(o.buf_offset, usize::MAX);
    }
}

/* ================================================================= C07 ==== */

#[test]
fn c07_every_rejected_first_byte() {
    let mut rng = Rng::new(0xC07);
    for b in 0u16..256 {
        let b = b as u8;
        if ACCEPTED.contains(&b) {
            continue;
        }
        for len in 1usize..=8 {
            let mut bytes = vec![b];
            for _ in 1..len {
                bytes.push(*rng.pick(ACCEPTED));
            }
            let o = diff(&Case::from_bytes(&bytes));
            assert_eq!(o.ret, 0, "byte {b:#04x} must hit `default:` and reject");
            assert_eq!(o.buf_offset, 0);
        }
    }
}

/* ================================================================= C08 ==== */

#[test]
fn c08_every_single_byte() {
    // Covers every arm of the switch, one byte at a time.
    for b in 0u16..256 {
        let bytes = [b as u8];
        diff(&Case::from_bytes(&bytes));
        // Same byte, but with a guard past `length`.
        diff(&Case::from_bytes(&bytes).length(1).with_guard(b"9999999"));
    }
}

/* ============================================================ C09 / C10 === */

#[test]
fn c09_single_accepted_byte_no_decimal_point() {
    for s in [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "+", "-", "e", "E",
    ] {
        let o = diff_str(s);
        if s.len() == 1 && s.as_bytes()[0].is_ascii_digit() {
            assert_eq!(o.ret, 1, "{s:?} must parse");
            assert_eq!(o.buf_offset, 1);
        } else {
            assert_eq!(o.ret, 0, "{s:?} must not parse");
            assert_eq!(o.buf_offset, 0);
        }
    }
}

#[test]
fn c10_single_dot_sets_has_decimal_point() {
    let o = diff_str(".");
    assert_eq!(o.ret, 0);
    assert_eq!(o.buf_offset, 0);
}

/* ================================================================= C11 ==== */

#[test]
fn c11_random_plain_integers() {
    let mut rng = Rng::new(0xC11);
    for _ in 0..3000 {
        let n = rng.range(1, 10) as usize;
        let s = rng.digits(n);
        let o = diff_str(&s);
        assert_eq!(o.ret, 1, "{s:?}");
        assert_eq!(o.buf_offset, s.len());
    }
}

/* ================================================================= C12 ==== */

#[test]
fn c12_random_negative_integers() {
    let mut rng = Rng::new(0xC12);
    for _ in 0..3000 {
        let n = rng.range(1, 12) as usize;
        let s = format!("-{}", rng.digits(n));
        let o = diff_str(&s);
        assert_eq!(o.ret, 1, "{s:?}");
    }
}

/* ================================================================= C13 ==== */

#[test]
fn c13_random_plus_signed_integers() {
    let mut rng = Rng::new(0xC13);
    for _ in 0..3000 {
        let n = rng.range(1, 12) as usize;
        let s = format!("+{}", rng.digits(n));
        diff_str(&s);
    }
}

/* ================================================================= C14 ==== */

#[test]
fn c14_random_decimals() {
    let mut rng = Rng::new(0xC14);
    for _ in 0..5000 {
        let ip = rng.range(1, 12) as usize;
        let fp = rng.range(1, 20) as usize;
        let sign = if rng.bool() {
            ""
        } else if rng.bool() {
            "-"
        } else {
            "+"
        };
        let s = format!("{sign}{}.{}", rng.digits(ip), rng.digits(fp));
        let o = diff_str(&s);
        assert_eq!(o.ret, 1, "{s:?}");
    }
}

/* ================================================================= C15 ==== */

#[test]
fn c15_leading_and_trailing_decimal_point() {
    let mut rng = Rng::new(0xC15);
    for _ in 0..2000 {
        let n = rng.range(1, 18) as usize;
        let sign = *rng.pick(&["", "-", "+"]);
        let a = format!("{sign}.{}", rng.digits(n));
        let b = format!("{sign}{}.", rng.digits(n));
        diff_str(&a);
        diff_str(&b);
    }
    for s in [".5", "-.5", "+.5", "5.", "-5.", "+5.", ".0", "0.", "..", ".."] {
        diff_str(s);
    }
}

/* ================================================================= C16 ==== */

#[test]
fn c16_exponent_forms() {
    let mut rng = Rng::new(0xC16);
    for _ in 0..5000 {
        let m = rng.range(1, 8) as usize;
        let e = rng.range(1, 3) as usize;
        let ec = *rng.pick(&['e', 'E']);
        let es = *rng.pick(&["", "+", "-"]);
        let s = format!("{}{}{}{}", rng.digits(m), ec, es, rng.digits(e));
        let o = diff_str(&s);
        assert_eq!(o.ret, 1, "{s:?}");
    }
}

/* ================================================================= C17 ==== */

#[test]
fn c17_full_grammar_random() {
    let mut rng = Rng::new(0xC17);
    for _ in 0..8000 {
        let sign = *rng.pick(&["", "-", "+"]);
        let ip = rng.below(10) as usize;
        let fp = rng.below(22) as usize;
        let dot = if rng.below(4) == 0 { "" } else { "." };
        let expo = if rng.bool() {
            String::new()
        } else {
            format!(
                "{}{}{}",
                rng.pick(&['e', 'E']),
                rng.pick(&["", "+", "-"]),
                rng.digits_between(1, 3)
            )
        };
        let s = format!("{sign}{}{dot}{}{expo}", rng.digits(ip), rng.digits(fp));
        diff_str(&s);
    }
}

/* ================================================================= C18 ==== */

#[test]
fn c18_partial_strtod_consumption() {
    // Accepted by the scanner, only partly consumed by strtod:
    // `offset` must advance by `after_end - number_c_string`, not by the scan.
    let fixed = [
        "1e", "1e+", "1e-", "1.2e", "1.2e+", "12--3", "1+2", "1.2.3", "1-2", "5ee5", "3EE",
        "0e", "0.e", ".5e", "7E+", "9-", "9+", "1.2.3.4", "1e5e5", "--1", "++1", "1..2",
        "12e++3", "8.8.8e8", "0-0", "1E", "1E-", "6.e", "6.e-",
    ];
    for s in fixed {
        diff_str(s);
    }

    let mut rng = Rng::new(0xC18);
    for _ in 0..8000 {
        // Random strings drawn only from the accepted alphabet -> the scanner
        // always consumes everything, and strtod decides where to stop.
        let n = rng.range(1, 24) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        diff(&Case::from_bytes(&bytes));
    }
}

/* ================================================================= C19 ==== */

#[test]
fn c19_scan_stops_at_unaccepted_byte() {
    for s in [
        "123abc", "1.5}", "7,", "-2 ", "0]", "42:", "3.14\"", "1e5x", "0\0", "5\n", "9\t",
        "1234567890abcdefg", "-0.0/", "6}}", "8|8",
    ] {
        diff_str(s);
    }

    let mut rng = Rng::new(0xC19);
    let stoppers: Vec<u8> = (0u16..256)
        .map(|b| b as u8)
        .filter(|b| !ACCEPTED.contains(b))
        .collect();
    for _ in 0..8000 {
        let pre = rng.range(1, 14) as usize;
        let post = rng.range(1, 8) as usize;
        let mut bytes: Vec<u8> = (0..pre).map(|_| *rng.pick(ACCEPTED)).collect();
        bytes.push(*rng.pick(&stoppers));
        for _ in 0..post {
            bytes.push(rng.next_u64() as u8);
        }
        diff(&Case::from_bytes(&bytes));
    }
}

/* ================================================================= C20 ==== */

#[test]
fn c20_scan_bound_is_length_not_nul() {
    // A valid number that runs exactly to `length`, followed in the same
    // allocation by bytes that must never be read.
    let mut rng = Rng::new(0xC20);
    for _ in 0..4000 {
        let n = rng.range(1, 18) as usize;
        let num: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        let guard_len = rng.range(1, 16) as usize;
        let guard: Vec<u8> = (0..guard_len).map(|_| *rng.pick(ACCEPTED)).collect();
        let o = diff(&Case::from_bytes(&num).length(n).with_guard(&guard));
        // Cross-check: the same prefix with the guard truncated away must give
        // the identical answer if `length` really bounds the scan.
        let o2 = diff(&Case::from_bytes(&num));
        assert_eq!(o.ret, o2.ret);
        assert_eq!(o.buf_offset, o2.buf_offset);
        assert_eq!(o.valuedouble_bits, o2.valuedouble_bits);
    }
    // Explicit non-NUL-terminated digits.
    let o = diff(&Case::from_bytes(b"12").length(2).with_guard(b"34"));
    assert_eq!(o.ret, 1);
    assert_eq!(o.valueint, 12);
    assert_eq!(o.buf_offset, 2);
}

/* ================================================================= C21 ==== */

#[test]
fn c21_number_embedded_mid_buffer() {
    let mut rng = Rng::new(0xC21);
    let stoppers: Vec<u8> = (0u16..256)
        .map(|b| b as u8)
        .filter(|b| !ACCEPTED.contains(b))
        .collect();
    for _ in 0..6000 {
        let junk_len = rng.range(1, 10) as usize;
        let mut bytes: Vec<u8> = (0..junk_len).map(|_| *rng.pick(&stoppers)).collect();
        let start = bytes.len();
        let n = rng.range(1, 20) as usize;
        for _ in 0..n {
            bytes.push(*rng.pick(ACCEPTED));
        }
        let tail = rng.below(10) as usize;
        for _ in 0..tail {
            bytes.push(rng.next_u64() as u8);
        }
        let length = bytes.len();
        let o = diff(&Case::from_bytes(&bytes).length(length).offset(start));
        assert!(o.buf_offset >= start);
    }
}

/* ============================================================ C22 / C23 === */

#[test]
fn c22_saturate_at_int_max() {
    for s in [
        "2147483647",
        "2147483647.0",
        "2147483647.5",
        "2147483648",
        "2147483649",
        "1e10",
        "9007199254740993",
        "4294967296",
        "1e18",
        "2147483646.9999999999999999",
    ] {
        let o = diff_str(s);
        assert_eq!(o.ret, 1, "{s:?}");
    }
    let mut rng = Rng::new(0xC22);
    for _ in 0..4000 {
        // Random values >= 2^31 in a variety of spellings.
        let v = 2147483647u64 + rng.below(1u64 << 40);
        for s in [
            format!("{v}"),
            format!("{v}.{}", rng.digits_between(1, 18)),
            format!("{v}e{}", rng.below(4)),
        ] {
            let o = diff_str(&s);
            assert_eq!(o.ret, 1, "{s:?}");
            assert_eq!(o.valueint, i32::MAX, "{s:?} must saturate to INT_MAX");
        }
    }
}

#[test]
fn c23_saturate_at_int_min() {
    for s in [
        "-2147483648",
        "-2147483648.0",
        "-2147483648.0000001",
        "-2147483649",
        "-1e10",
        "-9007199254740993",
        "-4294967296",
        "-1e18",
    ] {
        let o = diff_str(s);
        assert_eq!(o.ret, 1, "{s:?}");
        assert_eq!(o.valueint, i32::MIN, "{s:?}");
    }
    let mut rng = Rng::new(0xC23);
    for _ in 0..4000 {
        let v = 2147483648u64 + rng.below(1u64 << 40);
        for s in [
            format!("-{v}"),
            format!("-{v}.{}", rng.digits_between(1, 18)),
            format!("-{v}e{}", rng.below(4)),
        ] {
            let o = diff_str(&s);
            assert_eq!(o.ret, 1, "{s:?}");
            assert_eq!(o.valueint, i32::MIN, "{s:?} must saturate to INT_MIN");
        }
    }
}

/* ================================================================= C24 ==== */

#[test]
fn c24_just_inside_the_saturation_bounds() {
    for s in [
        "2147483646",
        "2147483646.5",
        "2147483646.9999999999",
        "-2147483647",
        "-2147483647.5",
        "-2147483647.9999999999",
        "0",
        "-0",
        "+0",
        "0.0",
        "-0.0",
        "0e0",
        "-0e0",
        "1",
        "-1",
        "0.5",
        "-0.5",
        "0.9999999999999999",
        "-0.9999999999999999",
    ] {
        let o = diff_str(s);
        assert_eq!(o.ret, 1, "{s:?}");
    }
    // Randomized truncation-toward-zero sweep on both signs.
    let mut rng = Rng::new(0xC24);
    for _ in 0..8000 {
        let mag = rng.below(2147483647);
        let frac = rng.digits_between(1, 18);
        for s in [format!("{mag}.{frac}"), format!("-{mag}.{frac}")] {
            let o = diff_str(&s);
            assert_eq!(o.ret, 1, "{s:?}");
        }
    }
}

/* ============================================================ C25 / C26 === */

#[test]
fn c25_overflow_to_positive_infinity() {
    let mut cases: Vec<String> = vec![
        "1e309".into(),
        "1e400".into(),
        "1e999".into(),
        "1e99999".into(),
        "1.7976931348623159e308".into(),
        "2e308".into(),
    ];
    cases.push("9".repeat(400));
    cases.push(format!("{}e{}", "9".repeat(30), "9".repeat(4)));
    for s in &cases {
        let o = diff_str(s);
        assert_eq!(o.ret, 1, "{s:?}");
        assert_eq!(o.valueint, i32::MAX, "{s:?}");
        assert_eq!(f64::from_bits(o.valuedouble_bits), f64::INFINITY, "{s:?}");
    }
    let mut rng = Rng::new(0xC25);
    for _ in 0..2000 {
        let s = format!("{}e{}", rng.digits_between(1, 6), rng.range(309, 99999));
        diff_str(&s);
    }
}

#[test]
fn c26_overflow_to_negative_infinity() {
    let mut cases: Vec<String> = vec![
        "-1e309".into(),
        "-1e400".into(),
        "-1e999".into(),
        "-1e99999".into(),
        "-2e308".into(),
    ];
    cases.push(format!("-{}", "9".repeat(400)));
    for s in &cases {
        let o = diff_str(s);
        assert_eq!(o.ret, 1, "{s:?}");
        assert_eq!(o.valueint, i32::MIN, "{s:?}");
        assert_eq!(
            f64::from_bits(o.valuedouble_bits),
            f64::NEG_INFINITY,
            "{s:?}"
        );
    }
    let mut rng = Rng::new(0xC26);
    for _ in 0..2000 {
        let s = format!("-{}e{}", rng.digits_between(1, 6), rng.range(309, 99999));
        diff_str(&s);
    }
}

/* ================================================================= C27 ==== */

#[test]
fn c27_underflow_and_denormals() {
    for s in [
        "1e-309",
        "1e-320",
        "1e-323",
        "1e-324",
        "1e-400",
        "1e-99999",
        "4.9e-324",
        "2.4703282292062327e-324",
        "5e-324",
        "-1e-400",
        "-4.9e-324",
        "-1e-320",
        "2.2250738585072011e-308",
        "2.2250738585072014e-308",
    ] {
        let o = diff_str(s);
        assert_eq!(o.ret, 1, "{s:?}");
        assert_eq!(o.valueint, 0, "{s:?}");
    }
    let mut rng = Rng::new(0xC27);
    for _ in 0..3000 {
        let sign = *rng.pick(&["", "-"]);
        let s = format!(
            "{sign}{}e-{}",
            rng.digits_between(1, 6),
            rng.range(300, 99999)
        );
        diff_str(&s);
    }
}

/* ================================================================= C28 ==== */

#[test]
fn c28_rounding_sensitive_mantissas() {
    for s in [
        "0.1",
        "0.2",
        "0.3",
        "1.7976931348623157e308",
        "9007199254740993",
        "9007199254740992.5",
        "0.500000000000000055511151231257827021181583404541015625",
        "2.00000000000000011102230246251565404236316680908203125",
        "1.000000000000000055511151231257827021181583404541015625",
        "123456789012345678901234567890",
        "1.2345678901234567890123456789e-5",
        "8.98846567431158e307",
        "0.99999999999999994",
        "0.99999999999999995",
        "1.0000000000000002",
    ] {
        let o = diff_str(s);
        assert_eq!(o.ret, 1, "{s:?}");
    }
    let mut rng = Rng::new(0xC28);
    for _ in 0..6000 {
        let ndig = rng.range(17, 30) as usize;
        let s = format!(
            "{}{}.{}e{}{}",
            rng.pick(&["", "-"]),
            rng.digits_between(1, 3),
            rng.digits(ndig),
            rng.pick(&["", "-", "+"]),
            rng.below(320)
        );
        let o = diff_str(&s);
        assert_eq!(o.ret, 1, "{s:?}");
    }
}

/* ================================================================= C29 ==== */

#[test]
fn c29_very_long_accepted_runs() {
    let mut rng = Rng::new(0xC29);
    for n in [4096usize, 8192, 10000] {
        // long digit run
        let s: String = (0..n).map(|_| rng.digit() as char).collect();
        let o = diff_str(&s);
        assert_eq!(o.ret, 1);
        assert_eq!(o.buf_offset, n);

        // long zero run
        let s = "0".repeat(n);
        let o = diff_str(&s);
        assert_eq!(o.ret, 1);
        assert_eq!(o.buf_offset, n);

        // long fraction
        let s = format!("0.{}", "0".repeat(n - 2));
        diff_str(&s);

        // long exponent digits
        let s = format!("1e{}", "1".repeat(n - 2));
        diff_str(&s);

        // long run of purely-accepted junk
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        diff(&Case::from_bytes(&s));
    }
}

/* ================================================================= C30 ==== */

#[test]
fn c30_very_long_with_decimal_points() {
    let mut rng = Rng::new(0xC30);
    for n in [4096usize, 10000] {
        // thousands of '.' -> replacement loop runs over the whole buffer
        let s = ".".repeat(n);
        let o = diff_str(&s);
        assert_eq!(o.ret, 0, "all dots must fail to parse");

        let s = format!("1{}", ".".repeat(n - 1));
        diff_str(&s);

        let s: String = (0..n)
            .map(|_| if rng.bool() { '.' } else { rng.digit() as char })
            .collect();
        diff_str(&s);
    }
}

/* ================================================================= C31 ==== */

#[test]
fn c31_depth_is_preserved() {
    let mut rng = Rng::new(0xC31);
    for depth in [0usize, 1, 2, 1000, usize::MAX, usize::MAX - 1] {
        for s in ["123", "abc", "", "1.5e3", "+"] {
            let case = Case::from_str(s).depth(depth);
            let o = diff(&case);
            assert_eq!(o.buf_depth, depth, "depth must be untouched");
        }
    }
    for _ in 0..1000 {
        let depth = rng.next_u64() as usize;
        let n = rng.range(1, 12) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.pick(ACCEPTED)).collect();
        let o = diff(&Case::from_bytes(&bytes).depth(depth));
        assert_eq!(o.buf_depth, depth);
    }
}

/* ================================================================= C32 ==== */

#[test]
fn c32_item_overwrite_and_preservation() {
    let mut rng = Rng::new(0xC32);
    for _ in 0..3000 {
        let mut case = if rng.bool() {
            // likely-success input
            Case::from_str(&format!("{}", rng.digits_between(1, 9)))
        } else {
            // likely-failure input
            Case::from_bytes(&[rng.next_u64() as u8])
        };
        case.item_type = rng.next_u64() as i32;
        case.item_valueint = rng.next_u64() as i32;
        case.item_valuedouble_bits = rng.next_u64();
        let o = diff(&case);
        if o.ret == 0 {
            assert_eq!(o.type_, case.item_type, "failure must not touch item");
            assert_eq!(o.valueint, case.item_valueint);
            assert_eq!(o.valuedouble_bits, case.item_valuedouble_bits);
        } else {
            assert_eq!(o.type_, 1 << 3, "success must set cJSON_Number");
        }
    }
}

/* ================================================================= C33 ==== */

#[test]
fn c33_streaming_multiple_numbers_one_buffer() {
    // Composed pipeline: feed the advanced offset straight back in, exactly like
    // a real cJSON array parse would.
    let mut rng = Rng::new(0xC33);
    for _ in 0..1500 {
        let count = rng.range(2, 8) as usize;
        let mut text = String::new();
        for i in 0..count {
            if i > 0 {
                text.push(*rng.pick(&[',', ' ', ']', '}', ':', 'x']));
            }
            let sign = *rng.pick(&["", "-", "+"]);
            let ip = rng.digits_between(1, 8);
            let frac = if rng.bool() {
                format!(".{}", rng.digits_between(1, 8))
            } else {
                String::new()
            };
            let ex = if rng.below(3) == 0 {
                format!("e{}{}", rng.pick(&["", "-", "+"]), rng.below(30))
            } else {
                String::new()
            };
            text.push_str(&format!("{sign}{ip}{frac}{ex}"));
        }
        let bytes = text.as_bytes().to_vec();
        let length = bytes.len();

        // Independently drive C and Rust through the whole stream, comparing at
        // each step.
        let mut c_off = 0usize;
        let mut r_off = 0usize;
        for _step in 0..(count * 3 + 4) {
            let c_case = Case::from_bytes(&bytes).length(length).offset(c_off);
            let r_case = Case::from_bytes(&bytes).length(length).offset(r_off);
            let c = observe_c(&c_case);
            let r = observe_rust(&r_case);
            assert_eq!(
                (c.ret, c.type_, c.valueint, c.valuedouble_bits, c.buf_offset),
                (r.ret, r.type_, r.valueint, r.valuedouble_bits, r.buf_offset),
                "stream divergence in {text:?} at c_off={c_off} r_off={r_off}"
            );
            if c.ret == 0 {
                // skip one delimiter and continue
                c_off += 1;
                r_off += 1;
            } else {
                assert!(c.buf_offset > c_off, "offset must make progress");
                c_off = c.buf_offset;
                r_off = r.buf_offset;
            }
            if c_off >= length {
                break;
            }
        }
    }
}

/* ================================================================= C34 ==== */

#[test]
fn c34_fuzz_biased_toward_number_bytes() {
    let mut rng = Rng::new(0xC34);
    for _ in 0..200_000 {
        let n = rng.below(40) as usize;
        let mut bytes = Vec::with_capacity(n);
        for _ in 0..n {
            if rng.below(10) < 9 {
                bytes.push(*rng.pick(ACCEPTED));
            } else {
                bytes.push(rng.next_u64() as u8);
            }
        }
        let length = if rng.below(20) == 0 {
            rng.below((n + 3) as u64) as usize
        } else {
            n
        };
        let offset = if rng.below(6) == 0 {
            rng.below((n + 3) as u64) as usize
        } else {
            0
        };
        let mut case = Case::from_bytes(&bytes).length(length).offset(offset);
        case.depth = rng.next_u64() as usize;
        diff(&case);
    }
}

/* ================================================================= C35 ==== */

#[test]
fn c35_fuzz_uniform_bytes() {
    let mut rng = Rng::new(0xC35);
    for _ in 0..50_000 {
        let n = rng.below(48) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
        let length = rng.below((n + 2) as u64) as usize;
        let offset = rng.below((n + 2) as u64) as usize;
        diff(&Case::from_bytes(&bytes).length(length).offset(offset));
    }
}
