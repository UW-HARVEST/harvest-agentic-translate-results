use simple_lang::token::{self, TokenType, Token};

#[test]
fn test_new_token() {
    let t = token::new_token(TokenType::TOKEN_INT, "42");
    assert_eq!(t.token_type, TokenType::TOKEN_INT);
    assert_eq!(t.value, "42");
}

#[test]
fn test_new_token_empty_value() {
    let t = token::new_token(TokenType::TOKEN_EOF, "");
    assert_eq!(t.token_type, TokenType::TOKEN_EOF);
    assert_eq!(t.value, "");
}

#[test]
fn test_all_token_types() {
    let types = vec![
        (TokenType::TOKEN_INT, "123"),
        (TokenType::TOKEN_IDENTIFIER, "x"),
        (TokenType::TOKEN_ASSIGN, "="),
        (TokenType::TOKEN_PLUS, "+"),
        (TokenType::TOKEN_MINUS, "-"),
        (TokenType::TOKEN_SEMICOLON, ";"),
        (TokenType::TOKEN_LET, "let"),
        (TokenType::TOKEN_EOF, ""),
        (TokenType::TOKEN_DIS, "dis"),
    ];
    for (tt, val) in types {
        let t = token::new_token(tt.clone(), val);
        assert_eq!(t.token_type, tt);
        assert_eq!(t.value, val);
    }
}

#[test]
fn test_free_token_no_panic() {
    let t = token::new_token(TokenType::TOKEN_INT, "1");
    token::free_token(t);
}

fn main() {}
