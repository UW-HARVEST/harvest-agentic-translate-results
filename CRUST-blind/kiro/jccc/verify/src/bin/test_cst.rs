use jccc::cst::{
    BlockStatement, ConcreteFileTree, Expression, FunctionCall, FunctionDeclaration, NodeType,
    TopLevelDeclaration,
};
use jccc::list::create_list;

#[test]
fn test_node_type_variants() {
    // Verify all NodeType variants exist and can be constructed
    let _ = NodeType::NT_STMT;
    let _ = NodeType::NT_EXPR;
    let _ = NodeType::NT_BLOCK_STMT;
    let _ = NodeType::NT_RETURN_STMT;
    let _ = NodeType::NT_FUNCDECL;
    let _ = NodeType::NT_FUNCCALL;
    let _ = NodeType::NT_LITERAL;
}

#[test]
fn test_block_statement() {
    let bs = BlockStatement {
        stmts: create_list(10),
    };
    assert_eq!(bs.stmts.blocksize, 10);
}

#[test]
fn test_function_declaration() {
    let fd = FunctionDeclaration {
        body: BlockStatement {
            stmts: create_list(5),
        },
        name: "main".to_string(),
    };
    assert_eq!(fd.name, "main");
    assert_eq!(fd.body.stmts.blocksize, 5);
}

#[test]
fn test_top_level_declaration() {
    let tld = TopLevelDeclaration {
        fd: FunctionDeclaration {
            body: BlockStatement {
                stmts: create_list(8),
            },
            name: "foo".to_string(),
        },
        node_type: NodeType::NT_FUNCDECL,
    };
    assert_eq!(tld.fd.name, "foo");
    match tld.node_type {
        NodeType::NT_FUNCDECL => {}
        _ => panic!("Expected NT_FUNCDECL"),
    }
}

#[test]
fn test_function_call() {
    let fc = FunctionCall {
        name: "printf".to_string(),
    };
    assert_eq!(fc.name, "printf");
}

#[test]
fn test_expression_with_function_call() {
    let expr = Expression {
        fc: Some(FunctionCall {
            name: "puts".to_string(),
        }),
        literal: None,
        node_type: NodeType::NT_FUNCCALL,
    };
    assert_eq!(expr.fc.as_ref().unwrap().name, "puts");
    assert!(expr.literal.is_none());
}

#[test]
fn test_expression_with_literal() {
    let expr = Expression {
        fc: None,
        literal: Some("42".to_string()),
        node_type: NodeType::NT_LITERAL,
    };
    assert!(expr.fc.is_none());
    assert_eq!(expr.literal.as_ref().unwrap(), "42");
}

#[test]
fn test_concrete_file_tree() {
    let cft = ConcreteFileTree {
        decls: create_list(4),
    };
    assert_eq!(cft.decls.blocksize, 4);
}

fn main() {}
