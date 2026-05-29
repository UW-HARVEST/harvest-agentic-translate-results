use worsp::worsp::*;

#[allow(dead_code)]
fn new_state() -> ParseState {
    ParseState { token: None, pos: 0 }
}

#[test]
fn test_match_token_eof_when_empty() {
    let mut s = new_state();
    next("", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Eof), 1);
    assert_eq!(match_token(&mut s, TokenKind::LParen), 0);
}

#[test]
fn test_next_lparen_rparen() {
    let mut s = new_state();
    next("(", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::LParen), 1);
    let mut s2 = new_state();
    next(")", &mut s2);
    assert_eq!(match_token(&mut s2, TokenKind::RParen), 1);
}

#[test]
fn test_next_quote() {
    let mut s = new_state();
    next("'", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Quote), 1);
}

#[test]
fn test_next_digit() {
    let mut s = new_state();
    next("12345", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Digit), 1);
    let t = s.token.as_ref().unwrap();
    assert_eq!(t.val, 12345);
}

#[test]
fn test_next_zero() {
    let mut s = new_state();
    next("0", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Digit), 1);
    let t = s.token.as_ref().unwrap();
    assert_eq!(t.val, 0);
}

#[test]
fn test_next_symbol() {
    let mut s = new_state();
    next("hello", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Symbol), 1);
    let t = s.token.as_ref().unwrap();
    assert_eq!(t.str, "hello");
}

#[test]
fn test_next_true_false() {
    let mut s1 = new_state();
    next("true", &mut s1);
    assert_eq!(match_token(&mut s1, TokenKind::True), 1);
    let mut s2 = new_state();
    next("false", &mut s2);
    assert_eq!(match_token(&mut s2, TokenKind::False), 1);
}

#[test]
fn test_next_string() {
    let mut s = new_state();
    next(r#""hello""#, &mut s);
    assert_eq!(match_token(&mut s, TokenKind::String), 1);
    let t = s.token.as_ref().unwrap();
    assert_eq!(t.str, "hello");
}

#[test]
fn test_next_op_symbol() {
    let mut s = new_state();
    next("+", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Symbol), 1);
    let t = s.token.as_ref().unwrap();
    assert_eq!(t.str, "+");
}

#[test]
fn test_next_skip_whitespace() {
    let mut s = new_state();
    next("   42", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Digit), 1);
    let t = s.token.as_ref().unwrap();
    assert_eq!(t.val, 42);
}

#[test]
fn test_next_comment_skip() {
    let mut s = new_state();
    next("; comment\n42", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Digit), 1);
    let t = s.token.as_ref().unwrap();
    assert_eq!(t.val, 42);
}

#[test]
fn test_sequential_tokens() {
    let mut s = new_state();
    next("(+ 1 2)", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::LParen), 1);
    next("(+ 1 2)", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Symbol), 1);
    assert_eq!(s.token.as_ref().unwrap().str, "+");
    next("(+ 1 2)", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Digit), 1);
    assert_eq!(s.token.as_ref().unwrap().val, 1);
    next("(+ 1 2)", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Digit), 1);
    assert_eq!(s.token.as_ref().unwrap().val, 2);
    next("(+ 1 2)", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::RParen), 1);
    next("(+ 1 2)", &mut s);
    assert_eq!(match_token(&mut s, TokenKind::Eof), 1);
}

fn main() {}
