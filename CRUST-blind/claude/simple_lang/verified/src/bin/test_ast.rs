use simple_lang::ast::{free_ast_node, new_ast_node, ASTNode};
use simple_lang::token::TokenType;

#[test]
fn test_new_ast_node_int() {
    let n = new_ast_node(TokenType::TOKEN_INT, "5");
    assert_eq!(n.type_, TokenType::TOKEN_INT);
    assert_eq!(n.value, "5");
    assert!(n.left.is_none());
    assert!(n.right.is_none());
}

#[test]
fn test_new_ast_node_identifier() {
    let n = new_ast_node(TokenType::TOKEN_IDENTIFIER, "x");
    assert_eq!(n.type_, TokenType::TOKEN_IDENTIFIER);
    assert_eq!(n.value, "x");
    assert!(n.left.is_none());
    assert!(n.right.is_none());
}

#[test]
fn test_new_ast_node_with_children() {
    let mut node = new_ast_node(TokenType::TOKEN_PLUS, "+");
    node.left = Some(new_ast_node(TokenType::TOKEN_INT, "1"));
    node.right = Some(new_ast_node(TokenType::TOKEN_INT, "2"));
    assert_eq!(node.type_, TokenType::TOKEN_PLUS);
    assert_eq!(node.value, "+");
    let l = node.left.as_ref().unwrap();
    assert_eq!(l.type_, TokenType::TOKEN_INT);
    assert_eq!(l.value, "1");
    let r = node.right.as_ref().unwrap();
    assert_eq!(r.type_, TokenType::TOKEN_INT);
    assert_eq!(r.value, "2");
}

#[test]
fn test_new_ast_node_let() {
    let n = new_ast_node(TokenType::TOKEN_LET, "x");
    assert_eq!(n.type_, TokenType::TOKEN_LET);
    assert_eq!(n.value, "x");
    assert!(n.left.is_none());
    assert!(n.right.is_none());
}

#[test]
fn test_new_ast_node_assign() {
    let n = new_ast_node(TokenType::TOKEN_ASSIGN, "x");
    assert_eq!(n.type_, TokenType::TOKEN_ASSIGN);
    assert_eq!(n.value, "x");
}

#[test]
fn test_new_ast_node_dis_empty() {
    let n = new_ast_node(TokenType::TOKEN_DIS, "");
    assert_eq!(n.type_, TokenType::TOKEN_DIS);
    assert_eq!(n.value, "");
}

#[test]
fn test_free_ast_node_no_panic() {
    let n = new_ast_node(TokenType::TOKEN_INT, "1");
    free_ast_node(n);
}

#[test]
fn test_ast_clone() {
    let n = new_ast_node(TokenType::TOKEN_INT, "99");
    let cloned: Box<ASTNode> = n.clone();
    assert_eq!(cloned.type_, TokenType::TOKEN_INT);
    assert_eq!(cloned.value, "99");
}

fn main() {}
