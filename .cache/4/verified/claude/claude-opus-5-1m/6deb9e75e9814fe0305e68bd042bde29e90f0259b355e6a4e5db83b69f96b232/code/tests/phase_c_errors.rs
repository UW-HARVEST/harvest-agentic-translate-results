// Phase C — error-path / rejection-surface differential tests.
//
// One test per row of ERRORS.md.
//
// `driver` is a `void` function with no branches, no return code, no assert and
// no pointer or enum parameter, so its rejection surface is empty (rows E1 and
// E15 verify that claim mechanically against the C source itself). For every
// other row the Phase C obligation becomes: for this degenerate / boundary /
// classically-invalid input, C and Rust must agree *exactly* — same bytes, and
// neither may crash, panic or abort.

mod common;

use common::{assert_same, from_fields, Rng, SEED};

/// The C source, embedded at compile time so the structural claims in ERRORS.md
/// are checked against the real file rather than asserted in prose.
const C_SOURCE: &str = include_str!("../c_src/src/driver.c");
const C_HEADER: &str = include_str!("../c_src/include/driver.h");

/// Strip `//` line comments (the C file uses no `/* */` comments) and return the
/// remaining code.
fn code_only(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Split into identifier-ish tokens so `#include` is not mistaken for `if`.
fn tokens(code: &str) -> Vec<String> {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// E1 — the error surface is empty (verified against the C source)
// ---------------------------------------------------------------------------

#[test]
fn error_e1_no_rejection_sites_exist() {
    let code = code_only(C_SOURCE);
    let toks = tokens(&code);

    // Every construct C code uses to reject input or report an error.
    for forbidden in [
        "return", "assert", "NULL", "nullptr", "errno", "exit", "abort", "goto",
        "if", "else", "switch", "case", "while", "for", "raise", "longjmp",
        "perror", "strerror",
    ] {
        assert!(
            !toks.iter().any(|t| t == forbidden),
            "ERRORS.md claims the C error surface is empty, but `{forbidden}` \
             appears in c_src/src/driver.c — the table is stale and must be \
             regenerated"
        );
    }

    // No comparison / conditional operators either: the function is branch-free.
    for op in ["?", "==", "!=", "<=", ">=", "&&", "||"] {
        assert!(
            !code.contains(op),
            "ERRORS.md claims `driver` is branch-free, but operator `{op}` \
             appears in c_src/src/driver.c"
        );
    }

    // And it really is a `void` function, so there is no code to compare.
    assert!(
        code.contains("void driver(double f)"),
        "unexpected signature in c_src/src/driver.c"
    );
}

// ---------------------------------------------------------------------------
// E15 — null-pointer / length / enum boundaries are unrepresentable
// ---------------------------------------------------------------------------

#[test]
fn error_e15_no_pointer_or_enum_params() {
    let header = code_only(C_HEADER);

    // The public API is exactly one by-value `double` parameter.
    assert!(
        header.contains("void driver(double f);"),
        "public API changed; ERRORS.md row E15 must be revisited"
    );

    // Therefore: nothing to nullify, no length to zero or oversize.
    let decl = header
        .lines()
        .find(|l| l.contains("driver("))
        .expect("declaration not found");
    assert!(
        !decl.contains('*'),
        "the declaration now takes a pointer: null-pointer tests are required"
    );
    assert!(
        !decl.contains("size") && !decl.contains("len") && !decl.contains("count"),
        "the declaration now takes a length/count: zero/oversize tests are required"
    );

    // No enum anywhere in the library, so there is no integer-with-no-valid-
    // variant to smuggle across the FFI boundary.
    for src in [C_SOURCE, C_HEADER] {
        assert!(
            !tokens(&code_only(src)).iter().any(|t| t == "enum"),
            "an enum was introduced: out-of-range-enum tests are required"
        );
    }

    // The nearest analogue for a by-value double is a non-canonical bit
    // pattern; those are covered by E6/E7 and CONFIGS.md row C21.
}

// ---------------------------------------------------------------------------
// E2 / E3 — infinities
// ---------------------------------------------------------------------------

#[test]
fn error_e2_infinity() {
    assert_same("E2 +inf", &[f64::INFINITY]);
    assert_same("E2 +inf via bits", &[f64::from_bits(0x7FF0_0000_0000_0000)]);
}

#[test]
fn error_e3_infinity() {
    assert_same("E3 -inf", &[f64::NEG_INFINITY]);
    assert_same("E3 -inf via bits", &[f64::from_bits(0xFFF0_0000_0000_0000)]);
}

// ---------------------------------------------------------------------------
// E4 / E5 — quiet NaNs, both signs
// ---------------------------------------------------------------------------

#[test]
fn error_e4_quiet_nan() {
    assert_same("E4 quiet NaN", &[f64::NAN]);
    assert_same("E4 quiet NaN via bits", &[f64::from_bits(0x7FF8_0000_0000_0000)]);
}

#[test]
fn error_e5_negative_quiet_nan() {
    // Sign bit set: %a and %.4f must both render `-nan`.
    assert_same("E5 -NaN", &[f64::from_bits(0xFFF8_0000_0000_0000)]);
    assert_same("E5 -NaN via negation", &[-f64::NAN]);
}

// ---------------------------------------------------------------------------
// E6 — signaling NaN must not be quieted and its payload must survive
// ---------------------------------------------------------------------------

#[test]
fn error_e6_signaling_nan() {
    // exponent all ones, mantissa MSB (quiet bit) CLEAR, payload non-zero.
    let snan_pos = from_fields(false, 0x7FF, 0x0000_0000_0000_0001);
    let snan_neg = from_fields(true, 0x7FF, 0x0000_0000_0000_0001);
    let snan_big = from_fields(false, 0x7FF, 0x0007_FFFF_FFFF_FFFF);
    let snan_big_neg = from_fields(true, 0x7FF, 0x0007_FFFF_FFFF_FFFF);

    for v in [snan_pos, snan_neg, snan_big, snan_big_neg] {
        assert!(v.is_nan(), "constructed value is not a NaN");
        assert_eq!(
            (v.to_bits() >> 51) & 1,
            0,
            "constructed value is not *signaling* (quiet bit is set)"
        );
    }

    // The union type-pun in C reproduces the raw bits verbatim, so `%llx` is
    // where a quieted NaN would show up as a divergence.
    assert_same("E6 signaling NaN", &[snan_pos, snan_neg, snan_big, snan_big_neg]);
}

// ---------------------------------------------------------------------------
// E7 — arbitrary / maximal NaN payloads
// ---------------------------------------------------------------------------

#[test]
fn error_e7_nan_payload_sweep() {
    let mut rng = Rng::new(SEED ^ 0xE7);
    let mut inputs = Vec::new();

    for payload in [
        0x0000_0000_0000_0001u64,
        0x000F_FFFF_FFFF_FFFF,
        0x0008_0000_0000_0000,
        0x0007_FFFF_FFFF_FFFF,
        0x000A_AAAA_AAAA_AAAA,
        0x0005_5555_5555_5555,
    ] {
        inputs.push(from_fields(false, 0x7FF, payload));
        inputs.push(from_fields(true, 0x7FF, payload));
    }
    for _ in 0..500 {
        let m = rng.next_mantissa() | 1; // non-zero => NaN, not infinity
        inputs.push(from_fields(false, 0x7FF, m));
        inputs.push(from_fields(true, 0x7FF, m));
    }

    assert!(inputs.iter().all(|v| v.is_nan()));
    assert_same("E7 NaN payload sweep", &inputs);
}

// ---------------------------------------------------------------------------
// E8 — negative zero
// ---------------------------------------------------------------------------

#[test]
fn error_e8_negative_zero() {
    let neg_zero = f64::from_bits(0x8000_0000_0000_0000);
    assert_eq!(neg_zero, 0.0, "should compare equal to +0.0");
    assert!(neg_zero.is_sign_negative(), "sign bit should be set");
    assert_same("E8 -0.0", &[neg_zero, -0.0, 0.0]);
}

// ---------------------------------------------------------------------------
// E9 / E10 — subnormal boundaries
// ---------------------------------------------------------------------------

#[test]
fn error_e9_min_subnormal() {
    let min_sub = f64::from_bits(1);
    assert!(min_sub.is_subnormal());
    assert_same("E9 min subnormal", &[min_sub, -min_sub]);
}

#[test]
fn error_e10_subnormal_normal_boundary() {
    let max_sub = f64::from_bits(0x000F_FFFF_FFFF_FFFF);
    let min_norm = f64::from_bits(0x0010_0000_0000_0000);
    assert!(max_sub.is_subnormal());
    assert!(min_norm.is_normal());
    assert_same(
        "E10 subnormal/normal boundary",
        &[max_sub, min_norm, -max_sub, -min_norm],
    );
}

// ---------------------------------------------------------------------------
// E11 — largest finite value: ~309 integer digits out of %.4f
// ---------------------------------------------------------------------------

#[test]
fn error_e11_max_finite_oversized_output() {
    let max = f64::MAX;
    let just_below = f64::from_bits(0x7FEF_FFFF_FFFF_FFFE);
    assert_same("E11 max finite", &[max, -max, just_below, -just_below]);

    // Confirm the row really does exercise an oversized output line.
    let out = common::capture(common::libs().c, &[max]);
    assert!(
        out.len() > 300,
        "expected a ~320-byte line for f64::MAX, got {} bytes",
        out.len()
    );
}

// ---------------------------------------------------------------------------
// E12 — one step past the largest finite value
// ---------------------------------------------------------------------------

#[test]
fn error_e12_one_step_past_finite_range() {
    // f64::MAX has bits 0x7FEFFFFFFFFFFFFF; +1 leaves the finite range.
    let past = f64::from_bits(0x7FEF_FFFF_FFFF_FFFF + 1);
    assert!(past.is_infinite() && past > 0.0);
    let past_neg = f64::from_bits(0xFFEF_FFFF_FFFF_FFFF + 1);
    assert!(past_neg.is_infinite() && past_neg < 0.0);
    assert_same("E12 one past finite range", &[past, past_neg]);
}

// ---------------------------------------------------------------------------
// E13 — the %.4f rounding cliff / exact ties
// ---------------------------------------------------------------------------

#[test]
fn error_e13_rounding_cliff_ties() {
    let mut inputs = vec![0.00005f64, -0.00005, 0.000049999999999, -0.000049999999999];
    for v in [
        0.00005f64, 0.00015, 0.00025, 0.00035, 0.00045, 0.00055, 0.5, 1.5, 2.5,
    ] {
        inputs.push(v);
        inputs.push(-v);
    }
    assert_same("E13 rounding cliff / ties", &inputs);
}

// ---------------------------------------------------------------------------
// E14 — repeated invocation / stdout buffering
// ---------------------------------------------------------------------------

#[test]
fn error_e14_repeated_calls_buffering() {
    let mut rng = Rng::new(SEED ^ 0xE14);
    let mut inputs = Vec::with_capacity(4096);
    for i in 0..4096 {
        // Long lines (~320 bytes) interleaved with short ones so records
        // straddle glibc's BUFSIZ boundary; plus non-finite values throughout.
        inputs.push(match i % 5 {
            0 => f64::MAX,
            1 => f64::NAN,
            2 => f64::NEG_INFINITY,
            3 => rng.next_bit_pattern(),
            _ => rng.next_signed_unit(),
        });
    }
    assert_same("E14 repeated calls", &inputs);
}
