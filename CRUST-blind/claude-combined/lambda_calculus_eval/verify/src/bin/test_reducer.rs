use lambda_calculus_eval::common::{
    AstNode, AstNodeType, AstNodeUnion,
};
use lambda_calculus_eval::hash_table::HashTable;
use lambda_calculus_eval::parser::{create_application, create_lambda, create_variable};
use lambda_calculus_eval::reducer::{
    deepcopy, deepcopy_application, deepcopy_lambda_expr, deepcopy_var, expand_definitions,
    print_reduction_order, reduce, reduce_ast, replace, set_reduction_order, substitute, SIZE,
};
use lambda_calculus_eval::config::reduction_order_t;

fn make_definition(name: &str) -> AstNode {
    let mut v = create_variable(name, "");
    v.type_ = AstNodeType::DEFINITION;
    v
}

#[test]
fn test_size_constant() {
    assert_eq!(SIZE, 122);
}

#[test]
fn test_set_reduction_order() {
    set_reduction_order(reduction_order_t::APPLICATIVE);
    set_reduction_order(reduction_order_t::NORMAL);
    set_reduction_order(reduction_order_t::APPLICATIVE);
}

#[test]
fn test_print_reduction_order_smoke() {
    print_reduction_order(reduction_order_t::APPLICATIVE);
    print_reduction_order(reduction_order_t::NORMAL);
}

#[test]
fn test_replace_in_var() {
    let mut v = create_variable("test", "Nat");
    replace(&mut v, "test", "replaced");
    if let AstNodeUnion::Variable(ref var) = v.node {
        assert_eq!(var.name, "replaced");
    } else {
        panic!("Expected variable");
    }
}

#[test]
fn test_replace_in_lambda() {
    let body = create_variable("test", "");
    let mut lam = create_lambda("test", &body, "Nat");
    replace(&mut lam, "test", "replaced");
    if let AstNodeUnion::LambdaExpr(ref le) = lam.node {
        assert_eq!(le.parameter, "replaced");
        if let AstNodeUnion::Variable(ref v) = le.body.as_ref().unwrap().node {
            assert_eq!(v.name, "replaced");
        } else {
            panic!("Expected variable in body");
        }
    } else {
        panic!("Expected lambda");
    }
}

#[test]
fn test_replace_does_not_affect_other_names() {
    let body = create_variable("other", "");
    let mut lam = create_lambda("test", &body, "Nat");
    replace(&mut lam, "test", "replaced");
    if let AstNodeUnion::LambdaExpr(ref le) = lam.node {
        assert_eq!(le.parameter, "replaced");
        if let AstNodeUnion::Variable(ref v) = le.body.as_ref().unwrap().node {
            assert_eq!(v.name, "other");
        }
    } else {
        panic!("Expected lambda");
    }
}

#[test]
fn test_replace_in_application() {
    let f = create_variable("test", "Nat");
    let a = create_variable("other", "Nat");
    let mut app = create_application(&f, &a);
    replace(&mut app, "test", "replaced");
    if let AstNodeUnion::Application(ref ap) = app.node {
        if let AstNodeUnion::Variable(ref v) = ap.function.as_ref().unwrap().node {
            assert_eq!(v.name, "replaced");
        }
        if let AstNodeUnion::Variable(ref v) = ap.argument.as_ref().unwrap().node {
            assert_eq!(v.name, "other");
        }
    }
}

#[test]
fn test_expand_definitions_replaces_definition() {
    // Mirrors C test_expand_definitions
    let mut table = HashTable::new();
    let definition = create_variable("testing", "Nat");
    table.insert("test", definition);

    let variable = make_definition("test");
    expand_definitions(&mut table, &variable);

    assert_eq!(variable.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref v) = variable.node {
        assert_eq!(v.name, "testing");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable union after expansion");
    }
}

#[test]
fn test_reduce_ast_basic_application() {
    // Mirrors C test_reduce_ast: (@x:Nat. x) y:Nat -> y:Nat
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    table.insert_empty("Nat");
    let lambda_body = create_variable("x", "");
    let y_var = create_variable("y", "Nat");
    let lambda_expr = create_lambda("x", &lambda_body, "Nat");
    let parsed = create_application(&lambda_expr, &y_var);

    let reduced = reduce_ast(&mut table, &parsed);
    assert_eq!(reduced.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref v) = reduced.node {
        assert_eq!(v.name, "y");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected variable result");
    }
}

#[test]
fn test_reduce_ast_var_unchanged() {
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    let v = create_variable("z", "Nat");
    let r = reduce_ast(&mut table, &v);
    assert_eq!(r.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref var) = r.node {
        assert_eq!(var.name, "z");
        assert_eq!(var.type_, "Nat");
    }
}

#[test]
fn test_reduce_calls_pipeline() {
    // The high-level reduce: same as reduce_ast for inputs without DEFINITION
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    let v = create_variable("x", "Nat");
    let r = reduce(&mut table, &v);
    assert_eq!(r.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref var) = r.node {
        assert_eq!(var.name, "x");
        assert_eq!(var.type_, "Nat");
    }
}

#[test]
fn test_substitute_basic() {
    // substitute(VAR x, "x", VAR y:Nat) -> VAR y:Nat
    let expr = create_variable("x", "");
    let repl = create_variable("y", "Nat");
    let r = substitute(&expr, "x", &repl);
    assert_eq!(r.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref v) = r.node {
        assert_eq!(v.name, "y");
        assert_eq!(v.type_, "Nat");
    }
}

#[test]
fn test_substitute_no_match() {
    let expr = create_variable("z", "Nat");
    let repl = create_variable("y", "Nat");
    let r = substitute(&expr, "x", &repl);
    assert_eq!(r.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref v) = r.node {
        assert_eq!(v.name, "z");
        assert_eq!(v.type_, "Nat");
    }
}

#[test]
fn test_substitute_in_lambda_unbound() {
    // substitute(@x:Nat. y, "y", VAR z:Nat) should produce @x:Nat. z
    let body = create_variable("y", "");
    let lam = create_lambda("x", &body, "Nat");
    let repl = create_variable("z", "Nat");
    let r = substitute(&lam, "y", &repl);
    assert_eq!(r.type_, AstNodeType::LAMBDA_EXPR);
    if let AstNodeUnion::LambdaExpr(ref le) = r.node {
        assert_eq!(le.parameter, "x");
        assert_eq!(le.type_, "Nat");
        if let AstNodeUnion::Variable(ref v) = le.body.as_ref().unwrap().node {
            assert_eq!(v.name, "z");
            assert_eq!(v.type_, "Nat");
        } else {
            panic!("Expected variable in body");
        }
    } else {
        panic!("Expected lambda");
    }
}

#[test]
fn test_deepcopy_var() {
    let v = deepcopy_var("foo", "Nat");
    assert_eq!(v.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref var) = v.node {
        assert_eq!(var.name, "foo");
        assert_eq!(var.type_, "Nat");
    }
}

#[test]
fn test_deepcopy_lambda_expr() {
    let body = create_variable("x", "");
    let lam = deepcopy_lambda_expr("x", &body, "Nat");
    assert_eq!(lam.type_, AstNodeType::LAMBDA_EXPR);
    if let AstNodeUnion::LambdaExpr(ref le) = lam.node {
        assert_eq!(le.parameter, "x");
        assert_eq!(le.type_, "Nat");
        assert!(le.body.is_some());
    }
}

#[test]
fn test_deepcopy_application() {
    let f = create_variable("f", "Nat");
    let a = create_variable("a", "Nat");
    let app = deepcopy_application(&f, &a);
    assert_eq!(app.type_, AstNodeType::APPLICATION);
    if let AstNodeUnion::Application(ref ap) = app.node {
        assert!(ap.function.is_some());
        assert!(ap.argument.is_some());
    }
}

#[test]
fn test_deepcopy_full() {
    // Mirrors C test_deepcopy
    let body = create_variable("x", "");
    let lam = create_lambda("x", &body, "Nat");
    let copy = deepcopy(&lam);
    assert_eq!(copy.type_, AstNodeType::LAMBDA_EXPR);
    if let AstNodeUnion::LambdaExpr(ref le) = copy.node {
        assert_eq!(le.parameter, "x");
        assert_eq!(le.type_, "Nat");
        let bn = le.body.as_ref().unwrap();
        assert_eq!(bn.type_, AstNodeType::VAR);
    }
}

fn main() {}
