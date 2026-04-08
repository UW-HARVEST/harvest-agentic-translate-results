use skp::skp::*;

// Helper: given src and the returned "end" slice, compute how many bytes were consumed
fn consumed(src: &str, end: &str) -> usize {
    src.len() - end.len()
}

// ============ Character classification helpers ============

#[test]
fn test_is_digit() {
    assert!(is_digit(b'0' as u32));
    assert!(is_digit(b'9' as u32));
    assert!(!is_digit(b'a' as u32));
    assert!(!is_digit(0));
}

#[test]
fn test_is_alpha() {
    assert!(is_alpha(b'a' as u32));
    assert!(is_alpha(b'Z' as u32));
    assert!(!is_alpha(b'0' as u32));
    assert!(!is_alpha(b'_' as u32));
}

#[test]
fn test_is_upper() {
    assert!(is_upper(b'A' as u32));
    assert!(is_upper(b'Z' as u32));
    assert!(!is_upper(b'a' as u32));
}

#[test]
fn test_is_lower() {
    assert!(is_lower(b'a' as u32));
    assert!(is_lower(b'z' as u32));
    assert!(!is_lower(b'A' as u32));
}

#[test]
fn test_is_xdigit() {
    assert!(is_xdigit(b'0' as u32));
    assert!(is_xdigit(b'9' as u32));
    assert!(is_xdigit(b'a' as u32));
    assert!(is_xdigit(b'F' as u32));
    assert!(!is_xdigit(b'g' as u32));
}

#[test]
fn test_is_blank() {
    assert!(is_blank(0x20)); // space
    assert!(is_blank(0x09)); // tab
    assert!(!is_blank(b'a' as u32));
    assert!(!is_blank(0x0A)); // newline is not blank
}

#[test]
fn test_is_break() {
    assert!(is_break(0x0A)); // LF
    assert!(is_break(0x0D)); // CR
    assert!(is_break(0x0C)); // FF
    assert!(!is_break(0x20)); // space is not break
    assert!(!is_break(b'a' as u32));
}

#[test]
fn test_is_space() {
    assert!(is_space(0x20)); // blank
    assert!(is_space(0x0A)); // break
    assert!(is_space(0x09)); // tab
    assert!(!is_space(b'a' as u32));
}

#[test]
fn test_is_alnum() {
    assert!(is_alnum(b'a' as u32));
    assert!(is_alnum(b'0' as u32));
    assert!(!is_alnum(b'_' as u32));
}

#[test]
fn test_is_idchr() {
    assert!(is_idchr(b'a' as u32));
    assert!(is_idchr(b'0' as u32));
    assert!(is_idchr(b'_' as u32));
    assert!(!is_idchr(b' ' as u32));
}

#[test]
fn test_is_ctrl() {
    assert!(is_ctrl(0x01));
    assert!(is_ctrl(0x1F));
    assert!(!is_ctrl(0x20));
    assert!(is_ctrl(0x7F));
}

// ============ Helper functions ============

#[test]
fn test_chr_cmp() {
    assert!(chr_cmp(b'a' as u32, b'a' as u32, 0));
    assert!(!chr_cmp(b'a' as u32, b'A' as u32, 0));
    assert!(chr_cmp(b'a' as u32, b'A' as u32, 1)); // case fold
}

#[test]
fn test_get_close() {
    assert_eq!(get_close(b'(' as u32), b')' as u32);
    assert_eq!(get_close(b'[' as u32), b']' as u32);
    assert_eq!(get_close(b'{' as u32), b'}' as u32);
    assert_eq!(get_close(b'<' as u32), b'>' as u32);
    assert_eq!(get_close(b'x' as u32), 0);
}

#[test]
fn test_get_qclose() {
    assert_eq!(get_qclose(b'\'' as u32), b'\'' as u32);
    assert_eq!(get_qclose(b'"' as u32), b'"' as u32);
    assert_eq!(get_qclose(b'`' as u32), b'`' as u32);
    assert_eq!(get_qclose(b'x' as u32), 0);
}

#[test]
fn test_skp_next_ascii() {
    let (c, rest) = skp_next("abc", 0);
    assert_eq!(c, b'a' as u32);
    assert_eq!(rest, "bc");
}

#[test]
fn test_skp_next_empty() {
    let (c, rest) = skp_next("", 0);
    assert_eq!(c, 0);
    assert_eq!(rest, "");
}

#[test]
fn test_skp_next_crlf() {
    let (c, rest) = skp_next("\r\n", 0);
    assert_eq!(c, 0x0D0A);
    assert_eq!(rest, "");
}

#[test]
fn test_is_oneof_basic() {
    assert!(is_oneof(b'a' as u32, "abc]", 0));
    assert!(is_oneof(b'c' as u32, "abc]", 0));
    assert!(!is_oneof(b'd' as u32, "abc]", 0));
}

#[test]
fn test_is_oneof_range() {
    assert!(is_oneof(b'c' as u32, "a-z]", 0));
    assert!(is_oneof(b'z' as u32, "a-z]", 0));
    assert!(!is_oneof(b'A' as u32, "a-z]", 0));
}

#[test]
fn test_is_oneof_bracket() {
    // ']' as first char in set means literal ']'
    assert!(is_oneof(b']' as u32, "]abc]", 0));
}

#[test]
fn test_is_oneof_null() {
    assert!(!is_oneof(0, "abc]", 0));
}

#[test]
fn test_is_string_basic() {
    assert_eq!(is_string("abc", "ab", 2, 0), 2);
    assert_eq!(is_string("abc", "xy", 2, 0), 0);
}

#[test]
fn test_skp_loop_len() {
    let s = "hello";
    assert_eq!(skp_loop_len(s, &s[3..]), 3);
    assert_eq!(skp_loop_len(s, s), 0);
}

// ============ Core skp_ function tests ============

#[test]
fn test_skp_alpha_single() {
    let (ret, _to, end) = skp_("abc", "a");
    assert_eq!(ret, 1);
    assert_eq!(consumed("abc", end), 1);
}

#[test]
fn test_skp_alpha_star() {
    let (ret, _to, end) = skp_("abc", "*a");
    assert_eq!(ret, 1);
    assert_eq!(consumed("abc", end), 3);
}

#[test]
fn test_skp_alpha_plus() {
    let (ret, _to, end) = skp_("abc", "+a");
    assert_eq!(ret, 1);
    assert_eq!(consumed("abc", end), 3);
}

#[test]
fn test_skp_digit_single() {
    let (ret, _to, end) = skp_("123", "d");
    assert_eq!(ret, 1);
    assert_eq!(consumed("123", end), 1);
}

#[test]
fn test_skp_digit_star() {
    let (ret, _to, end) = skp_("123", "*d");
    assert_eq!(ret, 1);
    assert_eq!(consumed("123", end), 3);
}

#[test]
fn test_skp_upper_single() {
    let (ret, _to, end) = skp_("ABC", "u");
    assert_eq!(ret, 1);
    assert_eq!(consumed("ABC", end), 1);
}

#[test]
fn test_skp_upper_star() {
    let (ret, _to, end) = skp_("ABC", "*u");
    assert_eq!(ret, 1);
    assert_eq!(consumed("ABC", end), 3);
}

#[test]
fn test_skp_lower_single() {
    let (ret, _to, end) = skp_("abc", "l");
    assert_eq!(ret, 1);
    assert_eq!(consumed("abc", end), 1);
}

#[test]
fn test_skp_lower_star() {
    let (ret, _to, end) = skp_("abc", "*l");
    assert_eq!(ret, 1);
    assert_eq!(consumed("abc", end), 3);
}

fn main() {}
