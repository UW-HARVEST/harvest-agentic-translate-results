// Tests for the skp_ functions (the main pattern-matching API).
use skp::skp::*;

// Helper: compute the byte offset of `rest` within `src`.  Both must come
// from the same allocation; this is what the C `to-from` pointer arithmetic
// does.  Rust slices retain their position via length, since `rest = &src[i..]`,
// so `to_off = src.len() - rest.len()`.
fn off(src: &str, rest: &str) -> usize {
    src.len() - rest.len()
}

// =========== ut_test1.c style tests ===========

#[test]
fn test_skp_d_match() {
    // skp("123X","D\2") => alt=2 len=3
    let from = "123X";
    let (alt, to, end) = skp_(from, "D\u{2}");
    assert_eq!(alt, 2);
    assert_eq!(off(from, to), 3);
    assert_eq!(off(from, end), 3);
}

#[test]
fn test_skp_i_no_match() {
    // skp("123X","I\2") => alt=0 len=0
    let from = "123X";
    let (alt, to, end) = skp_(from, "I\u{2}");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
    assert_eq!(off(from, end), 0);
}

#[test]
fn test_skp_quoted_1() {
    // skp("123X","'1'\2") => alt=2 len=1
    let from = "123X";
    let (alt, to, _) = skp_(from, "'1'\u{2}");
    assert_eq!(alt, 2);
    assert_eq!(off(from, to), 1);
}

#[test]
fn test_skp_quoted_2_no_match() {
    let from = "123X";
    let (alt, to, _) = skp_(from, "'2'\u{2}");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_quoted_12() {
    let from = "123X";
    let (alt, to, _) = skp_(from, "'12'\u{2}");
    assert_eq!(alt, 2);
    assert_eq!(off(from, to), 2);
}

#[test]
fn test_skp_quoted_13_no_match() {
    let from = "123X";
    let (alt, to, _) = skp_(from, "'13'\u{2}");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_optional_match() {
    // ?'12'\3 => alt=3 len=2
    let from = "123X";
    let (alt, to, _) = skp_(from, "?'12'\u{3}");
    assert_eq!(alt, 3);
    assert_eq!(off(from, to), 2);
}

#[test]
fn test_skp_optional_no_match_still_succeeds() {
    // ?'23'\3 => alt=3 len=0  (optional always succeeds)
    let from = "123X";
    let (alt, to, _) = skp_(from, "?'23'\u{3}");
    assert_eq!(alt, 3);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_negated_match_fails() {
    // !'12'\4 => alt=0  (negated, but it matches → fail)
    let from = "123X";
    let (alt, to, _) = skp_(from, "!'12'\u{4}");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_negated_no_match_succeeds() {
    // !'23'\4 => alt=4 len=0
    let from = "123X";
    let (alt, to, _) = skp_(from, "!'23'\u{4}");
    assert_eq!(alt, 4);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_concat_quoted_dot() {
    // "'1'.\2" => alt=2 len=2
    let from = "123X";
    let (alt, to, _) = skp_(from, "'1'.\u{2}");
    assert_eq!(alt, 2);
    assert_eq!(off(from, to), 2);
}

#[test]
fn test_skp_concat_dot_alpha() {
    let from = "123X";
    let (alt, to, _) = skp_(from, "'1'..a\u{2}");
    assert_eq!(alt, 2);
    assert_eq!(off(from, to), 4);
}

#[test]
fn test_skp_concat_with_spaces() {
    // Spaces in the pattern are skipped
    let from = "123X";
    let (alt, to, _) = skp_(from, "'1' . . a\u{2}");
    assert_eq!(alt, 2);
    assert_eq!(off(from, to), 4);
}

// =========== ut_test2.c style tests ===========

#[test]
fn test_skp_double_amp_no_match() {
    // skp("A&B","A&&B\2") => alt=0 len=0  (a literal '&&' in pattern can't be parsed
    // as expected by the user; both & are markers, so the second one isn't a literal)
    let from = "A&B";
    let (alt, to, _) = skp_(from, "A&&B\u{2}");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_string_case_sensitive() {
    let from = "abCD";
    let (alt, to, _) = skp_(from, "'abCD'");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 4);
}

#[test]
fn test_skp_string_lower_no_match() {
    // C-test: "'abcd'" against "abCD" → 0 because case-sensitive (default fold)
    // Actually wait — default is fold ON because C flag is initially 0 meaning fold off?
    // Let me re-read C: in C, `flg & 1` = fold flag. C flag op (uppercase 'C')
    // sets fold flag based on match_not. Default fold is 0 (case-sensitive matching).
    // So 'abcd' vs abCD doesn't match by default. C output: 0
    let from = "abCD";
    let (alt, to, _) = skp_(from, "'abcd'");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_case_insensitive() {
    // !C enables fold; then 'abcd' matches abCD
    let from = "abCD";
    let (alt, to, _) = skp_(from, "!C'abcd'");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 4);
}

#[test]
fn test_skp_set_match() {
    // Use is_oneof to match characters in a set
    let from = "hello";
    let (alt, to, _) = skp_(from, "+[a-z]");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 5);
}

#[test]
fn test_skp_skp_to() {
    // The '>' prefix says "skip to start of pattern"
    // Test: aèi with > 'è' → to=1 (skip 'a'), end=3 (after è)
    let from = "a\u{e8}i";
    let (alt, to, end) = skp_(from, "> 'è'");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 1);
    assert_eq!(off(from, end), 3);
}

// =========== ut_test3.c style tests ===========

#[test]
fn test_skp_alt_pattern_simple() {
    let from = "ABC";
    let (alt, to, _) = skp_(from, "'AB'");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 2);
}

#[test]
fn test_skp_no_alt_match() {
    let from = "ABC";
    let (alt, to, _) = skp_(from, "'XB'");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_alternation_first_fails() {
    // "'XB\xEAB'" (with 0x0e separator) -> tries XB, fails, tries AB, matches
    let from = "ABC";
    let (alt, to, _) = skp_(from, "'XB\u{e}AB'");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 2);
}

#[test]
fn test_skp_alternation_first_succeeds() {
    let from = "ABC";
    let (alt, to, _) = skp_(from, "'AB\u{e}XB'");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 2);
}

// =========== Pattern primitive tests ===========

#[test]
fn test_skp_float() {
    let from = "12.5e3";
    let (alt, to, _) = skp_(from, "F");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 6);
}

#[test]
fn test_skp_int_signed() {
    let from = "-42";
    let (alt, to, _) = skp_(from, "D");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 3);
}

#[test]
fn test_skp_hex_with_prefix() {
    let from = "0xAF";
    let (alt, to, _) = skp_(from, "X");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 4);
}

#[test]
fn test_skp_hex_without_prefix() {
    let from = "FF";
    let (alt, to, _) = skp_(from, "X");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 2);
}

#[test]
fn test_skp_identifier() {
    let from = "foo_bar";
    let (alt, to, _) = skp_(from, "I");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 7);
}

#[test]
fn test_skp_identifier_starts_with_digit() {
    let from = "123foo";
    let (alt, to, _) = skp_(from, "I");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_quoted_string_double() {
    let from = "\"abc\"";
    let (alt, to, _) = skp_(from, "Q");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 5);
}

#[test]
fn test_skp_quoted_string_with_escape() {
    let from = "'a\\'b'";
    let (alt, to, _) = skp_(from, "Q");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 6);
}

#[test]
fn test_skp_balanced_parens() {
    let from = "(a(b)c)";
    let (alt, to, _) = skp_(from, "B");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 7);
}

#[test]
fn test_skp_balanced_brackets() {
    let from = "[1,2,3]";
    let (alt, to, _) = skp_(from, "B");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 7);
}

#[test]
fn test_skp_past_end_of_line() {
    let from = "hello\nworld";
    let (alt, to, _) = skp_(from, "N");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 6);
}

#[test]
fn test_skp_skip_spaces_then_alpha() {
    let from = "   abc";
    let (alt, to, _) = skp_(from, "Sa");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 4);
}

#[test]
fn test_skp_alpha_match() {
    let from = "AbcD";
    let (alt, to, _) = skp_(from, "a");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 1);
}

#[test]
fn test_skp_upper_match() {
    let from = "AbcD";
    let (alt, to, _) = skp_(from, "u");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 1);
}

#[test]
fn test_skp_upper_no_match() {
    let from = "abcD";
    let (alt, to, _) = skp_(from, "u");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_kleene_star() {
    let from = "AbcD";
    let (alt, to, _) = skp_(from, "*a");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 4);
}

#[test]
fn test_skp_one_or_more() {
    let from = "AbcD";
    let (alt, to, _) = skp_(from, "+a");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 4);
}

#[test]
fn test_skp_optional() {
    let from = "AbcD";
    let (alt, to, _) = skp_(from, "?a");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 1);
}

#[test]
fn test_skp_optional_empty() {
    let from = "";
    let (alt, to, _) = skp_(from, "?a");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_dot_any() {
    let from = "abc";
    let (alt, to, _) = skp_(from, ".");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 1);
}

#[test]
fn test_skp_negated_dot_at_eof() {
    let from = "";
    let (alt, to, _) = skp_(from, "!.");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_negated_dot_not_at_eof() {
    let from = "a";
    let (alt, to, _) = skp_(from, "!.");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_goal() {
    // pattern "D &@" sets goal to position after digits, then continues to skip alnum
    let from = "12cm";
    let (alt, to, end) = skp_(from, "D &@");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 2);
    assert_eq!(off(from, end), 2);
}

#[test]
fn test_skp_empty_src_with_digit() {
    let from = "";
    let (alt, to, _) = skp_(from, "D");
    assert_eq!(alt, 0);
    assert_eq!(off(from, to), 0);
}

#[test]
fn test_skp_break_n() {
    let from = "\n";
    let (alt, to, _) = skp_(from, "n");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 1);
}

#[test]
fn test_skp_dollar_at_eof() {
    let from = "";
    let (alt, to, _) = skp_(from, "$");
    assert_eq!(alt, 1);
    assert_eq!(off(from, to), 0);
}

// ===== skp_2 / skp_3 / skp_4 wrappers =====
#[test]
fn test_skp_2_simple() {
    assert_eq!(skp_2("ABC", "'AB'"), 1);
    assert_eq!(skp_2("ABC", "'XY'"), 0);
}

#[test]
fn test_skp_3_with_to() {
    let mut to: &str = "";
    let alt = skp_3("123X", "D\u{2}", Some(&mut to));
    assert_eq!(alt, 2);
    assert_eq!(to, "X");
}

#[test]
fn test_skp_3_no_match() {
    let src = "abc";
    let mut to: &str = src;
    let alt = skp_3(src, "D", Some(&mut to));
    assert_eq!(alt, 0);
    assert_eq!(to, src);
}

#[test]
fn test_skp_4_with_to_and_end() {
    let src = "a\u{e8}i";
    let mut to: &str = "";
    let mut end: &str = "";
    let alt = skp_4(src, "> 'è'", Some(&mut to), Some(&mut end));
    assert_eq!(alt, 1);
    assert_eq!(to, "\u{e8}i");
    assert_eq!(end, "i");
}

fn main() {}
