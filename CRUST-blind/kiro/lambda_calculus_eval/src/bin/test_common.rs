use lambda_calculus_eval::common;

#[test]
fn test_ast_to_string_var_with_type() {
    let node = common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    assert_eq!(common::ast_to_string(&node), "(x : Nat) ");
}

#[test]
fn test_ast_to_string_var_no_type() {
    let node = common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "y".to_string(),
            type_: String::new(),
        }),
    };
    assert_eq!(common::ast_to_string(&node), "(y) ");
}

#[test]
fn test_ast_to_string_lambda() {
    let body = common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "y".to_string(),
            type_: String::new(),
        }),
    };
    let node = common::AstNode {
        type_: common::AstNodeType::LAMBDA_EXPR,
        node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
            parameter: "x".to_string(),
            type_: "Nat".to_string(),
            body: Some(Box::new(body)),
        }),
    };
    assert_eq!(common::ast_to_string(&node), "(@x : Nat .(y) ) ");
}

#[test]
fn test_ast_to_string_application() {
    let var_y = common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "y".to_string(),
            type_: String::new(),
        }),
    };
    let lambda = common::AstNode {
        type_: common::AstNodeType::LAMBDA_EXPR,
        node: common::AstNodeUnion::LambdaExpr(common::LambdaExpression {
            parameter: "x".to_string(),
            type_: "Nat".to_string(),
            body: Some(Box::new(var_y)),
        }),
    };
    let var_x = common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let app = common::AstNode {
        type_: common::AstNodeType::APPLICATION,
        node: common::AstNodeUnion::Application(common::Application {
            function: Some(Box::new(lambda)),
            argument: Some(Box::new(var_x)),
        }),
    };
    assert_eq!(common::ast_to_string(&app), "((@x : Nat .(y) ) (x : Nat) ) ");
}

#[test]
fn test_ast_to_string_definition() {
    let node = common::AstNode {
        type_: common::AstNodeType::DEFINITION,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "mydef".to_string(),
            type_: String::new(),
        }),
    };
    assert_eq!(common::ast_to_string(&node), "(mydef) ");
}

#[test]
fn test_append_to_buffer() {
    let mut buf = String::new();
    common::append_to_buffer(&mut buf, "hello");
    common::append_to_buffer(&mut buf, " world");
    assert_eq!(buf, "hello world");
}

#[test]
fn test_set_verbose() {
    common::set_verbose(false);
    // Should not panic
    let node = common::AstNode::default();
    common::print_ast_verbose(&node);
}

fn main() {}
