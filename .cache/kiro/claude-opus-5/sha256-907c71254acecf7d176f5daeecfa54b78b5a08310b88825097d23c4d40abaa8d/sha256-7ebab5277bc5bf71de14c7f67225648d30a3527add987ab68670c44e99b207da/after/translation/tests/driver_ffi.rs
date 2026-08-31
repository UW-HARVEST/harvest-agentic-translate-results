//! Differential test of `void driver(const char *in)`, which exercises the
//! internal `parse_val` / `strtol` path and then calls `run` twice.

mod common;

use common::{assert_same, call_driver, check_driver};

/// Inputs covering every branch of `parse_val`:
///   * plain decimals, signs, leading zeros
///   * leading whitespace (all six `isspace` bytes)
///   * trailing junk after a valid prefix (`endp != str`, still accepted)
///   * no conversion at all (`endp == str`)
///   * `long` overflow -> `ERANGE`
///   * in-`long` but out-of-`int` values
///   * the exact `INT_MIN` / `INT_MAX` boundaries
const CASES: &[&[u8]] = &[
    // --- straightforward decimals ------------------------------------------
    b"0",
    b"1",
    b"-1",
    b"+1",
    b"7",
    b"42",
    b"-42",
    b"+42",
    b"007",
    b"-007",
    b"+0",
    b"-0",
    b"1000000",
    b"-1000000",
    // --- int boundaries -----------------------------------------------------
    b"2147483647",
    b"-2147483648",
    b"2147483648",  // > INT_MAX, fits in long -> rejected
    b"-2147483649", // < INT_MIN, fits in long -> rejected
    b"+2147483647",
    b"4294967296",
    // --- long overflow (ERANGE) --------------------------------------------
    b"9223372036854775806",
    b"9223372036854775807",
    b"9223372036854775808",
    b"-9223372036854775807",
    b"-9223372036854775808",
    b"-9223372036854775809",
    b"99999999999999999999999999",
    b"-99999999999999999999999999",
    // --- leading whitespace, all isspace bytes -----------------------------
    b"   12",
    b"\t12",
    b"\n12",
    b"\x0b12",
    b"\x0c12",
    b"\r12",
    b" \t\n\r\x0b\x0c-13",
    b"   ",
    b"\t",
    b"  +8",
    // --- valid prefix, trailing junk ---------------------------------------
    b"12abc",
    b"12 34",
    b"3.9",
    b"1e5",
    b"0x10",
    b"-5xyz",
    b"9,000",
    b"12\n",
    b"5-3",
    // --- no conversion ------------------------------------------------------
    b"",
    b"abc",
    b"-",
    b"+",
    b".",
    b"--1",
    b"++1",
    b"- 1",
    b"+ 1",
    b"x1",
    b" -",
    b"\x80\x81", // non-ASCII bytes
    b"/",
    b":",
    b"e",
    b"\x1f9",  // byte just below space: not isspace
    b"\x0e9",  // byte just above \r: not isspace
    // --- repeats, to confirm the accumulated static state matches ----------
    b"1",
    b"0",
    b"garbage",
    b"-3",
];

#[test]
fn driver_matches_c() {
    for (i, input) in CASES.iter().enumerate() {
        check_driver(
            input,
            &format!("driver({:?}) [call #{i}]", String::from_utf8_lossy(input)),
        );
    }
}

/// Every single byte value as a one-character input, plus that byte followed by
/// a digit. This nails down the `isspace` set and the sign handling without
/// guessing which bytes matter.
#[test]
fn driver_matches_c_for_every_leading_byte() {
    for b in 1u8..=255 {
        check_driver(&[b], &format!("driver({:?})", b as char));
        check_driver(&[b, b'9'], &format!("driver({:?}, '9')", b as char));
        check_driver(&[b, b'-', b'9'], &format!("driver({:?}, \"-9\")", b as char));
    }
}

/// Long digit strings, to make sure the overflow accumulator does not diverge
/// from `strtol` for inputs far past `LONG_MAX`.
#[test]
fn driver_matches_c_for_long_digit_strings() {
    for len in [18usize, 19, 20, 21, 40, 200] {
        for sign in ["", "-", "+"] {
            for fill in [b'9', b'1', b'0'] {
                let mut s = sign.as_bytes().to_vec();
                s.extend(std::iter::repeat_n(fill, len));
                check_driver(&s, &format!("driver({:?})", String::from_utf8_lossy(&s)));

                // A leading 1 followed by zeros: powers of ten straddling the
                // `long` cutoff.
                let mut s = sign.as_bytes().to_vec();
                s.push(b'1');
                s.extend(std::iter::repeat_n(b'0', len));
                check_driver(&s, &format!("driver({:?})", String::from_utf8_lossy(&s)));
            }
        }
    }
}

/// Values straddling `LONG_MAX` / `LONG_MIN` one unit at a time, where the
/// `cutoff`/`cutlim` overflow test in the translation is most delicate.
#[test]
fn driver_matches_c_near_long_boundaries() {
    for delta in -3i128..=3 {
        for (base, sign) in [
            (i128::from(i64::MAX), ""),
            (i128::from(i64::MIN), ""),
            (i128::from(u64::MAX), ""),
        ] {
            let v = base + delta;
            let s = format!("{sign}{v}");
            check_driver(
                s.as_bytes(),
                &format!("driver({s:?}) (near long boundary)"),
            );
        }
    }
}

/// Exhaustive sweep over all 3-byte inputs from an alphabet of the bytes that
/// drive `strtol`'s state machine.
#[test]
fn driver_matches_c_over_short_input_alphabet() {
    const ALPHABET: &[u8] = b"0129+- \t.xa";

    for &a in ALPHABET {
        for &b in ALPHABET {
            for &c in ALPHABET {
                let input = [a, b, c];
                check_driver(
                    &input,
                    &format!("driver({:?})", String::from_utf8_lossy(&input)),
                );
            }
        }
    }
}

/// The output for a rejected input must be exactly the error line, and nothing
/// else, in both implementations.
#[test]
fn rejected_input_prints_only_the_error_line() {
    let long_digits = vec![b'1'; 30];
    let inputs: [&[u8]; 6] = [b"abc", b"", b"-", b"+", b"2147483648", &long_digits];
    for input in inputs {
        let (c_out, rust_out) = call_driver(input);
        assert_same(
            &format!("driver({:?})", String::from_utf8_lossy(input)),
            &c_out,
            &rust_out,
        );
        assert_eq!(
            c_out,
            b"An error occurred\n",
            "unexpected C output for {:?}",
            String::from_utf8_lossy(input)
        );
    }
}
