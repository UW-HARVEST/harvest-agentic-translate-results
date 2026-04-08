use jccc::cst::*;
use jccc::list::create_list;
use jccc::parse::parse_simple_main_func;

#[test]
fn test_node_type_variants() {
    // Just verify the enum variants exist and can be constructed
    let _ = NodeType::NT_STMT;
    let _ = NodeType::NT_EXPR;
    let _ = NodeType::NT_BLOCK_STMT;
    let _ = NodeType::NT_RETURN_STMT;
    let _ = NodeType::NT_FUNCDECL;
    let _ = NodeType::NT_FUNCCALL;
    let _ = NodeType::NT_LITERAL;
}

#[test]
fn test_function_declaration_struct() {
    let fd = FunctionDeclaration {
        body: BlockStatement { stmts: create_list(1) },
        name: "main".to_string(),
    };
    assert_eq!(fd.name, "main");
}

#[test]
fn test_expression_with_literal() {
    let ex = Expression {
        fc: None,
        literal: Some("42".to_string()),
        node_type: NodeType::NT_LITERAL,
    };
    assert_eq!(ex.literal.unwrap(), "42");
}

#[test]
fn test_expression_with_funccall() {
    let ex = Expression {
        fc: Some(FunctionCall { name: "foo".to_string() }),
        literal: None,
        node_type: NodeType::NT_FUNCCALL,
    };
    assert_eq!(ex.fc.unwrap().name, "foo");
}

#[test]
fn test_parse_simple_main_func() {
    // In C this is an empty function body, Rust returns 0
    assert_eq!(parse_simple_main_func(), 0);
}

fn main() {}
