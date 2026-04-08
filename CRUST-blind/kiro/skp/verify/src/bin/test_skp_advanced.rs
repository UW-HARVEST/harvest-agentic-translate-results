use skp::skp;

fn skp_alt_len(src: &str, pat: &str) -> (i32, usize) {
    let (alt, to, _end) = skp::skp_(src, pat);
    let len = to.as_ptr() as usize - src.as_ptr() as usize;
    (alt, len)
}

fn skp_skipto(src: &str, pat: &str) -> (i32, usize, usize) {
    let (alt, to, end) = skp::skp_(src, pat);
    let to_off = to.as_ptr() as usize - src.as_ptr() as usize;
    let len = end.as_ptr() as usize - to.as_ptr() as usize;
    (alt, to_off, len)
}

// ============================================================
// Additional pattern tests
// ============================================================

#[test]
fn test_star_digit() {
    let (alt, len) = skp_alt_len("42", "*d");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

#[test]
fn test_star_alpha() {
    let (alt, len) = skp_alt_len("abc", "*a");
    assert_eq!(alt, 1); assert_eq!(len, 3);
}

#[test]
fn test_star_alpha_star_digit() {
    let (alt, len) = skp_alt_len("abc123", "*a*d");
    assert_eq!(alt, 1); assert_eq!(len, 6);
}

#[test]
fn test_identifier_match() {
    let (alt, len) = skp_alt_len("_foo123", "I");
    assert_eq!(alt, 1); assert_eq!(len, 7);
}

#[test]
fn test_identifier_fail() {
    let (alt, len) = skp_alt_len("123foo", "I");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_balanced_parens() {
    let (alt, len) = skp_alt_len("(abc)", "B");
    assert_eq!(alt, 1); assert_eq!(len, 5);
}

#[test]
fn test_balanced_brackets() {
    let (alt, len) = skp_alt_len("[a[b]c]", "B");
    assert_eq!(alt, 1); assert_eq!(len, 7);
}

#[test]
fn test_balanced_braces() {
    let (alt, len) = skp_alt_len("{x}", "B");
    assert_eq!(alt, 1); assert_eq!(len, 3);
}

#[test]
fn test_balanced_unmatched() {
    let (alt, len) = skp_alt_len("(abc", "B");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_balanced_non_bracket() {
    let (alt, len) = skp_alt_len("abc", "B");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_quoted_single() {
    let (alt, len) = skp_alt_len("'hello'", "Q");
    assert_eq!(alt, 1); assert_eq!(len, 7);
}

#[test]
fn test_quoted_double() {
    let (alt, len) = skp_alt_len("\"world\"", "Q");
    assert_eq!(alt, 1); assert_eq!(len, 7);
}

#[test]
fn test_quoted_escape() {
    let (alt, len) = skp_alt_len("'he\\'llo'", "Q");
    assert_eq!(alt, 1); assert_eq!(len, 9);
}

#[test]
fn test_quoted_backtick() {
    let (alt, len) = skp_alt_len("`hello`", "Q");
    assert_eq!(alt, 1); assert_eq!(len, 7);
}

#[test]
fn test_quoted_unmatched() {
    let (alt, len) = skp_alt_len("'hello", "Q");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_hex_with_prefix() {
    let (alt, len) = skp_alt_len("0xFF", "X");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_hex_no_prefix() {
    let (alt, len) = skp_alt_len("FF", "X");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

#[test]
fn test_hex_no_prefix_long() {
    let (alt, len) = skp_alt_len("ABCD", "X");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_hex_invalid_after_0x() {
    // "0xGG" - 0 is a hex digit, matches. Then 'x' is not hex, stops.
    let (alt, len) = skp_alt_len("0xGG", "X");
    assert_eq!(alt, 1); assert_eq!(len, 1);
}

#[test]
fn test_decimal_negative() {
    let (alt, len) = skp_alt_len("-42", "D");
    assert_eq!(alt, 1); assert_eq!(len, 3);
}

#[test]
fn test_decimal_positive() {
    let (alt, len) = skp_alt_len("+7", "D");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

#[test]
fn test_float_simple() {
    let (alt, len) = skp_alt_len("3.14", "F");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_float_exponent() {
    let (alt, len) = skp_alt_len("1.5e10", "F");
    assert_eq!(alt, 1); assert_eq!(len, 6);
}

#[test]
fn test_float_negative_exponent() {
    let (alt, len) = skp_alt_len("-2.5E-3", "F");
    assert_eq!(alt, 1); assert_eq!(len, 7);
}

#[test]
fn test_float_dot_five() {
    let (alt, len) = skp_alt_len(".5", "F");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

#[test]
fn test_float_1e5() {
    let (alt, len) = skp_alt_len("1e5", "F");
    assert_eq!(alt, 1); assert_eq!(len, 3);
}

#[test]
fn test_set_match() {
    let (alt, len) = skp_alt_len("abc", "[abc]");
    assert_eq!(alt, 1); assert_eq!(len, 1);
}

#[test]
fn test_set_no_match() {
    let (alt, len) = skp_alt_len("xyz", "[abc]");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_set_range_star() {
    let (alt, len) = skp_alt_len("bcd", "*[a-d]");
    assert_eq!(alt, 1); assert_eq!(len, 3);
}

#[test]
fn test_negation_match() {
    let (alt, len) = skp_alt_len("xyz", "!d");
    assert_eq!(alt, 1); assert_eq!(len, 1);
}

#[test]
fn test_negation_fail() {
    let (alt, len) = skp_alt_len("123", "!d");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_skip_to() {
    let (alt, to_off, len) = skp_skipto("hello world", "> 'world'");
    assert_eq!(alt, 1); assert_eq!(to_off, 6); assert_eq!(len, 5);
}

#[test]
fn test_goal() {
    let (alt, len) = skp_alt_len("abc123", "*a & *d");
    assert_eq!(alt, 1); assert_eq!(len, 3);
}

#[test]
fn test_newline_pattern() {
    let (alt, len) = skp_alt_len("abc\ndef", "N");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_end_of_text() {
    let (alt, len) = skp_alt_len("", "!.");
    assert_eq!(alt, 1); assert_eq!(len, 0);
}

#[test]
fn test_end_of_text_fail() {
    let (alt, len) = skp_alt_len("a", "!.");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_explicit_balanced_parens() {
    let (alt, len) = skp_alt_len("(abc)", "()");
    assert_eq!(alt, 1); assert_eq!(len, 5);
}

#[test]
fn test_skip_spaces_s() {
    let (alt, len) = skp_alt_len("  abc", "S*a");
    assert_eq!(alt, 1); assert_eq!(len, 5);
}

#[test]
fn test_skip_blanks_w() {
    let (alt, len) = skp_alt_len("  abc", "W*a");
    assert_eq!(alt, 1); assert_eq!(len, 5);
}

#[test]
fn test_dollar_empty() {
    let (alt, len) = skp_alt_len("", "$");
    assert_eq!(alt, 1); assert_eq!(len, 0);
}

#[test]
fn test_dollar_newline() {
    let (alt, len) = skp_alt_len("\n", "$");
    assert_eq!(alt, 1); assert_eq!(len, 1);
}

#[test]
fn test_case_insensitive_flag() {
    let (alt, len) = skp_alt_len("abCD", "!C'ABCD'");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_negative_goal() {
    // !& sets negative goal, then *a matches. Since main pattern succeeds,
    // negative goal is not used. But the C output says alt=1 len=3.
    // Actually: !& returns MATCHED_GOALNOT, goalnot is set.
    // Then *a matches 3 chars. Pattern ends. matched != 0.
    // goal is set from goalnot? No - goal is only set from goalnot when matched==0.
    // Wait: the C code says: if (!matched && goalnot) { goal = goalnot; matched = MATCHED; }
    // Here matched is MATCHED (from *a), so this doesn't trigger.
    // But goalnot was set. Then: if (goal) s = goal; - goal is None.
    // So s stays at end of *a match = "abc" + 3 = "".
    // to = skp_to?start:s = s = end. len = to - from = 3.
    // Wait but C says len=3 for "!& *a" on "abc". Let me re-check.
    // Actually the C output for add_35 was: alt=1 len=3
    // Hmm, but !& sets goalnot. Then *a matches "abc" (3 chars).
    // matched = MATCHED (from *a). goalnot is set.
    // The check: if (!matched && goalnot) - matched is truthy, so skip.
    // if (goal) s = goal; - goal is None, skip.
    // matched && (*p <= 7) -> ret = 1, to = s = "abc"+3, len = 3.
    let (alt, len) = skp_alt_len("abc", "!& *a");
    assert_eq!(alt, 1); assert_eq!(len, 3);
}

#[test]
fn test_alternative_patterns() {
    // "*d\x01*a\x02" - *d matches 0 digits (min=0), then \x01 is return code
    let (alt, len) = skp_alt_len("abc", "*d\x01*a\x02");
    assert_eq!(alt, 1); assert_eq!(len, 0);
}

#[test]
fn test_empty_star_d() {
    let (alt, len) = skp_alt_len("", "*d");
    assert_eq!(alt, 1); assert_eq!(len, 0);
}

#[test]
fn test_optional_d_nondigit() {
    let (alt, len) = skp_alt_len("abc", "?d");
    assert_eq!(alt, 1); assert_eq!(len, 0);
}

#[test]
fn test_plus_d_nondigit() {
    let (alt, len) = skp_alt_len("abc", "+d");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_single_alpha() {
    let (alt, len) = skp_alt_len("hello", "a");
    assert_eq!(alt, 1); assert_eq!(len, 1);
}

#[test]
fn test_plus_alpha() {
    let (alt, len) = skp_alt_len("hello", "+a");
    assert_eq!(alt, 1); assert_eq!(len, 5);
}

#[test]
fn test_string_alternative() {
    let (alt, len) = skp_alt_len("XBC", "'AB\x0EXB'");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

#[test]
fn test_u_flag() {
    let (alt, len) = skp_alt_len("A", "U a");
    assert_eq!(alt, 1); assert_eq!(len, 1);
}

// ============================================================
// skp_2, skp_3, skp_4 wrappers
// ============================================================

#[test]
fn test_skp_2() {
    assert_eq!(skp::skp_2("123", "*d"), 1);
    assert_eq!(skp::skp_2("abc", "*d"), 1); // *d matches 0 digits
    assert_eq!(skp::skp_2("abc", "+d"), 0);
}

#[test]
fn test_skp_3() {
    let mut end = "";
    let alt = skp::skp_3("123X", "D", Some(&mut end));
    assert_eq!(alt, 1);
    assert_eq!(end, "X");
}

#[test]
fn test_skp_4() {
    let mut to = "";
    let mut end = "";
    let alt = skp::skp_4("hello world", "> 'world'", Some(&mut to), Some(&mut end));
    assert_eq!(alt, 1);
    assert_eq!(to, "world");
    assert_eq!(end, "");
}

fn main() {}
