use kairoCompiler::compiler::{
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NUMBER,
    TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT, TOKEN_TYPE_IDENTIFIER,
};
use kairoCompiler::token::*;

#[test]
fn test_token_is_keyword_match() {
    let mut t = Token {
        r#type: TOKEN_TYPE_KEYWORD,
        sval: Some("int".to_string()),
        ..Default::default()
    };
    // C bug: uses = instead of ==, assigns type=1 (KEYWORD) then checks sval
    // Rust mirrors this: assigns type = if eq {1} else {0}
    let result = token_is_keyword(&mut t, "int");
    assert!(result);
    assert_eq!(t.r#type, 1); // type set to 1 (true/KEYWORD)
}

#[test]
fn test_token_is_keyword_wrong_type_matching_sval() {
    let mut t = Token {
        r#type: TOKEN_TYPE_NUMBER, // type 4, not KEYWORD
        sval: Some("int".to_string()),
        ..Default::default()
    };
    // C: token->type = TOKEN_TYPE_KEYWORD(1) && S_EQ => type = (1 && true) = 1, returns 1
    // But Rust: eq = (type == KEYWORD && sval matches) = (4==1 && true) = false
    // So Rust returns false and sets type=0
    let result = token_is_keyword(&mut t, "int");
    assert!(!result);
    assert_eq!(t.r#type, 0);
}

#[test]
fn test_token_is_keyword_matching_type_wrong_sval() {
    let mut t = Token {
        r#type: TOKEN_TYPE_KEYWORD,
        sval: Some("float".to_string()),
        ..Default::default()
    };
    // C: type = (1 && false) = 0, returns 0
    // Rust: eq = (1==1 && "float"=="int") = false, type=0
    let result = token_is_keyword(&mut t, "int");
    assert!(!result);
    assert_eq!(t.r#type, 0);
}

#[test]
fn test_token_is_symbol_match() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('{'),
        ..Default::default()
    };
    assert!(token_is_symbol(&t, '{'));
}

#[test]
fn test_token_is_symbol_wrong_char() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('{'),
        ..Default::default()
    };
    assert!(!token_is_symbol(&t, '}'));
}

#[test]
fn test_token_is_symbol_wrong_type() {
    let t = Token {
        r#type: TOKEN_TYPE_NUMBER,
        cval: Some('{'),
        ..Default::default()
    };
    assert!(!token_is_symbol(&t, '{'));
}

#[test]
fn test_token_is_nl_newline() {
    let t = Token { r#type: TOKEN_TYPE_NEWLINE, ..Default::default() };
    assert!(token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_token_is_nl_comment() {
    let t = Token { r#type: TOKEN_TYPE_COMMENT, ..Default::default() };
    assert!(token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_token_is_nl_backslash() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('\\'),
        ..Default::default()
    };
    assert!(token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_token_is_nl_brace_false() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some('{'),
        ..Default::default()
    };
    assert!(!token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_token_is_nl_number_false() {
    let t = Token { r#type: TOKEN_TYPE_NUMBER, ..Default::default() };
    assert!(!token_is_nl_or_comment_or_newline_separator(&t));
}

fn main() {}
