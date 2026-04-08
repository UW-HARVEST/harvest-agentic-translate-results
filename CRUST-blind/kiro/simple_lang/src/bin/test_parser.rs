use simple_lang::lexer;
use simple_lang::parser;
use simple_lang::token::TokenType;

#[test]
fn test_parse_let_statement() {
    let tokens = lexer::tokenize("let x = 10;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    assert_eq!(asts[0].type_, TokenType::TOKEN_LET);
    assert_eq!(asts[0].value, "x");
    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, TokenType::TOKEN_INT);
    assert_eq!(left.value, "10");
}

#[test]
fn test_parse_assign_statement() {
    let tokens = lexer::tokenize("x = 5;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    assert_eq!(asts[0].type_, TokenType::TOKEN_ASSIGN);
    assert_eq!(asts[0].value, "x");
    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, TokenType::TOKEN_INT);
    assert_eq!(left.value, "5");
}

#[test]
fn test_parse_dis_statement() {
    let tokens = lexer::tokenize("dis 42;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    assert_eq!(asts[0].type_, TokenType::TOKEN_DIS);
    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, TokenType::TOKEN_INT);
    assert_eq!(left.value, "42");
}

#[test]
fn test_parse_addition_expression() {
    let tokens = lexer::tokenize("dis x + y;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    let expr = asts[0].left.as_ref().unwrap();
    assert_eq!(expr.type_, TokenType::TOKEN_PLUS);
    assert_eq!(expr.left.as_ref().unwrap().type_, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(expr.left.as_ref().unwrap().value, "x");
    assert_eq!(expr.right.as_ref().unwrap().type_, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(expr.right.as_ref().unwrap().value, "y");
}

#[test]
fn test_parse_subtraction_expression() {
    let tokens = lexer::tokenize("dis a - b;");
    let asts = parser::parse(&tokens);
    let expr = asts[0].left.as_ref().unwrap();
    assert_eq!(expr.type_, TokenType::TOKEN_MINUS);
}

#[test]
fn test_parse_multiple_statements() {
    let tokens = lexer::tokenize("let x = 10; let y = 20; dis x + y;");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 3);
    assert_eq!(asts[0].type_, TokenType::TOKEN_LET);
    assert_eq!(asts[1].type_, TokenType::TOKEN_LET);
    assert_eq!(asts[2].type_, TokenType::TOKEN_DIS);
}

#[test]
fn test_parse_chained_addition() {
    // a + b + c should parse as (a + b) + c (left-associative)
    let tokens = lexer::tokenize("dis a + b + c;");
    let asts = parser::parse(&tokens);
    let expr = asts[0].left.as_ref().unwrap();
    assert_eq!(expr.type_, TokenType::TOKEN_PLUS);
    // right is c
    assert_eq!(expr.right.as_ref().unwrap().value, "c");
    // left is (a + b)
    let left_expr = expr.left.as_ref().unwrap();
    assert_eq!(left_expr.type_, TokenType::TOKEN_PLUS);
    assert_eq!(left_expr.left.as_ref().unwrap().value, "a");
    assert_eq!(left_expr.right.as_ref().unwrap().value, "b");
}

#[test]
fn test_parse_empty_input() {
    let tokens = lexer::tokenize("");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 0);
}

fn main() {}
