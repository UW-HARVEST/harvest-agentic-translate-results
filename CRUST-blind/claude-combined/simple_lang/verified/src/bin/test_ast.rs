use simple_lang::ast;
use simple_lang::token;

#[test]
fn test_new_ast_node_int() {
    let node = ast::new_ast_node(token::TokenType::TOKEN_INT, "42");
    assert_eq!(node.type_, token::TokenType::TOKEN_INT);
    assert_eq!(node.value, "42");
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn test_new_ast_node_identifier() {
    let node = ast::new_ast_node(token::TokenType::TOKEN_IDENTIFIER, "x");
    assert_eq!(node.type_, token::TokenType::TOKEN_IDENTIFIER);
    assert_eq!(node.value, "x");
}

#[test]
fn test_new_ast_node_plus() {
    let node = ast::new_ast_node(token::TokenType::TOKEN_PLUS, "+");
    assert_eq!(node.type_, token::TokenType::TOKEN_PLUS);
    assert_eq!(node.value, "+");
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn test_ast_can_link_children() {
    let l = ast::new_ast_node(token::TokenType::TOKEN_INT, "1");
    let r = ast::new_ast_node(token::TokenType::TOKEN_INT, "2");
    let mut p = ast::new_ast_node(token::TokenType::TOKEN_PLUS, "+");
    p.left = Some(l);
    p.right = Some(r);
    assert_eq!(p.type_, token::TokenType::TOKEN_PLUS);
    assert_eq!(p.left.as_ref().unwrap().value, "1");
    assert_eq!(p.right.as_ref().unwrap().value, "2");
}

#[test]
fn test_free_ast_node_no_panic() {
    let node = ast::new_ast_node(token::TokenType::TOKEN_INT, "5");
    ast::free_ast_node(node);
}

fn main() {}
