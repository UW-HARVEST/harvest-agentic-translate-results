use leftpad::leftpad::leftpad;

/// Helper that calls leftpad and returns (return_value, output_string).
/// Truncates `buf` at the first null byte, mimicking C's strcmp.
fn run(s: &str, pad: &str, min_len: usize, buf_sz: usize) -> (usize, String) {
    let mut buf = vec![0u8; buf_sz];
    let n = leftpad(s, pad, min_len, &mut buf);
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let out = String::from_utf8(buf[..end].to_vec()).unwrap();
    (n, out)
}

#[test]
fn no_padding_needed() {
    // assert(leftpad("test", "*", 4, buf, 10) == 4);
    // assert(strcmp("test", buf) == 0);
    let (n, out) = run("test", "*", 4, 10);
    assert_eq!(n, 4);
    assert_eq!(out, "test");
}

#[test]
fn padding_is_applied() {
    // assert(leftpad("test", "*", 9, buf, 10) == 9);
    // assert(strcmp("*****test", buf) == 0);
    let (n, out) = run("test", "*", 9, 10);
    assert_eq!(n, 9);
    assert_eq!(out, "*****test");
}

#[test]
fn no_truncation_below_str_len() {
    // assert(leftpad("test", "*", 2, buf, 10) == 4);
    // assert(strcmp("test", buf) == 0);
    let (n, out) = run("test", "*", 2, 10);
    assert_eq!(n, 4);
    assert_eq!(out, "test");
}

#[test]
fn padding_repeated() {
    // assert(leftpad("test", " .", 9, buf, 10) == 9);
    // assert(strcmp(" . . test", buf) == 0);
    let (n, out) = run("test", " .", 9, 10);
    assert_eq!(n, 9);
    assert_eq!(out, " . . test");
}

#[test]
fn empty_padding_defaults_to_space() {
    // assert(leftpad("test", NULL, 9, buf, 10) == 9);
    // assert(strcmp("     test", buf) == 0);
    // The Rust API has &str (not Option<&str>), so the equivalent of NULL is "".
    let (n, out) = run("test", "", 9, 10);
    assert_eq!(n, 9);
    assert_eq!(out, "     test");
}

#[test]
fn empty_string_is_pure_padding() {
    // assert(leftpad(NULL, "*", 9, buf, 10) == 9);
    // assert(strcmp("*********", buf) == 0);
    let (n, out) = run("", "*", 9, 10);
    assert_eq!(n, 9);
    assert_eq!(out, "*********");
}

#[test]
fn buffer_truncation() {
    // assert(leftpad("test", "*", 11, buf, 10) == 9);
    // assert(strcmp("*******te", buf) == 0);
    let (n, out) = run("test", "*", 11, 10);
    assert_eq!(n, 9);
    assert_eq!(out, "*******te");
}

#[test]
fn calculates_required_size_with_empty_buf() {
    // assert(leftpad("test", " ", 2, NULL, 10) == 4);
    // (The "NULL,10" path in C is unreachable in Rust; the meaningful "no buf"
    //  path is dest_sz == 0, which is &mut [].)
    let mut empty: [u8; 0] = [];

    assert_eq!(leftpad("test", " ", 2, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 4, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 6, &mut empty), 6);
}

#[test]
fn calculates_required_size_with_dest_sz_zero() {
    // assert(leftpad("test", " ", 2, buf, 0) == 4);
    // assert(leftpad("test", " ", 4, buf, 0) == 4);
    // assert(leftpad("test", " ", 6, buf, 0) == 6);
    // In Rust, "dest_sz=0" is equivalent to passing an empty slice.
    let mut empty: [u8; 0] = [];
    assert_eq!(leftpad("test", " ", 2, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 4, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 6, &mut empty), 6);
}

#[test]
fn exact_fit_buffer() {
    // Buffer big enough for output + null terminator.
    let (n, out) = run("hi", "*", 5, 6);
    assert_eq!(n, 5);
    assert_eq!(out, "***hi");
}

#[test]
fn buffer_exactly_one_byte() {
    // dest_sz = 1: only the null terminator fits, no chars.
    let mut buf = [0xAAu8; 1];
    let n = leftpad("test", "*", 9, &mut buf);
    // C: writes nothing because dest_sz - 1 == 0; then dest[0] = '\0';
    //    return value is dest_len which is 0.
    assert_eq!(n, 0);
    assert_eq!(buf[0], 0);
}

#[test]
fn buffer_two_bytes_one_char_plus_null() {
    let mut buf = [0xAAu8; 2];
    let n = leftpad("test", "*", 9, &mut buf);
    // Only one byte of content fits (from padding) + null terminator.
    assert_eq!(n, 1);
    assert_eq!(buf[0], b'*');
    assert_eq!(buf[1], 0);
}

#[test]
fn padding_longer_than_needed() {
    // Padding string is longer than npad: only first npad chars used.
    let (n, out) = run("x", "abcdef", 4, 10);
    // npad = 3, take first 3 of "abcdef" => "abcx"
    assert_eq!(n, 4);
    assert_eq!(out, "abcx");
}

#[test]
fn min_len_zero() {
    let (n, out) = run("hello", "*", 0, 10);
    assert_eq!(n, 5);
    assert_eq!(out, "hello");
}

#[test]
fn empty_string_min_len_zero() {
    let (n, out) = run("", "*", 0, 10);
    assert_eq!(n, 0);
    assert_eq!(out, "");
}

#[test]
fn empty_string_empty_padding_min_len_zero() {
    let (n, out) = run("", "", 0, 10);
    assert_eq!(n, 0);
    assert_eq!(out, "");
}

#[test]
fn truncation_when_string_too_long_for_buf() {
    // String alone is larger than buffer.
    let (n, out) = run("abcdefghij", "*", 5, 5);
    // npad = 0, str_len = 10, dest_sz=5: only 4 chars + null fit.
    assert_eq!(n, 4);
    assert_eq!(out, "abcd");
}

#[test]
fn long_padding_cycle() {
    // " ." padding, min_len=11, str="x" => 10 padding chars cycling: " . . . . ."
    let (n, out) = run("x", " .", 11, 20);
    assert_eq!(n, 11);
    assert_eq!(out, " . . . . .x");
}

#[test]
fn three_char_padding() {
    // padding "abc" cycles: a,b,c,a,b,c,...
    let (n, out) = run("Z", "abc", 8, 20);
    // npad = 7: a b c a b c a + Z => "abcabcaZ"
    assert_eq!(n, 8);
    assert_eq!(out, "abcabcaZ");
}

fn main() {}
