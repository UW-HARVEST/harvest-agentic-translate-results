use kairoCompiler::compiler::{
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NUMBER,
    TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT, TOKEN_TYPE_IDENTIFIER,
};
use kairoCompiler::token::{token_is_keyword, token_is_symbol, token_is_nl_or_comment_or_newline_separator};

#[test]
fn test_token_is_keyword_match() {
    let mut t = Token {
        r#type: TOKEN_TYPE_KEYWORD,
        sval: Some("int".to_string()),
        ..Token::default()
    };
    assert!(token_is_keyword(&mut t, "int"));
    // C bug: type gets assigned result of (TOKEN_TYPE_KEYWORD && S_EQ) = 1
    assert_eq!(t.r#type, 1);
}

#[test]
fn test_token_is_keyword_no_match() {
    let mut t = Token {
        r#type: TOKEN_TYPE_KEYWORD,
        sval: Some("int".to_string()),
        ..Token::default()
    };
    assert!(!token_is_keyword(&mut t, "float"));
    // C bug: type gets assigned 0 when no match
    assert_eq!(t.r#type, 0);
}

#[test]
fn test_token_is_keyword_wrong_type_but_sval_matches() {
    // C bug: token->type = TOKEN_TYPE_KEYWORD && S_EQ(token->sval, value)
    // When type is NUMBER (4), the assignment is: type = (1 && 1) = 1
    // But the original type != TOKEN_TYPE_KEYWORD, so the && short-circuits differently
    // Actually: TOKEN_TYPE_KEYWORD is 1, which is truthy. S_EQ("int","int") is true.
    // So the result is 1 && 1 = 1 (true). But wait, the C code does:
    // return token->type = TOKEN_TYPE_KEYWORD && S_EQ(...)
    // This is: token->type = (TOKEN_TYPE_KEYWORD && S_EQ(...))
    // TOKEN_TYPE_KEYWORD = 1, S_EQ("int","int") = 1, so 1 && 1 = 1
    // token->type becomes 1, return value is 1 (true)
    // BUT: the original check should be token->type == TOKEN_TYPE_KEYWORD
    // The bug means it ALWAYS evaluates TOKEN_TYPE_KEYWORD (which is 1, truthy)
    // So it only depends on S_EQ.
    let mut t = Token {
        r#type: TOKEN_TYPE_NUMBER,
        sval: Some("int".to_string()),
        ..Token::default()
    };
    // Due to the bug, this returns true because TOKEN_TYPE_KEYWORD(1) is truthy
    // and S_EQ matches
    assert!(token_is_keyword(&mut t, "int"));
    assert_eq!(t.r#type, 1);
}

#[test]
fn test_token_is_symbol_match() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('{'),
        ..Token::default()
    };
    assert!(token_is_symbol(&t, '{'));
}

#[test]
fn test_token_is_symbol_no_match() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('{'),
        ..Token::default()
    };
    assert!(!token_is_symbol(&t, '}'));
}

#[test]
fn test_token_is_symbol_wrong_type() {
    let t = Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some('{'),
        ..Token::default()
    };
    assert!(!token_is_symbol(&t, '{'));
}

#[test]
fn test_is_nl_newline() {
    let t = Token {
        r#type: TOKEN_TYPE_NEWLINE,
        ..Token::default()
    };
    assert!(token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_is_nl_comment() {
    let t = Token {
        r#type: TOKEN_TYPE_COMMENT,
        ..Token::default()
    };
    assert!(token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_is_nl_backslash_symbol() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('\\'),
        ..Token::default()
    };
    assert!(token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_is_nl_number_false() {
    let t = Token {
        r#type: TOKEN_TYPE_NUMBER,
        ..Token::default()
    };
    assert!(!token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_is_nl_identifier_false() {
    let t = Token {
        r#type: TOKEN_TYPE_IDENTIFIER,
        ..Token::default()
    };
    assert!(!token_is_nl_or_comment_or_newline_separator(&t));
}

fn main() {}
