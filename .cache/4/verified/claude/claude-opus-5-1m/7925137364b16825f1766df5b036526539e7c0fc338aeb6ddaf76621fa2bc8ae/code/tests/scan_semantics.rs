// Phase B — high-resolution probes for the `%d` conversion semantics.
//
// WHY THIS FILE EXISTS
// --------------------
// The program's observable surface only reveals whether the converted value is
// *equal* to 1 (x), 2 (y) or 3 (z) — a 3-bit oracle per run.  Two different
// conversion semantics therefore agree on almost every input by coincidence:
// e.g. glibc's "saturate at LONG_MAX, then narrow to int" and a naive
// "wrap mod 2^64, then narrow" both yield a non-magic value for
// "99999999999999999999", so a plain random sweep cannot tell them apart.
//
// The inputs below are engineered so that each *plausible wrong* semantics maps
// onto one of the magic constants while the correct glibc semantics does not
// (or vice versa).  A divergence therefore always shows up as a different
// message, not as an invisible value difference.  Validated with a mutant
// battery (see run_all_configs.sh --mutants): every probe class kills the
// corresponding mutant.
//
// glibc reference semantics for `%d` (no width, base 10):
//   digits -> strtol()  => saturates at LONG_MAX / LONG_MIN and sets ERANGE
//                       => the (ignored) result is then stored via `*(int *)`,
//                          i.e. narrowed modulo 2^32.
// So: LONG_MAX -> -1, LONG_MIN -> 0, and in-range values wrap mod 2^32.

mod common;

use common::*;

const OK: &str = "Ok!\nResult: 0\n";
const R1: &str = "Error: x != 1\nOperation failed\nResult: 1\n";
const R2: &str = "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
const R3: &str = "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";

/// Kills "wrap mod 2^64 instead of saturating at LONG_MAX": the values below are
/// larger than LONG_MAX but ≡ 1 / 2 / 3 (mod 2^32), so a wrapping implementation
/// would accept them while glibc yields LONG_MAX -> -1.
#[test]
fn positive_overflow_is_saturated_not_wrapped() {
    // 2^64 + {1,2,3}
    assert_same_and_expect(b"18446744073709551617 2 3", R1, "2^64+1 must NOT become 1");
    assert_same_and_expect(b"1 18446744073709551618 3", R2, "2^64+2 must NOT become 2");
    assert_same_and_expect(b"1 2 18446744073709551619", R3, "2^64+3 must NOT become 3");

    // Higher powers of two, all ≡ 1 (mod 2^32) and all > LONG_MAX.
    let probes = [
        "79228162514264337593543950337",                        // 2^96 + 1
        "340282366920938463463374607431768211457",               // 2^128 + 1
        "36893488147419103233",                                  // 2*2^64 + 1
        "18446744073709551617000000000000000000000000000000001", // huge, ≡1 mod 2^32
    ];
    for p in probes {
        assert_same_and_expect(format!("{p} 2 3").as_bytes(), R1, "huge ≡1 mod 2^32 must saturate");
        assert_same_and_expect(format!("1 {p} 3").as_bytes(), R2, "huge in y slot");
        assert_same_and_expect(format!("1 2 {p}").as_bytes(), R3, "huge in z slot");
    }
}

/// Kills "wrap mod 2^64 instead of saturating at LONG_MIN": these are smaller
/// than LONG_MIN but ≡ 1 / 2 / 3 (mod 2^32) once wrapped.
#[test]
fn negative_overflow_is_saturated_not_wrapped() {
    assert_same_and_expect(b"-18446744073709551615 2 3", R1, "-(2^64-1) must NOT become 1");
    assert_same_and_expect(b"1 -18446744073709551614 3", R2, "-(2^64-2) must NOT become 2");
    assert_same_and_expect(b"1 2 -18446744073709551613", R3, "-(2^64-3) must NOT become 3");
    // -(2^96 - 1) ≡ 1 (mod 2^32) when wrapped.
    assert_same_and_expect(
        b"-79228162514264337593543950335 2 3",
        R1,
        "-(2^96-1) must saturate to LONG_MIN -> 0",
    );
}

/// Kills "saturate at INT_MAX/INT_MIN instead of narrowing mod 2^32":
/// 2^32 + {1,2,3} are inside `long` range, so glibc narrows them onto the magic
/// constants and the program prints "Ok!".
#[test]
fn in_long_range_values_are_narrowed_not_clamped() {
    assert_same_and_expect(b"4294967297 4294967298 4294967299", OK, "2^32+{1,2,3} narrow onto 1,2,3");
    assert_same_and_expect(b"-4294967295 -4294967294 -4294967293", OK, "negatives narrow onto 1,2,3");
    // LONG_MAX itself narrows to -1 (not INT_MAX), LONG_MIN narrows to 0.
    assert_same_and_expect(b"9223372036854775807 2 3", R1, "LONG_MAX -> -1");
    assert_same_and_expect(b"-9223372036854775808 2 3", R1, "LONG_MIN -> 0");
    // ...and the largest values still inside LONG range that narrow onto 1/2/3:
    // 2147483646 * 2^32 = 9223372028264841216, so +1/+2/+3 are ≡ 1/2/3 (mod 2^32)
    // and all are < LONG_MAX (9223372036854775807).
    assert_same_and_expect(b"9223372028264841217 2 3", OK, "largest in-range ≡1 mod 2^32");
    assert_same_and_expect(
        b"9223372028264841217 9223372028264841218 9223372028264841219",
        OK,
        "same for y and z",
    );
    // One step past LONG_MAX must flip to the saturated result.
    assert_same_and_expect(b"9223372036854775808 2 3", R1, "LONG_MAX+1 saturates");
    assert_same_and_expect(b"-9223372036854775809 2 3", R1, "LONG_MIN-1 saturates");
}

/// Kills "give up / saturate once the digit string is longer than N digits":
/// arbitrarily long zero padding must not change the value.
#[test]
fn unbounded_leading_zeros_keep_the_value() {
    for pad in [1usize, 19, 20, 64, 1000, 8192, 100_000] {
        let z = "0".repeat(pad);
        assert_same_and_expect(
            format!("{z}1 {z}2 {z}3").as_bytes(),
            OK,
            "long zero padding keeps 1/2/3",
        );
        assert_same_and_expect(
            format!("+{z}1 -{z}2 {z}3").as_bytes(),
            R2,
            "signed long zero padding (y = -2)",
        );
    }
    // A padded value that is still an overflow after the zeros are skipped.
    let z = "0".repeat(5000);
    assert_same_and_expect(
        format!("{z}18446744073709551617 2 3").as_bytes(),
        R1,
        "padding + overflow still saturates",
    );
}

/// Kills "stop the conversion early / consume too much": the byte immediately
/// after a number must be left for the next directive, and the magic constants
/// must be recognised only when the *whole* token converts to them.
#[test]
fn token_boundaries_are_exact() {
    // "12 3" is x=12, not x=1 followed by 2.
    assert_same_and_expect(b"12 3 4", R1, "no digit splitting");
    // Adjacent digits without a separator form one token.
    assert_same_and_expect(b"123", R1, "single token 123");
    // A '+'/'-' immediately after a number terminates that token and starts the
    // next one — no whitespace is required between the directives.
    assert_same_and_expect(b"1-2-3", R2, "1 then '-2' -> y = -2 rejected");
    assert_same_and_expect(b"1+2+3", OK, "1 then '+2' then '+3' all convert");
    // Non-digit terminators leave the byte unconsumed.
    assert_same_and_expect(b"1,2,3", R2, "comma terminates and then fails");
    assert_same_and_expect(b"1 2 3x", OK, "trailing letter after the last token");
    assert_same_and_expect(b"1 2 3-", OK, "trailing sign after the last token");
}

/// Kills "wrong whitespace set": every isspace() byte must separate tokens and
/// nothing else may.  A missing '\v'/'\f' (or an extra byte such as NBSP) shows
/// up as a different message.
#[test]
fn whitespace_set_is_exactly_c_isspace() {
    for &s in SPACES {
        let sep = s as char;
        assert_same_and_expect(
            format!("1{sep}2{sep}3").as_bytes(),
            OK,
            "isspace byte separates tokens",
        );
        assert_same_and_expect(
            format!("{sep}{sep}1{sep}{sep}2{sep}{sep}3").as_bytes(),
            OK,
            "runs of one isspace byte",
        );
    }
    // Bytes that are NOT isspace in the C locale must break the scan.
    for bad in [0x00u8, 0x01, 0x1c, 0x1d, 0x1e, 0x1f, 0x85, 0xa0, 0xff] {
        let input = vec![b'1', b' ', b'2', b' ', bad, b'3'];
        assert_same_and_expect(&input, R3, "non-isspace byte breaks the z conversion");
        let input = vec![bad, b'1', b' ', b'2', b' ', b'3'];
        assert_same_and_expect(&input, R1, "non-isspace byte breaks the x conversion");
    }
}

/// Randomized cross-check with a full model of the glibc semantics: for each
/// generated numeric spelling we predict the message from an independent model
/// and require BOTH programs to agree with it.  This raises the oracle's
/// resolution because the spellings are drawn to land on the magic constants
/// about half the time.
#[test]
fn randomized_model_cross_check() {
    fn glibc_scan_to_int(text: &str) -> i32 {
        // Independent model: parse as i128, saturate to i64, narrow to i32.
        let neg = text.starts_with('-');
        let digits: String = text.trim_start_matches(['+', '-']).to_string();
        let mut acc: i128 = 0;
        let mut sat = false;
        for c in digits.bytes() {
            if !sat {
                acc = acc * 10 + i128::from(c - b'0');
                if acc > i128::from(u64::MAX) {
                    sat = true;
                }
            }
        }
        let signed = if neg { -acc } else { acc };
        let clamped = signed.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        clamped as i32
    }

    let mut rng = Rng::new(0x5CA7);
    for _ in 0..600 {
        // Build a spelling that lands on {1,2,3} or near it, at a random scale.
        let spell = |rng: &mut Rng, magic: i64| -> String {
            let k = 1 + rng.below(3) as u32; // 2^(32k) multiples, k <= 3 (fits i128)
            let base: i128 = 1i128 << (32 * k);
            match rng.below(6) {
                0 => format!("{magic}"),
                1 => format!("{}", base * (1 + rng.below(3) as i128) + i128::from(magic)),
                2 => format!("-{}", base * (1 + rng.below(3) as i128) - i128::from(magic)),
                3 => format!("{}{}", "0".repeat(rng.below(30) as usize), magic),
                4 => format!("{}", i128::from(i64::MAX) + i128::from(magic)),
                _ => format!("{}", rng.next_i32()),
            }
        };
        let (sx, sy, sz) = (spell(&mut rng, 1), spell(&mut rng, 2), spell(&mut rng, 3));
        let input = format!("{sx} {sy} {sz}");

        let (x, y, z) = (
            glibc_scan_to_int(&sx),
            glibc_scan_to_int(&sy),
            glibc_scan_to_int(&sz),
        );
        let expected = if x != 1 {
            R1
        } else if y != 2 {
            R2
        } else if z != 3 {
            R3
        } else {
            OK
        };
        assert_same_and_expect(input.as_bytes(), expected, "model cross-check");
    }
}
