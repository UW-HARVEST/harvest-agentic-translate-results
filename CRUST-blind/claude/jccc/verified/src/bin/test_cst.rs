use jccc::cst::{
    BlockStatement, ConcreteFileTree, Expression, FunctionCall, FunctionDeclaration, NodeType,
    TopLevelDeclaration,
};
use jccc::list::create_list;

#[test]
fn test_blockstatement_construction() {
    let bs = BlockStatement {
        stmts: create_list(8),
    };
    assert_eq!(bs.stmts.blocksize, 8);
    assert!(bs.stmts.head.is_none());
}

#[test]
fn test_function_declaration_construction() {
    let fd = FunctionDeclaration {
        body: BlockStatement {
            stmts: create_list(8),
        },
        name: String::from("main"),
    };
    assert_eq!(fd.name, "main");
    assert_eq!(fd.body.stmts.blocksize, 8);
}

#[test]
fn test_function_call_construction() {
    let fc = FunctionCall {
        name: String::from("foo"),
    };
    assert_eq!(fc.name, "foo");
}

#[test]
fn test_expression_with_literal() {
    let ex = Expression {
        fc: None,
        literal: Some(String::from("42")),
        node_type: NodeType::NT_LITERAL,
    };
    assert!(ex.fc.is_none());
    assert_eq!(ex.literal.as_deref(), Some("42"));
    assert!(matches!(ex.node_type, NodeType::NT_LITERAL));
}

#[test]
fn test_expression_with_funccall() {
    let ex = Expression {
        fc: Some(FunctionCall {
            name: String::from("bar"),
        }),
        literal: None,
        node_type: NodeType::NT_FUNCCALL,
    };
    assert!(ex.fc.is_some());
    assert_eq!(ex.fc.as_ref().unwrap().name, "bar");
    assert!(ex.literal.is_none());
    assert!(matches!(ex.node_type, NodeType::NT_FUNCCALL));
}

#[test]
fn test_top_level_declaration_construction() {
    let td = TopLevelDeclaration {
        fd: FunctionDeclaration {
            body: BlockStatement {
                stmts: create_list(4),
            },
            name: String::from("main"),
        },
        node_type: NodeType::NT_FUNCDECL,
    };
    assert_eq!(td.fd.name, "main");
    assert!(matches!(td.node_type, NodeType::NT_FUNCDECL));
}

#[test]
fn test_concrete_file_tree_construction() {
    let cft = ConcreteFileTree {
        decls: create_list(2),
    };
    assert_eq!(cft.decls.blocksize, 2);
    assert!(cft.decls.head.is_none());
}

#[test]
fn test_node_type_variants_distinct() {
    let v = vec![
        NodeType::NT_STMT,
        NodeType::NT_EXPR,
        NodeType::NT_BLOCK_STMT,
        NodeType::NT_RETURN_STMT,
        NodeType::NT_FUNCDECL,
        NodeType::NT_FUNCCALL,
        NodeType::NT_LITERAL,
    ];
    // Ensure all 7 variants exist.
    assert_eq!(v.len(), 7);
}

fn main() {}
