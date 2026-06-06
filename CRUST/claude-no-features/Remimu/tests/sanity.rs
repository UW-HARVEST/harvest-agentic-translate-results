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
fn basic_matches() {
    assert_eq!(do_match("[0-9]+\\.[0-9]+", "23.53) "), Some(5));
    assert_eq!(do_match("hello", "hello world"), Some(5));
    assert_eq!(do_match("a+", "aaaa"), Some(4));
    assert_eq!(do_match("\\d{3}", "123abc"), Some(3));
}

#[test]
fn quantifiers() {
    assert_eq!(do_match("a?b", "ab"), Some(2));
    assert_eq!(do_match("a?b", "b"), Some(1));
    assert_eq!(do_match("a*b", "aaab"), Some(4));
    assert_eq!(do_match("a+b", "aaab"), Some(4));
    assert_eq!(do_match("a+b", "b"), None);
}

#[test]
fn alternation() {
    assert_eq!(do_match("a|b", "a"), Some(1));
    assert_eq!(do_match("a|b", "b"), Some(1));
    assert_eq!(do_match("(a|b)+", "abab"), Some(4));
}

#[test]
fn char_class() {
    assert_eq!(do_match("[a-z]+", "hello"), Some(5));
    assert_eq!(do_match("[A-Z]+", "HELLO"), Some(5));
    assert_eq!(do_match("[^0-9]+", "abc123"), Some(3));
}

#[test]
fn anchors() {
    assert_eq!(do_match("^abc", "abc"), Some(3));
    assert_eq!(do_match("abc$", "abc"), Some(3));
    assert_eq!(do_match("^abc$", "abc"), Some(3));
}

#[test]
fn shorthand() {
    assert_eq!(do_match("\\w+", "hello123"), Some(8));
    assert_eq!(do_match("\\d+", "12345abc"), Some(5));
    assert_eq!(do_match("\\s+", "   x"), Some(3));
}

#[test]
fn groups_with_captures() {
    let mut tokens = vec![RegexToken::default(); 256];
    let mut token_count: i16 = 256;
    let r = regex_parse("([0-9]+)\\.([0-9]+)", &mut tokens, &mut token_count, 0);
    assert!(r.is_ok());
    let mut cap_pos = [-1i64; 16];
    let mut cap_span = [-1i64; 16];
    let m = regex_match(&tokens, "23.53) ", 0, 5, &mut cap_pos[..], &mut cap_span[..]);
    assert_eq!(m, Some(5));
}
