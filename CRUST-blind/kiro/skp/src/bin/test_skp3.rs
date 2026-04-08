use skp::skp::*;

fn consumed(src: &str, end: &str) -> usize {
    src.len() - end.len()
}

// ============ String matching ============

#[test]
fn test_skp_string_match() {
    let src = "ABC";
    let (ret, _to, end) = skp_(src, "'AB'");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

#[test]
fn test_skp_string_no_match() {
    let src = "ABC";
    let (ret, _, _) = skp_(src, "'XB'");
    assert_eq!(ret, 0);
}

#[test]
fn test_skp_string_alternative() {
    // \x0E is the alternative separator
    let src = "ABC";
    let pat = "'XB\x0EAB'";
    let (ret, _to, end) = skp_(src, pat);
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

#[test]
fn test_skp_string_alternative_first() {
    let src = "ABC";
    let pat = "'AB\x0EXB'";
    let (ret, _to, end) = skp_(src, pat);
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 2);
}

// ============ Case insensitive (C) ============

#[test]
fn test_skp_case_insensitive() {
    let src = "abCD";
    let (ret, _to, end) = skp_(src, "!C'abcd'");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

#[test]
fn test_skp_case_sensitive_default() {
    let src = "abCD";
    let (ret, _, _) = skp_(src, "'abcd'");
    assert_eq!(ret, 0);
}

#[test]
fn test_skp_case_sensitive_exact() {
    let src = "abCD";
    let (ret, _to, end) = skp_(src, "'abCD'");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

// ============ Skip spaces (S) and blanks (W) ============

#[test]
fn test_skp_w_skip_blanks() {
    let src = "  abc";
    let (ret, _to, end) = skp_(src, "W*a");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 5);
}

#[test]
fn test_skp_s_skip_spaces() {
    let src = " \n abc";
    let (ret, _to, end) = skp_(src, "S*a");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 6);
}

// ============ Goal (&) ============

#[test]
fn test_skp_goal() {
    let src = "abc123";
    let (ret, _to, end) = skp_(src, "*a &*d");
    assert_eq!(ret, 1);
    // With goal, end points to the goal position (start of digits)
    assert_eq!(consumed(src, end), 3);
}

// ============ Negative goal (!&) ============

#[test]
fn test_skp_negative_goal() {
    // When pattern succeeds normally, !& sets goalnot but doesn't affect result
    // C: skp_("abc123", "*a !& *d") -> alt=1, to and end both point to end of string
    let src = "abc123";
    let (ret, _to, end) = skp_(src, "*a !&*d");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 6);
}

// ============ Skip to (>) ============

#[test]
fn test_skp_skip_to() {
    let src = "xxxABC";
    let (ret, to, end) = skp_(src, ">+u");
    assert_eq!(ret, 1);
    // to = start of match, end = end of match
    let to_off = src.len() - to.len();
    let end_off = src.len() - end.len();
    assert_eq!(to_off, 3);
    assert_eq!(end_off, 6);
}

#[test]
fn test_skp_skip_to_string() {
    let src = "hello world";
    let (ret, to, end) = skp_(src, ">'world'");
    assert_eq!(ret, 1);
    let to_off = src.len() - to.len();
    let end_off = src.len() - end.len();
    assert_eq!(to_off, 6);
    assert_eq!(end_off, 11);
}

// ============ Return codes ============

#[test]
fn test_skp_return_code_separator() {
    let src = "123X";
    let (ret, _to, end) = skp_(src, "D\x02");
    assert_eq!(ret, 2);
    assert_eq!(consumed(src, end), 3);
}

#[test]
fn test_skp_return_code_default() {
    let src = "abc";
    let (ret, _, _) = skp_(src, "*a");
    assert_eq!(ret, 1);
}

#[test]
fn test_skp_return_code_alternative() {
    let src = "abc";
    let (ret, _to, end) = skp_(src, "d\x02a\x03");
    assert_eq!(ret, 3);
    assert_eq!(consumed(src, end), 1);
}

// ============ Pattern alternatives (separated by bytes <= 7) ============

#[test]
fn test_skp_pattern_alternative() {
    let src = "123";
    let (ret, _to, end) = skp_(src, "a\x07d");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 1);
}

// ============ Combined patterns ============

#[test]
fn test_skp_string_then_any() {
    let src = "123X";
    let (ret, _to, end) = skp_(src, "'1'.\x02");
    assert_eq!(ret, 2);
    assert_eq!(consumed(src, end), 2);
}

#[test]
fn test_skp_string_dot_dot_alpha() {
    let src = "123X";
    let (ret, _to, end) = skp_(src, "'1'..a\x02");
    assert_eq!(ret, 2);
    assert_eq!(consumed(src, end), 4);
}

#[test]
fn test_skp_string_spaces_in_pattern() {
    let src = "123X";
    let (ret, _to, end) = skp_(src, "'1' . . a\x02");
    assert_eq!(ret, 2);
    assert_eq!(consumed(src, end), 4);
}

// ============ Optional (?) ============

#[test]
fn test_skp_optional_match() {
    let src = "123X";
    let (ret, _to, end) = skp_(src, "?'12'\x03");
    assert_eq!(ret, 3);
    assert_eq!(consumed(src, end), 2);
}

#[test]
fn test_skp_optional_no_match() {
    let src = "123X";
    let (ret, _to, end) = skp_(src, "?'23'\x03");
    assert_eq!(ret, 3);
    assert_eq!(consumed(src, end), 0);
}

// ============ Negated string ============

#[test]
fn test_skp_negated_string_match() {
    let src = "123X";
    let (ret, _, _) = skp_(src, "!'12'\x04");
    assert_eq!(ret, 0);
}

#[test]
fn test_skp_negated_string_no_match() {
    let src = "123X";
    let (ret, _to, end) = skp_(src, "!'23'\x04");
    assert_eq!(ret, 4);
    assert_eq!(consumed(src, end), 0);
}

// ============ Empty/null inputs ============

#[test]
fn test_skp_empty_src() {
    let (ret, _, _) = skp_("", "d");
    assert_eq!(ret, 0);
}

#[test]
fn test_skp_empty_pat() {
    let (ret, _, _) = skp_("abc", "");
    assert_eq!(ret, 0);
}

// ============ skp_2, skp_3, skp_4 wrappers ============

#[test]
fn test_skp_2() {
    assert_eq!(skp_2("abc", "*a"), 1);
    assert_eq!(skp_2("abc", "d"), 0);
}

#[test]
fn test_skp_3() {
    let mut end = "";
    let ret = skp_3("abc", "*a", Some(&mut end));
    assert_eq!(ret, 1);
    assert_eq!(end, "");
}

#[test]
fn test_skp_4() {
    let mut to = "";
    let mut end = "";
    let ret = skp_4("abc", "*a", Some(&mut to), Some(&mut end));
    assert_eq!(ret, 1);
    assert_eq!(to, "");
    assert_eq!(end, "");
}

// ============ Identifier (I) fail on digit start ============

#[test]
fn test_skp_identifier_digit_start() {
    let (ret, _, _) = skp_("123foo", "I");
    assert_eq!(ret, 0);
}

// ============ UTF-8 tests ============

#[test]
fn test_skp_utf8_any() {
    // "aèi" - è is 2 bytes in UTF-8
    let src = "a\u{00E8}i"; // aèi
    let (ret, _to, end) = skp_(src, "'a' . 'i'");
    assert_eq!(ret, 1);
    assert_eq!(consumed(src, end), 4);
}

fn main() {}
