use lambda_calculus_eval::common::*;

#[test]
fn test_ast_to_string_variable() {
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    assert_eq!(ast_to_string(&var), "(x : Nat) ");
}

#[test]
fn test_ast_to_string_variable_no_type() {
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    assert_eq!(ast_to_string(&var), "(x) ");
}

#[test]
fn test_ast_to_string_lambda() {
    let body = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    let lambda = AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: "x".to_string(),
            type_: "Nat".to_string(),
            body: Some(Box::new(body)),
        }),
    };
    assert_eq!(ast_to_string(&lambda), "(@x : Nat .(x) ) ");
}

#[test]
fn test_ast_to_string_application() {
    let f = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "f".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let a = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "a".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let app = AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(f)),
            argument: Some(Box::new(a)),
        }),
    };
    assert_eq!(ast_to_string(&app), "((f : Nat) (a : Nat) ) ");
}

#[test]
fn test_ast_to_string_definition() {
    let def = AstNode {
        type_: AstNodeType::DEFINITION,
        node: AstNodeUnion::Variable(Variable {
            name: "mydef".to_string(),
            type_: String::new(),
        }),
    };
    assert_eq!(ast_to_string(&def), "(mydef) ");
}

#[test]
fn test_ast_to_string_nested_lambda() {
    let inner_body = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "y".to_string(),
            type_: String::new(),
        }),
    };
    let inner_lambda = AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: "y".to_string(),
            type_: "Bool".to_string(),
            body: Some(Box::new(inner_body)),
        }),
    };
    let outer_lambda = AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: "x".to_string(),
            type_: "Nat".to_string(),
            body: Some(Box::new(inner_lambda)),
        }),
    };
    assert_eq!(ast_to_string(&outer_lambda), "(@x : Nat .(@y : Bool .(y) ) ) ");
}

#[test]
fn test_append_to_buffer() {
    let mut buf = String::new();
    append_to_buffer(&mut buf, "hello");
    append_to_buffer(&mut buf, " world");
    assert_eq!(buf, "hello world");
}

fn main() {}
