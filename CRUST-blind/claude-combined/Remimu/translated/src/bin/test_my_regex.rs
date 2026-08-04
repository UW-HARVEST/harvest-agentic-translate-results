use Remimu::my_regex::*;

fn parse(pattern: &str) -> Vec<RegexToken> {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse(pattern, &mut tokens, &mut count, 0);
    assert!(res.is_ok(), "regex parse failed: {}", pattern);
    tokens
}

fn match_len(pattern: &str, text: &str) -> Option<usize> {
    let tokens = parse(pattern);
    let mut cap_pos = vec![-1i64; 16];
    let mut cap_span = vec![-1i64; 16];
    regex_match(&tokens, text, 0, 16, &mut cap_pos, &mut cap_span)
}

#[test]
fn test_set_mask_and_check() {
    let mut tok = RegexToken::new(REMIMU_KIND_NORMAL, 0);
    tok.set_mask(b'a');
    assert!(tok.check_mask(b'a'));
    assert!(!tok.check_mask(b'b'));
    tok.set_mask(b'b');
    assert!(tok.check_mask(b'b'));
}

#[test]
fn test_set_mask_full_range() {
    let mut tok = RegexToken::new(REMIMU_KIND_NORMAL, 0);
    for c in 0u8..=255u8 {
        tok.set_mask(c);
    }
    for c in 0u8..=255u8 {
        assert!(tok.check_mask(c));
    }
}

#[test]
fn test_set_mask_specific() {
    let mut tok = RegexToken::new(REMIMU_KIND_NORMAL, 0);
    tok.set_mask(0);
    assert!(tok.check_mask(0));
    tok.set_mask(0xFF);
    assert!(tok.check_mask(0xFF));
    tok.set_mask(0x80);
    assert!(tok.check_mask(0x80));
    assert!(!tok.check_mask(0x7F));
}

#[test]
fn test_invert_mask() {
    let mut tok = RegexToken::new(REMIMU_KIND_NORMAL, REMIMU_MODE_INVERTED);
    tok.set_mask(b'a');
    tok.invert_mask();
    assert!(!tok.check_mask(b'a'));
    assert!(tok.check_mask(b'b'));
    // mode INVERTED bit cleared
    assert_eq!(tok.mode & REMIMU_MODE_INVERTED, 0);
}

#[test]
fn test_token_default() {
    let tok = RegexToken::default();
    assert_eq!(tok.kind, REMIMU_KIND_NORMAL);
    assert_eq!(tok.mode, 0);
    assert_eq!(tok.count_lo, 1);
    assert_eq!(tok.count_hi, 2);
    assert_eq!(tok.pair_offset, 0);
}

#[test]
fn test_matcher_state_new() {
    let s = RegexMatcherState::new(5, 10);
    assert_eq!(s.k, 5);
    assert_eq!(s.i, 10);
    assert_eq!(s.group_state, 0);
    assert_eq!(s.prev, 0);
    assert_eq!(s.range_min, 0);
    assert_eq!(s.range_max, 0);
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
fn test_parse_simple_digits() {
    // [0-9]+\\.[0-9]+ matches "23.53)" with len 5
    assert_eq!(match_len("[0-9]+\\.[0-9]+", "23.53) "), Some(5));
}

#[test]
fn test_parse_token_count() {
    // [0-9]+\.[0-9]+ produces 6 tokens (OPEN, NORMAL, NORMAL, NORMAL, CLOSE, END)
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("[0-9]+\\.[0-9]+", &mut tokens, &mut count, 0);
    assert!(res.is_ok());
    assert_eq!(count, 6);
    assert_eq!(tokens[0].kind, REMIMU_KIND_OPEN);
    assert_eq!(tokens[1].kind, REMIMU_KIND_NORMAL);
    assert_eq!(tokens[2].kind, REMIMU_KIND_NORMAL);
    assert_eq!(tokens[3].kind, REMIMU_KIND_NORMAL);
    assert_eq!(tokens[4].kind, REMIMU_KIND_CLOSE);
    assert_eq!(tokens[5].kind, REMIMU_KIND_END);
}

#[test]
fn test_match_no_match() {
    // [0-9]+\.[0-9]+ doesn't match "abc"
    assert_eq!(match_len("[0-9]+\\.[0-9]+", "abc"), None);
}

#[test]
fn test_match_anchors() {
    assert_eq!(match_len("^asdf", "asdf"), Some(4));
    assert_eq!(match_len("^asdf", "xxasdf"), None);
    assert_eq!(match_len("asdf$", "asdf"), Some(4));
}

#[test]
fn test_match_decimal_simple() {
    assert_eq!(match_len("\\d\\.\\d", "5.5"), Some(3));
    assert_eq!(match_len("\\d\\.\\d", "55"), None);
}

#[test]
fn test_match_dot_star() {
    // .*asdf
    assert_eq!(match_len(".*asdf", "XXXasdf"), Some(7));
    assert_eq!(match_len(".*asdf", "asdf"), Some(4));
    assert_eq!(match_len(".*asdf", "no match"), None);
}

#[test]
fn test_match_alternation() {
    // (a|b)
    assert_eq!(match_len("(a|b)", "a"), Some(1));
    assert_eq!(match_len("(a|b)", "b"), Some(1));
    assert_eq!(match_len("(a|b)", "c"), None);
}

#[test]
fn test_match_quantifier_braces() {
    assert_eq!(match_len("[0-9]{3,5}\\.[0-9]+", "123.45"), Some(6));
    assert_eq!(match_len("[0-9]{3,5}\\.[0-9]+", "12.45"), None);
}

#[test]
fn test_match_word_boundary() {
    // \basdf should match "asdf" since boundary matches at start
    assert_eq!(match_len("\\basdf", "asdf"), Some(4));
    // asdf\b matches asdf at start of "asdf "
    assert_eq!(match_len("asdf\\b", "asdf "), Some(4));
}

#[test]
fn test_match_possessive_no_captures() {
    // Verified against C: possessive groups return -1 for all captures
    let tokens = parse("((a)|(b))++");
    let mut cap_pos = vec![-1i64; 5];
    let mut cap_span = vec![-1i64; 5];
    let m = regex_match(&tokens, "aaaaaabbbabaqa", 0, 5, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(12));
    // C returns: cap 0 pos=0 span=12, others -1
    assert_eq!(cap_pos[0], 0);
    assert_eq!(cap_span[0], 12);
    for i in 1..5 {
        assert_eq!(cap_pos[i], -1, "cap_pos[{}] should be -1", i);
        assert_eq!(cap_span[i], -1, "cap_span[{}] should be -1", i);
    }
}

#[test]
fn test_match_three_captures_abc() {
    // Verified against C: (a)(b)(c) on "abc" -> len=3
    // C uses cap[0] for implicit outer match; user groups start at cap[1]
    let tokens = parse("(a)(b)(c)");
    let mut cap_pos = vec![-1i64; 16];
    let mut cap_span = vec![-1i64; 16];
    let m = regex_match(&tokens, "abc", 0, 16, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(3));
    assert_eq!(cap_pos[0], 0);
    assert_eq!(cap_span[0], 3); // whole match
    assert_eq!(cap_pos[1], 0);
    assert_eq!(cap_span[1], 1); // (a)
    assert_eq!(cap_pos[2], 1);
    assert_eq!(cap_span[2], 1); // (b)
    assert_eq!(cap_pos[3], 2);
    assert_eq!(cap_span[3], 1); // (c)
    assert_eq!(cap_pos[4], -1);
    assert_eq!(cap_span[4], -1);
}

#[test]
fn test_match_captures_words() {
    // Verified against C: (\w+)\s(\w+) on "hello world" -> len=11
    let tokens = parse("(\\w+)\\s(\\w+)");
    let mut cap_pos = vec![-1i64; 16];
    let mut cap_span = vec![-1i64; 16];
    let m = regex_match(&tokens, "hello world", 0, 16, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(11));
    assert_eq!(cap_pos[0], 0);
    assert_eq!(cap_span[0], 11); // whole match
    assert_eq!(cap_pos[1], 0);
    assert_eq!(cap_span[1], 5); // hello
    assert_eq!(cap_pos[2], 6);
    assert_eq!(cap_span[2], 5); // world
    assert_eq!(cap_pos[3], -1);
    assert_eq!(cap_span[3], -1);
}

#[test]
fn test_match_captures_alt() {
    // Verified against C: (a|b)(c|d) on "ad" -> len=2
    let tokens = parse("(a|b)(c|d)");
    let mut cap_pos = vec![-1i64; 16];
    let mut cap_span = vec![-1i64; 16];
    let m = regex_match(&tokens, "ad", 0, 16, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(2));
    assert_eq!(cap_pos[0], 0);
    assert_eq!(cap_span[0], 2); // whole match
    assert_eq!(cap_pos[1], 0);
    assert_eq!(cap_span[1], 1); // (a|b)
    assert_eq!(cap_pos[2], 1);
    assert_eq!(cap_span[2], 1); // (c|d)
}

#[test]
fn test_match_captures_quant_a() {
    // Verified against C: (a)+ on "aaaa" -> len=4
    // cap[0] = whole match (pos=0, span=4)
    // cap[1] = (a) last iteration (pos=3, span=1)
    let tokens = parse("(a)+");
    let mut cap_pos = vec![-1i64; 16];
    let mut cap_span = vec![-1i64; 16];
    let m = regex_match(&tokens, "aaaa", 0, 16, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(4));
    assert_eq!(cap_pos[0], 0);
    assert_eq!(cap_span[0], 4);
    assert_eq!(cap_pos[1], 3);
    assert_eq!(cap_span[1], 1);
}

#[test]
fn test_match_email_simple() {
    // Verified against C: testacc@example.com matches with len 19
    let pat = "(?:\\w+(?:\\.\\w+)*)@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)";
    assert_eq!(match_len(pat, "testacc@example.com"), Some(19));
}

#[test]
fn test_print_regex_tokens_does_not_panic() {
    let tokens = parse("[0-9]+\\.[0-9]+");
    print_regex_tokens(&tokens);
}

#[test]
fn test_match_empty_text_against_caret() {
    assert_eq!(match_len("^", ""), Some(0));
}

#[test]
fn test_match_lazy() {
    // (a)+? matches "a" (one a)
    assert_eq!(match_len("a+?", "aaa"), Some(1));
    // a+ matches "aaa"
    assert_eq!(match_len("a+", "aaa"), Some(3));
}

#[test]
fn test_match_optional() {
    assert_eq!(match_len("a?b", "ab"), Some(2));
    assert_eq!(match_len("a?b", "b"), Some(1));
}

#[test]
fn test_match_quantifier_zero_count() {
    // a{0} matches anything zero-length
    assert_eq!(match_len("a{0}", "aaa"), Some(0));
}

#[test]
fn test_match_dot_no_newlines_flag() {
    // NOTE: The C code has a bug - it XORs mask[1] which is wrong index for \n (0x0A is in mask[0]).
    // Our Rust mirrors this exact buggy behavior. With flag set, dot DOES still match \n in C.
    // Verified by running C with REMIMU_FLAG_DOT_NO_NEWLINES=1 -> match length 1 for "\n".
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse(".", &mut tokens, &mut count, REMIMU_FLAG_DOT_NO_NEWLINES);
    assert!(res.is_ok());
    let mut cap_pos = vec![-1i64; 16];
    let mut cap_span = vec![-1i64; 16];
    // matches \n (due to C bug)
    let m = regex_match(&tokens, "\n", 0, 16, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(1));
    // matches \r (due to C bug)
    let m = regex_match(&tokens, "\r", 0, 16, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(1));
    // dot matches normal char
    let m = regex_match(&tokens, "a", 0, 16, &mut cap_pos, &mut cap_span);
    assert_eq!(m, Some(1));
}

#[test]
fn test_match_dot_default_matches_newline() {
    assert_eq!(match_len(".", "\n"), Some(1));
}

#[test]
fn test_parse_invalid_unbalanced_paren() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("(abc", &mut tokens, &mut count, 0);
    assert!(res.is_err());
}

#[test]
fn test_parse_invalid_extra_close_paren() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("abc)", &mut tokens, &mut count, 0);
    assert!(res.is_err());
}

#[test]
fn test_parse_invalid_orphan_quantifier() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("*abc", &mut tokens, &mut count, 0);
    assert!(res.is_err());
}

#[test]
fn test_match_caret_dollar() {
    // ^asdf$
    assert_eq!(match_len("^asdf$", "asdf"), Some(4));
    assert_eq!(match_len("^asdf$", "asdf "), None);
    assert_eq!(match_len("^asdf$", "xasdf"), None);
}

#[test]
fn test_match_w_class() {
    assert_eq!(match_len("\\w+", "abc123"), Some(6));
    assert_eq!(match_len("\\w+", "   "), None);
}

#[test]
fn test_match_s_class() {
    assert_eq!(match_len("\\s+", "   "), Some(3));
    assert_eq!(match_len("\\s+", "abc"), None);
}

#[test]
fn test_match_exact_count() {
    // a{3} matches 3 a's
    assert_eq!(match_len("a{3}", "aaaa"), Some(3));
    assert_eq!(match_len("a{3}", "aa"), None);
}

#[test]
fn test_match_range_count_greedy() {
    assert_eq!(match_len("a{2,4}", "aaaaaa"), Some(4));
}

#[test]
fn test_match_range_count_lazy() {
    assert_eq!(match_len("a{2,4}?", "aaaaaa"), Some(2));
}

#[test]
fn test_match_star_greedy() {
    assert_eq!(match_len("a*", "aaaa"), Some(4));
}

#[test]
fn test_match_star_lazy() {
    assert_eq!(match_len("a*?", "aaaa"), Some(0));
}

#[test]
fn test_match_plus_no_match() {
    assert_eq!(match_len("a+", ""), None);
    assert_eq!(match_len("a+", "a"), Some(1));
}

#[test]
fn test_match_star_empty() {
    assert_eq!(match_len("a*", ""), Some(0));
}

#[test]
fn test_match_charclass_lowercase() {
    assert_eq!(match_len("[a-z]+", "abcXYZ"), Some(3));
    assert_eq!(match_len("[A-Z]+", "abcXYZ"), None);
    assert_eq!(match_len("[^a-z]+", "abcXYZ"), None);
}

#[test]
fn test_match_alt_full() {
    assert_eq!(match_len("ab|cd", "abef"), Some(2));
    assert_eq!(match_len("ab|cd", "cdef"), Some(2));
    assert_eq!(match_len("ab|cd", "xyz"), None);
}

#[test]
fn test_match_group_quant() {
    assert_eq!(match_len("(ab)*", "ababab"), Some(6));
    assert_eq!(match_len("(ab)*?", "ababab"), Some(0));
    assert_eq!(match_len("(ab)+", ""), None);
}

#[test]
fn test_match_digit_classes() {
    assert_eq!(match_len("\\d+", "12345"), Some(5));
    assert_eq!(match_len("\\D+", "abcde"), Some(5));
    assert_eq!(match_len("\\W+", "!!!"), Some(3));
    assert_eq!(match_len("\\S+", "hello"), Some(5));
}

#[test]
fn test_match_dot_with_newline() {
    // a.b matches "a\nb" because dot is permissive
    assert_eq!(match_len("a.b", "a\nb"), Some(3));
    assert_eq!(match_len("a.b", "axb"), Some(3));
}

#[test]
fn test_match_escaped_chars() {
    assert_eq!(match_len("\\\\", "\\"), Some(1));
    assert_eq!(match_len("\\.", "."), Some(1));
}

#[test]
fn test_match_quantifier_5_max() {
    // [0-9]{3,5} matches at most 5
    assert_eq!(match_len("[0-9]{3,5}", "123"), Some(3));
    assert_eq!(match_len("[0-9]{3,5}", "12345678"), Some(5));
}

#[test]
fn test_parse_check_bound_token() {
    // \basdf should produce: OPEN, BOUND, NORMAL...
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("\\basdf", &mut tokens, &mut count, 0);
    assert!(res.is_ok());
    assert_eq!(tokens[0].kind, REMIMU_KIND_OPEN);
    assert_eq!(tokens[1].kind, REMIMU_KIND_BOUND);
}

#[test]
fn test_parse_check_caret_token() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("^abc", &mut tokens, &mut count, 0);
    assert!(res.is_ok());
    assert_eq!(tokens[0].kind, REMIMU_KIND_OPEN);
    assert_eq!(tokens[1].kind, REMIMU_KIND_CARET);
}

#[test]
fn test_parse_check_dollar_token() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("abc$", &mut tokens, &mut count, 0);
    assert!(res.is_ok());
    // Last token before END should be DOLLAR (or close, then end)
    let kinds: Vec<u8> = tokens.iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&REMIMU_KIND_DOLLAR));
}

#[test]
fn test_parse_or_kind() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("a|b", &mut tokens, &mut count, 0);
    assert!(res.is_ok());
    let kinds: Vec<u8> = tokens.iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&REMIMU_KIND_OR));
}

#[test]
fn test_parse_ncopen() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut count: i16 = 1024;
    let res = regex_parse("(?:abc)", &mut tokens, &mut count, 0);
    assert!(res.is_ok());
    let kinds: Vec<u8> = tokens.iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&REMIMU_KIND_NCOPEN));
}

#[test]
fn test_match_b_no_word_boundary_inside() {
    // \bfoo\b on "foobar" -> -1 (no match)
    assert_eq!(match_len("\\bfoo\\b", "foobar"), None);
    // \bfoo\b on "foo" -> 3
    assert_eq!(match_len("\\bfoo\\b", "foo"), Some(3));
}

#[test]
fn test_match_long_alternations() {
    // (b|a|as|q|)*X matches "asqbX" with len 5
    assert_eq!(match_len("(b|a|as|q|)*X", "asqbX"), Some(5));
}

#[test]
fn test_push_to_vec_collapses_bound() {
    // BOUND followed by BOUND should collapse to single token
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut tok = RegexToken::new(REMIMU_KIND_BOUND, 0);
    tok.push_to_vec(&mut tokens, 100).unwrap();
    let mut tok2 = RegexToken::new(REMIMU_KIND_BOUND, 0);
    tok2.push_to_vec(&mut tokens, 100).unwrap();
    // Second BOUND should NOT be added
    assert_eq!(tokens.len(), 1);
}

#[test]
fn test_push_to_vec_max_len() {
    let mut tokens: Vec<RegexToken> = Vec::new();
    let mut tok = RegexToken::new(REMIMU_KIND_NORMAL, 0);
    let res = tok.push_to_vec(&mut tokens, 0);
    assert_eq!(res, Err(-2));
}

fn main() {}
