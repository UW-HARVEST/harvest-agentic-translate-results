use skp::skp::*;

// =========== is_blank ===========
#[test]
fn test_is_blank_basic() {
    assert!(is_blank(0x20));
    assert!(is_blank(0x09));
    assert!(!is_blank(0x41));
    assert!(!is_blank(0xA0));
    assert!(is_blank(0xC2A0));
    assert!(is_blank(0xE19A80));
    assert!(is_blank(0xE28080));
    assert!(is_blank(0xE2808A));
    assert!(is_blank(0xE280AF));
    assert!(!is_blank(0xE38080)); // C output 0
    assert!(!is_blank(0xFF));
    assert!(!is_blank(0x0A));
    assert!(!is_blank(0x00));
    assert!(!is_blank(0xE2808B));
}

// =========== is_break ===========
#[test]
fn test_is_break_basic() {
    assert!(is_break(0x0A));
    assert!(is_break(0x0C));
    assert!(is_break(0x0D));
    assert!(is_break(0x85));
    assert!(is_break(0x0D0A));
    assert!(is_break(0xC285));
    assert!(is_break(0xE280A8));
    assert!(is_break(0xE280A9));
    assert!(!is_break(0x20));
    assert!(!is_break(0x09));
    assert!(!is_break(0x41));
    assert!(!is_break(0x0E));
    assert!(!is_break(0xFE));
}

// =========== is_space ===========
#[test]
fn test_is_space_basic() {
    assert!(is_space(0x20));
    assert!(is_space(0x09));
    assert!(is_space(0x0A));
    assert!(is_space(0x0D));
    assert!(!is_space(0x41));
    assert!(!is_space(0xA0));
    assert!(is_space(0x0D0A));
    assert!(is_space(0xE19A80));
    assert!(!is_space(0x00));
}

// =========== is_digit ===========
#[test]
fn test_is_digit_range() {
    for c in b'0'..=b'9' {
        assert!(is_digit(c as u32), "0x{:X} should be digit", c);
    }
    assert!(!is_digit(b'/' as u32));
    assert!(!is_digit(b':' as u32));
    assert!(!is_digit(b'A' as u32));
    assert!(!is_digit(0));
}

// =========== is_xdigit ===========
#[test]
fn test_is_xdigit_basic() {
    assert!(is_xdigit(b'0' as u32));
    assert!(is_xdigit(b'9' as u32));
    assert!(is_xdigit(b'A' as u32));
    assert!(is_xdigit(b'F' as u32));
    assert!(!is_xdigit(b'G' as u32));
    assert!(is_xdigit(b'a' as u32));
    assert!(is_xdigit(b'f' as u32));
    assert!(!is_xdigit(b'g' as u32));
    assert!(!is_xdigit(b'/' as u32));
}

// =========== is_upper / is_lower ===========
#[test]
fn test_is_upper_lower() {
    assert!(!is_upper(b'@' as u32));
    for c in b'A'..=b'Z' {
        assert!(is_upper(c as u32));
        assert!(!is_lower(c as u32));
    }
    assert!(!is_upper(b'[' as u32));
    assert!(!is_lower(b'`' as u32));
    for c in b'a'..=b'z' {
        assert!(is_lower(c as u32));
        assert!(!is_upper(c as u32));
    }
    assert!(!is_lower(b'{' as u32));
}

// =========== is_alpha ===========
#[test]
fn test_is_alpha() {
    assert!(is_alpha(b'A' as u32));
    assert!(is_alpha(b'Z' as u32));
    assert!(is_alpha(b'a' as u32));
    assert!(is_alpha(b'z' as u32));
    assert!(!is_alpha(b'0' as u32));
    assert!(!is_alpha(b'9' as u32));
    assert!(!is_alpha(b'@' as u32));
    assert!(!is_alpha(b'[' as u32));
    assert!(!is_alpha(b'`' as u32));
    assert!(!is_alpha(b'{' as u32));
}

// =========== is_idchr ===========
#[test]
fn test_is_idchr() {
    assert!(is_idchr(b'A' as u32));
    assert!(is_idchr(b'Z' as u32));
    assert!(is_idchr(b'a' as u32));
    assert!(is_idchr(b'z' as u32));
    assert!(is_idchr(b'0' as u32));
    assert!(is_idchr(b'9' as u32));
    assert!(is_idchr(b'_' as u32));
    assert!(!is_idchr(b'-' as u32));
    assert!(!is_idchr(b'$' as u32));
    assert!(!is_idchr(b' ' as u32));
}

// =========== is_alnum ===========
#[test]
fn test_is_alnum() {
    assert!(is_alnum(b'A' as u32));
    assert!(is_alnum(b'9' as u32));
    assert!(!is_alnum(b'_' as u32)); // underscore is NOT alnum (it's idchr)
    assert!(is_alnum(b'a' as u32));
    assert!(!is_alnum(b'!' as u32));
    assert!(!is_alnum(b' ' as u32));
}

// =========== is_ctrl ===========
#[test]
fn test_is_ctrl() {
    assert!(is_ctrl(0x00));
    assert!(is_ctrl(0x1F));
    assert!(!is_ctrl(0x20));
    assert!(!is_ctrl(0x7E));
    assert!(is_ctrl(0x7F));
    assert!(is_ctrl(0x9F));
    assert!(!is_ctrl(0xA0));
    assert!(is_ctrl(0xC280));
    assert!(is_ctrl(0xC29F));
    assert!(!is_ctrl(0xC2A0));
}

// =========== chr_cmp ===========
#[test]
fn test_chr_cmp_basic() {
    assert!(chr_cmp(b'A' as u32, b'A' as u32, 0));
    assert!(!chr_cmp(b'A' as u32, b'B' as u32, 0));
    assert!(!chr_cmp(b'A' as u32, b'a' as u32, 0));
    assert!(chr_cmp(b'A' as u32, b'a' as u32, 1));
    assert!(chr_cmp(b'z' as u32, b'Z' as u32, 1));
    assert!(chr_cmp(0xC3A8, 0xC3A8, 0));
    // C output: chr_cmp(0xC3A8,0xC388,1)=0 (because both are >0x7F, no folding)
    assert!(!chr_cmp(0xC3A8, 0xC388, 1));
}

// =========== skp_next (ASCII only — multi-byte sign-extension differs) ===========
#[test]
fn test_skp_next_ascii() {
    let (c, rest) = skp_next("ABC", 0);
    assert_eq!(c, 0x41);
    assert_eq!(rest, "BC");

    // Empty string
    let (c, rest) = skp_next("", 0);
    assert_eq!(c, 0);
    assert_eq!(rest, "");

    // CR/LF combination
    let (c, rest) = skp_next("\r\nX", 0);
    assert_eq!(c, 0x0D0A);
    assert_eq!(rest, "X");

    // Single byte ISO mode
    let (c, rest) = skp_next("A", 1);
    assert_eq!(c, 0x41);
    assert_eq!(rest, "");
}

// =========== get_close ===========
#[test]
fn test_get_close() {
    assert_eq!(get_close(b'(' as u32), b')' as u32);
    assert_eq!(get_close(b'[' as u32), b']' as u32);
    assert_eq!(get_close(b'{' as u32), b'}' as u32);
    assert_eq!(get_close(b'<' as u32), b'>' as u32);
    assert_eq!(get_close(b'"' as u32), 0);
    assert_eq!(get_close(b'a' as u32), 0);
}

// =========== get_qclose ===========
#[test]
fn test_get_qclose() {
    assert_eq!(get_qclose(b'\'' as u32), b'\'' as u32);
    assert_eq!(get_qclose(b'"' as u32), b'"' as u32);
    assert_eq!(get_qclose(b'`' as u32), b'`' as u32);
    assert_eq!(get_qclose(b'a' as u32), 0);
}

// =========== is_oneof ===========
#[test]
fn test_is_oneof_basic() {
    assert!(is_oneof(b'a' as u32, "abc]", 0));
    assert!(!is_oneof(b'z' as u32, "abc]", 0));
    assert!(is_oneof(b']' as u32, "]abc]", 0));
    assert!(is_oneof(b'a' as u32, "]abc]", 0));
    // Range
    assert!(is_oneof(b'5' as u32, "0-9]", 0));
    assert!(!is_oneof(b'a' as u32, "0-9]", 0));
    assert!(is_oneof(b'A' as u32, "A-Za-z]", 0));
    assert!(is_oneof(b'z' as u32, "A-Za-z]", 0));
    // Null character
    assert!(!is_oneof(0, "abc]", 0));
    // Trailing dash treated literally when followed by ]
    assert!(is_oneof(b'-' as u32, "a-]", 0));
}

// =========== is_string ===========
#[test]
fn test_is_string_basic() {
    assert_eq!(is_string("abcd", "abc", 3, 0), 3);
    assert_eq!(is_string("abcd", "abc", 3, 1), 3);
    // case-insensitive match
    assert_eq!(is_string("ABCD", "abc", 3, 1), 3);
    assert_eq!(is_string("ABCD", "abc", 3, 0), 0);
    // alternative pattern: when first alt fails, use second alt
    // pattern "xyz\x0eabc" with len=7 — abcd matches abc via second alt
    let with_alt = "xyz\x0eabc";
    assert_eq!(is_string("abcd", with_alt, 7, 0), 3);
    // Same logic with shorter prefix
    let with_alt2 = "XY\x0eabc";
    assert_eq!(is_string("abcd", with_alt2, 6, 0), 3);
    assert_eq!(is_string("", "abc", 3, 0), 0);
}

// =========== skp_loop_len ===========
#[test]
fn test_skp_loop_len_zero() {
    let s = "Hello World";
    assert_eq!(skp_loop_len(s, s), 0);
    // Slice 5 chars in
    assert_eq!(skp_loop_len(s, &s[5..]), 5);
    // Negative case: returns 0 (because to is before start)
    assert_eq!(skp_loop_len(&s[5..], s), 0);
}

// =========== match_pat ===========
#[test]
fn test_match_pat_digit() {
    let mut flg = 0;
    let (ret, src_rem, _pat_rem) = match_pat("d", "5x", &mut flg);
    assert_eq!(ret, MATCHED);
    assert_eq!(src_rem, "x");
}

#[test]
fn test_match_pat_no_match() {
    let mut flg = 0;
    let (ret, src_rem, _pat_rem) = match_pat("d", "x5", &mut flg);
    assert_eq!(ret, MATCHED_FAIL);
    assert_eq!(src_rem, "x5");
}

#[test]
fn test_match_pat_goal() {
    let mut flg = 0;
    let (ret, _, _) = match_pat("&", "abc", &mut flg);
    assert_eq!(ret, MATCHED_GOAL);
}

#[test]
fn test_match_pat_goalnot() {
    let mut flg = 0;
    let (ret, _, _) = match_pat("!&", "abc", &mut flg);
    assert_eq!(ret, MATCHED_GOALNOT);
}

#[test]
fn test_match_pat_C_flag() {
    // C  toggles case-fold flag, should always succeed
    let mut flg = 0;
    let (ret, _, _) = match_pat("C", "anything", &mut flg);
    assert_eq!(ret, MATCHED);
    assert_eq!(flg & 1, 0); // !C toggle
    let (ret, _, _) = match_pat("!C", "anything", &mut flg);
    assert_eq!(ret, MATCHED);
    assert_eq!(flg & 1, 1);
}

// =========== version constants ===========
#[test]
fn test_version_constants() {
    assert_eq!(SKP_VER, 0x0003001C);
    assert_eq!(SKP_VER_STR, "0.3.1rc");
}

// =========== match constants ===========
#[test]
fn test_match_constants() {
    assert_eq!(MATCHED_FAIL, 0);
    assert_eq!(MATCHED, 1);
    assert_eq!(MATCHED_GOAL, 2);
    assert_eq!(MATCHED_GOALNOT, 3);
}

fn main() {}
