use leftpad::leftpad::leftpad;

fn pad(s: &str, padding: &str, min_len: usize, dest_sz: usize) -> (usize, String) {
    let mut buf = vec![0u8; dest_sz];
    let r = leftpad(s, padding, min_len, &mut buf);
    // Output is null-terminated in buf[0..r], extract the string portion
    let out = if dest_sz > 0 {
        String::from_utf8_lossy(&buf[..r]).to_string()
    } else {
        String::new()
    };
    (r, out)
}

// No padding needed (str_len == min_len)
#[test]
fn test_no_padding_needed() {
    let (r, s) = pad("test", "*", 4, 64);
    assert_eq!(r, 4);
    assert_eq!(s, "test");
}

// Padding applied
#[test]
fn test_padding_applied() {
    let (r, s) = pad("test", "*", 9, 64);
    assert_eq!(r, 9);
    assert_eq!(s, "*****test");
}

// min_len < str_len: no padding, full string
#[test]
fn test_min_len_less_than_str_len() {
    let (r, s) = pad("test", "*", 2, 64);
    assert_eq!(r, 4);
    assert_eq!(s, "test");
}

// Multi-char padding repeated cyclically
#[test]
fn test_multi_char_padding() {
    let (r, s) = pad("test", " .", 9, 64);
    assert_eq!(r, 9);
    assert_eq!(s, " . . test");
}

// Empty padding defaults to space
#[test]
fn test_empty_padding_defaults_to_space() {
    let (r, s) = pad("test", "", 9, 64);
    assert_eq!(r, 9);
    assert_eq!(s, "     test");
}

// Empty str with padding
#[test]
fn test_empty_str_with_padding() {
    let (r, s) = pad("", "*", 9, 64);
    assert_eq!(r, 9);
    assert_eq!(s, "*********");
}

// Truncation: output exceeds dest_sz
#[test]
fn test_truncation() {
    let (r, s) = pad("test", "*", 11, 10);
    assert_eq!(r, 9);
    assert_eq!(s, "*******te");
}

// Empty dest returns needed length (like NULL dest in C)
#[test]
fn test_empty_dest_returns_needed_length() {
    let mut buf: [u8; 0] = [];
    assert_eq!(leftpad("test", " ", 6, &mut buf), 6);
    assert_eq!(leftpad("test", " ", 4, &mut buf), 4);
    assert_eq!(leftpad("test", " ", 2, &mut buf), 4);
}

// Empty string, min_len=0
#[test]
fn test_empty_str_zero_min_len() {
    let (r, s) = pad("", "*", 0, 64);
    assert_eq!(r, 0);
    assert_eq!(s, "");
}

// min_len=0 with non-empty string
#[test]
fn test_zero_min_len() {
    let (r, s) = pad("hello", "*", 0, 64);
    assert_eq!(r, 5);
    assert_eq!(s, "hello");
}

// dest_sz=1 (only room for null terminator)
#[test]
fn test_dest_sz_one() {
    let (r, s) = pad("test", "*", 9, 1);
    assert_eq!(r, 0);
    assert_eq!(s, "");
}

// Empty padding with padding needed
#[test]
fn test_empty_padding_with_padding() {
    let (r, s) = pad("test", "", 8, 64);
    assert_eq!(r, 8);
    assert_eq!(s, "    test");
}

// Empty str, empty padding
#[test]
fn test_empty_str_empty_padding() {
    let (r, s) = pad("", "", 5, 64);
    assert_eq!(r, 5);
    assert_eq!(s, "     ");
}

// Exact fit: output fills dest_sz exactly (dest_sz = output_len + 1 for null)
#[test]
fn test_exact_fit() {
    let (r, s) = pad("ab", "*", 4, 5);
    assert_eq!(r, 4);
    assert_eq!(s, "**ab");
}

// dest_sz=2
#[test]
fn test_dest_sz_two() {
    let (r, s) = pad("test", "*", 9, 2);
    assert_eq!(r, 1);
    assert_eq!(s, "*");
}

// 3-char padding pattern
#[test]
fn test_three_char_padding() {
    let (r, s) = pad("x", "abc", 7, 64);
    assert_eq!(r, 7);
    assert_eq!(s, "abcabcx");
}

// Empty string, empty padding, min_len=0
#[test]
fn test_all_empty() {
    let (r, s) = pad("", "", 0, 64);
    assert_eq!(r, 0);
    assert_eq!(s, "");
}

// Padding only (empty string padded to length)
#[test]
fn test_empty_str_padded() {
    let (r, s) = pad("", "*", 5, 64);
    assert_eq!(r, 5);
    assert_eq!(s, "*****");
}

fn main() {}
