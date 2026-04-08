use simple_lang::ast;
use simple_lang::token::TokenType;

#[test]
fn test_new_ast_node() {
    let node = ast::new_ast_node(TokenType::TOKEN_INT, "5");
    assert_eq!(node.type_, TokenType::TOKEN_INT);
    assert_eq!(node.value, "5");
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn test_new_ast_node_empty_value() {
    // C uses NULL for DIS node value; Rust uses ""
    let node = ast::new_ast_node(TokenType::TOKEN_DIS, "");
    assert_eq!(node.type_, TokenType::TOKEN_DIS);
    assert_eq!(node.value, "");
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn test_ast_node_with_children() {
    let mut parent = ast::new_ast_node(TokenType::TOKEN_PLUS, "+");
    let left = ast::new_ast_node(TokenType::TOKEN_INT, "3");
    let right = ast::new_ast_node(TokenType::TOKEN_INT, "4");
    parent.left = Some(left);
    parent.right = Some(right);

    assert_eq!(parent.type_, TokenType::TOKEN_PLUS);
    assert_eq!(parent.left.as_ref().unwrap().value, "3");
    assert_eq!(parent.left.as_ref().unwrap().type_, TokenType::TOKEN_INT);
    assert_eq!(parent.right.as_ref().unwrap().value, "4");
    assert_eq!(parent.right.as_ref().unwrap().type_, TokenType::TOKEN_INT);
}

#[test]
fn test_free_ast_node_no_panic() {
    let mut node = ast::new_ast_node(TokenType::TOKEN_LET, "x");
    node.left = Some(ast::new_ast_node(TokenType::TOKEN_INT, "5"));
    ast::free_ast_node(node); // should not panic
}

#[test]
fn test_header_guard() {
    assert_eq!(ast::SIMPLE_LANG_AST_H, true);
}

fn main() {}
