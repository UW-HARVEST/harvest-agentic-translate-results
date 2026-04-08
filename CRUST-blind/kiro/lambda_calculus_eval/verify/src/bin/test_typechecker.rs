use lambda_calculus_eval::common::*;
use lambda_calculus_eval::typechecker::*;
use lambda_calculus_eval::reducer::deepcopy;

#[test]
fn test_type_equal_same() {
    let var1 = AstNode::default();
    let var2 = AstNode::default();
    let a = create_type("Nat", "", &var1);
    let b = create_type("Nat", "", &var2);
    assert!(type_equal(&a, &b));
}

#[test]
fn test_type_equal_different() {
    let var1 = AstNode::default();
    let var2 = AstNode::default();
    let a = create_type("Nat", "", &var1);
    let b = create_type("Bool", "", &var2);
    assert!(!type_equal(&a, &b));
}

#[test]
fn test_type_equal_with_return_types_same() {
    let var1 = AstNode::default();
    let var2 = AstNode::default();
    let a = create_type("Nat", "Bool", &var1);
    let b = create_type("Nat", "Bool", &var2);
    assert!(type_equal(&a, &b));
}

#[test]
fn test_type_equal_with_return_types_different() {
    let var1 = AstNode::default();
    let var2 = AstNode::default();
    let a = create_type("Nat", "Bool", &var1);
    let b = create_type("Nat", "Nat", &var2);
    assert!(!type_equal(&a, &b));
}

#[test]
fn test_type_equal_one_has_return_type() {
    let var1 = AstNode::default();
    let var2 = AstNode::default();
    let a = create_type("Nat", "Bool", &var1);
    let b = create_type("Nat", "", &var2);
    assert!(!type_equal(&a, &b));
}

#[test]
fn test_get_type_from_expr_variable() {
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    assert_eq!(get_type_from_expr(&var), "Nat");
}

#[test]
fn test_get_type_from_expr_lambda() {
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
            type_: "Bool".to_string(),
            body: Some(Box::new(body)),
        }),
    };
    assert_eq!(get_type_from_expr(&lambda), "Bool");
}

#[test]
fn test_get_type_from_expr_application() {
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
    assert_eq!(get_type_from_expr(&app), "");
}

#[test]
fn test_create_type_fields() {
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let t = create_type("Nat", "Bool", &var);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "Bool");
    assert_eq!(ast_to_string(&t.expr), "(x : Nat) ");
}

#[test]
fn test_parse_function_type() {
    let t = parse_function_type("Nat");
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_expr_type_equal_same_expr() {
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "test".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let t = create_type("Nat", "", &var);
    assert!(expr_type_equal(&t, &var));
}

#[test]
fn test_typecheck_application() {
    // (@x:Nat.x) (y:Nat) should typecheck without panic
    let lambda_body = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    let y_var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "y".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let lambda_expr = AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: "x".to_string(),
            type_: "Nat".to_string(),
            body: Some(Box::new(lambda_body)),
        }),
    };
    let app = AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(lambda_expr)),
            argument: Some(Box::new(y_var)),
        }),
    };
    let result = typecheck(&app, None);
    assert_eq!(result.type_, "Nat");
}

#[test]
fn test_add_to_env_and_lookup() {
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let t = create_type("Nat", "", &var);
    let mut env: Option<Box<TypeEnv>> = None;
    add_to_env(&mut env, t);
    let found = lookup_type(env.as_ref().unwrap(), &var);
    assert_eq!(found.type_, "Nat");
    assert_eq!(found.return_type, "");
}

fn main() {}
