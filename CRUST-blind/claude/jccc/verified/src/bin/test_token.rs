use jccc::token::{Token, TokenType, TOKEN_LENGTH};

#[test]
fn test_token_length_constant() {
    assert_eq!(TOKEN_LENGTH, 256);
}

#[test]
fn test_token_construction() {
    let t = Token {
        token_type: TokenType::TT_LITERAL,
        contents: String::from("42"),
        length: 2,
        source_file: String::from("test.c"),
        line: 1,
        column: 5,
    };
    assert_eq!(t.token_type, TokenType::TT_LITERAL);
    assert_eq!(t.contents, "42");
    assert_eq!(t.length, 2);
    assert_eq!(t.source_file, "test.c");
    assert_eq!(t.line, 1);
    assert_eq!(t.column, 5);
}

#[test]
fn test_token_type_eq() {
    assert!(TokenType::TT_LITERAL == TokenType::TT_LITERAL);
    assert!(TokenType::TT_PLUS != TokenType::TT_MINUS);
    assert!(TokenType::TT_INT != TokenType::TT_RETURN);
}

fn main() {}
