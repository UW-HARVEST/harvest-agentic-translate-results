use kairoCompiler::compiler::{
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NUMBER,
    TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
};
use kairoCompiler::token::{
    token_is_keyword, token_is_symbol, token_is_nl_or_comment_or_newline_separator,
};

#[test]
fn test_token_is_keyword_match() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_KEYWORD;
    t.sval = Some("if".to_string());
    let r = token_is_keyword(&mut t, "if");
    // C version returns true (1) on match
    assert_eq!(r, true);
    // Bug replica: token type is set to the assignment result (1)
    assert_eq!(t.r#type, 1);
}

#[test]
fn test_token_is_keyword_match_even_when_not_keyword_type() {
    // C version's bug: it overwrites the type field, so even an identifier with sval=="if" passes.
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_IDENTIFIER;
    t.sval = Some("if".to_string());
    let r = token_is_keyword(&mut t, "if");
    assert_eq!(r, true);
    assert_eq!(t.r#type, 1);
}

#[test]
fn test_token_is_keyword_mismatch() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_KEYWORD;
    t.sval = Some("while".to_string());
    let r = token_is_keyword(&mut t, "if");
    assert_eq!(r, false);
    assert_eq!(t.r#type, 0);
}

#[test]
fn test_token_is_keyword_no_sval() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_KEYWORD;
    t.sval = None;
    let r = token_is_keyword(&mut t, "if");
    assert_eq!(r, false);
    assert_eq!(t.r#type, 0);
}

#[test]
fn test_token_is_symbol_match() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some(';');
    assert_eq!(token_is_symbol(&t, ';'), true);
    assert_eq!(token_is_symbol(&t, ','), false);
}

#[test]
fn test_token_is_symbol_wrong_type() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.cval = Some(';');
    assert_eq!(token_is_symbol(&t, ';'), false);
}

#[test]
fn test_token_is_nl_or_comment_newline() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NEWLINE;
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), true);
}

#[test]
fn test_token_is_nl_or_comment_comment() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_COMMENT;
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), true);
}

#[test]
fn test_token_is_nl_or_comment_backslash_symbol() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some('\\');
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), true);
}

#[test]
fn test_token_is_nl_or_comment_other() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_IDENTIFIER;
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), false);
}

fn main() {}
