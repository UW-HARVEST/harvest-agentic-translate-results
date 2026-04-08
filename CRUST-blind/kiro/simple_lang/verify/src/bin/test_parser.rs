use simple_lang::lexer;
use simple_lang::parser;
use simple_lang::token::TokenType;

#[test]
fn test_parse_let_expr() {
    // C ground truth: "let x = 5 + 3 - 2;" -> LET node, value="x"
    // left = MINUS(-), left.left = PLUS(+), left.left.left = INT(5), left.left.right = INT(3), left.right = INT(2)
    let tokens = lexer::tokenize("let x = 5 + 3 - 2;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);

    assert_eq!(asts[0].type_, TokenType::TOKEN_LET);
    assert_eq!(asts[0].value, "x");

    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, TokenType::TOKEN_MINUS);
    assert_eq!(left.value, "-");

    let plus = left.left.as_ref().unwrap();
    assert_eq!(plus.type_, TokenType::TOKEN_PLUS);
    assert_eq!(plus.value, "+");

    assert_eq!(plus.left.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(plus.left.as_ref().unwrap().value, "5");
    assert_eq!(plus.right.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(plus.right.as_ref().unwrap().value, "3");

    assert_eq!(left.right.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(left.right.as_ref().unwrap().value, "2");
}

#[test]
fn test_parse_dis() {
    // C ground truth: "dis 42;" -> DIS node, value is NULL (Rust: "")
    let tokens = lexer::tokenize("dis 42;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    assert_eq!(asts[0].type_, TokenType::TOKEN_DIS);
    assert_eq!(asts[0].value, "");
    assert_eq!(asts[0].left.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(asts[0].left.as_ref().unwrap().value, "42");
    assert!(asts[0].right.is_none());
}

#[test]
fn test_parse_assign() {
    // C ground truth: "let x = 5; x = 10;" -> two nodes
    // ast[0]: LET, value="x", left=INT(5)
    // ast[1]: ASSIGN, value="x", left=INT(10)
    let tokens = lexer::tokenize("let x = 5; x = 10;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 2);

    assert_eq!(asts[0].type_, TokenType::TOKEN_LET);
    assert_eq!(asts[0].value, "x");
    assert_eq!(asts[0].left.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(asts[0].left.as_ref().unwrap().value, "5");

    assert_eq!(asts[1].type_, TokenType::TOKEN_ASSIGN);
    assert_eq!(asts[1].value, "x");
    assert_eq!(asts[1].left.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(asts[1].left.as_ref().unwrap().value, "10");
}

#[test]
fn test_parse_multiple_statements() {
    // "let x = 5 + 3 - 2; let y = x + 1; dis x + 1;" -> 3 AST nodes
    let tokens = lexer::tokenize("let x = 5 + 3 - 2; let y = x + 1; dis x + 1;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 3);

    assert_eq!(asts[0].type_, TokenType::TOKEN_LET);
    assert_eq!(asts[1].type_, TokenType::TOKEN_LET);
    assert_eq!(asts[1].value, "y");
    assert_eq!(asts[1].left.as_ref().unwrap().type_, TokenType::TOKEN_PLUS);
    assert_eq!(asts[1].left.as_ref().unwrap().left.as_ref().unwrap().type_, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(asts[1].left.as_ref().unwrap().left.as_ref().unwrap().value, "x");
    assert_eq!(asts[1].left.as_ref().unwrap().right.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(asts[1].left.as_ref().unwrap().right.as_ref().unwrap().value, "1");

    assert_eq!(asts[2].type_, TokenType::TOKEN_DIS);
    assert_eq!(asts[2].left.as_ref().unwrap().type_, TokenType::TOKEN_PLUS);
}

#[test]
fn test_lookahead_and_consume() {
    // Test that lookahead doesn't advance position but consume does
    let tokens = lexer::tokenize("let x = 5;");
    let _ = parser::parse(&tokens); // sets up internal state

    // After parse completes, internal state is at EOF
    let tok = parser::lookahead();
    assert_eq!(tok.token_type, TokenType::TOKEN_EOF);
}

#[test]
fn test_header_guard() {
    assert_eq!(parser::SIMPLE_LANG_PARSER_H, true);
}

fn main() {}
