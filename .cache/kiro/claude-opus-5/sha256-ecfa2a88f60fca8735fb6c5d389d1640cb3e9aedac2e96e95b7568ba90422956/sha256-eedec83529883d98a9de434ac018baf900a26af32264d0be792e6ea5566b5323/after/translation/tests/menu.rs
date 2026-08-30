//! The main loop: EOF handling, `sscanf("%d")` on the menu line, `fgets`
//! splitting long lines, and the `default:` arm of the switch.

mod harness;
use harness::{same, same_merged};

#[test]
fn empty_input() {
    // fgets returns NULL on the first call: menu printed once, then exit 0.
    same("empty_input", b"");
}

#[test]
fn exit_command() {
    same("exit_command", b"8\n");
}

#[test]
fn exit_command_without_newline() {
    same("exit_command_without_newline", b"8");
}

#[test]
fn commands_after_exit_are_never_read() {
    same("commands_after_exit", b"8\n1\nBoston\n3\n");
}

#[test]
fn eof_without_trailing_newline() {
    // A last line with no '\n' is still a complete fgets result.
    same("eof_without_trailing_newline", b"3");
}

#[test]
fn unparsable_choice() {
    same("unparsable_choice", b"abc\n");
}

#[test]
fn blank_line() {
    same("blank_line", b"\n8\n");
}

#[test]
fn whitespace_only_lines() {
    // %d skips whitespace and then finds no digit: matching failure.
    same("spaces_only", b"   \n8\n");
    same("tab_only", b"\t\n8\n");
    same("vertical_tab_and_formfeed", b"\x0b\x0c\n8\n");
    same("carriage_return_only", b"\r\n8\n");
}

#[test]
fn sign_without_digits() {
    same("plus_only", b"+\n8\n");
    same("minus_only", b"-\n8\n");
    same("minus_dot", b"-.5\n8\n");
    same("sign_with_space", b"- 3\n8\n");
}

#[test]
fn choice_with_leading_whitespace() {
    same("leading_whitespace_choice", b"   3\n8\n");
    same("newline_then_choice", b"\n\n3\n8\n");
}

#[test]
fn choice_with_trailing_junk() {
    // sscanf stops at the first non-digit and still reports one conversion.
    same("trailing_junk", b"3abc\n8\n");
    same("digits_then_space", b"3 4\n8\n");
    same("digits_then_sign", b"3-4\n8\n");
    same("leading_zeros", b"0003\n8\n");
    same("scientific_notation", b"3e2\n8\n");
}

#[test]
fn out_of_range_choices() {
    same("choice_zero_nine_negative", b"0\n9\n-1\n100\n8\n");
    same("choice_int_max", b"2147483647\n8\n");
    same("choice_int_min", b"-2147483648\n8\n");
    same("choice_minus_zero", b"-0\n8\n");
}

#[test]
fn choice_overflows_long_and_int() {
    // glibc's %d parses into a long that saturates at LONG_MAX/LONG_MIN and is
    // then truncated to int, so these do not mean what they say.
    same("choice_long_max", b"9223372036854775807\n8\n");
    same("choice_over_long_max", b"99999999999999999999\n8\n");
    same("choice_long_min", b"-9223372036854775808\n8\n");
    same("choice_under_long_min", b"-99999999999999999999\n8\n");
    // 4294967297 truncates to 1, i.e. it adds a city.
    same("choice_wraps_to_one", b"4294967297\nWrapped\n3\n8\n");
    // 2^31 truncates to INT_MIN.
    same("choice_two_pow_31", b"2147483648\n8\n");
    same("choice_many_digits", format!("{}\n8\n", "9".repeat(10_000)).as_bytes());
}

#[test]
fn menu_line_longer_than_the_buffer() {
    // fgets reads 255 bytes at a time; the tail becomes the next command.
    let mut input = Vec::new();
    input.push(b'1');
    input.extend(std::iter::repeat(b'0').take(300));
    input.extend_from_slice(b"\n8\n");
    same("long_menu_line", &input);
}

#[test]
fn menu_line_exactly_buffer_sized() {
    // 255 digits fill the buffer with no room for '\n', which is left for the
    // next fgets.
    let mut input = Vec::new();
    input.extend(std::iter::repeat(b'7').take(255));
    input.extend_from_slice(b"\n8\n");
    same("menu_line_255", &input);

    let mut input = Vec::new();
    input.extend(std::iter::repeat(b'7').take(256));
    input.extend_from_slice(b"\n8\n");
    same("menu_line_256", &input);
}

#[test]
fn embedded_nul_bytes() {
    // sscanf sees a C string, so everything from the NUL on is invisible to it.
    same("nul_after_digit", b"3\x004\n8\n");
    same("nul_first", b"\x003\n8\n");
    same("nul_after_whitespace", b" \x003\n8\n");
}

#[test]
fn crlf_line_endings() {
    // '\r' is not stripped: it survives into city names but not into %d.
    same("crlf", b"1\r\nA\r\n3\r\n4\r\nA\r\n4\r\nA\n8\r\n");
}

#[test]
fn many_invalid_lines() {
    let mut input = Vec::new();
    for _ in 0..200 {
        input.extend_from_slice(b"x\n");
    }
    input.extend_from_slice(b"8\n");
    same("many_invalid_lines", &input);
}

#[test]
fn utf8_and_high_bytes() {
    same("utf8_choice", "é\n8\n".as_bytes());
    same("high_bytes", b"\xff\xfe\n8\n");
}

#[test]
fn merged_streams_menu() {
    same_merged("merged_invalid_choices", b"0\n9\nabc\n8\n");
}
