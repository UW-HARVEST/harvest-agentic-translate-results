//! Level 2: the `strtod` conversion, the decimal-point substitution, the
//! parse-error path, and the `valueint` saturation logic.

mod common;

use common::Harness;

const WELL_FORMED: &[&str] = &[
    "0", "-0", "+0", "1", "-1", "+1", "42", "-42", "0000", "0001", "1234567890", "2147483647",
    "2147483648", "-2147483648", "-2147483649", "4294967295", "9223372036854775807",
    "0.0", "-0.0", "0.5", "-0.5", ".5", "-.5", "+.5", "5.", "-5.", "3.14159265358979",
    "1e0", "1E0", "1e1", "1e-1", "1e+1", "1E+10", "1e-10", "1e308", "1e309", "1e-308",
    "1e-323", "1e-324", "-1e308", "-1e309", "2.2250738585072014e-308", "4.9406564584124654e-324",
    "1.7976931348623157e308", "1.7976931348623159e308", "1e400", "-1e400", "1e-400",
    "123456789012345678901234567890", "0.000000000000000000001",
    "1.0000000000000002", "1.0000000000000001", "0.1", "0.2", "0.3",
    "9007199254740993", "9007199254740992", "18446744073709551616",
    "2147483646.9", "2147483647.5", "-2147483647.5", "-2147483648.5", "-2147483648.9",
    "0e0", "0e999", "-0e999", "0.0e-999",
];

/// Inputs where the accepted-character scan produces something `strtod` only
/// partially consumes, or rejects outright.
const PARTIAL_OR_INVALID: &[&str] = &[
    "", "+", "-", ".", "e", "E", "+-", "-+", "++", "--", "..", ".e", "e.", "e1", "E1",
    "+e", "-e", "1.2.3", "1e2e3", "1e+2+3", "1--2", "1++", "1..", "1.e", "1e.", "1ee1",
    "-.", "+.", ".-", ".+", "1-", "1+", "0x10", "12e", "12e+", "12e-", "12E-",
    "-", "3.", ".0.", "e-1", "-e-1", "1.5e", "1.5e+", "..1", "...", "+.e-",
];

#[test]
fn well_formed_numbers_match() {
    let h = Harness::new();
    for s in WELL_FORMED {
        h.check(s.as_bytes());
    }
}

#[test]
fn partial_and_invalid_inputs_match() {
    let h = Harness::new();
    for s in PARTIAL_OR_INVALID {
        h.check(s.as_bytes());
    }
}

#[test]
fn trailing_garbage_is_not_consumed() {
    let h = Harness::new();
    for s in WELL_FORMED {
        for suffix in ["", ",", "]", "}", " ", "\t", "\n", "abc", "\0", "\u{7f}"] {
            let mut v = s.as_bytes().to_vec();
            v.extend_from_slice(suffix.as_bytes());
            h.check(&v);
        }
    }
}

#[test]
fn offset_advances_only_by_strtod_consumption() {
    let h = Harness::new();
    // Cases where the scanned run is longer than what strtod accepts, so the
    // resulting offset delta is strictly less than the scan length.
    for s in [
        "1.2.3", "1e2e3", "12e", "12e+", "1--2", "1++", "3.5+", "7e-", "0.1.2.3", "5.5e5e5",
    ] {
        h.check(s.as_bytes());
        h.check_all_lengths(s.as_bytes());
    }
}

#[test]
fn valueint_saturation_boundaries() {
    let h = Harness::new();
    for s in [
        "2147483646", "2147483647", "2147483648", "2147483647.0000001", "2146999999",
        "-2147483647", "-2147483648", "-2147483649", "-2147483648.0000001",
        "1e10", "-1e10", "1e300", "-1e300", "1e400", "-1e400",
        "2147483646.999999999", "-2147483647.999999999",
        "0.9", "-0.9", "1.9", "-1.9", "0.0000001", "-0.0000001",
    ] {
        h.check(s.as_bytes());
    }
}

#[test]
fn very_long_numeric_runs() {
    let h = Harness::new();
    let long_digits = "9".repeat(5000);
    let h_ref = &h;
    h_ref.check(long_digits.as_bytes());

    let long_zeros = format!("{}1", "0".repeat(5000));
    h_ref.check(long_zeros.as_bytes());

    let long_frac = format!("0.{}", "1".repeat(5000));
    h_ref.check(long_frac.as_bytes());

    let long_exp = format!("1e{}", "9".repeat(500));
    h_ref.check(long_exp.as_bytes());

    let long_neg_exp = format!("1e-{}", "9".repeat(500));
    h_ref.check(long_neg_exp.as_bytes());

    // A run made only of accepted-but-meaningless characters.
    let junk = "+-eE.".repeat(1000);
    h_ref.check(junk.as_bytes());
}

#[test]
fn all_accepted_character_pairs_and_triples() {
    let h = Harness::new();
    const ACCEPTED: &[u8] = b"0123456789+-eE.";
    for &a in ACCEPTED {
        for &b in ACCEPTED {
            h.check(&[a, b]);
            for &c in ACCEPTED {
                h.check(&[a, b, c]);
            }
        }
    }
}
