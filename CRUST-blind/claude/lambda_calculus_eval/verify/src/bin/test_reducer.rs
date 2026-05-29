use lambda_calculus_eval::common::{
    self, AstNode, AstNodeType, AstNodeUnion, Application, LambdaExpression, Variable,
};
use lambda_calculus_eval::config::reduction_order_t;
use lambda_calculus_eval::hash_table::HashTable;
use lambda_calculus_eval::reducer::{
    deepcopy, deepcopy_application, deepcopy_lambda_expr, deepcopy_var,
    expand_definitions, print_reduction_order, reduce, reduce_ast, replace,
    set_reduction_order, substitute, SIZE,
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

fn make_def(name: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::DEFINITION,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: String::new(),
        }),
    }
}

#[test]
fn test_size_constant() {
    assert_eq!(SIZE, 122);
}

#[test]
fn test_set_reduction_order_does_not_panic() {
    set_reduction_order(reduction_order_t::APPLICATIVE);
    set_reduction_order(reduction_order_t::NORMAL);
    set_reduction_order(reduction_order_t::APPLICATIVE);
}

#[test]
fn test_print_reduction_order_does_not_panic() {
    print_reduction_order(reduction_order_t::APPLICATIVE);
    print_reduction_order(reduction_order_t::NORMAL);
}

#[test]
fn test_deepcopy_var_basic() {
    let copy = deepcopy_var("hello", "World");
    assert!(matches!(copy.type_, AstNodeType::VAR));
    if let AstNodeUnion::Variable(v) = &copy.node {
        assert_eq!(v.name, "hello");
        assert_eq!(v.type_, "World");
    } else {
        panic!("expected variable");
    }
}

#[test]
fn test_deepcopy_var_no_type() {
    let copy = deepcopy_var("x", "");
    if let AstNodeUnion::Variable(v) = &copy.node {
        assert_eq!(v.name, "x");
        assert_eq!(v.type_, "");
    } else {
        panic!("expected variable");
    }
}

#[test]
fn test_deepcopy_lambda_expr() {
    let body = make_var("x", "");
    let lam = deepcopy_lambda_expr("x", &body, "Nat");
    assert!(matches!(lam.type_, AstNodeType::LAMBDA_EXPR));
    if let AstNodeUnion::LambdaExpr(l) = &lam.node {
        assert_eq!(l.parameter, "x");
        assert_eq!(l.type_, "Nat");
        assert!(l.body.is_some());
        if let Some(body) = &l.body {
            if let AstNodeUnion::Variable(v) = &body.node {
                assert_eq!(v.name, "x");
            } else {
                panic!("expected var body");
            }
        }
    } else {
        panic!("expected lambda");
    }
}

#[test]
fn test_deepcopy_application() {
    let f = make_var("f", "Nat");
    let a = make_var("a", "Nat");
    let app = deepcopy_application(&f, &a);
    assert!(matches!(app.type_, AstNodeType::APPLICATION));
    if let AstNodeUnion::Application(application) = &app.node {
        assert!(application.function.is_some());
        assert!(application.argument.is_some());
    } else {
        panic!("expected application");
    }
}

#[test]
fn test_deepcopy_lambda_full() {
    let body = make_var("x", "");
    let lam = make_lambda("x", body, "Nat");
    let copy = deepcopy(&lam);
    assert!(matches!(copy.type_, AstNodeType::LAMBDA_EXPR));
    if let AstNodeUnion::LambdaExpr(l) = &copy.node {
        assert_eq!(l.parameter, "x");
        assert_eq!(l.type_, "Nat");
        assert!(l.body.is_some());
        if let Some(b) = &l.body {
            assert!(matches!(b.type_, AstNodeType::VAR));
        }
    } else {
        panic!("expected lambda");
    }
}

#[test]
fn test_deepcopy_complex() {
    // C: lambda("x", app(var("x"), var("y", "Nat")), "Nat")
    let inner = make_app(make_var("x", ""), make_var("y", "Nat"));
    let lam = make_lambda("x", inner, "Nat");
    let s_orig = common::ast_to_string(&lam);
    let copy = deepcopy(&lam);
    let s_copy = common::ast_to_string(&copy);
    assert_eq!(s_orig, s_copy);
    assert_eq!(s_copy, "(@x : Nat .((x) (y : Nat) ) ) ");
}

#[test]
fn test_replace_lambda_param_and_body() {
    let var = make_var("test", "");
    let mut lam = make_lambda("test", var, "Nat");
    replace(&mut lam, "test", "replaced");
    if let AstNodeUnion::LambdaExpr(l) = &lam.node {
        assert_eq!(l.parameter, "replaced");
        if let Some(body) = &l.body {
            if let AstNodeUnion::Variable(v) = &body.node {
                assert_eq!(v.name, "replaced");
            } else {
                panic!("expected var body");
            }
        } else {
            panic!("expected body");
        }
    } else {
        panic!("expected lambda");
    }
}

#[test]
fn test_replace_only_matching_var() {
    let mut var = make_var("foo", "");
    replace(&mut var, "bar", "baz");
    if let AstNodeUnion::Variable(v) = &var.node {
        assert_eq!(v.name, "foo");
    } else {
        panic!("expected var");
    }
}

#[test]
fn test_replace_in_application() {
    let mut app = make_app(make_var("x", ""), make_var("x", ""));
    replace(&mut app, "x", "y");
    if let AstNodeUnion::Application(a) = &app.node {
        if let Some(f) = &a.function {
            if let AstNodeUnion::Variable(v) = &f.node {
                assert_eq!(v.name, "y");
            }
        }
        if let Some(arg) = &a.argument {
            if let AstNodeUnion::Variable(v) = &arg.node {
                assert_eq!(v.name, "y");
            }
        }
    } else {
        panic!("expected application");
    }
}

#[test]
fn test_expand_definitions_basic() {
    let mut table = HashTable::new();
    let mut variable = make_def("test");
    let definition = make_var("testing", "Nat");
    table.insert("test", definition);

    expand_definitions(&mut table, &mut variable);
    assert!(matches!(variable.type_, AstNodeType::VAR));
    if let AstNodeUnion::Variable(v) = &variable.node {
        assert_eq!(v.name, "testing");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("expected variable");
    }
}

#[test]
fn test_expand_definitions_non_definition() {
    let mut table = HashTable::new();
    let mut var = make_var("just", "Nat");
    expand_definitions(&mut table, &mut var);
    if let AstNodeUnion::Variable(v) = &var.node {
        assert_eq!(v.name, "just");
        assert_eq!(v.type_, "Nat");
    }
    assert!(matches!(var.type_, AstNodeType::VAR));
}

#[test]
fn test_substitute_variable_match() {
    let body = make_var("x", "");
    let replacement = make_var("y", "Nat");
    let result = substitute(&body, "x", &replacement);
    assert!(matches!(result.type_, AstNodeType::VAR));
    if let AstNodeUnion::Variable(v) = &result.node {
        assert_eq!(v.name, "y");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("expected variable");
    }
}

#[test]
fn test_substitute_variable_no_match() {
    let body = make_var("z", "Nat");
    let replacement = make_var("y", "Nat");
    let result = substitute(&body, "x", &replacement);
    if let AstNodeUnion::Variable(v) = &result.node {
        assert_eq!(v.name, "z");
        assert_eq!(v.type_, "Nat");
    }
}

#[test]
fn test_reduce_ast_beta_reduction() {
    // C: (@x:Nat. x) (y:Nat) -> y:Nat
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    let body = make_var("x", "");
    let lam = make_lambda("x", body, "Nat");
    let yvar = make_var("y", "Nat");
    let app = make_app(lam, yvar);
    let reduced = reduce_ast(&mut table, &app);
    assert!(matches!(reduced.type_, AstNodeType::VAR));
    if let AstNodeUnion::Variable(v) = &reduced.node {
        assert_eq!(v.name, "y");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("expected variable result");
    }
}

#[test]
fn test_reduce_ast_lambda_only_applicative() {
    // C: applicative-order reducing a lambda only reduces the body in place;
    // result is still a lambda.
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    let inner = make_app(make_var("x", ""), make_var("x", ""));
    let lam = make_lambda("x", inner, "Nat");
    let reduced = reduce_ast(&mut table, &lam);
    let s = common::ast_to_string(&reduced);
    assert_eq!(s, "(@x : Nat .((x) (x) ) ) ");
}

#[test]
fn test_reduce_ast_complex_app() {
    // C: ((@x:Nat.((x) (z:Nat))) (y:Nat)) -> ((y:Nat) (z:Nat))
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    let lam_body = make_app(make_var("x", ""), make_var("z", "Nat"));
    let lam = make_lambda("x", lam_body, "Nat");
    let yvar = make_var("y", "Nat");
    let app = make_app(lam, yvar);
    let reduced = reduce_ast(&mut table, &app);
    let s = common::ast_to_string(&reduced);
    assert_eq!(s, "((y : Nat) (z : Nat) ) ");
}

#[test]
fn test_reduce_ast_normal_order() {
    // C: same example with NORMAL order produces the same final result.
    set_reduction_order(reduction_order_t::NORMAL);
    let mut table = HashTable::new();
    let lam_body = make_app(make_var("x", ""), make_var("z", "Nat"));
    let lam = make_lambda("x", lam_body, "Nat");
    let yvar = make_var("y", "Nat");
    let app = make_app(lam, yvar);
    let reduced = reduce_ast(&mut table, &app);
    let s = common::ast_to_string(&reduced);
    assert_eq!(s, "((y : Nat) (z : Nat) ) ");
    set_reduction_order(reduction_order_t::APPLICATIVE);
}

#[test]
fn test_reduce_with_definitions() {
    // C: (@x:Nat.x)(y:Nat) reduce -> y:Nat
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    let body = make_var("x", "");
    let lam = make_lambda("x", body, "Nat");
    let yvar = make_var("y", "Nat");
    let app = make_app(lam, yvar);
    let reduced = reduce(&mut table, &app);
    if let AstNodeUnion::Variable(v) = &reduced.node {
        assert_eq!(v.name, "y");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("expected variable");
    }
}

fn main() {}
