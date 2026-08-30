//! Differential tests: the Rust binary must be byte-for-byte indistinguishable
//! from the C binary on stdout, stderr and exit status.
//!
//! Input classes are derived directly from `c_src/src/main.c`:
//!
//! ```text
//! static int y = 123;                       // pre-set, observable if scanf skips it
//! scanf("%d %d %d", &x, &y, &z);            // 0..3 conversions may succeed
//! if (x != 1) -> result 1                   // stage 1
//! if (y != 2) -> result 2                   // stage 2
//! if (z != 3) -> result 3                   // stage 3
//! else        -> "Ok!", result 0
//! fail:       -> "Operation failed"         // reached by all three stages
//! printf("Result: %d\n", result);
//! return 0;                                 // exit status is always 0
//! ```
//!
//! `x` and `z` are locals pre-set to 0; `y` is the global pre-set to 123. A
//! `%d` conversion that fails leaves its target untouched and aborts the rest
//! of the format string, so the number of successful conversions is itself an
//! input class.

mod common;

use common::{assert_same, check};

// ---------------------------------------------------------------------------
// Number of successful scanf conversions: 0, 1, 2, 3
// ---------------------------------------------------------------------------

#[test]
fn zero_conversions_empty_input() {
    // EOF before any conversion: x stays 0, y stays 123, z stays 0 -> stage 1.
    check("empty", "");
}

#[test]
fn zero_conversions_whitespace_only() {
    check("spaces_only", "   ");
    check("newlines_only", "\n\n\n");
    check("mixed_ws_only", " \t\n\r\x0b\x0c ");
}

#[test]
fn zero_conversions_matching_failure() {
    // Non-numeric first token: matching failure, nothing is assigned.
    check("alpha", "abc");
    check("alpha_then_numbers", "abc 1 2 3");
    check("dot_first", ".1 2 3");
    check("sign_only_minus", "-");
    check("sign_only_plus", "+");
    check("sign_then_space", "- 1 2 3");
    check("double_minus", "--1 2 3");
    check("double_plus", "++1 2 3");
    check("plus_minus", "+-1 2 3");
    check("empty_exponent", "e 2 3");
}

#[test]
fn one_conversion_only_x() {
    // Only x is read; y keeps its initialiser 123 -> stage 2 error.
    check("single_one", "1");
    check("single_one_newline", "1\n");
    check("x_ok_y_alpha", "1 abc");
    check("x_ok_y_sign_only", "1 -");
    check("x_ok_y_sign_space", "1 - 2 3");
}

#[test]
fn two_conversions_only_x_and_y() {
    // z keeps its initialiser 0 -> stage 3 error.
    check("one_two", "1 2");
    check("one_two_newline", "1 2\n");
    check("x_y_ok_z_alpha", "1 2 abc");
    check("x_y_ok_z_sign_only", "1 2 +");
}

#[test]
fn three_conversions_happy_path() {
    check("one_two_three", "1 2 3");
    check("one_two_three_newline", "1 2 3\n");
    check("extra_tokens_ignored", "1 2 3 4 5");
    check("extra_garbage_ignored", "1 2 3 nonsense");
}

// ---------------------------------------------------------------------------
// multi_stage: every stage, and every result value 0..3
// ---------------------------------------------------------------------------

#[test]
fn stage1_x_not_one() {
    check("zeros", "0 0 0");
    check("x_five_y_two_z_three", "5 2 3");
    check("x_two", "2 2 3");
    check("x_negative_one", "-1 2 3");
    check("x_zero_valid_rest", "0 2 3");
    // x != 1 short-circuits before y and z are ever examined.
    check("x_bad_everything_else_good", "7 2 3");
}

#[test]
fn stage2_y_not_two() {
    check("y_zero", "1 0 3");
    check("y_one", "1 1 3");
    check("y_three", "1 3 3");
    check("y_negative_two", "1 -2 3");
    check("y_default_123", "1");
    check("y_int_min", "1 -2147483648 3");
    check("y_int_max", "1 2147483647 3");
}

#[test]
fn stage3_z_not_three() {
    check("z_zero", "1 2 0");
    check("z_two", "1 2 2");
    check("z_four", "1 2 4");
    check("z_negative_three", "1 2 -3");
    check("z_int_max", "1 2 2147483647");
    check("z_int_min", "1 2 -2147483648");
}

#[test]
fn stage_all_pass() {
    check("plain", "1 2 3");
    check("leading_zeros", "0000000001 000000002 0000003");
    check("explicit_plus_signs", "+1 +2 +3");
    check("plus_and_zeros", "+0000001 +2 +00003");
}

// ---------------------------------------------------------------------------
// scanf whitespace handling: %d skips arbitrary leading whitespace, including
// newlines, so line structure is irrelevant.
// ---------------------------------------------------------------------------

#[test]
fn whitespace_between_fields() {
    check("newline_separated", "1\n2\n3\n");
    check("tab_separated", "\t1\t2\t3\t");
    check("crlf_separated", "1\r\n2\r\n3\r\n");
    check("vertical_tab_form_feed", "\x0b1\x0c2 3");
    check("leading_blank_lines", "\n\n\n1 2 3");
    check("wide_gaps", "   1        2\n\n\n    3   ");
    check("all_glued_to_next_token", "1 2 3abc");
}

#[test]
fn large_whitespace_runs() {
    let mut input = " ".repeat(100_000);
    input.push_str("1 2 3");
    check("100k_spaces", &input);

    let mut input = "\n".repeat(50_000);
    input.push_str("1\n\n\n2\n\n\n3");
    check("50k_newlines", &input);
}

// ---------------------------------------------------------------------------
// Integer overflow / truncation exactly as glibc's %d performs it: the digits
// are accumulated with strtol semantics (saturating at long range) and the
// result is then truncated to int.
// ---------------------------------------------------------------------------

#[test]
fn values_at_int_boundaries() {
    check("x_int_max", "2147483647 2 3");
    check("x_int_min", "-2147483648 2 3");
    // 2^31: does not fit in int, truncates to INT_MIN.
    check("x_2_pow_31", "2147483648 2 3");
    check("x_neg_2_pow_31_minus_1", "-2147483649 2 3");
}

#[test]
fn truncation_to_int_is_observable() {
    // 2^32 + 1 truncates to 1, so x passes stage 1 despite being out of range.
    check("x_2_pow_32_plus_1", "4294967297 2 3");
    // 2^32 truncates to 0.
    check("x_2_pow_32", "4294967296 2 3");
    // Same trick on y and z.
    check("y_2_pow_32_plus_2", "1 4294967298 3");
    check("z_2_pow_32_plus_3", "1 2 4294967299");
    check("y_2_pow_32", "1 4294967296 3");
    // All three out of range but congruent to the required values.
    check("all_wrapped", "4294967297 4294967298 4294967299");
}

#[test]
fn saturation_at_long_boundaries() {
    check("x_long_max", "9223372036854775807 2 3");
    check("x_long_max_plus_1", "9223372036854775808 2 3");
    check("x_long_min", "-9223372036854775808 2 3");
    check("x_long_min_minus_1", "-9223372036854775809 2 3");
    check("y_long_min", "1 -9223372036854775808 3");
    check("y_long_min_minus_1", "1 -9223372036854775809 3");
    // 2^64 + 2 would truncate to 2 if the accumulator wrapped, but glibc
    // saturates at LONG_MAX first, so y becomes -1 and stage 2 fails.
    check("y_2_pow_64_plus_2_saturates", "1 18446744073709551618 3");
    check("x_2_pow_64_plus_1_saturates", "18446744073709551617 2 3");
}

#[test]
fn very_long_digit_strings() {
    let big = "9".repeat(400);
    check("400_nines_x", &format!("{big} 2 3"));
    check("400_nines_y", &format!("1 {big} 3"));
    check("400_nines_z", &format!("1 2 {big}"));
    check("400_nines_negative", &format!("-{big} 2 3"));

    // A huge run of leading zeros is still just zero, and must not overflow.
    let zeros = "0".repeat(5_000);
    check("5k_leading_zeros_then_1", &format!("{zeros}1 {zeros}2 {zeros}3"));
    check("5k_zeros_only", &format!("{zeros} 2 3"));

    // Far beyond any internal scratch buffer either implementation might use.
    let huge = "1234567890".repeat(10_000); // 100_000 digits
    check("100k_digits_x", &format!("{huge} 2 3"));
    check("100k_digits_negative_y", &format!("1 -{huge} 3"));
}

#[test]
fn negative_zero_and_signs() {
    check("x_negative_zero", "-0 2 3");
    check("y_negative_zero", "1 -0 3");
    check("z_negative_zero", "1 2 -0");
    check("all_negative_zero", "-0 -0 -0");
    check("x_positive_zero_signed", "+0 2 3");
    check("y_positive_zero_signed", "1 +0 3");
}

// ---------------------------------------------------------------------------
// %d is base 10 only, so "0x10" stops after the leading 0.
// ---------------------------------------------------------------------------

#[test]
fn no_hex_or_octal_prefix_handling() {
    check("hex_x", "0x10 2 3");
    // x reads 0, then the 'x' causes a matching failure for y.
    check("hex_after_one", "1 0x2 3");
    check("octal_looking", "010 02 03");
    check("float_looking", "1.5 2.5 3.5");
    check("scientific_looking", "1e2 2e2 3e2");
}

// ---------------------------------------------------------------------------
// Raw byte inputs: NUL bytes and non-UTF-8 data must behave identically.
// ---------------------------------------------------------------------------

#[test]
fn nul_and_binary_bytes() {
    assert_same("nul_leading", &[], b"\x001 2 3");
    assert_same("nul_after_x", &[], b"1\x002 3");
    assert_same("nul_after_y", &[], b"1 2\x003");
    assert_same("nul_only", &[], b"\x00");
    assert_same("high_bytes", &[], b"\xff\xfe 1 2 3");
    assert_same("utf8_digits_fullwidth", &[], "１ ２ ３".as_bytes());
    assert_same("latin1_after_valid", &[], b"1 2 3\xff\xfe");
    assert_same("all_bytes", &[], &(0u8..=255).collect::<Vec<u8>>());
}

// ---------------------------------------------------------------------------
// The C `main` takes no parameters, so argv is ignored entirely.
// ---------------------------------------------------------------------------

#[test]
fn argv_is_ignored() {
    assert_same("args_empty_stdin", &["a", "b", "c"], b"");
    assert_same("args_happy_stdin", &["1", "2", "3"], b"1 2 3");
    assert_same("args_dashes", &["--help"], b"1 2");
    assert_same("args_version", &["-v", "--verbose", "extra"], b"1");
}

// ---------------------------------------------------------------------------
// A large payload where the interesting tokens sit far past the first stdio
// buffer refill.
// ---------------------------------------------------------------------------

#[test]
fn input_larger_than_stdio_buffer() {
    let filler = "0 ".repeat(200_000); // ~400 KiB before anything matters
    check("huge_prefix_then_ok", &format!("1 2 3 {filler}"));

    let mut input = "\t".repeat(300_000);
    input.push_str("1 2 3");
    check("huge_tab_prefix", &input);
}

// ---------------------------------------------------------------------------
// Exhaustive sweep of the small-value neighbourhood that the three stage
// comparisons actually branch on, plus a deterministic pseudo-random sweep of
// token/separator combinations.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_small_value_grid() {
    for x in -2..=4 {
        for y in -2..=4 {
            for z in -2..=4 {
                check(&format!("grid_{x}_{y}_{z}"), &format!("{x} {y} {z}"));
            }
        }
    }
}

#[test]
fn deterministic_random_sweep() {
    const TOKENS: &[&str] = &[
        "1", "2", "3", "0", "-1", "-2", "-3", "+1", "+2", "+3", "abc", "-", "+", "--1", "0x2",
        "2147483647", "-2147483648", "4294967297", "4294967298", "4294967299",
        "18446744073709551618", "99999999999999999999999", "-99999999999999999999999",
        "0000001", "0000002", "0000003", ".5", "1.5", "e", "1e2", "007", "\u{0}",
    ];
    const SEPS: &[&str] = &[" ", "\t", "\n", "\r", "\x0b", "\x0c", "  ", "\n\n", "", "\r\n"];

    // xorshift64* so the case list is fixed without pulling in a dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for case in 0..600 {
        let count = (next() % 6) as usize;
        let mut input = String::new();
        for _ in 0..count {
            input.push_str(TOKENS[(next() as usize) % TOKENS.len()]);
            input.push_str(SEPS[(next() as usize) % SEPS.len()]);
        }
        assert_same(&format!("random_{case}"), &[], input.as_bytes());
    }
}

#[test]
fn deterministic_random_binary_sweep() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    for case in 0..300 {
        let len = (next() % 20) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
        assert_same(&format!("random_bytes_{case}"), &[], &bytes);
    }
}
