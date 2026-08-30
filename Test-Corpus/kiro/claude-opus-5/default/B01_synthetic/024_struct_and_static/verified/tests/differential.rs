//! Differential tests: the C program in `c_src/` is ground truth. For every
//! input class the C source branches on, both executables are run as
//! subprocesses and stdout, stderr and exit status are compared byte for byte.
//!
//! Branch inventory taken from `c_src/src/main.c`:
//!
//! * `scanf("%d", &x)` — the only input-dependent branch in the program.
//!   Sub-cases: EOF before any conversion, leading whitespace runs (all six
//!   `isspace` bytes), optional `+`/`-` sign, sign not followed by a digit
//!   (matching failure leaves `x` at its initial `0`), one or more digits,
//!   digits terminated by EOF / whitespace / an arbitrary byte, a value that
//!   fits `long` but not `int` (truncation), and a value that overflows `long`
//!   (glibc saturates to `LONG_MAX`/`LONG_MIN`, then truncates).
//! * `run()` is called twice, so `x` is added to `bedrooms` twice and the
//!   file-scope `the_house` state carries across calls: `floors` 2→3→4,
//!   `bathrooms` 2.5→3.5→4.5.
//! * `add_bedrooms` performs a signed `int` addition that can overflow.
//! * `printf("%.1f")` formats the bathroom count.
//! * `main()` takes no parameters, so argv is ignored, and it always
//!   `return 0`.

mod common;

use common::{assert_same, assert_same_full, assert_same_with_args, c_bin, run_with_args, rust_bin};

// ---------------------------------------------------------------------------
// scanf: no conversion performed (x keeps its initial value of 0)
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same("empty", b"");
}

#[test]
fn whitespace_only_inputs() {
    assert_same("single_space", b" ");
    assert_same("single_tab", b"\t");
    assert_same("single_newline", b"\n");
    assert_same("many_newlines", b"\n\n\n\n");
    assert_same("all_isspace_bytes", b" \t\n\x0b\x0c\r");
    assert_same("trailing_ws_run", b"   \r\n\t  ");
}

#[test]
fn non_numeric_inputs() {
    assert_same("letters", b"abc");
    assert_same("letters_newline", b"abc\n");
    assert_same("dot_five", b".5");
    assert_same("leading_punct", b"#5");
    assert_same("nul_byte_first", b"\x005");
    assert_same("nul_after_whitespace", b"  \x005");
    assert_same("non_ascii_utf8", "\u{00a0}5".as_bytes());
    assert_same("high_byte", b"\xff5");
}

#[test]
fn sign_without_digits() {
    // `-`/`+` consumed, then a matching failure: nothing is assigned.
    assert_same("minus_eof", b"-");
    assert_same("plus_eof", b"+");
    assert_same("minus_newline", b"-\n");
    assert_same("minus_space_digit", b"- 5");
    assert_same("plus_space_digit", b"+ 3");
    assert_same("double_minus", b"--5");
    assert_same("minus_dot", b"-.");
    assert_same("minus_letter", b"-a");
}

#[test]
fn closed_stdin() {
    // No stdin at all: scanf sees an immediate read failure.
    let c = run_with_args(c_bin(), b"", &[]);
    let r = run_with_args(rust_bin(), b"", &[]);
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.code, r.code);
}

// ---------------------------------------------------------------------------
// scanf: successful conversion
// ---------------------------------------------------------------------------

#[test]
fn single_item_no_newline() {
    assert_same("bare_zero", b"0");
    assert_same("bare_one", b"1");
    assert_same("bare_five", b"5");
}

#[test]
fn single_item_with_newline() {
    assert_same("zero_nl", b"0\n");
    assert_same("five_nl", b"5\n");
    assert_same("five_crlf", b"5\r\n");
}

#[test]
fn signed_values() {
    assert_same("negative", b"-3");
    assert_same("negative_nl", b"-3\n");
    assert_same("explicit_plus", b"+4");
    assert_same("negative_zero", b"-0");
    assert_same("plus_zero", b"+0");
}

#[test]
fn leading_whitespace_before_number() {
    // %d skips whitespace, including newlines, before converting.
    assert_same("space_then_number", b"  7");
    assert_same("newline_then_number", b"\n7");
    assert_same("mixed_ws_then_number", b"   \n  7\n");
    assert_same("vtab_ff_cr_then_number", b"\x0b\x0c\r\n\t 6\n");
    assert_same("cr_run_then_number", b"\r\r\r12");
    assert_same("ws_before_sign", b"  \t -8\n");
}

#[test]
fn extra_input_after_number_is_ignored() {
    // Only one conversion is requested; the rest of stdin is never read.
    assert_same("two_numbers_space", b"3 4");
    assert_same("two_numbers_lines", b"8\n9\n");
    assert_same("digits_then_letters", b"12abc");
    assert_same("digits_then_sign", b"12-3");
    assert_same("hex_like", b"0x10");
    assert_same("float_like", b"1.9");
    assert_same("exponent_like", b"1e5");
    assert_same("digits_then_nul", b"7\x00garbage");
    assert_same("digits_then_long_tail", &{
        let mut v = b"6\n".to_vec();
        v.extend(std::iter::repeat(b'z').take(100_000));
        v
    });
}

#[test]
fn leading_zeros_are_decimal() {
    assert_same("leading_zeros", b"007");
    assert_same("negative_leading_zeros", b"-0009");
    assert_same("many_zeros_then_digit", &{
        let mut v = vec![b'0'; 5_000];
        v.push(b'5');
        v
    });
}

// ---------------------------------------------------------------------------
// int boundaries, truncation and the signed overflow in add_bedrooms
// ---------------------------------------------------------------------------

#[test]
fn int_boundary_values() {
    assert_same("int_max", b"2147483647");
    assert_same("int_max_minus_1", b"2147483646");
    assert_same("int_min", b"-2147483648");
    assert_same("int_min_plus_1", b"-2147483647");
}

#[test]
fn values_beyond_int_are_truncated() {
    assert_same("two_pow_31", b"2147483648");
    assert_same("two_pow_32", b"4294967296");
    assert_same("two_pow_32_plus_1", b"4294967297");
    assert_same("neg_two_pow_31_minus_1", b"-2147483649");
    assert_same("neg_two_pow_32", b"-4294967296");
}

#[test]
fn values_beyond_long_saturate_then_truncate() {
    assert_same("long_max", b"9223372036854775807");
    assert_same("long_max_plus_1", b"9223372036854775808");
    assert_same("long_min", b"-9223372036854775808");
    assert_same("long_min_minus_1", b"-9223372036854775809");
    assert_same("twenty_one_nines", b"999999999999999999999");
    assert_same("neg_twenty_one_nines", b"-999999999999999999999");
    assert_same("ten_thousand_nines", &vec![b'9'; 10_000]);
    assert_same("neg_ten_thousand_nines", &{
        let mut v = vec![b'-'];
        v.extend(std::iter::repeat(b'9').take(10_000));
        v
    });
    // Saturation with a leading-zero prefix that must not change the value.
    assert_same("padded_huge", &{
        let mut v = vec![b'0'; 100];
        v.extend(std::iter::repeat(b'9').take(400));
        v
    });
}

#[test]
fn bedrooms_addition_overflows() {
    // bedrooms starts at 5 and x is added twice, so these straddle INT_MAX
    // and INT_MIN in the first and/or second run() call.
    assert_same("overflow_first_add", b"2147483647");
    assert_same("overflow_second_add", b"1073741824");
    assert_same("overflow_just_under", b"2147483643");
    assert_same("underflow_first_add", b"-2147483648");
    assert_same("underflow_second_add", b"-1073741824");
    assert_same("underflow_just_under", b"-2147483643");
    // 2 * -2^31 == 0 mod 2^32: the two wraps cancel.
    assert_same("double_wrap_cancels", b"2147483648");
}

// ---------------------------------------------------------------------------
// large / awkward stdin shapes
// ---------------------------------------------------------------------------

#[test]
fn large_leading_whitespace() {
    let mut input = vec![b' '; 1_000_000];
    input.push(b'9');
    assert_same("one_mb_of_spaces", &input);
}

#[test]
fn long_digit_runs() {
    for len in [1usize, 2, 9, 10, 18, 19, 20, 21, 64, 1_000] {
        let input = vec![b'1'; len];
        assert_same(&format!("ones_len_{len}"), &input);
        let mut neg = vec![b'-'];
        neg.extend(std::iter::repeat(b'1').take(len));
        assert_same(&format!("neg_ones_len_{len}"), &neg);
    }
}

#[test]
fn every_single_byte_as_sole_input() {
    // Exhaustively covers the whitespace / sign / digit / other partition of
    // the first byte scanf examines.
    for b in 0u8..=255 {
        assert_same(&format!("byte_{b:#04x}"), &[b]);
    }
}

#[test]
fn every_byte_after_a_digit() {
    // The terminating byte decides where the conversion stops.
    for b in 0u8..=255 {
        assert_same(&format!("digit_then_byte_{b:#04x}"), &[b'4', b]);
    }
}

// ---------------------------------------------------------------------------
// argv is ignored; exit status is always 0
// ---------------------------------------------------------------------------

#[test]
fn command_line_arguments_are_ignored() {
    assert_same_with_args("one_arg", b"5\n", &["ignored"]);
    assert_same_with_args("several_args", b"5\n", &["a", "b", "c"]);
    assert_same_with_args("dash_arg", b"", &["--help"]);
    assert_same_with_args("empty_arg", b"2\n", &[""]);
}

#[test]
fn locale_does_not_change_formatting() {
    // The C program never calls setlocale(), so it stays in the "C" locale and
    // `%.1f` keeps a '.' separator no matter what the environment says. A
    // locale-aware Rust translation would print "2,5" here.
    for loc in [
        "C",
        "C.UTF-8",
        "POSIX",
        "de_DE.UTF-8",
        "fr_FR.UTF-8",
        "en_US.UTF-8",
        "ru_RU.UTF-8",
        "not-a-locale",
    ] {
        assert_same_full(
            &format!("locale_{loc}"),
            b"5\n",
            &[],
            &[("LC_ALL", loc), ("LC_NUMERIC", loc), ("LANG", loc)],
        );
    }
}

// ---------------------------------------------------------------------------
// Golden output shape (guards printf spacing, %.1f precision, newline count)
// ---------------------------------------------------------------------------

#[test]
fn golden_output_for_input_five() {
    const EXPECTED: &str = "\
The house has 2 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 3 floors, 10 bedrooms, and 3.5 bathrooms
The house has 3 floors, 10 bedrooms, and 3.5 bathrooms
The house has 4 floors, 10 bedrooms, and 3.5 bathrooms
The house has 4 floors, 10 bedrooms, and 4.5 bathrooms
The house has 4 floors, 15 bedrooms, and 4.5 bathrooms
";

    let c = run_with_args(c_bin(), b"5\n", &[]);
    let r = run_with_args(rust_bin(), b"5\n", &[]);

    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        EXPECTED,
        "C reference output changed shape"
    );
    assert_eq!(c.stdout, r.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.code, Some(0));
    assert_eq!(r.code, Some(0));
}

#[test]
fn eight_lines_always_written() {
    for input in [
        &b""[..],
        &b"0"[..],
        &b"-1"[..],
        &b"abc"[..],
        &b"2147483647"[..],
    ] {
        let c = run_with_args(c_bin(), input, &[]);
        let r = run_with_args(rust_bin(), input, &[]);
        assert_eq!(
            c.stdout.iter().filter(|&&b| b == b'\n').count(),
            8,
            "C emitted an unexpected number of lines"
        );
        assert_eq!(c.stdout, r.stdout);
        assert_eq!(c.stderr, r.stderr);
        assert_eq!(c.code, r.code);
    }
}
