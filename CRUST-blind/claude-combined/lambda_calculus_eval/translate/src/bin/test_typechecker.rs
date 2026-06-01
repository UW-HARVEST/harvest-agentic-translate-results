use lambda_calculus_eval::common::{AstNode, AstNodeType, AstNodeUnion};
use lambda_calculus_eval::parser::{create_application, create_lambda, create_variable};
use lambda_calculus_eval::typechecker::{
    add_to_env, assert_, create_type, expr_type_equal, get_type_from_expr, lookup_type,
    p_print_type, parse_function_type, type_equal, typecheck, Type, TypeEnv,
};

fn empty_node() -> AstNode {
    AstNode::default()
}

#[test]
fn test_assert_true_does_nothing() {
    assert_(true, "should not error");
}

#[test]
fn test_create_type() {
    let n = empty_node();
    let t = create_type("Nat", "Bool", &n);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "Bool");
}

#[test]
fn test_create_type_with_empty_return() {
    let n = empty_node();
    let t = create_type("Nat", "", &n);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_type_equal_same() {
    let n = empty_node();
    let a = create_type("Nat", "", &n);
    let b = create_type("Nat", "", &n);
    assert_eq!(type_equal(&a, &b), true);
}

#[test]
fn test_type_equal_different_type() {
    let n = empty_node();
    let a = create_type("Nat", "", &n);
    let c = create_type("Bool", "", &n);
    assert_eq!(type_equal(&a, &c), false);
}

#[test]
fn test_type_equal_with_return_types() {
    let n = empty_node();
    let a = create_type("Nat", "Bool", &n);
    let b = create_type("Nat", "Bool", &n);
    let c = create_type("Nat", "Other", &n);
    assert_eq!(type_equal(&a, &b), true);
    assert_eq!(type_equal(&a, &c), false);
}

#[test]
fn test_type_equal_one_has_return() {
    let n = empty_node();
    let a = create_type("Nat", "Bool", &n);
    let b = create_type("Nat", "", &n);
    assert_eq!(type_equal(&a, &b), false);
}

#[test]
fn test_get_type_from_expr_var() {
    let n = create_variable("test", "Nat");
    let t = get_type_from_expr(&n);
    assert_eq!(t, "Nat");
}

#[test]
fn test_get_type_from_expr_lambda() {
    let body = create_variable("x", "");
    let n = create_lambda("test", &body, "Bool");
    let t = get_type_from_expr(&n);
    assert_eq!(t, "Bool");
}

#[test]
fn test_get_type_from_expr_application_returns_empty() {
    let f = create_variable("f", "Nat");
    let a = create_variable("a", "Nat");
    let app = create_application(&f, &a);
    let t = get_type_from_expr(&app);
    assert_eq!(t, "");
}

#[test]
fn test_p_print_type_smoke() {
    let n = empty_node();
    let t = create_type("Nat", "", &n);
    p_print_type(&t);
    let t2 = create_type("Nat", "Bool", &n);
    p_print_type(&t2);
}

#[test]
fn test_parse_function_type() {
    let t = parse_function_type("Nat");
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_expr_type_equal_basic() {
    // Mirrors C test_expr_type_equal: var with type Nat matches Type Nat
    let n = create_variable("test", "Nat");
    let a = create_type("Nat", "", &n);
    assert_eq!(expr_type_equal(&a, &n), true);
}

#[test]
fn test_typecheck_smoke_for_variable() {
    let v = create_variable("test", "Nat");
    let t = typecheck(&v, None);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_typecheck_smoke_for_lambda() {
    let body = create_variable("x", "");
    let lam = create_lambda("x", &body, "Nat");
    let t = typecheck(&lam, None);
    assert_eq!(t.type_, "Nat");
}

#[test]
fn test_typecheck_application_compatible() {
    // Mirrors C test_typecheck: (@x:Nat. x) y:Nat -> typechecks
    let lambda_body = create_variable("x", "");
    let y_var = create_variable("y", "Nat");
    let lambda_expr = create_lambda("x", &lambda_body, "Nat");
    let parsed = create_application(&lambda_expr, &y_var);
    let t = typecheck(&parsed, None);
    assert_eq!(t.type_, "Nat");
}

#[test]
fn test_add_to_env_grows_environment() {
    let mut env: Option<Box<TypeEnv>> = None;
    let n = empty_node();
    add_to_env(&mut env, create_type("Nat", "", &n));
    assert!(env.is_some());
    add_to_env(&mut env, create_type("Bool", "", &n));
    let cur = env.as_deref().unwrap();
    assert_eq!(cur.type_.type_, "Bool");
    let next = cur.next.as_deref().unwrap();
    assert_eq!(next.type_.type_, "Nat");
}

#[test]
fn test_lookup_type_finds_match() {
    let n = create_variable("test", "Nat");
    let mut env: Option<Box<TypeEnv>> = None;
    let ty = create_type("Nat", "", &n);
    add_to_env(&mut env, ty);
    let env_ref = env.as_deref().unwrap();
    let found = lookup_type(env_ref, &n);
    assert_eq!(found.type_, "Nat");
}

#[test]
fn test_lookup_type_returns_empty_if_not_found() {
    let n1 = create_variable("a", "Nat");
    let n2 = create_variable("b", "Bool");
    let mut env: Option<Box<TypeEnv>> = None;
    let ty = create_type("Nat", "", &n1);
    add_to_env(&mut env, ty);
    let env_ref = env.as_deref().unwrap();
    let found = lookup_type(env_ref, &n2);
    assert_eq!(found.type_, "");
}

#[test]
fn test_type_struct_field_access() {
    let n = empty_node();
    let t: Type = create_type("Nat", "Bool", &n);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "Bool");
    // expr is set to a deepcopy of n (default node)
    assert_eq!(t.expr.type_, AstNodeType::VAR);
}

#[test]
fn test_typeenv_struct_field_access() {
    let n = empty_node();
    let t1 = create_type("Nat", "", &n);
    let env = TypeEnv {
        type_: t1,
        next: None,
    };
    assert_eq!(env.type_.type_, "Nat");
    assert!(env.next.is_none());
}

fn main() {}
