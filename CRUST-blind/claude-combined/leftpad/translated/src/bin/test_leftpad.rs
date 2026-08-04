#[allow(unused_imports)]
use leftpad::leftpad::leftpad;

#[allow(dead_code)]
fn buf_to_str(buf: &[u8], len: usize) -> String {
    std::str::from_utf8(&buf[..len]).unwrap().to_string()
}

#[test]
fn test_no_padding_needed() {
    // C: leftpad("test", "*", 4, buf, 10) == 4, buf == "test"
    let mut buf = [b'X'; 10];
    let r = leftpad("test", "*", 4, &mut buf);
    assert_eq!(r, 4);
    assert_eq!(buf_to_str(&buf, 4), "test");
    assert_eq!(buf[4], 0); // null terminator
}

#[test]
fn test_padding_applied() {
    // C: leftpad("test", "*", 9, buf, 10) == 9, buf == "*****test"
    let mut buf = [b'X'; 10];
    let r = leftpad("test", "*", 9, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(buf_to_str(&buf, 9), "*****test");
    assert_eq!(buf[9], 0);
}

#[test]
fn test_string_not_truncated_smaller_min() {
    // C: leftpad("test", "*", 2, buf, 10) == 4, buf == "test"
    let mut buf = [b'X'; 10];
    let r = leftpad("test", "*", 2, &mut buf);
    assert_eq!(r, 4);
    assert_eq!(buf_to_str(&buf, 4), "test");
    assert_eq!(buf[4], 0);
}

#[test]
fn test_padding_repeated() {
    // C: leftpad("test", " .", 9, buf, 10) == 9, buf == " . . test"
    let mut buf = [b'X'; 10];
    let r = leftpad("test", " .", 9, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(buf_to_str(&buf, 9), " . . test");
    assert_eq!(buf[9], 0);
}

#[test]
fn test_default_padding_is_space() {
    // C: leftpad("test", NULL, 9, buf, 10) == 9, buf == "     test"
    // In Rust we use empty string for "no padding"
    let mut buf = [b'X'; 10];
    let r = leftpad("test", "", 9, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(buf_to_str(&buf, 9), "     test");
    assert_eq!(buf[9], 0);
}

#[test]
fn test_empty_string_pads_full() {
    // C: leftpad(NULL, "*", 9, buf, 10) == 9, buf == "*********"
    let mut buf = [b'X'; 10];
    let r = leftpad("", "*", 9, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(buf_to_str(&buf, 9), "*********");
    assert_eq!(buf[9], 0);
}

#[test]
fn test_truncation() {
    // C: leftpad("test", "*", 11, buf, 10) == 9, buf == "*******te"
    let mut buf = [b'X'; 10];
    let r = leftpad("test", "*", 11, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(buf_to_str(&buf, 9), "*******te");
    assert_eq!(buf[9], 0);
}

#[test]
fn test_size_query_no_buf() {
    // C: leftpad("test", " ", 2, NULL, 10) == 4
    let mut empty: [u8; 0] = [];
    assert_eq!(leftpad("test", " ", 2, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 4, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 6, &mut empty), 6);
}

#[test]
fn test_size_query_zero_buf_sz() {
    // In C, this is dest != NULL but dest_sz == 0; in Rust the
    // empty slice case is the only equivalent representation.
    let mut empty: [u8; 0] = [];
    assert_eq!(leftpad("test", " ", 2, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 4, &mut empty), 4);
    assert_eq!(leftpad("test", " ", 6, &mut empty), 6);
}

#[test]
fn test_multichar_padding_truncation() {
    // C: leftpad("hi", "abc", 8, buf, 10) == 8, buf == "abcabchi"
    let mut buf = [b'X'; 10];
    let r = leftpad("hi", "abc", 8, &mut buf);
    assert_eq!(r, 8);
    assert_eq!(buf_to_str(&buf, 8), "abcabchi");
    assert_eq!(buf[8], 0);
}

#[test]
fn test_min_len_zero() {
    // C: leftpad("hello", "*", 0, buf, 10) == 5, buf == "hello"
    let mut buf = [b'X'; 10];
    let r = leftpad("hello", "*", 0, &mut buf);
    assert_eq!(r, 5);
    assert_eq!(buf_to_str(&buf, 5), "hello");
    assert_eq!(buf[5], 0);
}

#[test]
fn test_dest_sz_one() {
    // C: leftpad("test", "*", 5, buf, 1) == 0, buf == ""
    let mut buf = [b'X'; 1];
    let r = leftpad("test", "*", 5, &mut buf);
    assert_eq!(r, 0);
    assert_eq!(buf[0], 0);
}

#[test]
fn test_empty_string_full_pad() {
    // C: leftpad("", "*", 5, buf, 10) == 5, buf == "*****"
    let mut buf = [b'X'; 10];
    let r = leftpad("", "*", 5, &mut buf);
    assert_eq!(r, 5);
    assert_eq!(buf_to_str(&buf, 5), "*****");
    assert_eq!(buf[5], 0);
}

#[test]
fn test_dest_sz_equal_min_len() {
    // C: leftpad("test", "*", 5, buf, 5) == 4, buf == "*tes"
    let mut buf = [b'X'; 5];
    let r = leftpad("test", "*", 5, &mut buf);
    assert_eq!(r, 4);
    assert_eq!(buf_to_str(&buf, 4), "*tes");
    assert_eq!(buf[4], 0);
}

#[test]
fn test_three_char_padding() {
    // C: leftpad("zz", "abc", 7, buf, 10) == 7, buf == "abcabzz"
    let mut buf = [b'X'; 10];
    let r = leftpad("zz", "abc", 7, &mut buf);
    assert_eq!(r, 7);
    assert_eq!(buf_to_str(&buf, 7), "abcabzz");
    assert_eq!(buf[7], 0);
}

fn main() {}
