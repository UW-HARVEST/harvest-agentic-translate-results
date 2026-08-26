//! Phase B -- valid-path differential tests, one test per `CONFIGS.md` row
//! group (rows 5-30).
//!
//! These drive the *composed* pipeline end to end: the C executable
//! (`c_src/build/driver`) and the Rust executable (`target/<prof>/driver`) are
//! fed identical stdin and their stdout is compared byte-for-byte.  This is the
//! only channel that reaches `scanf("%f")`, which is reachable solely from
//! `main` in the C source.
//!
//! Every row uses many randomized inputs from a fixed-seed RNG.

mod common;

use common::{diff_all, exact_decimal, hex_literal, Rng, DEC, HEX};

fn signs() -> [&'static str; 3] {
    ["", "+", "-"]
}

/// CONFIGS.md row 5 -- plain decimal integers, no sign, no whitespace, EOF after.
#[test]
fn row05_plain_decimal_integers() {
    let rng = Rng::new(1_005);
    let mut c: Vec<String> = vec![
        "0".into(),
        "1".into(),
        "9".into(),
        "16777215".into(),
        "16777216".into(),
        "16777217".into(),
        "16777218".into(),
        "16777219".into(),
        "33554431".into(),
        "33554433".into(),
        "123456789012345678".into(),
    ];
    for _ in 0..600 {
        let n = 1 + rng.below(18);
        c.push(rng.digits(n, DEC));
    }
    diff_all("row05", c);
}

/// CONFIGS.md row 6 -- every leading-whitespace byte and long mixed runs.
#[test]
fn row06_leading_whitespace() {
    let ws: [&[u8]; 8] = [
        b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r", b"\n\n\n", b" \t\n\x0b\x0c\r ",
    ];
    let rng = Rng::new(1_006);
    let mut c: Vec<Vec<u8>> = Vec::new();
    for w in ws {
        for body in ["1.5", "-2.5e3", "0x1p4", "inf", "nan", "", "z"] {
            let mut v = w.to_vec();
            v.extend_from_slice(body.as_bytes());
            c.push(v);
        }
    }
    for _ in 0..400 {
        let n = rng.below(40);
        let mut v: Vec<u8> = (0..n)
            .map(|_| *rng.pick(&[b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r']))
            .collect();
        v.extend_from_slice(
            format!("{}{}.{}", rng.pick(&signs()), rng.digits(1 + rng.below(6), DEC), rng.digits(rng.below(6), DEC))
                .as_bytes(),
        );
        c.push(v);
    }
    // a very long whitespace run (crosses stdio's buffer boundary)
    let mut big = vec![b' '; 100_000];
    big.extend_from_slice(b"1.5");
    c.push(big);
    c.push(vec![b'\n'; 70_000]);
    diff_all("row06", c);
}

/// CONFIGS.md row 7 -- sign x `int.frac`.
#[test]
fn row07_sign_times_int_frac() {
    let rng = Rng::new(1_007);
    let mut c: Vec<String> = Vec::new();
    for s in signs() {
        for base in ["0.0", "1.5", "0.1", "3.14159265358979", "123456.789012345"] {
            c.push(format!("{s}{base}"));
        }
    }
    for _ in 0..700 {
        let s = *rng.pick(&signs());
        let i = rng.digits(1 + rng.below(12), DEC);
        let f = rng.digits(1 + rng.below(14), DEC);
        c.push(format!("{s}{i}.{f}"));
    }
    diff_all("row07", c);
}

/// CONFIGS.md row 8 -- `.frac` with no integer digits.
#[test]
fn row08_leading_point() {
    let rng = Rng::new(1_008);
    let mut c: Vec<String> = Vec::new();
    for s in signs() {
        for base in [".0", ".5", ".00000001", ".999999999999999999"] {
            c.push(format!("{s}{base}"));
        }
    }
    for _ in 0..500 {
        let s = *rng.pick(&signs());
        let f = rng.digits(1 + rng.below(20), DEC);
        let e = if rng.chance(1, 2) {
            format!("e{}{}", rng.pick(&signs()), rng.digits(1 + rng.below(2), DEC))
        } else {
            String::new()
        };
        c.push(format!("{s}.{f}{e}"));
    }
    diff_all("row08", c);
}

/// CONFIGS.md row 9 -- `int.` with no fraction digits.
#[test]
fn row09_trailing_point() {
    let rng = Rng::new(1_009);
    let mut c: Vec<String> = vec![
        "1.".into(),
        "-1.".into(),
        "0.".into(),
        "-0.".into(),
        "1.e5".into(),
        "1.E-5".into(),
        "-12345.e+7".into(),
    ];
    for _ in 0..400 {
        let s = *rng.pick(&signs());
        let i = rng.digits(1 + rng.below(10), DEC);
        let e = if rng.chance(1, 2) {
            format!("e{}{}", rng.pick(&signs()), rng.digits(1 + rng.below(2), DEC))
        } else {
            String::new()
        };
        c.push(format!("{s}{i}.{e}"));
    }
    diff_all("row09", c);
}

/// CONFIGS.md row 10 -- the `"0"` prefix special case in glibc's collector.
#[test]
fn row10_leading_zero_special_case() {
    let rng = Rng::new(1_010);
    let mut c: Vec<String> = vec![
        "0".into(),
        "00".into(),
        "000".into(),
        "0.5".into(),
        "000123".into(),
        "00x5".into(),
        "0e5".into(),
        "0E5".into(),
        "0.0e0".into(),
        "-0".into(),
        "-00".into(),
        "-0.0".into(),
        "+0".into(),
        "0x0".into(),
        "-0x0".into(),
        "0x00000000p0".into(),
    ];
    for _ in 0..400 {
        let s = *rng.pick(&signs());
        let zeros = "0".repeat(1 + rng.below(8));
        let rest = match rng.below(4) {
            0 => String::new(),
            1 => rng.digits(1 + rng.below(6), DEC),
            2 => format!(".{}", rng.digits(1 + rng.below(6), DEC)),
            _ => format!("x{}", rng.digits(1 + rng.below(6), HEX)),
        };
        c.push(format!("{s}{zeros}{rest}"));
    }
    diff_all("row10", c);
}

/// CONFIGS.md row 11 -- decimal exponents: `e`/`E` x sign x 1-3 digits.
#[test]
fn row11_decimal_exponent() {
    let rng = Rng::new(1_011);
    let mut c: Vec<String> = Vec::new();
    for s in signs() {
        for ec in ["e", "E"] {
            for es in signs() {
                for d in ["0", "1", "9", "38", "45", "126", "149", "400"] {
                    c.push(format!("{s}1.5{ec}{es}{d}"));
                }
            }
        }
    }
    for _ in 0..700 {
        let s = *rng.pick(&signs());
        let m = format!(
            "{}.{}",
            rng.digits(1 + rng.below(9), DEC),
            rng.digits(rng.below(9), DEC)
        );
        let ec = *rng.pick(&["e", "E"]);
        let es = *rng.pick(&signs());
        let d = rng.digits(1 + rng.below(3), DEC);
        c.push(format!("{s}{m}{ec}{es}{d}"));
    }
    diff_all("row11", c);
}

/// CONFIGS.md row 12 -- exponents with enough digits to hit the internal clamp.
#[test]
fn row12_huge_exponent_digit_counts() {
    let rng = Rng::new(1_012);
    let mut c: Vec<String> = vec![
        "1e1000000".into(),
        "1e1000001".into(),
        "1e-1000000".into(),
        "1e-1000001".into(),
        "1e999999".into(),
        "1e99999999999999999999".into(),
        "1e-99999999999999999999".into(),
        "-1e99999999999999999999".into(),
        format!("1e{}", "9".repeat(400)),
        format!("1e-{}", "9".repeat(400)),
        format!("0.{}1e{}", "0".repeat(50), "1".repeat(20)),
    ];
    for _ in 0..200 {
        let s = *rng.pick(&signs());
        let m = rng.digits(1 + rng.below(20), DEC);
        let es = *rng.pick(&["", "-", "+"]);
        let d = rng.digits(7 + rng.below(20), DEC);
        c.push(format!("{s}{m}e{es}{d}"));
    }
    diff_all("row12", c);
}

/// CONFIGS.md row 13 -- hex integers with no `p` exponent, mixed letter case.
#[test]
fn row13_hex_integers() {
    let rng = Rng::new(1_013);
    let mut c: Vec<String> = vec![
        "0x0".into(),
        "0x1".into(),
        "0xf".into(),
        "0xF".into(),
        "0X10".into(),
        "0xabcdef".into(),
        "0xABCDEF".into(),
        "0xffffff".into(),
        "0x1000000".into(),
        "0x1000001".into(),
        "-0xdeadbeef".into(),
    ];
    for _ in 0..600 {
        let s = *rng.pick(&signs());
        let p = *rng.pick(&["0x", "0X"]);
        let d = rng.digits(1 + rng.below(20), HEX);
        c.push(format!("{s}{p}{d}"));
    }
    diff_all("row13", c);
}

/// CONFIGS.md row 14 -- hex `int.frac` with a `p`/`P` exponent of either sign.
#[test]
fn row14_hex_with_binary_exponent() {
    let rng = Rng::new(1_014);
    let mut c: Vec<String> = Vec::new();
    for s in signs() {
        for pc in ["p", "P"] {
            for es in signs() {
                for d in ["0", "1", "4", "23", "24", "126", "127", "149", "150", "300"] {
                    c.push(format!("{s}0x1.8{pc}{es}{d}"));
                }
            }
        }
    }
    for _ in 0..900 {
        let s = *rng.pick(&signs());
        let p = *rng.pick(&["0x", "0X"]);
        let i = rng.digits(1 + rng.below(8), HEX);
        let f = if rng.chance(2, 3) {
            format!(".{}", rng.digits(1 + rng.below(10), HEX))
        } else {
            String::new()
        };
        let pc = *rng.pick(&["p", "P"]);
        let e = rng.range_i32(-200, 200);
        c.push(format!("{s}{p}{i}{f}{pc}{e:+}"));
    }
    diff_all("row14", c);
}

/// CONFIGS.md row 15 -- hex `.frac` only, and hex `int.` with no fraction.
#[test]
fn row15_hex_point_edges() {
    let rng = Rng::new(1_015);
    let mut c: Vec<String> = vec![
        "0x.8p1".into(),
        "-0x.8p1".into(),
        "0x.8".into(),
        "0X.FFFFFFp0".into(),
        "0x5.p2".into(),
        "-0x5.p2".into(),
        "0x5.".into(),
        "0x.0p0".into(),
        "0x.".into(),
        "-0x.".into(),
        "0x..".into(),
    ];
    for _ in 0..400 {
        let s = *rng.pick(&signs());
        let p = *rng.pick(&["0x", "0X"]);
        if rng.chance(1, 2) {
            let f = rng.digits(1 + rng.below(12), HEX);
            let e = rng.range_i32(-160, 160);
            c.push(format!("{s}{p}.{f}p{e:+}"));
        } else {
            let i = rng.digits(1 + rng.below(8), HEX);
            let e = rng.range_i32(-160, 160);
            c.push(format!("{s}{p}{i}.p{e:+}"));
        }
    }
    diff_all("row15", c);
}

/// CONFIGS.md row 16 -- hex significands wider than 128 bits (sticky-bit path).
#[test]
fn row16_very_wide_hex_significands() {
    let rng = Rng::new(1_016);
    let mut c: Vec<String> = vec![
        format!("0x{}p-160", "f".repeat(40)),
        format!("0x{}p0", "1".repeat(64)),
        format!("0x1{}p-256", "0".repeat(64)),
        format!("0x1{}1p-260", "0".repeat(64)),
    ];
    for _ in 0..500 {
        let s = *rng.pick(&signs());
        let n = 28 + rng.below(50);
        let i = rng.digits(n, HEX);
        let f = if rng.chance(1, 2) {
            format!(".{}", rng.digits(1 + rng.below(40), HEX))
        } else {
            String::new()
        };
        let e = rng.range_i32(-420, 220);
        c.push(format!("{s}0x{i}{f}p{e:+}"));
    }
    diff_all("row16", c);
}

/// CONFIGS.md row 17 -- the subnormal range, in decimal and in hex.
#[test]
fn row17_subnormal_range() {
    let rng = Rng::new(1_017);
    let mut c: Vec<String> = Vec::new();
    for _ in 0..400 {
        let mant = 1 + (rng.next_u32() & 0x007f_ffff) as u64;
        let neg = rng.chance(1, 2);
        c.push(hex_literal(neg, mant, -149));
        c.push(exact_decimal(neg, mant as u128, -149));
    }
    for e in -50i32..=-30 {
        for m in [1u32, 2, 3, 5, 7, 9, 14, 15, 17] {
            c.push(format!("{m}e{e}"));
            c.push(format!("-{m}e{e}"));
        }
    }
    diff_all("row17", c);
}

/// CONFIGS.md row 18 -- the subnormal/normal boundary and its half-way ties.
#[test]
fn row18_subnormal_normal_boundary() {
    let mut c: Vec<String> = Vec::new();
    for k in 1u64..=64 {
        for neg in [false, true] {
            c.push(hex_literal(neg, k, -149)); // exact subnormals
            c.push(hex_literal(neg, 2 * k + 1, -150)); // exact ties
            c.push(hex_literal(neg, 4 * k + 1, -151)); // just below the tie
            c.push(hex_literal(neg, 4 * k + 3, -151)); // just above the tie
            c.push(exact_decimal(neg, (2 * k + 1) as u128, -150));
            c.push(exact_decimal(neg, (4 * k + 1) as u128, -151));
        }
    }
    for e in -160i32..=-120 {
        for m in ["0x1", "0x1.8", "0x1.fffffe", "0x1.ffffff", "0x3"] {
            c.push(format!("{m}p{e}"));
            c.push(format!("-{m}p{e}"));
        }
    }
    for s in [
        "1.4e-45",
        "1.40129846432481707e-45",
        "7.00649232162408535e-46",
        "7.00649232162408534e-46",
        "7.00649232162408536e-46",
        "1e-45",
        "1e-46",
        "7e-46",
        "1.1754942e-38",
        "1.1754943e-38",
        "1.1754944e-38",
        "1.17549421069244107548702944485e-38",
    ] {
        c.push(s.to_string());
        c.push(format!("-{s}"));
    }
    diff_all("row18", c);
}

/// CONFIGS.md row 19 -- the max-normal / overflow boundary.
#[test]
fn row19_overflow_boundary() {
    let mut c: Vec<String> = Vec::new();
    for e in 120i32..=140 {
        for m in ["0x1", "0x1.fffffe", "0x1.ffffff", "0x1.fffffe1", "0x1.ffffffe"] {
            c.push(format!("{m}p{e}"));
            c.push(format!("-{m}p{e}"));
        }
    }
    for s in [
        "3.4028234e38",
        "3.4028235e38",
        "3.4028236e38",
        "340282346638528859811704183484516925440",
        "340282356779733661637539395458142568447",
        "340282356779733661637539395458142568448",
        "340282356779733661637539395458142568449",
        "1e38",
        "1e39",
        "1e400",
        "1.7014118e38",
    ] {
        c.push(s.to_string());
        c.push(format!("-{s}"));
    }
    // exact FLT_MAX and the exact tie above it
    let (_, mant, e) = common::decompose(f32::MAX);
    for neg in [false, true] {
        c.push(hex_literal(neg, mant, e));
        c.push(hex_literal(neg, 2 * mant + 1, e - 1));
        c.push(hex_literal(neg, 4 * mant + 1, e - 2));
        c.push(hex_literal(neg, 4 * mant + 3, e - 2));
        c.push(exact_decimal(neg, (2 * mant + 1) as u128, e - 1));
        c.push(exact_decimal(neg, (4 * mant + 1) as u128, e - 2));
    }
    diff_all("row19", c);
}

/// CONFIGS.md row 20 -- exact values, +-1 ulp neighbours, and the exact
/// half-way points between adjacent floats (ties-to-even in both directions),
/// expressed both as exact hex literals and as exact decimal expansions.
#[test]
fn row20_rounding_boundaries() {
    let rng = Rng::new(1_020);
    let mut c: Vec<String> = Vec::new();
    for _ in 0..220 {
        // any finite, non-max float
        let bits = rng.next_u32() & 0x7fff_ffff;
        let bits = if bits >= 0x7f7f_ffff { bits % 0x7f7f_ffff } else { bits };
        let x = f32::from_bits(bits);
        let (_, mant, e) = common::decompose(x);
        let neg = rng.chance(1, 2);
        c.push(hex_literal(neg, mant, e)); // exact
        c.push(hex_literal(neg, 2 * mant + 1, e - 1)); // exact tie
        c.push(hex_literal(neg, 4 * mant + 1, e - 2)); // below tie
        c.push(hex_literal(neg, 4 * mant + 3, e - 2)); // above tie
        c.push(exact_decimal(neg, mant as u128, e));
        c.push(exact_decimal(neg, (2 * mant + 1) as u128, e - 1));
        c.push(exact_decimal(neg, (4 * mant + 1) as u128, e - 2));
        c.push(exact_decimal(neg, (4 * mant + 3) as u128, e - 2));
        // a tie nudged upwards by appending a digit to its exact expansion
        c.push(format!(
            "{}1",
            exact_decimal(neg, (2 * mant + 1) as u128, e - 1)
        ));
    }
    diff_all("row20", c);
}

/// CONFIGS.md row 21 -- ordinary floats rendered in many textual forms.
#[test]
fn row21_many_textual_forms() {
    let rng = Rng::new(1_021);
    let mut c: Vec<String> = Vec::new();
    for _ in 0..500 {
        let bits = rng.next_u32();
        let x = f32::from_bits(bits);
        if !x.is_finite() {
            continue;
        }
        c.push(format!("{x}")); // shortest round-trip / expanded
        c.push(format!("{x:e}")); // scientific
        let prec = rng.below(24);
        c.push(format!("{x:.*e}", prec)); // fixed significant digits
        let (neg, mant, e) = common::decompose(x);
        c.push(hex_literal(neg, mant, e)); // exact hex
        c.push(exact_decimal(neg, mant as u128, e)); // exact decimal
    }
    diff_all("row21", c);
}

/// CONFIGS.md row 22 -- very long significands.
#[test]
fn row22_long_significands() {
    let rng = Rng::new(1_022);
    let mut c: Vec<Vec<u8>> = Vec::new();
    for _ in 0..120 {
        let i = rng.digits(200 + rng.below(2800), DEC);
        let f = rng.digits(rng.below(500), DEC);
        let e = rng.range_i32(-3200, 3200);
        c.push(format!("{i}.{f}e{e:+}").into_bytes());
    }
    for s in [
        format!("1{}", "0".repeat(100_000)),
        format!("0.{}1", "0".repeat(100_000)),
        "9".repeat(100_000),
        format!("0x{}p-40000", "f".repeat(10_000)),
        format!("{}e-3010", "1".repeat(3_000)),
        format!("-{}e-3010", "1".repeat(3_000)),
        format!("1e{}", "9".repeat(5_000)),
        format!("1e-{}", "9".repeat(5_000)),
        format!("0x1p{}", "9".repeat(5_000)),
        format!("0x1p-{}", "9".repeat(5_000)),
    ] {
        c.push(s.into_bytes());
    }
    diff_all("row22", c);
}

fn case_permutations(word: &str) -> Vec<String> {
    let bytes: Vec<u8> = word.bytes().collect();
    let n = bytes.len();
    let mut out = Vec::new();
    for mask in 0u32..(1u32 << n) {
        let s: String = bytes
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                if mask & (1 << i) != 0 {
                    b.to_ascii_uppercase() as char
                } else {
                    b.to_ascii_lowercase() as char
                }
            })
            .collect();
        out.push(s);
    }
    out
}

/// CONFIGS.md row 23 -- `inf` / `infinity` in every letter case, every sign.
#[test]
fn row23_infinity_words() {
    let mut c: Vec<String> = Vec::new();
    for w in case_permutations("inf") {
        for s in signs() {
            c.push(format!("{s}{w}"));
            c.push(format!("{s}{w}x"));
            c.push(format!("{s}{w} 5"));
        }
    }
    for w in case_permutations("infinity") {
        for s in signs() {
            c.push(format!("{s}{w}"));
        }
    }
    for s in signs() {
        c.push(format!("{s}infinityx"));
        c.push(format!("{s}infinity5"));
        c.push(format!("{s}INFINITY."));
    }
    diff_all("row23", c);
}

/// CONFIGS.md row 24 -- `nan` in every letter case, every sign, with and
/// without an `(n-char-sequence)` payload (which `scanf` must not consume).
#[test]
fn row24_nan_words() {
    let mut c: Vec<String> = Vec::new();
    for w in case_permutations("nan") {
        for s in signs() {
            for suffix in ["", "()", "(0)", "(1)", "(123)", "(0x1f)", "(abc)", "(", "x", " 5"] {
                c.push(format!("{s}{w}{suffix}"));
            }
        }
    }
    diff_all("row24", c);
}

/// CONFIGS.md row 25 -- what follows a valid number stops the scan.
#[test]
fn row25_trailing_content() {
    let rng = Rng::new(1_025);
    let mut c: Vec<String> = vec![
        "1.5 2.5".into(),
        "1.5\n2.5".into(),
        "1.5abc".into(),
        "1_000".into(),
        "1,5".into(),
        "1.5,".into(),
        "0x1p2q".into(),
        "1.5(".into(),
        "1.5)".into(),
        "1.5-2".into(),
        "1.5+2".into(),
        "-1.5-".into(),
        "12 34 56".into(),
        "inf inf".into(),
        "nan nan".into(),
    ];
    for _ in 0..300 {
        let head = format!(
            "{}{}.{}",
            rng.pick(&signs()),
            rng.digits(1 + rng.below(6), DEC),
            rng.digits(rng.below(6), DEC)
        );
        let tail: String = (0..1 + rng.below(4))
            .map(|_| *rng.pick(b"abzZ_,()+- \t\n.xXpPeE0123456789") as char)
            .collect();
        c.push(format!("{head}{tail}"));
    }
    diff_all("row25", c);
}

/// CONFIGS.md row 26 -- exponent characters without digits, and a second one.
#[test]
fn row26_exponent_without_digits() {
    let mut c: Vec<String> = Vec::new();
    for s in signs() {
        for base in ["1", "1.5", ".5", "0", "0x1", "0x1.8", "0x.8"] {
            let ec = if base.starts_with("0x") { "p" } else { "e" };
            c.push(format!("{s}{base}{ec}"));
            c.push(format!("{s}{base}{ec}+"));
            c.push(format!("{s}{base}{ec}-"));
            c.push(format!("{s}{base}{ec}+x"));
            c.push(format!("{s}{base}{ec}{ec}"));
            c.push(format!("{s}{base}{ec}1{ec}1"));
            c.push(format!("{s}{base}{ec}1{ec}"));
            c.push(format!("{s}{base}{ec}+1{ec}-1"));
            c.push(format!("{s}{base}{ec}1-1"));
            c.push(format!("{s}{base}{ec}1+1"));
            c.push(format!("{s}{base}{ec}+-1"));
            c.push(format!("{s}{base}{ec}-+1"));
        }
        // the "wrong" exponent character for the radix
        c.push(format!("{s}1p5"));
        c.push(format!("{s}0x1e5"));
        c.push(format!("{s}0x1E5"));
        c.push(format!("{s}0x1p2e3"));
    }
    diff_all("row26", c);
}

/// CONFIGS.md row 27 -- a second '.', and a '.' after the exponent.
#[test]
fn row27_repeated_decimal_points() {
    let mut c: Vec<String> = Vec::new();
    for s in signs() {
        for base in [
            "1.5.5", "1..5", "..5", "1.5.", "..", "...", "1.5e2.5", "1e5.5", "0x1.8.8p1",
            "0x1.8p1.5", ".5.5", "0..", "0.0.0",
        ] {
            c.push(format!("{s}{base}"));
        }
    }
    diff_all("row27", c);
}

/// CONFIGS.md row 28 -- property sweep over the whole alphabet the collection
/// state machine distinguishes.
#[test]
fn row28_random_alphabet_strings() {
    const ALPHA: &[u8] = b"0123456789abcdefxXpPeE.+-_,()infty NAT\t";
    let rng = Rng::new(1_028);
    let mut c: Vec<Vec<u8>> = Vec::new();
    for _ in 0..2500 {
        let n = rng.below(9);
        let mut v: Vec<u8> = Vec::new();
        if rng.chance(1, 3) {
            v.push(*rng.pick(b"+-"));
        }
        for _ in 0..n {
            v.push(*rng.pick(ALPHA));
        }
        c.push(v);
    }
    diff_all("row28", c);
}

/// CONFIGS.md row 29 -- exhaustive sweep over every string of length <= 4 from
/// the minimal alphabet that reaches every branch, with and without a sign.
#[test]
fn row29_exhaustive_short_strings() {
    const ALPHA: &[u8] = b"0x.pe1-+";
    let mut c: Vec<Vec<u8>> = Vec::new();
    for len in 0..=4usize {
        let total = ALPHA.len().pow(len as u32);
        for i in 0..total {
            let mut n = i;
            let mut body = Vec::with_capacity(len);
            for _ in 0..len {
                body.push(ALPHA[n % ALPHA.len()]);
                n /= ALPHA.len();
            }
            c.push(body.clone());
            let mut signed = vec![b'-'];
            signed.extend_from_slice(&body);
            c.push(signed);
        }
    }
    diff_all("row29", c);
}

/// CONFIGS.md row 30 -- random raw byte strings over the full 0..=255 range.
#[test]
fn row30_random_raw_bytes() {
    let rng = Rng::new(1_030);
    let mut c: Vec<Vec<u8>> = Vec::new();
    for _ in 0..2000 {
        let n = rng.below(17);
        c.push((0..n).map(|_| (rng.next_u32() & 0xff) as u8).collect());
    }
    diff_all("row30", c);
}
