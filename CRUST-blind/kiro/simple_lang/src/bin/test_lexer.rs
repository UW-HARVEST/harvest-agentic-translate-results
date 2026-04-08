use simple_lang::lexer;
use simple_lang::token::TokenType;

#[test]
fn test_tokenize_integer() {
    let tokens = lexer::tokenize("42");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[0].value, "42");
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_identifier() {
    let tokens = lexer::tokenize("xyz");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[0].value, "xyz");
}

#[test]
fn test_tokenize_let_keyword() {
    let tokens = lexer::tokenize("let");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[0].value, "let");
}

#[test]
fn test_tokenize_dis_keyword() {
    let tokens = lexer::tokenize("dis");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_DIS);
    assert_eq!(tokens[0].value, "dis");
}

#[test]
fn test_tokenize_operators() {
    let tokens = lexer::tokenize("+ - = ;");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_PLUS);
    assert_eq!(tokens[0].value, "+");
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_MINUS);
    assert_eq!(tokens[1].value, "-");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[2].value, "=");
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[3].value, ";");
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_let_statement() {
    let tokens = lexer::tokenize("let x = 10;");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[1].value, "x");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[3].value, "10");
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[5].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_dis_expression() {
    let tokens = lexer::tokenize("dis x + y;");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_DIS);
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[1].value, "x");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_PLUS);
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[3].value, "y");
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[5].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_whitespace_handling() {
    let tokens = lexer::tokenize("  let   x  =  10 ;  ");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[5].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_empty_string() {
    let tokens = lexer::tokenize("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_multi_digit_number() {
    let tokens = lexer::tokenize("12345");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[0].value, "12345");
}

#[test]
fn test_tokenize_no_spaces() {
    let tokens = lexer::tokenize("let x=10;");
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[1].value, "x");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[3].value, "10");
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[5].token_type, TokenType::TOKEN_EOF);
}

fn main() {}
