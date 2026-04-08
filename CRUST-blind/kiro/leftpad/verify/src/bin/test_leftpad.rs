use leftpad::leftpad::leftpad;

fn call(s: &str, pad: &str, min_len: usize, dest: &mut [u8]) -> (usize, String) {
    let r = leftpad(s, pad, min_len, dest);
    let out = String::from_utf8_lossy(&dest[..r]).to_string();
    (r, out)
}

#[test]
fn test_no_padding_needed() {
    let mut buf = [0u8; 64];
    let (r, s) = call("test", "*", 4, &mut buf);
    assert_eq!(r, 4);
    assert_eq!(s, "test");
}

#[test]
fn test_padding_applied() {
    let mut buf = [0u8; 64];
    let (r, s) = call("test", "*", 9, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(s, "*****test");
}

#[test]
fn test_min_len_less_than_str_len() {
    let mut buf = [0u8; 64];
    let (r, s) = call("test", "*", 2, &mut buf);
    assert_eq!(r, 4);
    assert_eq!(s, "test");
}

#[test]
fn test_multi_char_padding() {
    let mut buf = [0u8; 64];
    let (r, s) = call("test", " .", 9, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(s, " . . test");
}

#[test]
fn test_empty_padding_defaults_to_spaces() {
    let mut buf = [0u8; 64];
    let (r, s) = call("test", "", 9, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(s, "     test");
}

#[test]
fn test_truncation() {
    let mut buf = [0u8; 10];
    let (r, s) = call("test", "*", 11, &mut buf);
    assert_eq!(r, 9);
    assert_eq!(s, "*******te");
}

#[test]
fn test_dest_sz_zero_with_padding() {
    let mut buf = [0u8; 0];
    let r = leftpad("test", " ", 6, &mut buf);
    assert_eq!(r, 6);
}

#[test]
fn test_dest_sz_zero_no_padding() {
    let mut buf = [0u8; 0];
    let r = leftpad("test", " ", 2, &mut buf);
    assert_eq!(r, 4);
}

#[test]
fn test_empty_str_with_padding() {
    let mut buf = [0u8; 64];
    let (r, s) = call("", "*", 5, &mut buf);
    assert_eq!(r, 5);
    assert_eq!(s, "*****");
}

#[test]
fn test_min_len_zero() {
    let mut buf = [0u8; 64];
    let (r, s) = call("test", "*", 0, &mut buf);
    assert_eq!(r, 4);
    assert_eq!(s, "test");
}

#[test]
fn test_dest_sz_one() {
    let mut buf = [0u8; 1];
    let (r, s) = call("test", "*", 9, &mut buf);
    assert_eq!(r, 0);
    assert_eq!(s, "");
}

#[test]
fn test_dest_sz_two() {
    let mut buf = [0u8; 2];
    let (r, s) = call("test", "*", 9, &mut buf);
    assert_eq!(r, 1);
    assert_eq!(s, "*");
}

#[test]
fn test_three_char_padding_pattern() {
    let mut buf = [0u8; 64];
    let (r, s) = call("hi", "abc", 8, &mut buf);
    assert_eq!(r, 8);
    assert_eq!(s, "abcabchi");
}

#[test]
fn test_exact_fit() {
    let mut buf = [0u8; 9];
    let (r, s) = call("test", "*", 8, &mut buf);
    assert_eq!(r, 8);
    assert_eq!(s, "****test");
}

#[test]
fn test_empty_padding_spaces() {
    let mut buf = [0u8; 64];
    let (r, s) = call("test", "", 8, &mut buf);
    assert_eq!(r, 8);
    assert_eq!(s, "    test");
}

fn main() {}
