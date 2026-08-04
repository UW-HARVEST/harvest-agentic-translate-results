use Remimu::my_regex::{regex_match, regex_parse, RegexToken};

fn do_match(pat: &str, text: &str) -> Option<usize> {
    let mut tokens = vec![RegexToken::default(); 1024];
    let mut token_count: i16 = 1024;
    let r = regex_parse(pat, &mut tokens, &mut token_count, 0);
    assert!(r.is_ok(), "Parse failed for pattern '{}'", pat);
    let mut cap_pos = [0i64; 16];
    let mut cap_span = [0i64; 16];
    regex_match(
        &tokens,
        text,
        0,
        0u16,
        &mut cap_pos[..],
        &mut cap_span[..],
    )
}

#[test]
fn lazy_quantifiers() {
    // Lazy quantifier should match the minimum
    assert_eq!(do_match("a*?b", "aaab"), Some(4));
    assert_eq!(do_match("a+?b", "aaab"), Some(4));
}

#[test]
fn possessive_quantifiers() {
    // Possessive quantifier won't backtrack
    assert_eq!(do_match("a++ab", "aaaab"), None);
    assert_eq!(do_match("a*+a", "aaaa"), None);
}

#[test]
fn nested_groups() {
    assert_eq!(do_match("((a)|(b))+", "abab"), Some(4));
    assert_eq!(do_match("((a)|(b))++", "abab"), Some(4));
}

#[test]
fn ranges() {
    assert_eq!(do_match("a{3}", "aaaa"), Some(3));
    assert_eq!(do_match("a{3,5}", "aaaaaaa"), Some(5));
    assert_eq!(do_match("a{3,}", "aaaaa"), Some(5));
}

#[test]
fn dot_match() {
    assert_eq!(do_match(".+", "hello"), Some(5));
    assert_eq!(do_match("a.c", "abc"), Some(3));
}

#[test]
fn anchored_patterns() {
    assert_eq!(do_match("^asdf$", "asdf"), Some(4));
    assert_eq!(do_match("^asdf", "asdfg"), Some(4));
    assert_eq!(do_match("asdf$", "asdf"), Some(4));
    assert_eq!(do_match("asdf$", "asdfg"), None);
}

#[test]
fn word_boundary() {
    assert_eq!(do_match("\\basdf\\b", "asdf "), Some(4));
}

#[test]
fn character_class_complex() {
    assert_eq!(do_match("[a-zA-Z0-9]+", "Hello123"), Some(8));
    assert_eq!(do_match("[^abc]+", "xyzabc"), Some(3));
}
