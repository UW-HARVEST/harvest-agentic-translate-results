#![allow(dead_code)]

use Remimu::my_regex::*;

// Helper to parse a regex into a Vec<RegexToken>.
fn parse(pattern: &str, flags: i32) -> (Result<(), i32>, Vec<RegexToken>, i16) {
    let mut tokens: Vec<RegexToken> = Vec::with_capacity(1024);
    let mut token_count: i16 = 1024;
    let r = regex_parse(pattern, &mut tokens, &mut token_count, flags);
    (r, tokens, token_count)
}

fn match_simple(pattern: &str, text: &str) -> Option<usize> {
    let (r, tokens, _) = parse(pattern, 0);
    assert!(r.is_ok(), "regex_parse failed for {:?}", pattern);
    let mut cap_pos = vec![0i64; 16];
    let mut cap_span = vec![0i64; 16];
    regex_match(&tokens, text, 0, 0, &mut cap_pos, &mut cap_span)
}

fn match_with_caps(pattern: &str, text: &str, n: u16) -> (Option<usize>, Vec<i64>, Vec<i64>) {
    let (r, tokens, _) = parse(pattern, 0);
    assert!(r.is_ok(), "regex_parse failed for {:?}", pattern);
    let mut cap_pos = vec![-1i64; n as usize];
    let mut cap_span = vec![-1i64; n as usize];
    let m = regex_match(&tokens, text, 0, n, &mut cap_pos, &mut cap_span);
    (m, cap_pos, cap_span)
}

#[test]
fn test_basic_literal_match() {
    assert_eq!(match_simple("abc", "abc"), Some(3));
    assert_eq!(match_simple("abc", "abd"), None);
    assert_eq!(match_simple("abc", ""), None);
    assert_eq!(match_simple("a", "a"), Some(1));
    assert_eq!(match_simple("", "abc"), Some(0));
}

#[test]
fn test_digit_decimal() {
    assert_eq!(match_simple("[0-9]+\\.[0-9]+", "23.53) "), Some(5));
    assert_eq!(match_simple("\\d+\\.\\d*", "3.1415926535"), Some(12));
    assert_eq!(match_simple("\\.\\d+|\\d+\\.\\d*", "3.1415926535"), Some(12));
    assert_eq!(match_simple("\\.\\d+|\\d+\\.\\d*", "5.5"), Some(3));
    assert_eq!(match_simple("[0-9]\\.[0-9]", "5.5"), Some(3));
    assert_eq!(match_simple("\\d\\.\\d", "5.5"), Some(3));
}

#[test]
fn test_quantifiers_basic() {
    assert_eq!(match_simple("a*", ""), Some(0));
    assert_eq!(match_simple("a*", "aaab"), Some(3));
    assert_eq!(match_simple("a+", ""), None);
    assert_eq!(match_simple("a+", "aaab"), Some(3));
    assert_eq!(match_simple("a?", ""), Some(0));
    assert_eq!(match_simple("a?", "ab"), Some(1));
}

#[test]
fn test_quantifiers_explicit() {
    assert_eq!(match_simple("a{3}", "aaaa"), Some(3));
    assert_eq!(match_simple("a{2,5}", "aaaaaa"), Some(5));
    assert_eq!(match_simple("a{3,}", "aaaaa"), Some(5));
    assert_eq!(match_simple("a{0}b", "b"), Some(0));
    assert_eq!(match_simple("\\d{4}", "12345"), Some(4));
    assert_eq!(match_simple("\\d{2,4}", "1"), None);
    assert_eq!(match_simple("a{2,4}?", "aaaaa"), Some(2));
}

#[test]
fn test_invalid_quantifier_range() {
    let (r, _, _) = parse("a{5,3}", 0);
    assert!(r.is_err());
    assert_eq!(r, Err(-1));
}

#[test]
fn test_invalid_patterns() {
    // unbalanced (
    let (r, _, _) = parse("(", 0);
    assert_eq!(r, Err(-1));
    // unbalanced )
    let (r, _, _) = parse(")", 0);
    assert_eq!(r, Err(-1));
    // unsupported escape
    let (r, _, _) = parse("\\z", 0);
    assert_eq!(r, Err(-1));
    // unterminated character class
    let (r, _, _) = parse("[", 0);
    assert_eq!(r, Err(-1));
    // dangling backslash
    let (r, _, _) = parse("abc\\", 0);
    assert_eq!(r, Err(-1));
    // quantifier at start
    let (r, _, _) = parse("*abc", 0);
    assert_eq!(r, Err(-1));
}

#[test]
fn test_character_classes() {
    assert_eq!(match_simple("[abc]", "a"), Some(1));
    assert_eq!(match_simple("[abc]", "d"), None);
    assert_eq!(match_simple("[^abc]", "d"), Some(1));
    assert_eq!(match_simple("[^abc]", "a"), None);
    assert_eq!(match_simple("[a-z0-9]+", "abc123"), Some(6));
    assert_eq!(match_simple("[a-c]", "b"), Some(1));
    assert_eq!(match_simple("[]]", "]"), Some(1));
    assert_eq!(match_simple("[abc-]", "-"), Some(1));
    assert_eq!(match_simple("[-]", "-"), Some(1));
    let (r, _, _) = parse("[c-a]", 0);
    assert_eq!(r, Err(-1));
}

#[test]
fn test_escape_classes() {
    assert_eq!(match_simple("\\d", "5"), Some(1));
    assert_eq!(match_simple("\\d", "a"), None);
    assert_eq!(match_simple("\\D", "5"), None);
    assert_eq!(match_simple("\\D", "A"), Some(1));
    assert_eq!(match_simple("\\s", " "), Some(1));
    assert_eq!(match_simple("\\S", " "), None);
    assert_eq!(match_simple("\\W", "A"), None);
    assert_eq!(match_simple("\\W", "+"), Some(1));
    assert_eq!(match_simple("\\w+", "hello world"), Some(5));
}

#[test]
fn test_escape_whitespace_chars() {
    assert_eq!(match_simple("\\n", "\n"), Some(1));
    assert_eq!(match_simple("\\n", "\nx"), Some(1));
    assert_eq!(match_simple("\\t", "\t"), Some(1));
    // \r, \v, \f
    assert_eq!(match_simple("\\r", "\r"), Some(1));
    assert_eq!(match_simple("\\v", "\x0B"), Some(1));
    assert_eq!(match_simple("\\f", "\x0C"), Some(1));
}

#[test]
fn test_escape_literal() {
    assert_eq!(match_simple("\\(", "("), Some(1));
    assert_eq!(match_simple("\\\\", "\\"), Some(1));
    assert_eq!(match_simple("\\.", "."), Some(1));
    assert_eq!(match_simple("\\.", "a"), None);
    assert_eq!(match_simple("\\-", "-"), Some(1));
    // bare dash literal interpretation outside CC: bare "\-" should escape to '-'
    assert_eq!(match_simple("\\-z", "-z"), Some(2));
}

#[test]
fn test_hex_escape_known_buggy() {
    // The C source has a bug: it reads pattern[i+1] twice instead of i+1 and i+2.
    // So \xNN actually sets the byte (n0<<4 | n0), where n0 = first hex digit value.
    // For \x42, n0=4, the resulting byte is (4<<4)|4 = 0x44 = 'D'
    // We faithfully replicate this in the Rust translation.
    assert_eq!(match_simple("\\x42", "D"), Some(1));
    assert_eq!(match_simple("\\x42", "B"), None);
    assert_eq!(match_simple("\\x44", "D"), Some(1));
    // For \x4B (where B is not a digit, it's a hex letter - should still produce 0x44):
    // first hex digit is 4 -> n0=4 ; both n0 and n1 are 4, byte = 0x44 = 'D'
    // But wait - for 'K' which is not a valid hex char in [0-9A-Fa-f] range, the second arg
    // is actually the same as the first since both come from i+1.
    // Actually B is 0x42, valid hex digit; K is invalid hex.
    // Test the known good behavior for digits/digits:
    assert_eq!(match_simple("\\x33", "3"), Some(1)); // n0=3, byte = 0x33 = '3'
    assert_eq!(match_simple("\\x66", "f"), Some(1)); // n0=6, byte = 0x66 = 'f'
}

#[test]
fn test_dot() {
    assert_eq!(match_simple(".", "X"), Some(1));
    assert_eq!(match_simple(".", "\n"), Some(1));
    // With DOT_NO_NEWLINES flag — note: the C source has a bug where it XORs mask[1]
    // (which represents bytes 0x10-0x1F) instead of mask[0] (0x00-0x0F). Since \n=0x0A
    // and \r=0x0D are in mask[0], the flag doesn't actually exclude newlines.
    // The Rust translation faithfully replicates this. We document the actual C behavior:
    let (r, tokens, _) = parse(".", REMIMU_FLAG_DOT_NO_NEWLINES);
    assert!(r.is_ok());
    let mut cp = vec![-1i64; 4];
    let mut cs = vec![-1i64; 4];
    // Faithful C behavior: \n and \r still match because of the C bug:
    assert_eq!(regex_match(&tokens, "\n", 0, 0, &mut cp, &mut cs), Some(1));
    assert_eq!(regex_match(&tokens, "\r", 0, 0, &mut cp, &mut cs), Some(1));
    assert_eq!(regex_match(&tokens, "X", 0, 0, &mut cp, &mut cs), Some(1));
    // The flag DOES alter mask[1] (bytes 0x10-0x1F). Per C source: 0x04 and 0x20 are flipped.
    // 0x04 in mask[1] corresponds to byte 0x12; 0x20 in mask[1] corresponds to byte 0x15.
    // After XOR, those bytes become non-matching.
    assert_eq!(regex_match(&tokens, "\x12", 0, 0, &mut cp, &mut cs), None);
    assert_eq!(regex_match(&tokens, "\x15", 0, 0, &mut cp, &mut cs), None);
}

#[test]
fn test_anchors() {
    assert_eq!(match_simple("^abc$", "abc"), Some(3));
    assert_eq!(match_simple("^a", "abc"), Some(1));
    assert_eq!(match_simple("abc$", "abc"), Some(3));
    assert_eq!(match_simple(".h$", "bh"), Some(2));
    assert_eq!(match_simple(".h$", "abh"), None); // .h$ requires no chars before .h
    assert_eq!(match_simple("^", ""), Some(0));
    assert_eq!(match_simple("$", ""), Some(0));
}

#[test]
fn test_word_boundary() {
    assert_eq!(match_simple("\\bfoo", "foo"), Some(3));
    assert_eq!(match_simple("\\bfoo\\b", "foo"), Some(3));
    assert_eq!(match_simple("asdf\\b", "asdf"), Some(4));
    assert_eq!(match_simple("asdf\\b", "asdfg"), None);
    assert_eq!(match_simple("a\\Bb", "ab"), Some(2));
    assert_eq!(match_simple("a\\bb", "ab"), None);
    assert_eq!(match_simple("\\B", "abc"), None);
}

#[test]
fn test_alternation() {
    assert_eq!(match_simple("foo|bar", "foo"), Some(3));
    assert_eq!(match_simple("foo|bar", "bar"), Some(3));
    assert_eq!(match_simple("foo|bar", "baz"), None);
    assert_eq!(match_simple("(a|b)c", "ac"), Some(2));
    assert_eq!(match_simple("(a|b)c", "bc"), Some(2));
    assert_eq!(match_simple("(?:a|b)c", "ac"), Some(2));
}

#[test]
fn test_groups_capture() {
    // The C engine has an implicit outermost group at cap index 0 covering the whole match.
    // So `(a)(b)(c)` produces caps: [whole, (a), (b), (c)].
    let (m, cp, cs) = match_with_caps("(a)(b)(c)", "abc", 5);
    assert_eq!(m, Some(3));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 3);
    assert_eq!(cp[1], 0); assert_eq!(cs[1], 1);
    assert_eq!(cp[2], 1); assert_eq!(cs[2], 1);
    assert_eq!(cp[3], 2); assert_eq!(cs[3], 1);
    assert_eq!(cp[4], -1); assert_eq!(cs[4], -1);
}

#[test]
fn test_groups_quantified_capture() {
    // cap[0] is the implicit whole-match capture; cap[1] is the inner (a)+
    let (m, cp, cs) = match_with_caps("(a)+", "aaa", 5);
    assert_eq!(m, Some(3));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 3); // whole match
    assert_eq!(cp[1], 2); assert_eq!(cs[1], 1); // greedy: last iter

    let (m, cp, cs) = match_with_caps("(a)+?", "aaa", 5);
    assert_eq!(m, Some(1));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 1);
    assert_eq!(cp[1], 0); assert_eq!(cs[1], 1);

    let (m, cp, cs) = match_with_caps("(a){3}", "aaa", 5);
    assert_eq!(m, Some(3));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 3);
    assert_eq!(cp[1], 2); assert_eq!(cs[1], 1);
}

#[test]
fn test_possessive_atomic() {
    assert_eq!(match_simple("a*+a", "aaaa"), None); // possessive eats all 'a's, no left over
    assert_eq!(match_simple("a++", "aaaa"), Some(4));
    assert_eq!(match_simple("a++ab", "aaaab"), None);
    assert_eq!(match_simple("(?>a|b)c", "ac"), Some(2));
}

#[test]
fn test_emoji_pathological() {
    // (.*,){11}P needs 11 commas before P with greedy
    assert_eq!(match_simple("(.*,){11}P", "1,2,3,4,5,6,7,8,9,10,11,12"), None);
    assert_eq!(match_simple("(.*?,){11}P", "1,2,3,4,5,6,7,8,9,10,11,12"), None);
}

#[test]
fn test_email_pattern() {
    let p = "(?:\\w+(?:\\.\\w+)*)@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)";
    assert_eq!(match_simple(p, "testacc@example.com"), Some("testacc@example.com".len()));
    assert_eq!(match_simple(p, "test.acc@sub.example.com"), Some("test.acc@sub.example.com".len()));
    assert_eq!(match_simple(p, "@example.com"), None);
}

#[test]
fn test_lazy_vs_greedy() {
    assert_eq!(match_simple("a*?b", "aaab"), Some(4));
    assert_eq!(match_simple("a*b", "aaab"), Some(4));
    assert_eq!(match_simple("a*a*?", "aaaa"), Some(4));
    assert_eq!(match_simple("a*?a*", "aaaa"), Some(4));
}

#[test]
fn test_simple_caps_with_atomic() {
    // The C engine actually does return capture data inside (?>) for simple cases.
    // cap[0] is the whole match; cap[1] is the inner (a) capture.
    let (m, cp, cs) = match_with_caps("(?>(a))b", "ab", 5);
    assert_eq!(m, Some(2));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 2);
    assert_eq!(cp[1], 0); assert_eq!(cs[1], 1);
}

#[test]
fn test_match_with_start_i() {
    // start_i offsets where matching begins. Return value is end index (absolute).
    assert_eq!(match_simple_start("bc", "abc", 1), Some(3));
    assert_eq!(match_simple_start("b", "ab", 1), Some(2));
    assert_eq!(match_simple_start("a", "xyz", 0), None);
}

fn match_simple_start(pattern: &str, text: &str, start_i: usize) -> Option<usize> {
    let (r, tokens, _) = parse(pattern, 0);
    assert!(r.is_ok(), "parse failed for {:?}", pattern);
    let mut cp = vec![-1i64; 4];
    let mut cs = vec![-1i64; 4];
    regex_match(&tokens, text, start_i, 0, &mut cp, &mut cs)
}

#[test]
fn test_complex_patterns_from_test_suite() {
    // (b|a|as|q|)*?X
    assert_eq!(match_simple("(b|a|as|q|)*?X", "asqbX"), Some(5));

    // (\d)*?\.(\d)+ on "5.5" -> match=3
    // cap[0]=whole match (0,3); cap[1]=outer (\d)*? -> matches "5" at 0; cap[2]=(\d)+ -> "5" at 2
    let (m, cp, cs) = match_with_caps("(\\d)*?\\.(\\d)+", "5.5", 5);
    assert_eq!(m, Some(3));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 3);
    assert_eq!(cp[1], 0); assert_eq!(cs[1], 1);
    assert_eq!(cp[2], 2); assert_eq!(cs[2], 1);

    // ([a-z][a-z0-9]*,)+
    let (m, cp, cs) = match_with_caps("([a-z][a-z0-9]*,)+", "a5,b7,c9", 5);
    assert_eq!(m, Some(6));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 6);
    assert_eq!(cp[1], 3); assert_eq!(cs[1], 3);

    // ((a)|(b))++ - possessive: no inner capture data
    let (m, cp, cs) = match_with_caps("((a)|(b))++", "aaaaaabbbabaqa", 5);
    assert_eq!(m, Some(12));
    assert_eq!(cp[0], 0); assert_eq!(cs[0], 12); // whole match still captured
    assert_eq!(cp[1], -1); assert_eq!(cs[1], -1); // possessive: no data
    assert_eq!(cp[2], -1); assert_eq!(cs[2], -1);
    assert_eq!(cp[3], -1); assert_eq!(cs[3], -1);
    assert_eq!(cp[4], -1); assert_eq!(cs[4], -1);
}

#[test]
fn test_token_count_too_small() {
    let mut tokens: Vec<RegexToken> = Vec::with_capacity(2);
    let mut count: i16 = 2;
    let r = regex_parse("abcdefg", &mut tokens, &mut count, 0);
    assert!(r.is_err());
    assert_eq!(r, Err(-2));
}

#[test]
fn test_token_count_zero() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 0;
    let r = regex_parse("a", &mut tokens, &mut count, 0);
    assert_eq!(r, Err(-2));
}

#[test]
fn test_regex_token_helpers() {
    // Constructor and field defaults.
    let t = RegexToken::new(REMIMU_KIND_NORMAL, 0);
    assert_eq!(t.kind, REMIMU_KIND_NORMAL);
    assert_eq!(t.mode, 0);
    assert_eq!(t.count_lo, 1);
    assert_eq!(t.count_hi, 2);
    assert_eq!(t.mask, [0u16; 16]);
    assert_eq!(t.pair_offset, 0);

    // set_mask / check_mask
    let mut t = RegexToken::new(REMIMU_KIND_NORMAL, 0);
    t.set_mask(b'A');
    assert!(t.check_mask(b'A'));
    assert!(!t.check_mask(b'B'));
    t.set_mask(0xFF);
    assert!(t.check_mask(0xFF));
    t.set_mask(0x00);
    assert!(t.check_mask(0x00));

    // invert_mask: flips all bits and clears INVERTED mode flag
    let mut t = RegexToken::new(REMIMU_KIND_NORMAL, REMIMU_MODE_INVERTED);
    t.set_mask(b'A');
    t.invert_mask();
    assert!(!t.check_mask(b'A'));
    assert!(t.check_mask(b'B'));
    assert_eq!(t.mode & REMIMU_MODE_INVERTED, 0);

    // Default
    let d: RegexToken = Default::default();
    assert_eq!(d.kind, REMIMU_KIND_NORMAL);
    assert_eq!(d.count_lo, 1);
    assert_eq!(d.count_hi, 2);
}

#[test]
fn test_constants() {
    assert_eq!(REMIMU_FLAG_DOT_NO_NEWLINES, 1);
    assert_eq!(REMIMU_KIND_NORMAL, 0);
    assert_eq!(REMIMU_KIND_OPEN, 1);
    assert_eq!(REMIMU_KIND_NCOPEN, 2);
    assert_eq!(REMIMU_KIND_CLOSE, 3);
    assert_eq!(REMIMU_KIND_OR, 4);
    assert_eq!(REMIMU_KIND_CARET, 5);
    assert_eq!(REMIMU_KIND_DOLLAR, 6);
    assert_eq!(REMIMU_KIND_BOUND, 7);
    assert_eq!(REMIMU_KIND_NBOUND, 8);
    assert_eq!(REMIMU_KIND_END, 9);
    assert_eq!(REMIMU_MODE_POSSESSIVE, 1);
    assert_eq!(REMIMU_MODE_LAZY, 2);
    assert_eq!(REMIMU_MODE_INVERTED, 128);
}

#[test]
fn test_regex_matcher_state() {
    let s = RegexMatcherState::new(7, 42);
    assert_eq!(s.k, 7);
    assert_eq!(s.i, 42);
    assert_eq!(s.group_state, 0);
    assert_eq!(s.prev, 0);
    assert_eq!(s.range_min, 0);
    assert_eq!(s.range_max, 0);
}

#[test]
fn test_print_regex_tokens_smoke() {
    // Ensure print_regex_tokens does not panic and runs to completion.
    let (r, tokens, _) = parse("[0-9]+\\.[0-9]+", 0);
    assert!(r.is_ok());
    print_regex_tokens(&tokens);
}

#[test]
fn test_token_count_after_parse() {
    // Pattern "[0-9]+\\.[0-9]+" should parse to 6 tokens (per C):
    // OPEN, NORMAL, NORMAL, NORMAL, CLOSE, END
    let (r, tokens, count) = parse("[0-9]+\\.[0-9]+", 0);
    assert!(r.is_ok());
    assert_eq!(count, 6);
    assert_eq!(tokens.len(), 6);
    assert_eq!(tokens[0].kind, REMIMU_KIND_OPEN);
    assert_eq!(tokens[1].kind, REMIMU_KIND_NORMAL);
    assert_eq!(tokens[2].kind, REMIMU_KIND_NORMAL);
    assert_eq!(tokens[3].kind, REMIMU_KIND_NORMAL);
    assert_eq!(tokens[4].kind, REMIMU_KIND_CLOSE);
    assert_eq!(tokens[5].kind, REMIMU_KIND_END);
}

#[test]
fn test_ab_question_b() {
    assert_eq!(match_simple("(ab?)b", "ab"), Some(2));
    assert_eq!(match_simple("(ab?)*b", "ab"), Some(2));
    assert_eq!(match_simple("(ab?)*?b", "ab"), Some(2));
}

#[test]
fn test_zero_quantifiers() {
    // a{0} matches nothing; a{0}b matches "b" (b consumed)
    assert_eq!(match_simple("a{0}", "aaa"), Some(0));
    assert_eq!(match_simple("a{0}b", "b"), Some(0)); // C says match=0 for "b"
    // For a quantified group with {0}, the C engine matches the whole pattern through 'b'.
    assert_eq!(match_simple("(a){0}b", "b"), Some(1));
}

#[test]
fn test_inverted_class() {
    assert_eq!(match_simple("[^a]+", "bcdax"), Some(3));
    assert_eq!(match_simple("[^0-9]+", "abc123"), Some(3));
}

#[test]
fn test_capture_three_a_b_c() {
    // (a)(b)(c) yields whole-match capture at index 0, plus three group captures.
    let (m, cp, cs) = match_with_caps("(a)(b)(c)", "abc", 5);
    assert_eq!(m, Some(3));
    assert_eq!((cp[0], cs[0]), (0, 3)); // whole match
    assert_eq!((cp[1], cs[1]), (0, 1));
    assert_eq!((cp[2], cs[2]), (1, 1));
    assert_eq!((cp[3], cs[3]), (2, 1));
}

#[test]
fn test_no_match_returns_none() {
    assert_eq!(match_simple("xyz", "abc"), None);
}

#[test]
fn test_alternation_with_empty() {
    assert_eq!(match_simple("(a|)+", ""), Some(0));
    assert_eq!(match_simple("()+", ""), Some(0));
    assert_eq!(match_simple("(a|)*b", "ab"), Some(2));
}

#[test]
fn test_long_complex_patterns() {
    let p1 = "(b|a|as|q|)*X";
    assert_eq!(match_simple(p1, "asqbX"), Some(5));
    let p2 = "(b|a|as|q)*X";
    assert_eq!(match_simple(p2, "asqbX"), Some(5));
}

#[test]
fn test_caret_nonzero_position_fails() {
    // ^ only matches at position 0
    assert_eq!(match_simple_start("^abc", "abc", 0), Some(3));
    assert_eq!(match_simple_start("^abc", "abc", 1), None);
}

fn main() {}
