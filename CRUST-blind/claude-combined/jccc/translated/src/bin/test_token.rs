use jccc::token::{Token, TokenType, TOKEN_LENGTH};

#[test]
fn test_token_length_constant() {
    assert_eq!(TOKEN_LENGTH, 256);
}

#[test]
fn test_token_construction() {
    let t = Token {
        token_type: TokenType::TT_INT,
        contents: "int".to_string(),
        length: 3,
        source_file: "foo.c".to_string(),
        line: 1,
        column: 2,
    };
    assert!(matches!(t.token_type, TokenType::TT_INT));
    assert_eq!(t.contents, "int");
    assert_eq!(t.length, 3);
    assert_eq!(t.source_file, "foo.c");
    assert_eq!(t.line, 1);
    assert_eq!(t.column, 2);
}

#[test]
fn test_token_type_distinct() {
    assert!(TokenType::TT_LITERAL != TokenType::TT_IDENTIFIER);
    assert!(TokenType::TT_LITERAL == TokenType::TT_LITERAL);
    assert!(TokenType::TT_PLUS != TokenType::TT_MINUS);
    assert!(TokenType::TT_INT != TokenType::TT_FLOAT);
}

fn main() {}
