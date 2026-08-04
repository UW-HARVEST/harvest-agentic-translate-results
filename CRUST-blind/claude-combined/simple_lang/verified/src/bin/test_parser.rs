use simple_lang::lexer;
use simple_lang::parser;
use simple_lang::token;

#[test]
fn test_parse_full_example() {
    let source = "let x = 5 + 3 - 2; let y = x + 1; dis x + 1;";
    let tokens = lexer::tokenize(source);
    let ast_nodes = parser::parse(&tokens);

    assert_eq!(ast_nodes.len(), 3);

    // First statement: let x = 5 + 3 - 2;
    let n0 = &ast_nodes[0];
    assert_eq!(n0.type_, token::TokenType::TOKEN_LET);
    assert_eq!(n0.value, "x");
    let n0_left = n0.left.as_ref().expect("left exists");
    assert_eq!(n0_left.type_, token::TokenType::TOKEN_MINUS);
    let n0_ll = n0_left.left.as_ref().expect("ll exists");
    assert_eq!(n0_ll.type_, token::TokenType::TOKEN_PLUS);
    let n0_lll = n0_ll.left.as_ref().expect("lll exists");
    assert_eq!(n0_lll.type_, token::TokenType::TOKEN_INT);
    assert_eq!(n0_lll.value, "5");
    let n0_llr = n0_ll.right.as_ref().expect("llr exists");
    assert_eq!(n0_llr.type_, token::TokenType::TOKEN_INT);
    assert_eq!(n0_llr.value, "3");
    let n0_lr = n0_left.right.as_ref().expect("lr exists");
    assert_eq!(n0_lr.type_, token::TokenType::TOKEN_INT);
    assert_eq!(n0_lr.value, "2");

    // Second statement: let y = x + 1;
    let n1 = &ast_nodes[1];
    assert_eq!(n1.type_, token::TokenType::TOKEN_LET);
    assert_eq!(n1.value, "y");
    let n1_left = n1.left.as_ref().expect("left exists");
    assert_eq!(n1_left.type_, token::TokenType::TOKEN_PLUS);
    let n1_ll = n1_left.left.as_ref().expect("ll");
    assert_eq!(n1_ll.type_, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(n1_ll.value, "x");
    let n1_lr = n1_left.right.as_ref().expect("lr");
    assert_eq!(n1_lr.type_, token::TokenType::TOKEN_INT);
    assert_eq!(n1_lr.value, "1");

    // Third statement: dis x + 1;
    let n2 = &ast_nodes[2];
    assert_eq!(n2.type_, token::TokenType::TOKEN_DIS);
    let n2_left = n2.left.as_ref().expect("left exists");
    assert_eq!(n2_left.type_, token::TokenType::TOKEN_PLUS);
    let n2_ll = n2_left.left.as_ref().expect("ll");
    assert_eq!(n2_ll.type_, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(n2_ll.value, "x");
    let n2_lr = n2_left.right.as_ref().expect("lr");
    assert_eq!(n2_lr.type_, token::TokenType::TOKEN_INT);
    assert_eq!(n2_lr.value, "1");
}

#[test]
fn test_parse_simple_let() {
    let source = "let foo = 7;";
    let tokens = lexer::tokenize(source);
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    assert_eq!(asts[0].type_, token::TokenType::TOKEN_LET);
    assert_eq!(asts[0].value, "foo");
    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, token::TokenType::TOKEN_INT);
    assert_eq!(left.value, "7");
}

#[test]
fn test_parse_assign_statement() {
    let source = "x = 9;";
    let tokens = lexer::tokenize(source);
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    assert_eq!(asts[0].type_, token::TokenType::TOKEN_ASSIGN);
    assert_eq!(asts[0].value, "x");
    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, token::TokenType::TOKEN_INT);
    assert_eq!(left.value, "9");
}

#[test]
fn test_parse_dis_statement() {
    let source = "dis 5;";
    let tokens = lexer::tokenize(source);
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 1);
    assert_eq!(asts[0].type_, token::TokenType::TOKEN_DIS);
    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, token::TokenType::TOKEN_INT);
    assert_eq!(left.value, "5");
}

#[test]
fn test_consume_lookahead_after_parse() {
    // After parse() returns, position should be at the EOF token.
    let source = "let a = 1;";
    let tokens = lexer::tokenize(source);
    let _ = parser::parse(&tokens);
    let look = parser::lookahead();
    assert_eq!(look.token_type, token::TokenType::TOKEN_EOF);
    let consumed = parser::consume();
    assert_eq!(consumed.token_type, token::TokenType::TOKEN_EOF);
}

#[test]
fn test_parse_primary_int() {
    // Set up parser state with a single int token + EOF.
    let tokens = vec![
        token::Token { token_type: token::TokenType::TOKEN_INT, value: "7".to_string() },
        token::Token { token_type: token::TokenType::TOKEN_EOF, value: "".to_string() },
    ];
    // parse() initializes the global state and consumes... but it would error
    // because TOKEN_INT is not a valid statement starter. Instead we test
    // parse_primary by setting state via parse() with a valid statement first
    // that resets pos=0 and contains the INT we want at position 0.
    // Since parse() would consume the int as a statement starter, we use a
    // workaround: tokenize a primary-friendly program and use parse_expression
    // by calling it after manually positioning state. For this we use parse()
    // on a let, then inspect children. parse_primary is exercised indirectly
    // through parse(...) for "let a = 7;".
    let _ = tokens;
    let source = "let a = 7;";
    let toks = lexer::tokenize(source);
    let asts = parser::parse(&toks);
    // The expression is a primary INT 7
    let left = asts[0].left.as_ref().unwrap();
    assert_eq!(left.type_, token::TokenType::TOKEN_INT);
    assert_eq!(left.value, "7");
}

#[test]
fn test_parse_expression_binary_chain_left_associative() {
    // 5 + 3 - 2 should produce ((5+3)-2): outer is MINUS.
    let source = "let q = 5 + 3 - 2;";
    let tokens = lexer::tokenize(source);
    let asts = parser::parse(&tokens);
    let outer = asts[0].left.as_ref().unwrap();
    assert_eq!(outer.type_, token::TokenType::TOKEN_MINUS);
    let inner = outer.left.as_ref().unwrap();
    assert_eq!(inner.type_, token::TokenType::TOKEN_PLUS);
}

#[test]
fn test_parse_empty_returns_empty_vec() {
    let tokens = lexer::tokenize("");
    let asts = parser::parse(&tokens);
    assert_eq!(asts.len(), 0);
}

fn main() {}
