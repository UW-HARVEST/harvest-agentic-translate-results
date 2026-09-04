//! Differential tests for `main()`: the menu loop, `fgets` on the choice line
//! and `sscanf(input, "%d", &choice)`.

mod common;

use common::assert_same;

#[test]
fn empty_stdin() {
    // fgets returns NULL straight away -> the loop breaks after one menu.
    assert_same("empty_stdin", b"");
}

#[test]
fn only_newline() {
    // sscanf finds no conversion at all (returns EOF) -> "Invalid input".
    assert_same("only_newline", b"\n");
}

#[test]
fn blank_lines_then_eof() {
    assert_same("blank_lines", b"\n\n\n");
}

#[test]
fn exit_choice() {
    assert_same("exit", b"7\n");
}

#[test]
fn exit_without_trailing_newline() {
    assert_same("exit_no_nl", b"7");
}

#[test]
fn non_numeric_choice() {
    assert_same("abc", b"abc\n");
    assert_same("only_spaces", b"   \n");
    assert_same("only_tabs", b"\t\t\n");
    assert_same("plus_only", b"+\n");
    assert_same("minus_only", b"-\n");
    assert_same("dot_only", b".\n");
}

#[test]
fn out_of_range_choices() {
    for choice in ["0", "8", "9", "10", "42", "-1", "-7", "100"] {
        assert_same(choice, format!("{choice}\n7\n").as_bytes());
    }
}

#[test]
fn choice_with_trailing_garbage() {
    assert_same("7abc", b"7abc\n");
    assert_same("7 8", b"7 8\n");
    assert_same("float", b"7.9\n");
    assert_same("1_then_text", b"1x\nhello\n\n7\n");
}

#[test]
fn choice_with_leading_whitespace() {
    assert_same("ws7", b"  \t 7\n");
    assert_same("nl_then_7", b"\n7\n");
    assert_same("plus7", b"+7\n");
    assert_same("minus0", b"-0\n7\n");
}

#[test]
fn choice_hex_is_decimal_zero() {
    // "%d" stops at the 'x', converting just the leading 0.
    assert_same("hex", b"0x7\n7\n");
}

#[test]
fn choice_integer_truncation_and_overflow() {
    // strtol saturates at LONG_MAX/LONG_MIN and the result is truncated to int.
    assert_same("intmax", b"2147483647\n7\n");
    assert_same("intmax_plus_1", b"2147483648\n7\n");
    assert_same("intmin", b"-2147483648\n7\n");
    assert_same("intmin_minus_1", b"-2147483649\n7\n");
    assert_same("pow2_32_plus_1", b"4294967297\n7\n");
    assert_same("overflow", b"99999999999999999999\n7\n");
    assert_same("negative_overflow", b"-99999999999999999999\n7\n");
    // 4294967303 == 7 mod 2^32, so the truncated int is 7 and the program exits.
    assert_same("wraps_to_7", b"4294967303\n7\n");
}

#[test]
fn choice_line_longer_than_the_buffer() {
    // fgets only takes 255 bytes, so a long line is split across iterations and
    // the tail is parsed as the next choice.
    let mut input = vec![b'x'; 300];
    input.extend_from_slice(b"7\n");
    assert_same("long_choice_line", &input);

    let mut digits = vec![b'1'; 260];
    digits.extend_from_slice(b"\n7\n");
    assert_same("260_ones", &digits);

    let mut padded = vec![b' '; 255];
    padded.extend_from_slice(b"7\n7\n");
    assert_same("255_spaces_then_7", &padded);

    // Exactly 255 bytes plus the newline: the newline lands in the next read.
    let mut exact = vec![b'7'; 255];
    exact.extend_from_slice(b"\n");
    assert_same("255_sevens", &exact);
}

#[test]
fn choice_with_embedded_nul() {
    assert_same("nul_then_7", b"\0\0 7\n7\n");
    assert_same("7_then_nul", b"7\0\n");
    assert_same("nul_line", b"\0\n7\n");
}

#[test]
fn choice_with_high_bytes() {
    assert_same("high_bytes", b"\xff\xfe\n7\n");
    assert_same("high_then_digit", b"\xff7\n7\n");
}

#[test]
fn choice_with_carriage_return() {
    assert_same("crlf", b"7\r\n");
    assert_same("cr_only", b"\r7\n");
}

#[test]
fn many_menu_rounds() {
    let mut input = Vec::new();
    for _ in 0..50 {
        input.extend_from_slice(b"9\n");
    }
    input.extend_from_slice(b"7\n");
    assert_same("many_rounds", &input);
}

#[test]
fn no_exit_just_eof() {
    // Falling off the end of the while loop returns 0 as well.
    assert_same("eof_after_3", b"3\n");
    assert_same("eof_after_4", b"4\n");
}
