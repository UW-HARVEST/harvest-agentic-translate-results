use Remimu::my_regex::*;

// Helper: parse a pattern, return tokens and token_count
fn parse(pattern: &str, flags: i32) -> Result<(Vec<RegexToken>, i16), i32> {
    let mut tokens = Vec::new();
    let mut tc: i16 = 256;
    regex_parse(pattern, &mut tokens, &mut tc, flags).map_err(|e| e)?;
    Ok((tokens, tc))
}

// Helper: parse + match, return match length (or None)
fn pm(pattern: &str, text: &str) -> Option<usize> {
    let (tokens, _) = parse(pattern, 0).ok()?;
    regex_match(&tokens, text, 0, 0, &mut [], &mut [])
}

// Helper: parse + match with captures
fn pm_cap(pattern: &str, text: &str, cap_slots: u16) -> (Option<usize>, Vec<i64>, Vec<i64>) {
    let (tokens, _) = match parse(pattern, 0) {
        Ok(t) => t,
        Err(_) => return (None, vec![], vec![]),
    };
    let mut cap_pos = vec![-1i64; cap_slots as usize];
    let mut cap_span = vec![-1i64; cap_slots as usize];
    // Initialize to 0xFF pattern like C's memset(0xFF)
    for p in cap_pos.iter_mut() { *p = -1; }
    for s in cap_span.iter_mut() { *s = -1; }
    let r = regex_match(&tokens, text, 0, cap_slots, &mut cap_pos, &mut cap_span);
    (r, cap_pos, cap_span)
}

// ===== regex_parse tests =====

#[test]
fn test_parse_success() {
    assert!(parse("[0-9]+\\.[0-9]+", 0).is_ok());
    assert!(parse("", 0).is_ok());
    assert!(parse("abc", 0).is_ok());
    assert!(parse("(a|b)+", 0).is_ok());
    assert!(parse("(?:abc)+", 0).is_ok());
    assert!(parse("(?>(a|b)*)", 0).is_ok());
}

#[test]
fn test_parse_errors() {
    // Unbalanced open paren
    assert!(parse("(abc", 0).is_err());
    // Unbalanced close paren
    assert!(parse("abc)", 0).is_err());
    // Quantifier in non-quantifier context
    assert!(parse("*abc", 0).is_err());
    assert!(parse("+abc", 0).is_err());
    assert!(parse("?abc", 0).is_err());
    // Bad escape
    assert!(parse("\\z", 0).is_err());
    // Open character class
    assert!(parse("[abc", 0).is_err());
    // Trailing backslash
    assert!(parse("abc\\", 0).is_err());
    // Shorthand in char class (C code returns -1 for [\d])
    assert!(parse("[\\d]+", 0).is_err());
    assert!(parse("[\\w]+", 0).is_err());
}

#[test]
fn test_parse_buffer_too_small() {
    let mut tokens = Vec::new();
    let mut tc: i16 = 2;
    let r = regex_parse("[0-9]+\\.[0-9]+", &mut tokens, &mut tc, 0);
    assert!(r.is_err());
}

#[test]
fn test_parse_token_count() {
    let (_, tc) = parse("[0-9]+\\.[0-9]+", 0).unwrap();
    assert_eq!(tc, 6);

    let (_, tc) = parse("", 0).unwrap();
    assert_eq!(tc, 3);

    let (_, tc) = parse("^asdf$", 0).unwrap();
    assert_eq!(tc, 9);
}

// ===== Basic literal matching =====

#[test]
fn test_basic_literal() {
    assert_eq!(pm("[0-9]+\\.[0-9]+", "23.53) "), Some(5));
    assert_eq!(pm("[0-9]+\\.[0-9]+", "abc"), None);
    assert_eq!(pm("xyz", "abc"), None);
}

#[test]
fn test_empty_pattern() {
    assert_eq!(pm("", "anything"), Some(0));
    assert_eq!(pm("", ""), Some(0));
}

#[test]
fn test_empty_text() {
    assert_eq!(pm("a*", ""), Some(0));
}

// ===== Anchors =====

#[test]
fn test_anchors() {
    assert_eq!(pm("^asdf$", "asdf"), Some(4));
    assert_eq!(pm("^asdf$", "asdfg"), None);
    assert_eq!(pm("^asdf", "asdf"), Some(4));
    assert_eq!(pm("asdf$", "asdf"), Some(4));
    assert_eq!(pm(".*asdf$", "XXXasdf"), Some(7));
    assert_eq!(pm(".*asdf$", "asdfXXX"), None);
}

// ===== Quantifiers =====

#[test]
fn test_greedy_quantifiers() {
    assert_eq!(pm("a*", "aaa"), Some(3));
    assert_eq!(pm("a+", "aaa"), Some(3));
    assert_eq!(pm("a?", "aaa"), Some(1));
    assert_eq!(pm("a{3}", "aaaa"), Some(3));
    assert_eq!(pm("a{3,5}", "aaaa"), Some(4));
    assert_eq!(pm("a{3,5}", "aaaaaa"), Some(5));
    assert_eq!(pm("a{3,}", "aaaaaa"), Some(6));
}

#[test]
fn test_lazy_quantifiers() {
    assert_eq!(pm("a*?", "aaa"), Some(0));
    assert_eq!(pm("a+?", "aaa"), Some(1));
    assert_eq!(pm("a??", "aaa"), Some(0));
}

#[test]
fn test_possessive_quantifiers() {
    // Possessive a++ consumes all a's and can't backtrack, so a++ab fails
    assert_eq!(pm("a++ab", "aaab"), None);
    assert_eq!(pm("a++", "aaa"), Some(3));
}

// ===== Dot =====

#[test]
fn test_dot() {
    assert_eq!(pm(".", "a"), Some(1));
    assert_eq!(pm(".", "\n"), Some(1));
    assert_eq!(pm(".", "\r"), Some(1));
}

#[test]
fn test_dot_no_newlines_flag() {
    // The C code has a bug where DOT_NO_NEWLINES XORs wrong mask bits,
    // so \n and \r still match. We replicate that behavior.
    let (tokens, _) = parse(".", REMIMU_FLAG_DOT_NO_NEWLINES).unwrap();
    assert_eq!(regex_match(&tokens, "\n", 0, 0, &mut [], &mut []), Some(1));
    assert_eq!(regex_match(&tokens, "\r", 0, 0, &mut [], &mut []), Some(1));
    assert_eq!(regex_match(&tokens, "a", 0, 0, &mut [], &mut []), Some(1));
}

// ===== Character classes =====

#[test]
fn test_char_class() {
    assert_eq!(pm("[abc]", "b"), Some(1));
    assert_eq!(pm("[a-z]", "m"), Some(1));
    assert_eq!(pm("[^abc]", "d"), Some(1));
    assert_eq!(pm("[^abc]", "a"), None);
}

#[test]
fn test_char_class_dash_at_end() {
    assert_eq!(pm("[a-]", "a"), Some(1));
    assert_eq!(pm("[a-]", "-"), Some(1));
    assert_eq!(pm("[a-]", "b"), None);
}

// ===== Shorthand character classes =====

#[test]
fn test_shorthand_classes() {
    assert_eq!(pm("\\d+", "12345abc"), Some(5));
    assert_eq!(pm("\\w+", "hello world"), Some(5));
    assert_eq!(pm("\\s+", "   abc"), Some(3));
    assert_eq!(pm("\\D+", "abc123"), Some(3));
}

// ===== Escape sequences =====

#[test]
fn test_escape_sequences() {
    assert_eq!(pm("\\t", "\t"), Some(1));
    assert_eq!(pm("\\n", "\n"), Some(1));
    assert_eq!(pm("\\r", "\r"), Some(1));
    assert_eq!(pm("\\.", "."), Some(1));
    assert_eq!(pm("\\.", "a"), None);
    assert_eq!(pm("\\*", "*"), Some(1));
}

// ===== Word boundaries =====

#[test]
fn test_word_boundary() {
    assert_eq!(pm("asdf\\b", "asdf "), Some(4));
    assert_eq!(pm("asdf\\b", "asdfg"), None);
    assert_eq!(pm("\\basdf", "asdf"), Some(4));
    assert_eq!(pm("\\basdf", "Xasdf"), None);
}

// ===== Groups =====

#[test]
fn test_non_capturing_group() {
    assert_eq!(pm("(?:abc)+", "abcabc"), Some(6));
    assert_eq!(pm("(?:ab)+c", "ababc"), Some(5));
}

#[test]
fn test_empty_group() {
    let (r, pos, span) = pm_cap("()", "aa", 1);
    assert_eq!(r, Some(0));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 0);

    let (r, pos, span) = pm_cap("a()a", "aa", 1);
    assert_eq!(r, Some(2));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 2);
}

#[test]
fn test_atomic_group() {
    assert_eq!(pm("(?>(a|b)*)c", "aabc"), Some(4));
    assert_eq!(pm("(?>(a|b)*)c", "aac"), Some(3));
}

// ===== Alternation =====

#[test]
fn test_alternation() {
    assert_eq!(pm("a|b|c", "a"), Some(1));
    assert_eq!(pm("a|b|c", "b"), Some(1));
    assert_eq!(pm("a|b|c", "c"), Some(1));
    assert_eq!(pm("a|b|c", "d"), None);
}

#[test]
fn test_alternation_in_group() {
    let (r, pos, _) = pm_cap("(a|a|ab)bc", "abbc) ", 1);
    assert_eq!(r, Some(4));
    assert_eq!(pos[0], 0);

    let (r, pos, _) = pm_cap("(ab|ab|a)bc", "abbc) ", 1);
    assert_eq!(r, Some(4));
    assert_eq!(pos[0], 0);
}

// ===== Captures =====

#[test]
fn test_captures_possessive() {
    // Possessive groups return no capture info
    let (r, pos, span) = pm_cap("((a)|(b))++", "aaaaaabbbabaqa", 5);
    assert_eq!(r, Some(12));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 12);
    // Inner captures are -1 for possessive
    assert_eq!(pos[1], -1);
    assert_eq!(span[1], -1);
}

#[test]
fn test_captures_greedy_group() {
    let (r, pos, span) = pm_cap("((a)|(b))+", "aaaaaabbbabaqa", 5);
    assert_eq!(r, Some(12));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 12);
    assert_eq!(pos[1], 11);
    assert_eq!(span[1], 1);
    assert_eq!(pos[2], 11);
    assert_eq!(span[2], 1);
    assert_eq!(pos[3], 10);
    assert_eq!(span[3], 1);
    assert_eq!(pos[4], -1);
    assert_eq!(span[4], -1);
}

#[test]
fn test_captures_star_group() {
    let (r, pos, span) = pm_cap("((a)|(b))*", "aaaaaabbbabaqa", 5);
    assert_eq!(r, Some(12));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 12);
    assert_eq!(pos[1], 11);
    assert_eq!(span[1], 1);
    assert_eq!(pos[2], 11);
    assert_eq!(span[2], 1);
    assert_eq!(pos[3], 10);
    assert_eq!(span[3], 1);
}

#[test]
fn test_captures_nested() {
    let (r, pos, span) = pm_cap("((a)|((b)q))*", "aabqaaaaba", 5);
    assert_eq!(r, Some(8));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 8);
    assert_eq!(pos[1], 7);
    assert_eq!(span[1], 1);
    assert_eq!(pos[2], 7);
    assert_eq!(span[2], 1);
    assert_eq!(pos[3], 2);
    assert_eq!(span[3], 2);
    assert_eq!(pos[4], 2);
    assert_eq!(span[4], 1);
}

// ===== Complex patterns =====

#[test]
fn test_complex_alternation_group() {
    let (r, _, _) = pm_cap("(b|a|as|q)*X", "asqbX", 1);
    assert_eq!(r, Some(5));

    let (r, _, _) = pm_cap("(b|a|as|q)*?X", "asqbX", 1);
    assert_eq!(r, Some(5));

    let (r, _, _) = pm_cap("(b|a|as|q)+X", "asqbX", 1);
    assert_eq!(r, Some(5));

    let (r, _, _) = pm_cap("(b|a|as|q|)*?X", "asqbX", 1);
    assert_eq!(r, Some(5));

    let (r, _, _) = pm_cap("(b|a|as|q|)+?X", "asqbX", 1);
    assert_eq!(r, Some(5));
}

#[test]
fn test_optional_group_with_fixed_count() {
    let (r, _, _) = pm_cap("(a?)*a{10}", "aaaaaaaaaa", 1);
    assert_eq!(r, Some(10));

    let (r, _, _) = pm_cap("(a?)*?a{10}", "aaaaaaaaaa", 1);
    assert_eq!(r, Some(10));

    let (r, _, _) = pm_cap("(z?)*a{10}", "aaaaaaaaaa", 1);
    assert_eq!(r, Some(10));
}

#[test]
fn test_ab_group_patterns() {
    let (r, _, _) = pm_cap("(a|ab)*b", "aab", 1);
    assert_eq!(r, Some(3));

    let (r, _, _) = pm_cap("(a|ab)*b", "aaaaaababab", 1);
    assert_eq!(r, Some(7));

    let (r, pos, span) = pm_cap("(ab?)*b", "abba) ", 1);
    assert_eq!(r, Some(3));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 3);

    let (r, pos, span) = pm_cap("(ab?)*?b", "abba) ", 1);
    assert_eq!(r, Some(3));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 3);

    let (r, pos, span) = pm_cap("(ab?)b", "abc) ", 1);
    assert_eq!(r, Some(2));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 2);
}

#[test]
fn test_or_empty_group() {
    let (r, _, _) = pm_cap("(a|)*b", "b", 1);
    assert_eq!(r, Some(1));

    let (r, _, _) = pm_cap("(a|)*b", "aab", 1);
    assert_eq!(r, Some(3));
}

#[test]
fn test_possessive_group() {
    let (r, pos, span) = pm_cap("(b|a|)*+", "aaaaaababab", 1);
    assert_eq!(r, Some(11));
    assert_eq!(pos[0], 0);
    assert_eq!(span[0], 11);
}

// ===== start_i parameter =====

#[test]
fn test_start_i() {
    let (tokens, _) = parse("[0-9]+", 0).unwrap();
    let r = regex_match(&tokens, "abc123def", 3, 0, &mut [], &mut []);
    assert_eq!(r, Some(6));

    let (tokens, _) = parse("\\w+", 0).unwrap();
    let r = regex_match(&tokens, "   hello", 3, 0, &mut [], &mut []);
    assert_eq!(r, Some(8));
}

// ===== Quantifier range patterns =====

#[test]
fn test_quantifier_ranges() {
    assert_eq!(pm("[0-9]{3,5}\\.[0-9]+", "12345.67"), Some(8));
    assert_eq!(pm("[0-9]{3,5}?\\.[0-9]+", "12345.67"), Some(8));
}

// ===== Email-like pattern =====

#[test]
fn test_email_pattern() {
    assert_eq!(
        pm("(?:\\w+(?:\\.\\w+)*)@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)", "testacc@example.com"),
        Some(19)
    );
}

// ===== Shorthand patterns =====

#[test]
fn test_digit_dot_digit() {
    assert_eq!(pm("\\d\\.\\d", "5.5"), Some(3));
    assert_eq!(pm("\\d*\\.\\d*", ".53) "), Some(3));
}

// ===== print_regex_tokens smoke test =====

#[test]
fn test_print_regex_tokens_no_panic() {
    let (tokens, _) = parse("[0-9]+\\.[0-9]+", 0).unwrap();
    // Just ensure it doesn't panic
    print_regex_tokens(&tokens);
}

fn main() {}
