use jccc::token::{Token, TokenType, TOKEN_LENGTH};

#[test]
fn test_token_type_default() {
    let tt: TokenType = Default::default();
    assert_eq!(tt, TokenType::TT_NO_TOKEN);
}

#[test]
fn test_token_default() {
    let t: Token = Default::default();
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
fn test_token_type_variants_exist() {
    // Verify enum variants match C ordering by checking discriminant values
    assert_eq!(TokenType::TT_LITERAL as usize, 0);
    assert_eq!(TokenType::TT_IDENTIFIER as usize, 1);
    assert_eq!(TokenType::TT_OPAREN as usize, 2);
    assert_eq!(TokenType::TT_CPAREN as usize, 3);
    assert_eq!(TokenType::TT_OBRACE as usize, 4);
    assert_eq!(TokenType::TT_CBRACE as usize, 5);
    assert_eq!(TokenType::TT_OBRACKET as usize, 6);
    assert_eq!(TokenType::TT_CBRACKET as usize, 7);
    assert_eq!(TokenType::TT_SEMI as usize, 8);
    assert_eq!(TokenType::TT_NO_TOKEN as usize, 9);
    assert_eq!(TokenType::TT_EOF as usize, 10);
    assert_eq!(TokenType::TT_NEWLINE as usize, 11);
    assert_eq!(TokenType::TT_POUND as usize, 12);
    assert_eq!(TokenType::TT_PERIOD as usize, 13);
    assert_eq!(TokenType::TT_COMMA as usize, 14);
    assert_eq!(TokenType::TT_QMARK as usize, 15);
    assert_eq!(TokenType::TT_WHILE as usize, 84);
}

fn main() {}
