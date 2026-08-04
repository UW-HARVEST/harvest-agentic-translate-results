use lambda_calculus_eval::common::{
    self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable,
};
use lambda_calculus_eval::typechecker::{
    add_to_env, assert_, create_type, expr_type_equal, get_type_from_expr,
    lookup_type, p_print_type, parse_function_type, type_equal, typecheck,
    Type, TypeEnv,
};

fn make_var(name: &str, ty: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: ty.to_string(),
        }),
    }
}

fn make_lambda(param: &str, body: AstNode, ty: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: param.to_string(),
            type_: ty.to_string(),
            body: Some(Box::new(body)),
        }),
    }
}

fn make_app(f: AstNode, a: AstNode) -> AstNode {
    AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(f)),
            argument: Some(Box::new(a)),
        }),
    }
}

#[test]
fn test_type_equal_same() {
    let dummy = make_var("d", "");
    let a = create_type("Nat", "", &dummy);
    let b = create_type("Nat", "", &dummy);
    assert!(type_equal(&a, &b));
}

#[test]
fn test_type_equal_different() {
    let dummy = make_var("d", "");
    let a = create_type("Nat", "", &dummy);
    let c = create_type("Bool", "", &dummy);
    assert!(!type_equal(&a, &c));
}

#[test]
fn test_type_equal_different_return() {
    let dummy = make_var("d", "");
    let a = create_type("Nat", "Nat", &dummy);
    let b = create_type("Nat", "Bool", &dummy);
    assert!(!type_equal(&a, &b));
}

#[test]
fn test_type_equal_both_empty_return() {
    let dummy = make_var("d", "");
    let a = create_type("Nat", "", &dummy);
    let b = create_type("Nat", "", &dummy);
    assert!(type_equal(&a, &b));
}

#[test]
fn test_get_type_from_expr_variable() {
    let v = make_var("test", "Nat");
    let ty = get_type_from_expr(&v);
    assert_eq!(ty, "Nat");
}

#[test]
fn test_get_type_from_expr_lambda() {
    let body = make_var("test", "");
    let lam = make_lambda("test", body, "Bool");
    let ty = get_type_from_expr(&lam);
    assert_eq!(ty, "Bool");
}

#[test]
fn test_get_type_from_expr_application_returns_empty() {
    let f = make_var("f", "Nat");
    let a = make_var("a", "Nat");
    let app = make_app(f, a);
    let ty = get_type_from_expr(&app);
    assert_eq!(ty, "");
}

#[test]
fn test_create_type() {
    let v = make_var("x", "Nat");
    let t = create_type("Nat", "", &v);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
    assert_eq!(common::ast_to_string(&t.expr), common::ast_to_string(&v));
}

#[test]
fn test_parse_function_type() {
    let t = parse_function_type("Nat");
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_expr_type_equal_match() {
    let v = make_var("test", "Nat");
    let t = create_type("Nat", "", &v);
    assert!(expr_type_equal(&t, &v));
}

#[test]
fn test_expr_type_equal_different_expr() {
    // Different expressions: t.expr is "x" but expr is "y".
    let v1 = make_var("x", "Nat");
    let v2 = make_var("y", "Nat");
    let t = create_type("Nat", "", &v1);
    assert!(!expr_type_equal(&t, &v2));
}

#[test]
fn test_typecheck_var() {
    let v = make_var("x", "Nat");
    let t = typecheck(&v, None);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_typecheck_lambda() {
    let body = make_var("x", "");
    let lam = make_lambda("x", body, "Nat");
    let t = typecheck(&lam, None);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_typecheck_application_compatible() {
    // C: (@x:Nat.x)(y:Nat) typechecks to "Nat".
    let body = make_var("x", "");
    let lam = make_lambda("x", body, "Nat");
    let yvar = make_var("y", "Nat");
    let app = make_app(lam, yvar);
    let t = typecheck(&app, None);
    assert_eq!(t.type_, "Nat");
}

#[test]
fn test_assert_does_not_panic_on_true() {
    // Should be a no-op when expr is true.
    assert_(true, "should not error");
}

#[test]
fn test_p_print_type_does_not_panic() {
    let dummy = make_var("d", "");
    let t = create_type("Nat", "Nat", &dummy);
    p_print_type(&t);
    let t2 = create_type("Bool", "", &dummy);
    p_print_type(&t2);
}

#[test]
fn test_add_to_env_and_lookup() {
    let v = make_var("x", "Nat");
    let t = create_type("Nat", "", &v);
    let mut env: Option<Box<TypeEnv>> = None;
    add_to_env(&mut env, t);
    assert!(env.is_some());

    if let Some(e) = env.as_deref() {
        let res = lookup_type(e, &v);
        assert_eq!(res.type_, "Nat");
    }
}

#[test]
fn test_lookup_type_not_found() {
    let v1 = make_var("x", "Nat");
    let v2 = make_var("y", "Bool");
    let t = create_type("Nat", "", &v1);
    let mut env: Option<Box<TypeEnv>> = None;
    add_to_env(&mut env, t);
    if let Some(e) = env.as_deref() {
        // looking up the v2 expr; t.expr's string is for v1, so mismatch
        let res = lookup_type(e, &v2);
        // Returns a "default" Type with empty type_ and return_type
        assert_eq!(res.type_, "");
        assert_eq!(res.return_type, "");
    }
}

#[test]
fn test_type_struct_fields() {
    // Verify the Type struct stores the fields we set.
    let v = make_var("x", "");
    let t = Type {
        type_: "MyType".to_string(),
        return_type: "MyReturn".to_string(),
        expr: v,
    };
    assert_eq!(t.type_, "MyType");
    assert_eq!(t.return_type, "MyReturn");
}

fn main() {}
