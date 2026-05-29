use simple_lang::token::{free_token, new_token, Token, TokenType};

#[test]
fn test_new_token_int() {
    let t = new_token(TokenType::TOKEN_INT, "42");
    assert_eq!(t.token_type, TokenType::TOKEN_INT);
    assert_eq!(t.value, "42");
}

#[test]
fn test_new_token_identifier() {
    let t = new_token(TokenType::TOKEN_IDENTIFIER, "x");
    assert_eq!(t.token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(t.value, "x");
}

#[test]
fn test_new_token_assign() {
    let t = new_token(TokenType::TOKEN_ASSIGN, "=");
    assert_eq!(t.token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(t.value, "=");
}

#[test]
fn test_new_token_plus() {
    let t = new_token(TokenType::TOKEN_PLUS, "+");
    assert_eq!(t.token_type, TokenType::TOKEN_PLUS);
    assert_eq!(t.value, "+");
}

#[test]
fn test_new_token_minus() {
    let t = new_token(TokenType::TOKEN_MINUS, "-");
    assert_eq!(t.token_type, TokenType::TOKEN_MINUS);
    assert_eq!(t.value, "-");
}

#[test]
fn test_new_token_semicolon() {
    let t = new_token(TokenType::TOKEN_SEMICOLON, ";");
    assert_eq!(t.token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(t.value, ";");
}

#[test]
fn test_new_token_let() {
    let t = new_token(TokenType::TOKEN_LET, "let");
    assert_eq!(t.token_type, TokenType::TOKEN_LET);
    assert_eq!(t.value, "let");
}

#[test]
fn test_new_token_eof() {
    let t = new_token(TokenType::TOKEN_EOF, "");
    assert_eq!(t.token_type, TokenType::TOKEN_EOF);
    assert_eq!(t.value, "");
}

#[test]
fn test_new_token_dis() {
    let t = new_token(TokenType::TOKEN_DIS, "dis");
    assert_eq!(t.token_type, TokenType::TOKEN_DIS);
    assert_eq!(t.value, "dis");
}

#[test]
fn test_token_clone() {
    let t = new_token(TokenType::TOKEN_INT, "100");
    let c: Token = t.clone();
    assert_eq!(c.token_type, TokenType::TOKEN_INT);
    assert_eq!(c.value, "100");
}

#[test]
fn test_free_token_no_panic() {
    let t = new_token(TokenType::TOKEN_INT, "5");
    free_token(t);
}

#[test]
fn test_token_types_distinct() {
    let a = new_token(TokenType::TOKEN_INT, "1");
    let b = new_token(TokenType::TOKEN_PLUS, "+");
    assert_ne!(a.token_type, b.token_type);
}

fn main() {}
