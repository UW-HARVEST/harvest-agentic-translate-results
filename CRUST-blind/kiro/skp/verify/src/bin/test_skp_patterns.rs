use skp::skp;

// Helper to get (alt, len) from skp_ like the C tests do
fn skp_alt_len(src: &str, pat: &str) -> (i32, usize) {
    let (alt, to, _end) = skp::skp_(src, pat);
    let len = to.as_ptr() as usize - src.as_ptr() as usize;
    (alt, len)
}

// Helper for skip-to patterns that need both to and end
fn skp_skipto(src: &str, pat: &str) -> (i32, usize, usize) {
    let (alt, to, end) = skp::skp_(src, pat);
    let to_off = to.as_ptr() as usize - src.as_ptr() as usize;
    let len = end.as_ptr() as usize - to.as_ptr() as usize;
    (alt, to_off, len)
}

// ============================================================
// ut_test1 equivalents
// ============================================================

#[test]
fn test_ut1_decimal_number() {
    let (alt, len) = skp_alt_len("123X", "D\x02");
    assert_eq!(alt, 2); assert_eq!(len, 3);
}

#[test]
fn test_ut1_identifier_fail() {
    let (alt, len) = skp_alt_len("123X", "I\x02");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_ut1_string_match_single() {
    let (alt, len) = skp_alt_len("123X", "'1'\x02");
    assert_eq!(alt, 2); assert_eq!(len, 1);
}

#[test]
fn test_ut1_string_no_match() {
    let (alt, len) = skp_alt_len("123X", "'2'\x02");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_ut1_string_match_two() {
    let (alt, len) = skp_alt_len("123X", "'12'\x02");
    assert_eq!(alt, 2); assert_eq!(len, 2);
}

#[test]
fn test_ut1_string_no_match_two() {
    let (alt, len) = skp_alt_len("123X", "'13'\x02");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_ut1_optional_string_match() {
    let (alt, len) = skp_alt_len("123X", "?'12'\x03");
    assert_eq!(alt, 3); assert_eq!(len, 2);
}

#[test]
fn test_ut1_optional_string_no_match() {
    let (alt, len) = skp_alt_len("123X", "?'23'\x03");
    assert_eq!(alt, 3); assert_eq!(len, 0);
}

#[test]
fn test_ut1_negated_string_match() {
    let (alt, len) = skp_alt_len("123X", "!'12'\x04");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_ut1_negated_string_no_match() {
    let (alt, len) = skp_alt_len("123X", "!'23'\x04");
    assert_eq!(alt, 4); assert_eq!(len, 0);
}

#[test]
fn test_ut1_string_then_dot() {
    let (alt, len) = skp_alt_len("123X", "'1'.\x02");
    assert_eq!(alt, 2); assert_eq!(len, 2);
}

#[test]
fn test_ut1_string_dot_dot_alpha() {
    let (alt, len) = skp_alt_len("123X", "'1'..a\x02");
    assert_eq!(alt, 2); assert_eq!(len, 4);
}

#[test]
fn test_ut1_string_dot_dot_alpha_spaces() {
    let (alt, len) = skp_alt_len("123X", "'1' . . a\x02");
    assert_eq!(alt, 2); assert_eq!(len, 4);
}

// ============================================================
// ut_test2 equivalents
// ============================================================

#[test]
fn test_ut2_goal_pattern() {
    // from="&B", pat="&\3" - & sets goal, \3 is return code
    let (alt, len) = skp_alt_len("&B", "&\x03");
    assert_eq!(alt, 3); assert_eq!(len, 0);
}

#[test]
fn test_ut2_literal_ampersand_fail() {
    // "A&&B\2" - 'A' is not a pattern char, fails
    let (alt, len) = skp_alt_len("A&B", "A&&B\x02");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_ut2_case_sensitive_string() {
    let (alt, len) = skp_alt_len("abCD", "'abCD'");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_ut2_case_sensitive_fail() {
    let (alt, len) = skp_alt_len("abCD", "'abcd'");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_ut2_case_insensitive_string() {
    let (alt, len) = skp_alt_len("abCD", "!C'abcd'");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_ut2_utf8_dot_match() {
    // "aèi" with "'a' . 'i'" - dot matches the 2-byte è
    let (alt, len) = skp_alt_len("a\u{00E8}i", "'a' . 'i'");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_ut2_utf8_set_match() {
    let (alt, len) = skp_alt_len("a\u{00E8}i", ". [\u{00E8}\u{00EC}] .");
    assert_eq!(alt, 1); assert_eq!(len, 4);
}

#[test]
fn test_ut2_utf8_string_match() {
    let (alt, len) = skp_alt_len("a\u{00E8}i", "'a\u{00E8}'\x02 .");
    assert_eq!(alt, 2); assert_eq!(len, 3);
}

#[test]
fn test_ut2_skip_to_utf8() {
    let (alt, to_off, len) = skp_skipto("a\u{00E8}i", "> '\u{00E8}'");
    assert_eq!(alt, 1); assert_eq!(len, 2); assert_eq!(to_off, 1);
}

// ============================================================
// ut_test3 equivalents
// ============================================================

#[test]
fn test_ut3_string_match() {
    let (alt, len) = skp_alt_len("ABC", "'AB'");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

#[test]
fn test_ut3_string_no_match() {
    let (alt, len) = skp_alt_len("ABC", "'XB'");
    assert_eq!(alt, 0); assert_eq!(len, 0);
}

#[test]
fn test_ut3_string_alternative_second() {
    // 'XB\x0EAB' - try XB first (fail), then AB (match)
    let (alt, len) = skp_alt_len("ABC", "'XB\x0EAB'");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

#[test]
fn test_ut3_string_alternative_first() {
    let (alt, len) = skp_alt_len("ABC", "'AB\x0EXB'");
    assert_eq!(alt, 1); assert_eq!(len, 2);
}

fn main() {}
