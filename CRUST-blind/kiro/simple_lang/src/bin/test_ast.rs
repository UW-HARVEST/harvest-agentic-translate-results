use simple_lang::ast;
use simple_lang::token::TokenType;

#[test]
fn test_new_ast_node() {
    let node = ast::new_ast_node(TokenType::TOKEN_INT, "42");
    assert_eq!(node.type_, TokenType::TOKEN_INT);
    assert_eq!(node.value, "42");
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn test_new_ast_node_empty_value() {
    let node = ast::new_ast_node(TokenType::TOKEN_DIS, "");
    assert_eq!(node.type_, TokenType::TOKEN_DIS);
    assert_eq!(node.value, "");
}

#[test]
fn test_ast_node_with_children() {
    let left = ast::new_ast_node(TokenType::TOKEN_INT, "1");
    let right = ast::new_ast_node(TokenType::TOKEN_INT, "2");
    let mut parent = ast::new_ast_node(TokenType::TOKEN_PLUS, "+");
    parent.left = Some(left);
    parent.right = Some(right);
    assert!(parent.left.is_some());
    assert!(parent.right.is_some());
    assert_eq!(parent.left.as_ref().unwrap().value, "1");
    assert_eq!(parent.right.as_ref().unwrap().value, "2");
}

#[test]
fn test_free_ast_node_no_panic() {
    let mut node = ast::new_ast_node(TokenType::TOKEN_PLUS, "+");
    node.left = Some(ast::new_ast_node(TokenType::TOKEN_INT, "1"));
    node.right = Some(ast::new_ast_node(TokenType::TOKEN_INT, "2"));
    ast::free_ast_node(node);
}

fn main() {}
