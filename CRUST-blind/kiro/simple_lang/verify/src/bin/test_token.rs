use simple_lang::token::{self, TokenType};

#[test]
fn test_new_token() {
    let t = token::new_token(TokenType::TOKEN_INT, "42");
    assert_eq!(t.token_type, TokenType::TOKEN_INT);
    assert_eq!(t.value, "42");
}

#[test]
fn test_new_token_identifier() {
    let t = token::new_token(TokenType::TOKEN_IDENTIFIER, "myvar");
    assert_eq!(t.token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(t.value, "myvar");
}

#[test]
fn test_new_token_empty_value() {
    let t = token::new_token(TokenType::TOKEN_EOF, "");
    assert_eq!(t.token_type, TokenType::TOKEN_EOF);
    assert_eq!(t.value, "");
}

#[test]
fn test_free_token_no_panic() {
    let t = token::new_token(TokenType::TOKEN_PLUS, "+");
    token::free_token(t); // should not panic
}

#[test]
fn test_header_guard() {
    assert_eq!(token::SIMPLE_LANG_TOKEN_H, true);
}

#[test]
fn test_token_type_clone_eq() {
    let a = TokenType::TOKEN_LET;
    let b = a.clone();
    assert_eq!(a, b);
}

fn main() {}
