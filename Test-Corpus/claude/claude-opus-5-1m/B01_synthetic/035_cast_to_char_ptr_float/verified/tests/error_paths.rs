//! Phase C — error-path differential tests.
//!
//! One test per ERRORS.md row. Each test
//!   1. constructs the exact invalid input / condition,
//!   2. runs BOTH the C program and the Rust program, and
//!   3. asserts they agree **and** that they produce the specific documented
//!      sentinel — not merely that "both failed somehow".
//!
//! The sentinel for every rejection in this program is the object
//! representation of the untouched initialiser `float x = 0.f`, i.e. stdout
//! `00000000\n` with exit status 0, because `main` discards `scanf`'s return
//! value. Rows that are *not* rejections (14–21) assert their own specific
//! non-zero sentinel, which is what makes them distinguishable from a
//! rejection at all.

mod common;

use common::corpus::fixed;
use common::{assert_same_all, c_exe, run_exe, rust_exe};

/// The bytes printed when `scanf` leaves `x` at its initialiser.
const ZERO: &str = "00000000\n";
const NEG_ZERO: &str = "00000080\n";
const POS_INF: &str = "0000807f\n";
const NEG_INF: &str = "000080ff\n";
const POS_QNAN: &str = "0000c07f\n";
const NEG_QNAN: &str = "0000c0ff\n";
const ONE: &str = "0000803f\n";

/// Assert that both programs agree *and* that the shared result is exactly
/// `expected` with exit status 0.
#[track_caller]
fn assert_both(inputs: &[&str], expected: &str, row: &str) {
    let c = c_exe();
    let r = rust_exe();
    for input in inputs {
        let co = run_exe(&c, input.as_bytes());
        let ro = run_exe(&r, input.as_bytes());
        assert_eq!(
            co, ro,
            "{row}: C and Rust disagree on {input:?}\n  C:    {co:?}\n  RUST: {ro:?}"
        );
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            expected,
            "{row}: unexpected sentinel for {input:?} (C produced {co:?})"
        );
        assert_eq!(co.code, Some(0), "{row}: exit status must be 0 for {input:?}");
    }
}

/// Row 1 — input failure: completely empty stdin.
#[test]
fn row01_empty_stdin_is_input_failure() {
    assert_both(&[""], ZERO, "row 1");
}

/// Row 2 — input failure: whitespace only, EOF while skipping it.
#[test]
fn row02_whitespace_only_is_input_failure() {
    let cases: Vec<&str> = fixed::WHITESPACE_ONLY.iter().copied().collect();
    assert_both(&cases, ZERO, "row 2");
}

/// Row 3 — matching failure: first non-whitespace byte cannot begin a float.
#[test]
fn row03_bad_first_byte_is_matching_failure() {
    assert_both(fixed::BAD_START, ZERO, "row 3");
    // and every byte that is neither whitespace, sign, digit, '.', nor the
    // first letter of inf/nan
    let bad: Vec<String> = (1u8..=127)
        .filter(|b| {
            let c = *b as char;
            !c.is_ascii_whitespace()
                && !c.is_ascii_digit()
                && c != '.'
                && c != '+'
                && c != '-'
                && !matches!(c.to_ascii_lowercase(), 'i' | 'n')
        })
        .map(|b| (b as char).to_string())
        .collect();
    let refs: Vec<&str> = bad.iter().map(|s| s.as_str()).collect();
    assert_both(&refs, ZERO, "row 3 (all bad first bytes)");
}

/// Row 4 — matching failure: a decimal point with no digit anywhere.
#[test]
fn row04_lone_point_is_matching_failure() {
    assert_both(fixed::BAD_POINT, ZERO, "row 4");
}

/// Row 5 — input failure: sign consumed, then EOF.
#[test]
fn row05_sign_then_eof_is_input_failure() {
    assert_both(&["-", "+"], ZERO, "row 5");
}

/// Row 6 — matching failure: sign followed by a byte that cannot continue.
#[test]
fn row06_sign_then_bad_byte_is_matching_failure() {
    assert_both(fixed::BAD_SIGN, ZERO, "row 6");
}

/// Row 7 — matching failure: `n` prefix not completed to `nan`.
#[test]
fn row07_incomplete_nan_is_matching_failure() {
    assert_both(&["nax", "n5", "nAx", "na5", "n.", "n-"], ZERO, "row 7");
}

/// Row 8 — input failure: `n` or `na` then EOF.
#[test]
fn row08_nan_prefix_then_eof() {
    assert_both(&["n", "N", "na", "NA", "nA", "-n", "-na", "+na"], ZERO, "row 8");
}

/// Row 9 — matching failure: `i` prefix not completed to `inf`.
#[test]
fn row09_incomplete_inf_is_matching_failure() {
    assert_both(&["ix", "inx", "in5", "i.", "i-", "I5"], ZERO, "row 9");
}

/// Row 10 — input failure: `i` or `in` then EOF.
#[test]
fn row10_inf_prefix_then_eof() {
    assert_both(&["i", "I", "in", "IN", "iN", "-i", "-in", "+in"], ZERO, "row 10");
}

/// Row 11 — matching failure: `inf` then a partial `inity`. The already
/// consumed characters cannot be pushed back, so the whole conversion fails
/// even though a valid `inf` was seen first.
#[test]
fn row11_partial_infinity_is_matching_failure() {
    assert_both(
        &[
            "infi", "infin", "infini", "infinit", "INFINIT", "infix",
            "infinitx", "-infi", "+infin", "infiX",
        ],
        ZERO,
        "row 11",
    );
}

/// Row 12 — matching failure: `0x` with no hex digit after it.
#[test]
fn row12_hex_prefix_without_digits_is_matching_failure() {
    assert_both(
        &["0x", "0X", "0xg", "0x.g", "0xz", "0x-1", "0x+1", "0x.", "0X.", "0x.p1", "0xx"],
        ZERO,
        "row 12",
    );
}

/// Row 13 — the same, with a sign, i.e. token length == sign + 2.
#[test]
fn row13_signed_hex_prefix_without_digits() {
    assert_both(&["-0x", "+0x", "-0X", "+0X", "-0xg", "+0x."], ZERO, "row 13");
}

/// Row 14 — `ERANGE` overflow must saturate to `±HUGE_VALF`, not wrap or NaN.
#[test]
fn row14_overflow_saturates_to_infinity() {
    assert_both(
        &[
            "1e39", "1e40", "1e308", "3.4028236e38", "0x1p128",
            "340282366920938463463374607431768211456",
            "99999999999999999999999999999999999999999",
        ],
        POS_INF,
        "row 14 (+)",
    );
    assert_both(
        &["-1e39", "-1e308", "-0x1p128", "-340282366920938463463374607431768211456"],
        NEG_INF,
        "row 14 (-)",
    );
}

/// Row 15 — `ERANGE` underflow must produce a correctly signed zero.
#[test]
fn row15_underflow_produces_signed_zero() {
    assert_both(
        &["1e-50", "1e-46", "0x1p-150", "7.0064923e-46", "1e-100", "0x1p-1000"],
        ZERO,
        "row 15 (+)",
    );
    assert_both(
        &["-1e-50", "-1e-46", "-0x1p-150", "-1e-100", "-0x1p-1000"],
        NEG_ZERO,
        "row 15 (-)",
    );
}

/// Row 16 — `e` with no digits: the mantissa alone is converted. The result is
/// deliberately **not** the rejection sentinel, which is what proves the C is
/// backing the exponent characters out rather than failing.
#[test]
fn row16_exponent_marker_without_digits_keeps_mantissa() {
    assert_both(&["1e", "1e+", "1e-", "1ee", "1E", "1E+"], ONE, "row 16");
}

/// Row 17 — `p` with no digits: the hex mantissa alone is converted.
#[test]
fn row17_binary_exponent_marker_without_digits_keeps_mantissa() {
    assert_both(&["0x1p", "0x1p+", "0x1p-", "0x1pp", "0X1P"], ONE, "row 17");
}

/// Row 18 — exponents too wide for any integer type must saturate, not wrap.
/// A wrapping implementation would turn `1e999…9` into a finite value.
#[test]
fn row18_absurd_exponents_saturate() {
    assert_both(
        &[
            "1e999999999999999999999",
            "1e2147483647",
            "1e4294967296",
            "1e18446744073709551616",
            "1e99999999999999999999999999999999999999",
        ],
        POS_INF,
        "row 18 (overflow)",
    );
    assert_both(
        &[
            "1e-999999999999999999999",
            "1e-2147483648",
            "1e-4294967296",
            "1e-18446744073709551616",
        ],
        ZERO,
        "row 18 (underflow)",
    );
}

/// Row 19 — a zero mantissa short-circuits: no exponent, however large, may
/// turn it into an infinity.
#[test]
fn row19_zero_mantissa_never_overflows() {
    assert_both(
        &[
            "0e999999999999999999999",
            "0.0e999999999999999999999",
            "0e2147483647",
            "0.000e999999999999999999999",
            "0e-999999999999999999999",
        ],
        ZERO,
        "row 19",
    );
    assert_both(&["-0e999999999999999999999"], NEG_ZERO, "row 19 (-)");
}

/// Row 20 — `nan` with a payload yields the same quiet NaN as bare `nan`; the
/// payload is not decoded into the significand.
#[test]
fn row20_nan_payload_is_ignored() {
    assert_both(
        &["nan", "NAN", "NaN", "nan(", "nan()", "nan(1)", "nan(123)", "nan(0x7f)", "+nan"],
        POS_QNAN,
        "row 20 (+)",
    );
    assert_both(&["-nan", "-NAN", "-nan(5)"], NEG_QNAN, "row 20 (-)");
}

/// Row 21 — `driver` performs no validation: infinities, negative zero and
/// subnormals are printed as their raw object representation.
#[test]
fn row21_driver_does_not_validate() {
    assert_both(&["inf", "infinity", "INF"], POS_INF, "row 21 (+inf)");
    assert_both(&["-inf", "-infinity"], NEG_INF, "row 21 (-inf)");
    assert_both(&["-0", "-0.0", "-0.000"], NEG_ZERO, "row 21 (-0)");
    // smallest subnormal: 0x00000001
    assert_both(&["1e-45", "1.4e-45", "0x1p-149"], "01000000\n", "row 21 (subnormal)");
}

/// Row 22 — the output is always exactly 8 hex digits and one newline,
/// because `print_hex` is only ever called with `len == sizeof(float)`.
#[test]
fn row22_output_is_always_nine_bytes() {
    let c = c_exe();
    let r = rust_exe();
    let mut inputs: Vec<String> = fixed::WHITESPACE_ONLY.iter().map(|s| s.to_string()).collect();
    inputs.extend(fixed::HEX_STICKY.iter().map(|s| s.to_string()));
    inputs.extend(fixed::OVERFLOW.iter().map(|s| s.to_string()));
    inputs.extend(fixed::NAN_PAYLOAD.iter().map(|s| s.to_string()));
    for input in inputs {
        for (name, exe) in [("C", &c), ("RUST", &r)] {
            let o = run_exe(exe, input.as_bytes());
            assert_eq!(
                o.stdout.len(),
                9,
                "{name} produced {} bytes for {input:?} ({o:?})",
                o.stdout.len()
            );
            assert_eq!(o.stdout[8], b'\n', "{name}: missing trailing newline for {input:?}");
        }
    }
}

/// Row 23 — the exit status is unconditionally 0: no input can make the
/// program fail, because `scanf`'s return value is discarded.
#[test]
fn row23_exit_status_is_always_zero() {
    let c = c_exe();
    let r = rust_exe();
    let mut inputs: Vec<Vec<u8>> = common::corpus::all_fixed()
        .into_iter()
        .map(|s| s.into_bytes())
        .collect();
    inputs.extend(common::corpus::binary_inputs());
    inputs.extend(common::corpus::long_literals().into_iter().map(|s| s.into_bytes()));
    for input in inputs {
        for (name, exe) in [("C", &c), ("RUST", &r)] {
            let o = run_exe(exe, &input);
            assert_eq!(
                o.code,
                Some(0),
                "{name} exited {:?} for {:?}",
                o.code,
                String::from_utf8_lossy(&input)
            );
        }
    }
}

/// Row 24 — embedded NUL and non-ASCII bytes. `scanf` is byte-oriented, so a
/// NUL is just another non-matching byte; a translation that treated stdin as
/// text would diverge here.
#[test]
fn row24_nul_and_non_ascii_bytes() {
    // a leading NUL is a matching failure
    let c = c_exe();
    let r = rust_exe();
    for input in [
        &b"\x00"[..],
        &b"\x00\x00"[..],
        &b"\x001"[..],
        &b"\x80"[..],
        &b"\xff"[..],
        &b"\x80\xff"[..],
    ] {
        let co = run_exe(&c, input);
        let ro = run_exe(&r, input);
        assert_eq!(co, ro, "row 24: disagreement on {input:?}");
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            ZERO,
            "row 24: expected rejection for {input:?}"
        );
    }
    // a NUL *after* a complete token terminates it like any other junk byte,
    // so the token before the NUL converts normally
    for (input, expected) in [
        (&b"1\x00"[..], ONE),
        (&b"1\x002"[..], ONE),
        (&b"1.5\x00"[..], "0000c03f\n"),
        (&b"1.5\x002.5"[..], "0000c03f\n"),
        (&b"inf\x00"[..], POS_INF),
        (&b"-0\x00"[..], NEG_ZERO),
    ] {
        let co = run_exe(&c, input);
        let ro = run_exe(&r, input);
        assert_eq!(co, ro, "row 24: disagreement on {input:?}");
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            expected,
            "row 24: expected the token before the NUL to convert, got {co:?}"
        );
    }
    // full byte-value sweep, both alone and adjacent to a digit
    let sweep: Vec<Vec<u8>> = (0u8..=255)
        .flat_map(|b| [vec![b], vec![b, b'1'], vec![b'1', b], vec![b'.', b]])
        .collect();
    assert_same_all(sweep, "row 24 byte sweep");
}

/// Row 25 — stdin closed outright, so the first `read`/`getc` errors rather
/// than reporting EOF. The program must still print the initialiser and
/// exit 0.
#[test]
fn row25_closed_stdin() {
    use std::process::{Command, Stdio};
    let mut results = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec 0<&- ; exec {}", exe.display()))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("spawn via sh");
        results.push((out.status.code(), String::from_utf8_lossy(&out.stdout).to_string()));
    }
    assert_eq!(
        results[0], results[1],
        "row 25: closed-stdin divergence C={:?} RUST={:?}",
        results[0], results[1]
    );
    assert_eq!(results[0].1, ZERO, "row 25: expected the zero sentinel");
    assert_eq!(results[0].0, Some(0), "row 25: expected exit 0");
}

/// Generic boundary: one step past the valid range on both ends, asserted as
/// a *pair* so the transition itself is pinned.
#[test]
fn test_range_transitions_are_identical() {
    // last finite / first infinite
    assert_both(&["3.4028234663852886e38", "0x1.fffffep127"], "ffff7f7f\n", "FLT_MAX");
    assert_both(&["3.402823669e38", "0x1p128"], POS_INF, "just past FLT_MAX");
    // smallest subnormal / first value that rounds to zero
    assert_both(&["0x1p-149", "1.4012984643e-45"], "01000000\n", "FLT_TRUE_MIN");
    assert_both(&["0x1p-150", "7.006492321e-46"], ZERO, "half of FLT_TRUE_MIN");
    // exactly half of the smallest subnormal rounds to even => zero
    assert_both(&["0x0.8p-149"], ZERO, "exact half ulp ties to even");
    // just above half rounds up
    assert_both(&["0x0.81p-149"], "01000000\n", "just above half ulp");
}

/// The C API takes no enum, so the analogous "value with no valid variant"
/// check is: every 32-bit pattern is a legal `float`. Feed inputs that
/// produce NaNs with unusual payloads and confirm both sides print the same
/// bits (the payload must not be normalised).
#[test]
fn test_no_enum_but_all_bit_patterns_are_valid_input() {
    // scanf can only ever produce the canonical quiet NaN, so verify that,
    // and leave arbitrary payloads to the FFI-level test on `driver`.
    assert_both(
        &["nan", "NAN", "nan(1)", "nan(4194303)", "nan(99999999999999)"],
        POS_QNAN,
        "canonical qNaN only",
    );
}
