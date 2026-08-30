//! Differential tests: run the C executable and the Rust executable as
//! subprocesses on the same stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! What the C program actually branches on
//! ---------------------------------------
//! `main` is `scanf("%f", &x); driver(x);` with `x` pre-initialised to `0.f`,
//! and `driver` just hexdumps the 4 raw bytes of the float. So every input
//! class comes from `scanf("%f", ...)`:
//!
//!   * EOF / whitespace-only  -> no conversion, `x` keeps its initial `0.f`
//!   * matching failure       -> no conversion, `x` keeps its initial `0.f`
//!     (return value is ignored, so a failure is silent: still exit 0)
//!   * decimal forms          -> `[+-]digits[.digits][e[+-]digits]`
//!   * leading/trailing dot   -> `.5`, `5.`, `5.e3`
//!   * hex forms              -> `0x` hexdigits `[.hexdigits][p[+-]digits]`
//!   * infinities             -> `inf`, `infinity` (any case, optional sign)
//!   * NaNs                   -> `nan`, `nan(n-char-sequence)`
//!   * partial prefixes       -> `1e`, `1e+`, `0x`, `0x1p`, `infi`, `nan(`
//!   * range errors           -> overflow to +/-inf, underflow to 0/subnormal
//!   * `%f` skips leading whitespace and reads ACROSS newlines, and stops at
//!     the first byte that cannot extend the subject sequence.
//!
//! Every one of those classes has a test below.

mod common;
use common::{assert_all_same, assert_same};

// ---------------------------------------------------------------------------
// Phase B: the classes the C program branches on
// ---------------------------------------------------------------------------

#[test]
fn empty_input_no_conversion() {
    // EOF before any input item: scanf returns EOF, x stays 0.f.
    assert_same(b"");
}

#[test]
fn whitespace_only_inputs() {
    // %f skips whitespace, then hits EOF: still no conversion.
    assert_all_same([
        &b" "[..],
        b"\n",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b"   ",
        b"\t\x0b\x0c\r\n ",
        b"\n\n\n",
        b"   \n  ",
    ]);
}

#[test]
fn single_item_happy_path() {
    assert_all_same([
        &b"0"[..], b"1", b"-1", b"+1", b"1.5", b"-1.5", b"0.0", b"2", b"42", b"3.25",
    ]);
}

#[test]
fn signed_zeros() {
    // -0.0 must keep its sign bit: 00000080 vs 00000000.
    assert_all_same([&b"0"[..], b"-0", b"+0", b"0.0", b"-0.0", b"-0.0e10", b"-0x0p0"]);
}

#[test]
fn leading_whitespace_is_skipped_and_newlines_crossed() {
    // scanf (unlike fgets) reads across newlines while skipping whitespace.
    assert_all_same([
        &b"   42"[..],
        b"\n\n\t 3.25",
        b"\n\n\n\n7",
        b"\t\t-8.5",
        b"\r\n1.25",
        b"\x0b\x0c9",
    ]);
}

#[test]
fn stops_at_first_non_matching_byte() {
    // Trailing junk after a complete number is left unread; only the number
    // is converted.
    assert_all_same([
        &b"1 2"[..],
        b"1\n2",
        b"1abc",
        b"2.5xyz",
        b"3,4",
        b"1/3",
        b"5;",
        b"1.5.5",
        b"1e5e5",
        b"0x1p2p3",
    ]);
}

#[test]
fn matching_failure_leaves_x_at_zero() {
    // No digits at all: matching failure, x keeps 0.f, exit status still 0.
    assert_all_same([
        &b"abc"[..],
        b"x",
        b".",
        b"+",
        b"-",
        b"++1",
        b"- 1",
        b"e5",
        b"E",
        b".e5",
        b"/",
        b"?",
        b"z9",
        b"--",
        b"+.",
        b"-.",
        b"+-1",
    ]);
}

#[test]
fn dot_forms() {
    assert_all_same([
        &b".5"[..], b"-.5", b"+.5", b"5.", b"-5.", b"5.e3", b"0.", b".0", b"0.e5", b".25e2",
        b"5.e-3",
    ]);
}

#[test]
fn exponent_forms_including_truncated_ones() {
    // glibc backs out of an incomplete exponent and keeps the mantissa:
    // "1e" and "1e+" both convert to 1.0.
    assert_all_same([
        &b"1e"[..],
        b"1e+",
        b"1e-",
        b"1E",
        b"1e5",
        b"1e+5",
        b"1E-5",
        b"1e05",
        b"1e0",
        b"2.5e",
        b"2.5e+",
        b"1e+e",
        b"1ex",
    ]);
}

#[test]
fn hex_forms_including_truncated_ones() {
    assert_all_same([
        &b"0x1"[..],
        b"0X1",
        b"0x1p4",
        b"0x1P4",
        b"0x1.8p1",
        b"-0x1.8p1",
        b"0x.8p1",
        b"0x8.",
        b"0x8.p0",
        b"0xabcdef",
        b"0xABCDEF",
        b"0x1p+2",
        b"0x1P-2",
        b"0x0p0",
        b"0x10p-4",
    ]);
}

#[test]
fn hex_prefix_without_digits_falls_back_to_zero() {
    // "0x" is not a valid hex float, so glibc converts just the leading "0".
    assert_all_same([
        &b"0x"[..], b"0X", b"0xg", b"0x."[..].as_ref(), b"0x.g", b"-0x", b"0x1g", b"0x1p", b"0x1p+",
        b"0x1p-", b"0x1px", b"0x.p1",
    ]);
}

#[test]
fn signed_bare_hex_prefix_loses_its_sign() {
    // Regression: glibc treats a buffer that is nothing but "[sign]0x" as a
    // MATCHING FAILURE, so x keeps its initial +0.f and "-0x" prints
    // 00000000 -- not the -0.0 (00000080) that strtof("-0x") would give.
    // One '.' after the prefix is enough to make glibc convert again.
    assert_all_same([
        &b"-0x"[..], b"-0X", b"-0xg", b"-0xp", b"-0xx", b"-0x ", b"-0x\n", b"+0x", b"0x",
        // ...but these all convert, keeping the sign:
        b"-0x.", b"-0X.", b"-0x.g", b"-0x.p", b"-0x..", b"-0x0", b"-00x", b"-0e", b"-0.x",
    ]);
}

#[test]
fn infinity_forms() {
    assert_all_same([
        &b"inf"[..],
        b"INF",
        b"Inf",
        b"iNf",
        b"-inf",
        b"+inf",
        b"infinity",
        b"INFINITY",
        b"iNfInItY",
        b"-infinity",
        b"+infinity",
        b"infinityy",
        b"inf inity",
        b"infx",
    ]);
}

#[test]
fn truncated_infinity_prefixes_are_matching_failures() {
    // Once glibc's scanf has committed to "infinity" it cannot fall back to
    // the shorter "inf", so these are matching failures (x stays 0.f).
    assert_all_same([
        &b"infi"[..], b"infin", b"infini", b"infinit", b"i", b"in", b"inx", b"if", b"-infi",
        b"INFI", b"infinit9",
    ]);
}

#[test]
fn nan_forms() {
    assert_all_same([
        &b"nan"[..],
        b"NAN",
        b"NaN",
        b"nAn",
        b"-nan",
        b"+nan",
        b"nan()",
        b"nan(123)",
        b"nan(abc_9)",
        b"nan(0x1)",
        b"-nan(1)",
        b"nan(",
        b"nan(abc",
        b"nan(a b)",
        b"nan(-)",
        b"nanx",
        b"na",
        b"n",
        b"nax",
    ]);
}

#[test]
fn overflow_and_underflow_range_errors() {
    assert_all_same([
        &b"1e39"[..],
        b"1e40",
        b"-1e40",
        b"1e999",
        b"-1e999",
        b"1e-999",
        b"1e-46",
        b"1e-45",
        b"1e-40",
        b"3.4028235e38",  // FLT_MAX
        b"3.4028236e38",  // just over FLT_MAX -> inf
        b"1.1754944e-38", // FLT_MIN
        b"1.4e-45",       // smallest subnormal
        b"7e-46",         // rounds to smallest subnormal
        b"1e-50",
        b"0x1p128",
        b"0x1p-149",
        b"0x1p-150",
        b"0x1p-1000",
        b"0x1p99999999999999999999",
        b"0x1p-99999999999999999999",
        b"1e99999999999999999999",
        b"1e-99999999999999999999",
    ]);
}

#[test]
fn rounding_boundaries() {
    // f32 ties-to-even and double-rounding traps.
    assert_all_same([
        &b"16777216"[..],
        b"16777217",
        b"16777215.5",
        b"8388609.5",
        b"33554434",
        b"33554435",
        b"33554436",
        b"1.00000005960464477539063",
        b"1.00000011920928955078125",
        b"1.000000178813934326171875",
        b"0x1.000001p0",
        b"0x1.0000010000001p0",
        b"0x1.000002p0",
        b"0x1.0000008p0",
        b"0x1.0000018p0",
        b"0x1.fffffep127",
        b"0x1.ffffffp127",
        b"0x1.fffffe0000001p127",
        b"0x1.8p-149",
        b"0x1.000000000000000000001p-149",
        b"0x0.000001p-126",
        b"0.1",
        b"0.2",
        b"0.3",
        b"3.14159265358979",
        b"2.718281828459045",
        b"5e-324",
        b"1.7976931348623157e308",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: paths not covered above
// ---------------------------------------------------------------------------

#[test]
fn long_inputs_beyond_any_internal_buffer() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push([b"1".as_ref(), &vec![b'0'; 10_000]].concat());
    cases.push([b"0.".as_ref(), &vec![b'0'; 10_000], b"1".as_ref()].concat());
    cases.push([b"1.".as_ref(), &vec![b'9'; 20_000], b"e-5".as_ref()].concat());
    cases.push([b"0x1.".as_ref(), &vec![b'f'; 5_000], b"p0".as_ref()].concat());
    cases.push([b"0x".as_ref(), &vec![b'1'; 5_000], b"p-20000".as_ref()].concat());
    cases.push([vec![b' '; 5_000], b"2.5".to_vec()].concat());
    cases.push(vec![b'\n'; 5_000]);
    cases.push([b"1e".as_ref(), &vec![b'9'; 100]].concat());
    cases.push([b"1e-".as_ref(), &vec![b'9'; 100]].concat());
    cases.push([b"nan(".as_ref(), &vec![b'a'; 5_000], b")".as_ref()].concat());
    cases.push([&vec![b'0'; 5_000], b"1.5".as_ref()].concat());
    assert_all_same(cases);
}

#[test]
fn leading_zero_runs() {
    assert_all_same([
        &b"000000000000000000000000001"[..],
        b"0000000000",
        b"00.5",
        b"-000.125",
        b"0000e5",
        b"0x0000000001p0",
    ]);
}

#[test]
fn huge_literal_digit_strings() {
    assert_all_same([
        &b"1234567890123456789012345678901234567890"[..],
        b"0.000000000000000000000000000000000000000001",
        b"340282346638528859811704183484516925440",
        b"340282356779733661637539395458142568448",
        b"100000000000000000000000000000000000000000",
        b"-1e38",
        b"0x1.fffffffffffffp1023",
        b"0x10000000000000000000p0",
        b"0x1.0000000000000000001p0",
    ]);
}

#[test]
fn embedded_nul_and_non_ascii_bytes() {
    assert_all_same([
        &b"\x001"[..],
        b"1\x002",
        b"\xff\xfe",
        b"\xc3\xa91",
        b"1\xc3\xa9",
        b"\x80",
        b"\x7f",
    ]);
}

#[test]
fn all_single_bytes() {
    // Every possible first byte: the whole first-character branch of the DFA.
    let cases: Vec<Vec<u8>> = (0u16..=255).map(|b| vec![b as u8]).collect();
    assert_all_same(cases);
}

#[test]
fn every_byte_after_a_digit() {
    // Second-character branch after "1", "0", "1.", "1e", "0x", "inf", "nan".
    let prefixes: [&[u8]; 9] = [
        b"1", b"0", b"1.", b"1e", b"1e+", b"0x", b"0x1", b"inf", b"nan",
    ];
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for p in prefixes {
        for b in 0u16..=255 {
            let mut v = p.to_vec();
            v.push(b as u8);
            cases.push(v);
        }
    }
    assert_all_same(cases);
}

#[test]
fn exhaustive_prefixes_of_infinity_and_nan_words() {
    let words: [&[u8]; 6] = [b"infinity", b"INFINITY", b"nan(abc)", b"NAN(1)", b"inf", b"nan"];
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for w in words {
        for n in 0..=w.len() {
            cases.push(w[..n].to_vec());
            cases.push([b"-".as_ref(), &w[..n]].concat());
        }
    }
    assert_all_same(cases);
}

#[test]
fn randomized_token_soup() {
    // Deterministic pseudo-random byte strings over the alphabet the float
    // grammar cares about, to shake out DFA transitions no hand-written case
    // reaches.
    const ALPHA: &[u8] = b"0123456789abcdefABCDEFxXpPeE+-. \t\n()_infINFnaNty/,;";
    let mut rng: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for _ in 0..1500 {
        let len = (next() % 13) as usize;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(ALPHA[(next() % ALPHA.len() as u64) as usize]);
        }
        cases.push(v);
    }
    assert_all_same(cases);
}

#[test]
fn randomized_float_round_trips() {
    // Random f32 bit patterns rendered as decimal and as C99 hex floats, plus
    // random decimal/hex literals: checks the conversion, not just the parse.
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut cases: Vec<Vec<u8>> = Vec::new();

    for _ in 0..400 {
        let f = f32::from_bits(next() as u32);
        if f.is_finite() {
            cases.push(format!("{f:?}").into_bytes());
            cases.push(format!("{f:.30e}").into_bytes());
            cases.push(format!("{:.60}", f as f64).into_bytes());
        }
        let d = f64::from_bits(next());
        if d.is_finite() {
            cases.push(format!("{d:?}").into_bytes());
        }
    }
    for _ in 0..400 {
        // decimal literal: [-]digits[.digits][e[+-]dd]
        let mut s = String::new();
        if next() % 3 == 0 {
            s.push('-');
        }
        for _ in 0..=(next() % 25) {
            s.push((b'0' + (next() % 10) as u8) as char);
        }
        if next() % 10 < 7 {
            s.push('.');
            for _ in 0..(next() % 25) {
                s.push((b'0' + (next() % 10) as u8) as char);
            }
        }
        if next() % 10 < 7 {
            s.push('e');
            s.push(if next() % 2 == 0 { '+' } else { '-' });
            s.push_str(&(next() % 60).to_string());
        }
        cases.push(s.into_bytes());

        // hex literal: [-]0x hexdigits [.hexdigits][p[+-]dd]
        let mut h = String::new();
        if next() % 3 == 0 {
            h.push('-');
        }
        h.push_str("0x");
        for _ in 0..=(next() % 20) {
            h.push(std::char::from_digit((next() % 16) as u32, 16).unwrap());
        }
        if next() % 10 < 7 {
            h.push('.');
            for _ in 0..(next() % 20) {
                h.push(std::char::from_digit((next() % 16) as u32, 16).unwrap());
            }
        }
        if next() % 10 < 8 {
            h.push('p');
            h.push(if next() % 2 == 0 { '+' } else { '-' });
            h.push_str(&(next() % 200).to_string());
        }
        cases.push(h.into_bytes());
    }
    assert_all_same(cases);
}

#[test]
fn exhaustive_short_inputs() {
    // Every string of length 1..=3 over the alphabet the float grammar reacts
    // to. This reaches every state of the subject-sequence DFA and every way
    // of leaving it early, without needing to guess which combinations matter.
    const A: &[u8] = b"019afxXpPeE+-.intyN() \n_";
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for a in A {
        cases.push(vec![*a]);
        for b in A {
            cases.push(vec![*a, *b]);
            for c in A {
                cases.push(vec![*a, *b, *c]);
            }
        }
    }
    assert_all_same(cases);
}

#[test]
fn exhaustive_length_four_over_core_alphabet() {
    const A: &[u8] = b"01xp.e-in";
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for a in A {
        for b in A {
            for c in A {
                for d in A {
                    cases.push(vec![*a, *b, *c, *d]);
                }
            }
        }
    }
    assert_all_same(cases);
}

#[test]
fn exact_decimal_midpoints_between_adjacent_floats() {
    // The hardest rounding inputs: decimal literals that land exactly halfway
    // between two neighbouring f32 values, where the result depends on
    // ties-to-even, plus the same values nudged just off the tie.
    //
    // A midpoint of two adjacent f32s needs 25 significand bits, so it is
    // exact in f64, and Rust's `{:.N}` formatting prints f64 exactly.
    let mut rng: u64 = 0xD1B5_4A32_D192_ED03;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for _ in 0..1200 {
        let bits = (next() as u32) & 0x7fff_ffff;
        let lo = f32::from_bits(bits);
        let hi = f32::from_bits(bits + 1);
        if !lo.is_finite() || !hi.is_finite() {
            continue;
        }
        let mid = (lo as f64 + hi as f64) / 2.0;
        // 170 fraction digits covers even 2^-150 exactly.
        let s = format!("{mid:.170}");
        let s = s.trim_end_matches('0').to_string();
        cases.push(s.clone().into_bytes());
        cases.push(format!("-{s}").into_bytes());
        cases.push(format!("{s}1").into_bytes()); // just above the tie
        cases.push(format!("-{s}1").into_bytes());
    }
    assert_all_same(cases);
}

#[test]
fn exact_float_values_and_hex_round_trips() {
    // Every random f32 printed exactly in decimal and in C99 hex-float form
    // must come back as the identical bit pattern.
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for _ in 0..1200 {
        let f = f32::from_bits(next() as u32);
        if !f.is_finite() {
            continue;
        }
        cases.push(format!("{f:.170}").into_bytes());
        cases.push(format!("{f:e}").into_bytes());
        // C99 hex float, built from the raw fields: 0x1.<mantissa>p<exp>
        let bits = f.to_bits();
        let sign = if bits >> 31 == 1 { "-" } else { "" };
        let biased = ((bits >> 23) & 0xff) as i32;
        let frac = bits & 0x7f_ffff;
        let (lead, exp) = if biased == 0 { (0, -126) } else { (1, biased - 127) };
        cases.push(format!("{sign}0x{lead}.{frac:06x}p{exp}").into_bytes());
    }
    assert_all_same(cases);
}

#[test]
fn subnormal_and_overflow_boundary_literals() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    // Walk the whole subnormal ladder and the normal/subnormal frontier.
    for k in 0..48 {
        let f = f32::from_bits(k);
        cases.push(format!("{f:.170}").into_bytes());
        cases.push(format!("-{f:.170}").into_bytes());
        cases.push(format!("0x{k:x}p-149").into_bytes());
    }
    for k in 0..24u32 {
        let f = f32::from_bits(0x7f7f_ffff - k); // just under FLT_MAX
        cases.push(format!("{f:.1}").into_bytes());
        cases.push(format!("{f:e}").into_bytes());
    }
    // The exact overflow threshold: halfway between FLT_MAX and 2^128.
    cases.push(b"340282356779733661637539395458142568447".to_vec());
    cases.push(b"340282356779733661637539395458142568448".to_vec());
    cases.push(b"340282356779733661637539395458142568449".to_vec());
    cases.push(b"-340282356779733661637539395458142568448".to_vec());
    // The exact underflow threshold: half of the smallest subnormal.
    cases.push(b"0x1p-150".to_vec());
    cases.push(b"0x1.0000000000001p-150".to_vec());
    cases.push(b"0x0.8p-149".to_vec());
    cases.push(b"-0x1p-150".to_vec());
    assert_all_same(cases);
}

#[test]
fn output_shape_matches_expected_hexdump() {
    // Pins printf("%02x") formatting: exactly 8 lowercase hex digits plus one
    // trailing newline, nothing on stderr, exit 0.
    let c = common::run(common::c_bin(), b"1.0");
    let r = common::run(&common::rust_bin(), b"1.0");
    assert_eq!(c.stdout, b"0000803f\n".to_vec());
    assert_eq!(r.stdout, c.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.code, Some(0));
    assert_eq!(r.code, Some(0));
}
