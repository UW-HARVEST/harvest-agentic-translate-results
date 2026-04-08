use jccc::token::*;

#[test]
fn test_token_default() {
    let t = Token::default();
    assert_eq!(t.token_type, TokenType::TT_NO_TOKEN);
    assert_eq!(t.contents, "");
    assert_eq!(t.length, 0);
    assert_eq!(t.source_file, "");
    assert_eq!(t.line, 0);
    assert_eq!(t.column, 0);
}

#[test]
fn test_token_length_constant() {
    assert_eq!(TOKEN_LENGTH, 256);
}

#[test]
fn test_token_type_clone_eq() {
    let a = TokenType::TT_PLUS;
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn test_token_type_ne() {
    assert_ne!(TokenType::TT_PLUS, TokenType::TT_MINUS);
    assert_ne!(TokenType::TT_LITERAL, TokenType::TT_IDENTIFIER);
}

fn main() {}
