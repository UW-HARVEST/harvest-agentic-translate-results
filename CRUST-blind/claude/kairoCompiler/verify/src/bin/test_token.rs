use kairoCompiler::token::{
    token_is_keyword, token_is_symbol, token_is_nl_or_comment_or_newline_separator,
};
use kairoCompiler::compiler::{
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_NUMBER,
    TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
};

#[test]
fn test_token_is_keyword_match() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_KEYWORD;
    t.sval = Some("if".to_string());
    let res = token_is_keyword(&mut t, "if");
    assert_eq!(res, true);
    // C bug: assignment of `TOKEN_TYPE_KEYWORD && S_EQ(...)` results in 1 if match
    assert_eq!(t.r#type, TOKEN_TYPE_KEYWORD);
}

#[test]
fn test_token_is_keyword_mismatch_value() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_IDENTIFIER;
    t.sval = Some("abc".to_string());
    let res = token_is_keyword(&mut t, "if");
    assert_eq!(res, false);
    // C bug: when no match, token->type becomes 0
    assert_eq!(t.r#type, 0);
}

#[test]
fn test_token_is_keyword_match_overwrites_type() {
    // C bug: even if type was TOKEN_TYPE_NUMBER (=4) and sval matches, it gets overwritten
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.sval = Some("if".to_string());
    let res = token_is_keyword(&mut t, "if");
    assert_eq!(res, true);
    assert_eq!(t.r#type, TOKEN_TYPE_KEYWORD);
}

#[test]
fn test_token_is_keyword_no_sval() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_KEYWORD;
    t.sval = None;
    let res = token_is_keyword(&mut t, "if");
    assert_eq!(res, false);
    assert_eq!(t.r#type, 0);
}

#[test]
fn test_token_is_symbol_match() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some('{');
    assert_eq!(token_is_symbol(&t, '{'), true);
}

#[test]
fn test_token_is_symbol_wrong_char() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some('{');
    assert_eq!(token_is_symbol(&t, '}'), false);
}

#[test]
fn test_token_is_symbol_wrong_type() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NUMBER;
    t.cval = Some('{');
    assert_eq!(token_is_symbol(&t, '{'), false);
}

#[test]
fn test_token_is_symbol_no_cval() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = None;
    assert_eq!(token_is_symbol(&t, '{'), false);
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
    t.r#type = TOKEN_TYPE_NUMBER;
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), false);
}

#[test]
fn test_token_is_nl_or_comment_other_symbol() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some(';');
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), false);
}

fn main() {}
