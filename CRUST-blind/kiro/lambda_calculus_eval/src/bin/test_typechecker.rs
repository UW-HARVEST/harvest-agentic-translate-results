use lambda_calculus_eval::{common, typechecker, parser};

#[test]
fn test_type_equal_same() {
    let var = parser::create_variable("x", "Nat");
    let a = typechecker::create_type("Nat", "", &var);
    let b = typechecker::create_type("Nat", "", &var);
    assert!(typechecker::type_equal(&a, &b));
}

#[test]
fn test_type_equal_different() {
    let var = parser::create_variable("x", "Nat");
    let a = typechecker::create_type("Nat", "", &var);
    let b = typechecker::create_type("Bool", "", &var);
    assert!(!typechecker::type_equal(&a, &b));
}

#[test]
fn test_type_equal_with_return_type() {
    let var = parser::create_variable("x", "Nat");
    let a = typechecker::create_type("Nat", "Bool", &var);
    let b = typechecker::create_type("Nat", "Bool", &var);
    assert!(typechecker::type_equal(&a, &b));
}

#[test]
fn test_type_equal_different_return_type() {
    let var = parser::create_variable("x", "Nat");
    let a = typechecker::create_type("Nat", "Bool", &var);
    let b = typechecker::create_type("Nat", "Nat", &var);
    assert!(!typechecker::type_equal(&a, &b));
}

#[test]
fn test_type_equal_one_has_return_type() {
    let var = parser::create_variable("x", "Nat");
    let a = typechecker::create_type("Nat", "", &var);
    let b = typechecker::create_type("Nat", "Bool", &var);
    assert!(!typechecker::type_equal(&a, &b));
}

#[test]
fn test_get_type_from_expr_var() {
    let var = parser::create_variable("test", "Nat");
    assert_eq!(typechecker::get_type_from_expr(&var), "Nat");
}

#[test]
fn test_get_type_from_expr_lambda() {
    let body = parser::create_variable("x", "");
    let lambda = parser::create_lambda("test", &body, "Bool");
    assert_eq!(typechecker::get_type_from_expr(&lambda), "Bool");
}

#[test]
fn test_get_type_from_expr_application() {
    let f = parser::create_variable("f", "Nat");
    let a = parser::create_variable("a", "Nat");
    let app = parser::create_application(&f, &a);
    assert_eq!(typechecker::get_type_from_expr(&app), "");
}

#[test]
fn test_create_type() {
    let var = parser::create_variable("x", "Nat");
    let t = typechecker::create_type("Nat", "Bool", &var);
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "Bool");
}

#[test]
fn test_parse_function_type() {
    let t = typechecker::parse_function_type("Nat");
    assert_eq!(t.type_, "Nat");
    assert_eq!(t.return_type, "");
}

#[test]
fn test_expr_type_equal_matching() {
    let var = parser::create_variable("test", "Nat");
    let t = typechecker::create_type("Nat", "", &var);
    assert!(typechecker::expr_type_equal(&t, &var));
}

#[test]
fn test_typecheck_application() {
    // (@x.x) y : Nat  -- should typecheck without error
    let lambda_body = parser::create_variable("x", "");
    let y_var = parser::create_variable("y", "Nat");
    let lambda_expr = parser::create_lambda("x", &lambda_body, "Nat");
    let parsed = parser::create_application(&lambda_expr, &y_var);
    let result = typechecker::typecheck(&parsed, None);
    assert_eq!(result.type_, "Nat");
}

#[test]
fn test_typecheck_var() {
    let var = parser::create_variable("x", "Nat");
    let result = typechecker::typecheck(&var, None);
    assert_eq!(result.type_, "Nat");
}

#[test]
fn test_typecheck_lambda() {
    let body = parser::create_variable("x", "");
    let lambda = parser::create_lambda("x", &body, "Bool");
    let result = typechecker::typecheck(&lambda, None);
    assert_eq!(result.type_, "Bool");
}

fn main() {}
