use simple_lang::lexer::{free_token, new_token, tokenize};
use simple_lang::token::TokenType;

#[test]
fn test_tokenize_full_program() {
    let src = "let x = 5 + 3 - 2; dis x;";
    let tokens = tokenize(src);
    assert_eq!(tokens.len(), 13);
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
    assert_eq!(tokens[12].value, "");
}

#[test]
fn test_tokenize_just_dis() {
    let tokens = tokenize("dis 42;");
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_DIS);
    assert_eq!(tokens[0].value, "dis");
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[1].value, "42");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[2].value, ";");
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_EOF);
    assert_eq!(tokens[3].value, "");
}

#[test]
fn test_tokenize_multi_let() {
    let tokens = tokenize("let x = 5; let y = 10; let z = x + y;");
    assert_eq!(tokens.len(), 18);
    // First statement
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[1].value, "x");
    assert_eq!(tokens[2].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[3].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[3].value, "5");
    assert_eq!(tokens[4].token_type, TokenType::TOKEN_SEMICOLON);
    // Second statement
    assert_eq!(tokens[5].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[6].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[6].value, "y");
    assert_eq!(tokens[7].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[8].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[8].value, "10");
    assert_eq!(tokens[9].token_type, TokenType::TOKEN_SEMICOLON);
    // Third statement
    assert_eq!(tokens[10].token_type, TokenType::TOKEN_LET);
    assert_eq!(tokens[11].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[11].value, "z");
    assert_eq!(tokens[12].token_type, TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[13].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[13].value, "x");
    assert_eq!(tokens[14].token_type, TokenType::TOKEN_PLUS);
    assert_eq!(tokens[15].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[15].value, "y");
    assert_eq!(tokens[16].token_type, TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[17].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_only_eof() {
    let tokens = tokenize("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_EOF);
    assert_eq!(tokens[0].value, "");
}

#[test]
fn test_tokenize_only_whitespace() {
    let tokens = tokenize("   \t\n   ");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_EOF);
    assert_eq!(tokens[0].value, "");
}

#[test]
fn test_tokenize_multidigit() {
    let tokens = tokenize("12345");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[0].value, "12345");
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_long_identifier() {
    let tokens = tokenize("variable");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[0].value, "variable");
    assert_eq!(tokens[1].token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_keywords_as_keyword_only_when_exact() {
    let tokens = tokenize("letter");
    // "letter" is alphabetic only and not "let" exactly => identifier
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[0].value, "letter");
}

#[test]
fn test_tokenize_each_punct() {
    let tokens = tokenize("+ - = ;");
    assert_eq!(tokens.len(), 5);
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
fn test_new_token_via_lexer() {
    let t = new_token(TokenType::TOKEN_INT, "7");
    assert_eq!(t.token_type, TokenType::TOKEN_INT);
    assert_eq!(t.value, "7");
}

#[test]
fn test_free_token_via_lexer_no_panic() {
    let t = new_token(TokenType::TOKEN_INT, "1");
    free_token(t);
}

#[test]
fn test_tokenize_zero() {
    let tokens = tokenize("0");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, TokenType::TOKEN_INT);
    assert_eq!(tokens[0].value, "0");
}

fn main() {}
