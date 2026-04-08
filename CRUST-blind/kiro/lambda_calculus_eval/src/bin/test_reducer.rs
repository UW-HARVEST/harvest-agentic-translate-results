use lambda_calculus_eval::{common, reducer, parser, hash_table};

#[test]
fn test_replace_variable() {
    let mut var = parser::create_variable("test", "");
    reducer::replace(&mut var, "test", "replaced");
    if let common::AstNodeUnion::Variable(ref v) = var.node {
        assert_eq!(v.name, "replaced");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_replace_lambda() {
    let body = parser::create_variable("test", "");
    let mut lambda = parser::create_lambda("test", &body, "Nat");
    reducer::replace(&mut lambda, "test", "replaced");
    if let common::AstNodeUnion::LambdaExpr(ref le) = lambda.node {
        assert_eq!(le.parameter, "replaced");
        if let Some(ref b) = le.body {
            if let common::AstNodeUnion::Variable(ref v) = b.node {
                assert_eq!(v.name, "replaced");
            } else {
                panic!("Expected Variable in body");
            }
        }
    } else {
        panic!("Expected LambdaExpr");
    }
}

#[test]
fn test_replace_application() {
    let f = parser::create_variable("test", "");
    let a = parser::create_variable("other", "");
    let mut app = parser::create_application(&f, &a);
    reducer::replace(&mut app, "test", "replaced");
    if let common::AstNodeUnion::Application(ref ap) = app.node {
        if let common::AstNodeUnion::Variable(ref v) = ap.function.as_ref().unwrap().node {
            assert_eq!(v.name, "replaced");
        }
        if let common::AstNodeUnion::Variable(ref v) = ap.argument.as_ref().unwrap().node {
            assert_eq!(v.name, "other");
        }
    }
}

#[test]
fn test_reduce_ast_beta_reduction() {
    // (@x.x) y => y
    let mut table = hash_table::HashTable::new();
    let lambda_body = parser::create_variable("x", "");
    let y_var = parser::create_variable("y", "Nat");
    let lambda_expr = parser::create_lambda("x", &lambda_body, "Nat");
    let app = parser::create_application(&lambda_expr, &y_var);
    let reduced = reducer::reduce_ast(&mut table, &app);
    assert_eq!(reduced.type_, common::AstNodeType::VAR);
    if let common::AstNodeUnion::Variable(ref v) = reduced.node {
        assert_eq!(v.name, "y");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_reduce_ast_variable_unchanged() {
    let mut table = hash_table::HashTable::new();
    let var = parser::create_variable("x", "Nat");
    let reduced = reducer::reduce_ast(&mut table, &var);
    assert_eq!(reduced.type_, common::AstNodeType::VAR);
    if let common::AstNodeUnion::Variable(ref v) = reduced.node {
        assert_eq!(v.name, "x");
    }
}

#[test]
fn test_substitute_var_match() {
    let expr = parser::create_variable("x", "");
    let replacement = parser::create_variable("y", "Nat");
    let result = reducer::substitute(&expr, "x", &replacement);
    if let common::AstNodeUnion::Variable(ref v) = result.node {
        assert_eq!(v.name, "y");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_substitute_var_no_match() {
    let expr = parser::create_variable("z", "Bool");
    let replacement = parser::create_variable("y", "Nat");
    let result = reducer::substitute(&expr, "x", &replacement);
    if let common::AstNodeUnion::Variable(ref v) = result.node {
        assert_eq!(v.name, "z");
        assert_eq!(v.type_, "Bool");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_substitute_application() {
    let f = parser::create_variable("x", "");
    let a = parser::create_variable("z", "Bool");
    let app = parser::create_application(&f, &a);
    let replacement = parser::create_variable("y", "Nat");
    let result = reducer::substitute(&app, "x", &replacement);
    if let common::AstNodeUnion::Application(ref ap) = result.node {
        if let common::AstNodeUnion::Variable(ref v) = ap.function.as_ref().unwrap().node {
            assert_eq!(v.name, "y");
        }
        if let common::AstNodeUnion::Variable(ref v) = ap.argument.as_ref().unwrap().node {
            assert_eq!(v.name, "z");
        }
    }
}

#[test]
fn test_deepcopy_var() {
    let var = parser::create_variable("x", "Nat");
    let copy = reducer::deepcopy(&var);
    assert_eq!(copy.type_, common::AstNodeType::VAR);
    if let common::AstNodeUnion::Variable(ref v) = copy.node {
        assert_eq!(v.name, "x");
        assert_eq!(v.type_, "Nat");
    }
}

#[test]
fn test_deepcopy_lambda() {
    let body = parser::create_variable("x", "");
    let lambda = parser::create_lambda("x", &body, "Nat");
    let copy = reducer::deepcopy(&lambda);
    assert_eq!(copy.type_, common::AstNodeType::LAMBDA_EXPR);
    if let common::AstNodeUnion::LambdaExpr(ref le) = copy.node {
        assert_eq!(le.parameter, "x");
        assert_eq!(le.type_, "Nat");
        assert!(le.body.is_some());
        assert_eq!(le.body.as_ref().unwrap().type_, common::AstNodeType::VAR);
    }
}

#[test]
fn test_deepcopy_application() {
    let f = parser::create_variable("f", "Nat");
    let a = parser::create_variable("a", "Bool");
    let app = parser::create_application(&f, &a);
    let copy = reducer::deepcopy(&app);
    assert_eq!(copy.type_, common::AstNodeType::APPLICATION);
    if let common::AstNodeUnion::Application(ref ap) = copy.node {
        if let common::AstNodeUnion::Variable(ref v) = ap.function.as_ref().unwrap().node {
            assert_eq!(v.name, "f");
        }
        if let common::AstNodeUnion::Variable(ref v) = ap.argument.as_ref().unwrap().node {
            assert_eq!(v.name, "a");
        }
    }
}

#[test]
fn test_expand_definitions() {
    let mut table = hash_table::HashTable::new();
    let definition = parser::create_variable("testing", "Nat");
    table.insert("test", definition);

    let mut node = common::AstNode {
        type_: common::AstNodeType::DEFINITION,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "test".to_string(),
            type_: String::new(),
        }),
    };

    // expand_definitions takes &AstNode, but the actual expansion happens in reduce()
    // which uses expand_definitions_mut internally. Let's test via reduce.
    let reduced = reducer::reduce(&mut table, &node);
    assert_eq!(reduced.type_, common::AstNodeType::VAR);
    if let common::AstNodeUnion::Variable(ref v) = reduced.node {
        assert_eq!(v.name, "testing");
        assert_eq!(v.type_, "Nat");
    }
}

fn main() {}
