use skp::skp;

// ============================================================
// skp_next
// ============================================================

#[test]
fn test_skp_next_ascii() {
    let (c, rest) = skp::skp_next("ABC", 0);
    assert_eq!(c, 65);
    assert_eq!(rest, "BC");
}

#[test]
fn test_skp_next_ascii_iso() {
    let (c, rest) = skp::skp_next("ABC", 1);
    assert_eq!(c, 65);
    assert_eq!(rest, "BC");
}

#[test]
fn test_skp_next_utf8_egrave() {
    // è = bytes C3 A8. Rust uses u8, so c = (0xC3 << 8) | 0xA8 = 0xC3A8
    let (c, rest) = skp::skp_next("\u{00E8}", 0);
    assert_eq!(c, 0xC3A8);
    assert_eq!(rest, "");
}

#[test]
fn test_skp_next_iso_egrave() {
    // In ISO mode, only first byte is read: 0xC3
    let (c, rest) = skp::skp_next("\u{00E8}", 1);
    assert_eq!(c, 0xC3);
    assert_eq!(rest.len(), 1); // one continuation byte left
}

#[test]
fn test_skp_next_empty() {
    let (c, rest) = skp::skp_next("", 0);
    assert_eq!(c, 0);
    assert_eq!(rest, "");
}

#[test]
fn test_skp_next_crlf() {
    let (c, rest) = skp::skp_next("\r\n", 0);
    assert_eq!(c, 0x0D0A);
    assert_eq!(rest, "");
}

#[test]
fn test_skp_next_tab() {
    let (c, _) = skp::skp_next("\t", 0);
    assert_eq!(c, 9);
}

#[test]
fn test_skp_next_three_byte_utf8() {
    // U+2000 = E2 80 80 -> (0xE2 << 16) | (0x80 << 8) | 0x80 = 0xE28080
    let (c, rest) = skp::skp_next("\u{2000}", 0);
    assert_eq!(c, 0xE28080);
    assert_eq!(rest, "");
}

// ============================================================
// chr_cmp
// ============================================================

#[test]
fn test_chr_cmp_same_no_fold() {
    assert!(skp::chr_cmp(b'A' as u32, b'A' as u32, 0));
}

#[test]
fn test_chr_cmp_diff_case_no_fold() {
    assert!(!skp::chr_cmp(b'A' as u32, b'a' as u32, 0));
}

#[test]
fn test_chr_cmp_diff_case_fold() {
    assert!(skp::chr_cmp(b'A' as u32, b'a' as u32, 1));
}

#[test]
fn test_chr_cmp_diff_chars_fold() {
    assert!(!skp::chr_cmp(b'A' as u32, b'B' as u32, 1));
}

// ============================================================
// is_blank
// ============================================================

#[test]
fn test_is_blank() {
    assert!(skp::is_blank(0x20));
    assert!(skp::is_blank(0x09));
    assert!(!skp::is_blank(0x41));
    assert!(!skp::is_blank(0xA0)); // < 0xFF, not 0x20 or 0x09
    assert!(skp::is_blank(0xC2A0));
    assert!(skp::is_blank(0xE19A80));
    assert!(skp::is_blank(0xE28080));
    assert!(skp::is_blank(0xE2808A));
    assert!(skp::is_blank(0xE280AF));
    assert!(!skp::is_blank(0xE38080)); // mask 0xE38000 doesn't match case 0xE38080
    assert!(!skp::is_blank(0));
}

// ============================================================
// is_break
// ============================================================

#[test]
fn test_is_break() {
    assert!(skp::is_break(0x0A));
    assert!(skp::is_break(0x0C));
    assert!(skp::is_break(0x0D));
    assert!(skp::is_break(0x85));
    assert!(skp::is_break(0x0D0A));
    assert!(skp::is_break(0xC285));
    assert!(skp::is_break(0xE280A8));
    assert!(skp::is_break(0xE280A9));
    assert!(!skp::is_break(0x20));
    assert!(!skp::is_break(0x41));
}

// ============================================================
// is_space
// ============================================================

#[test]
fn test_is_space() {
    assert!(skp::is_space(0x20));
    assert!(skp::is_space(0x0A));
    assert!(!skp::is_space(0x41));
}

// ============================================================
// Character class functions
// ============================================================

#[test]
fn test_is_digit() {
    assert!(skp::is_digit(0x30));
    assert!(skp::is_digit(0x39));
    assert!(!skp::is_digit(0x40));
}

#[test]
fn test_is_xdigit() {
    assert!(skp::is_xdigit(0x41)); // A
    assert!(skp::is_xdigit(0x46)); // F
    assert!(!skp::is_xdigit(0x47)); // G
    assert!(skp::is_xdigit(0x61)); // a
    assert!(skp::is_xdigit(0x66)); // f
}

#[test]
fn test_is_upper() {
    assert!(skp::is_upper(0x41));
    assert!(skp::is_upper(0x5A));
    assert!(!skp::is_upper(0x61));
}

#[test]
fn test_is_lower() {
    assert!(skp::is_lower(0x61));
    assert!(skp::is_lower(0x7A));
    assert!(!skp::is_lower(0x41));
}

#[test]
fn test_is_alpha() {
    assert!(skp::is_alpha(0x41));
    assert!(skp::is_alpha(0x61));
    assert!(!skp::is_alpha(0x30));
}

#[test]
fn test_is_idchr() {
    assert!(skp::is_idchr(0x5F)); // _
    assert!(skp::is_idchr(0x41)); // A
    assert!(skp::is_idchr(0x30)); // 0
    assert!(!skp::is_idchr(0x20)); // space
}

#[test]
fn test_is_alnum() {
    assert!(skp::is_alnum(0x41));
    assert!(skp::is_alnum(0x30));
    assert!(!skp::is_alnum(0x5F)); // _ is not alnum
}

#[test]
fn test_is_ctrl() {
    assert!(skp::is_ctrl(0x00));
    assert!(skp::is_ctrl(0x1F));
    assert!(skp::is_ctrl(0x7F));
    assert!(!skp::is_ctrl(0x20));
    assert!(skp::is_ctrl(0xC280));
    assert!(skp::is_ctrl(0xC29F));
    assert!(!skp::is_ctrl(0xC2A0));
    assert!(skp::is_ctrl(0x9F));
}

// ============================================================
// get_close / get_qclose
// ============================================================

#[test]
fn test_get_close() {
    assert_eq!(skp::get_close(b'(' as u32), 41);
    assert_eq!(skp::get_close(b'[' as u32), 93);
    assert_eq!(skp::get_close(b'{' as u32), 125);
    assert_eq!(skp::get_close(b'<' as u32), 62);
    assert_eq!(skp::get_close(b'A' as u32), 0);
}

#[test]
fn test_get_qclose() {
    assert_eq!(skp::get_qclose(39), 39);  // '
    assert_eq!(skp::get_qclose(34), 34);  // "
    assert_eq!(skp::get_qclose(96), 96);  // `
    assert_eq!(skp::get_qclose(65), 0);   // A
}

// ============================================================
// is_oneof
// ============================================================

#[test]
fn test_is_oneof_basic() {
    assert!(skp::is_oneof(b'A' as u32, "ABC]", 0));
    assert!(!skp::is_oneof(b'D' as u32, "ABC]", 0));
    assert!(!skp::is_oneof(0, "ABC]", 0));
}

#[test]
fn test_is_oneof_range() {
    assert!(skp::is_oneof(b'B' as u32, "A-D]", 0));
    assert!(!skp::is_oneof(b'E' as u32, "A-D]", 0));
    assert!(skp::is_oneof(b'A' as u32, "A-Z]", 0));
    assert!(skp::is_oneof(b'Z' as u32, "A-Z]", 0));
    assert!(!skp::is_oneof(b'a' as u32, "A-Z]", 0));
}

#[test]
fn test_is_oneof_bracket_first() {
    assert!(skp::is_oneof(b']' as u32, "]ABC]", 0));
    assert!(skp::is_oneof(b'A' as u32, "]ABC]", 0));
}

// ============================================================
// is_string
// ============================================================

#[test]
fn test_is_string_match() {
    assert_eq!(skp::is_string("ABC", "ABC", 3, 0), 3);
}

#[test]
fn test_is_string_no_match() {
    assert_eq!(skp::is_string("ABC", "ABX", 3, 0), 0);
}

#[test]
fn test_is_string_case_fold() {
    assert_eq!(skp::is_string("abc", "ABC", 3, 1), 3);
    assert_eq!(skp::is_string("abc", "ABC", 3, 0), 0);
}

fn main() {}
