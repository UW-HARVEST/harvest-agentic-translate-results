use simple_lang::lexer;
use simple_lang::token;

#[test]
fn test_tokenize_full_program() {
    let source = "let x = 5 + 3 - 2; dis x;";
    let tokens = lexer::tokenize(source);

    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_LET);
    assert_eq!(tokens[0].value, "let");
    assert_eq!(tokens[1].token_type, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[1].value, "x");
    assert_eq!(tokens[2].token_type, token::TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[2].value, "=");
    assert_eq!(tokens[3].token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tokens[3].value, "5");
    assert_eq!(tokens[4].token_type, token::TokenType::TOKEN_PLUS);
    assert_eq!(tokens[4].value, "+");
    assert_eq!(tokens[5].token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tokens[5].value, "3");
    assert_eq!(tokens[6].token_type, token::TokenType::TOKEN_MINUS);
    assert_eq!(tokens[6].value, "-");
    assert_eq!(tokens[7].token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tokens[7].value, "2");
    assert_eq!(tokens[8].token_type, token::TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[8].value, ";");
    assert_eq!(tokens[9].token_type, token::TokenType::TOKEN_DIS);
    assert_eq!(tokens[9].value, "dis");
    assert_eq!(tokens[10].token_type, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[10].value, "x");
    assert_eq!(tokens[11].token_type, token::TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[11].value, ";");
    assert_eq!(tokens[12].token_type, token::TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_empty() {
    let tokens = lexer::tokenize("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_just_int() {
    let tokens = lexer::tokenize("123");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tokens[0].value, "123");
    assert_eq!(tokens[1].token_type, token::TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_just_identifier() {
    let tokens = lexer::tokenize("foo");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[0].value, "foo");
}

#[test]
fn test_tokenize_whitespace_only() {
    let tokens = lexer::tokenize("    \t\n  ");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_multidigit_int() {
    let tokens = lexer::tokenize("100 200");
    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tokens[0].value, "100");
    assert_eq!(tokens[1].token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tokens[1].value, "200");
    assert_eq!(tokens[2].token_type, token::TokenType::TOKEN_EOF);
}

#[test]
fn test_tokenize_keywords_distinguished() {
    let tokens = lexer::tokenize("let dis foo");
    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_LET);
    assert_eq!(tokens[1].token_type, token::TokenType::TOKEN_DIS);
    assert_eq!(tokens[2].token_type, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(tokens[2].value, "foo");
}

#[test]
fn test_tokenize_punctuation() {
    let tokens = lexer::tokenize("+ - = ;");
    assert_eq!(tokens[0].token_type, token::TokenType::TOKEN_PLUS);
    assert_eq!(tokens[0].value, "+");
    assert_eq!(tokens[1].token_type, token::TokenType::TOKEN_MINUS);
    assert_eq!(tokens[1].value, "-");
    assert_eq!(tokens[2].token_type, token::TokenType::TOKEN_ASSIGN);
    assert_eq!(tokens[2].value, "=");
    assert_eq!(tokens[3].token_type, token::TokenType::TOKEN_SEMICOLON);
    assert_eq!(tokens[3].value, ";");
}

#[test]
fn test_lexer_new_token() {
    let tok = lexer::new_token(token::TokenType::TOKEN_INT, "5");
    assert_eq!(tok.token_type, token::TokenType::TOKEN_INT);
    assert_eq!(tok.value, "5");
}

#[test]
fn test_lexer_free_token() {
    let tok = lexer::new_token(token::TokenType::TOKEN_PLUS, "+");
    lexer::free_token(tok); // no-op, must not panic
}

fn main() {}
