use simple_lang::token;

#[test]
fn test_new_token_int() {
    let tok = token::new_token(token::TokenType::TOKEN_INT, "42");
    assert_eq!(tok.token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tok.value, "42");
}

#[test]
fn test_new_token_identifier() {
    let tok = token::new_token(token::TokenType::TOKEN_IDENTIFIER, "x");
    assert_eq!(tok.token_type, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tok.value, "x");
}

#[test]
fn test_new_token_let() {
    let tok = token::new_token(token::TokenType::TOKEN_LET, "let");
    assert_eq!(tok.token_type, token::TokenType::TOKEN_LET);
    assert_eq!(tok.value, "let");
}

#[test]
fn test_new_token_eof() {
    let tok = token::new_token(token::TokenType::TOKEN_EOF, "");
    assert_eq!(tok.token_type, token::TokenType::TOKEN_EOF);
    assert_eq!(tok.value, "");
}

#[test]
fn test_free_token() {
    let tok = token::new_token(token::TokenType::TOKEN_PLUS, "+");
    token::free_token(tok); // should not panic
}

#[test]
fn test_token_clone() {
    let tok = token::new_token(token::TokenType::TOKEN_DIS, "dis");
    let cloned = tok.clone();
    assert_eq!(cloned.token_type, token::TokenType::TOKEN_DIS);
    assert_eq!(cloned.value, "dis");
}

fn main() {}
