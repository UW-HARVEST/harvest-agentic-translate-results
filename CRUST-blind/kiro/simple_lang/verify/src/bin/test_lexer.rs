use simple_lang::lexer;
use simple_lang::token::TokenType;

#[test]
fn test_tokenize_let_expr_dis() {
    // Ground truth from C: "let x = 5 + 3 - 2; dis x;" produces 12 tokens + EOF
    let tokens = lexer::tokenize("let x = 5 + 3 - 2; dis x;");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[0].value, "let");
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[1].value, "x");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[2].value, "=");
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[3].value, "5");
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_PLUS);
    assert_eq!(tokens[4].value, "+");
    assert_eq!(tokens[5].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[5].value, "3");
    assert_eq!(tokens[6].token_type, TokenType::TOKEN_MINUS);
    assert_eq!(tokens[6].value, "-");
    assert_eq!(tokens[7].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[7].value, "2");
    assert_eq!(tokens[8].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[8].value, ";");
    assert_eq!(tokens[9].token_type, TokenType::TOKEN_DIS);
    assert_eq!(tokens[9].value, "dis");
    assert_eq!(tokens[10].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[10].value, "x");
    assert_eq!(tokens[11].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[11].value, ";");
    assert_eq!(tokens[12].token_type, TokenType::TOKEN_EOF);
    // 12 non-EOF tokens + 1 EOF = 13 total
    assert_eq!(tokens.len(), 13);
}

#[test]
fn test_tokenize_numbers() {
    // C ground truth: "123 456 789;" -> INT(123), INT(456), INT(789), SEMICOLON, EOF
    let tokens = lexer::tokenize("123 456 789;");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[0].value, "123");
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[1].value, "456");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[2].value, "789");
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_EOF);
    assert_eq!(tokens.len(), 5);
}

#[test]
fn test_tokenize_empty() {
    let tokens = lexer::tokenize("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_whitespace_only() {
    let tokens = lexer::tokenize("   \t\n  ");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_header_guard() {
    assert_eq!(lexer::SIMPLE_LANG_LEXER_H, true);
}

fn main() {}
