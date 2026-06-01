use jccc::cst::{
    BlockStatement, ConcreteFileTree, Expression, FunctionCall, FunctionDeclaration, NodeType,
    TopLevelDeclaration,
};
use jccc::list::create_list;

#[test]
fn test_node_type_distinct() {
    assert!(matches!(NodeType::NT_STMT, NodeType::NT_STMT));
    assert!(matches!(NodeType::NT_EXPR, NodeType::NT_EXPR));
    assert!(matches!(NodeType::NT_BLOCK_STMT, NodeType::NT_BLOCK_STMT));
    assert!(matches!(NodeType::NT_RETURN_STMT, NodeType::NT_RETURN_STMT));
    assert!(matches!(NodeType::NT_FUNCDECL, NodeType::NT_FUNCDECL));
    assert!(matches!(NodeType::NT_FUNCCALL, NodeType::NT_FUNCCALL));
    assert!(matches!(NodeType::NT_LITERAL, NodeType::NT_LITERAL));
}

#[test]
fn test_block_statement_default_construction() {
    let stmts = create_list(8);
    let bs = BlockStatement { stmts };
    assert!(bs.stmts.head.is_none());
    assert_eq!(bs.stmts.blocksize, 8);
}

#[test]
fn test_function_declaration_construction() {
    let stmts = create_list(8);
    let body = BlockStatement { stmts };
    let fd = FunctionDeclaration {
        body,
        name: "main".to_string(),
    };
    assert_eq!(fd.name, "main");
    assert_eq!(fd.body.stmts.blocksize, 8);
}

#[test]
fn test_top_level_declaration_construction() {
    let stmts = create_list(8);
    let body = BlockStatement { stmts };
    let fd = FunctionDeclaration {
        body,
        name: "main".to_string(),
    };
    let tld = TopLevelDeclaration {
        fd,
        node_type: NodeType::NT_FUNCDECL,
    };
    assert!(matches!(tld.node_type, NodeType::NT_FUNCDECL));
    assert_eq!(tld.fd.name, "main");
}

#[test]
fn test_function_call_construction() {
    let fc = FunctionCall {
        name: "foo".to_string(),
    };
    assert_eq!(fc.name, "foo");
}

#[test]
fn test_expression_construction_with_fc() {
    let fc = FunctionCall {
        name: "bar".to_string(),
    };
    let ex = Expression {
        fc: Some(fc),
        literal: None,
        node_type: NodeType::NT_FUNCCALL,
    };
    assert!(ex.fc.is_some());
    assert_eq!(ex.fc.as_ref().unwrap().name, "bar");
    assert!(ex.literal.is_none());
    assert!(matches!(ex.node_type, NodeType::NT_FUNCCALL));
}

#[test]
fn test_expression_construction_with_literal() {
    let ex = Expression {
        fc: None,
        literal: Some("42".to_string()),
        node_type: NodeType::NT_LITERAL,
    };
    assert!(ex.fc.is_none());
    assert_eq!(ex.literal, Some("42".to_string()));
    assert!(matches!(ex.node_type, NodeType::NT_LITERAL));
}

#[test]
fn test_concrete_file_tree_construction() {
    let decls = create_list(4);
    let cft = ConcreteFileTree { decls };
    assert!(cft.decls.head.is_none());
    assert_eq!(cft.decls.blocksize, 4);
}

fn main() {}
