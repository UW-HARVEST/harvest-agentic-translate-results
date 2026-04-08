use lambda_calculus_eval::common::*;
use lambda_calculus_eval::reducer::*;
use lambda_calculus_eval::hash_table::HashTable;
use lambda_calculus_eval::config::reduction_order_t;

#[test]
fn test_deepcopy_variable() {
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let copy = deepcopy(&var);
    assert_eq!(copy.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(v) = &copy.node {
        assert_eq!(v.name, "x");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_deepcopy_lambda() {
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
    let copy = deepcopy(&lambda);
    assert_eq!(ast_to_string(&copy), "(@x : Nat .(x) ) ");
}

#[test]
fn test_deepcopy_application() {
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
    let copy = deepcopy(&app);
    assert_eq!(ast_to_string(&copy), "((f : Nat) (a : Nat) ) ");
}

#[test]
fn test_replace() {
    let body = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    let mut lambda = AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: "x".to_string(),
            type_: "Nat".to_string(),
            body: Some(Box::new(body)),
        }),
    };
    replace(&mut lambda, "x", "z");
    assert_eq!(ast_to_string(&lambda), "(@z : Nat .(z) ) ");
}

#[test]
fn test_substitute_variable_match() {
    let expr = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    let repl = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "y".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let result = substitute(&expr, "x", &repl);
    assert_eq!(ast_to_string(&result), "(y : Nat) ");
}

#[test]
fn test_substitute_variable_no_match() {
    let expr = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "z".to_string(),
            type_: "Bool".to_string(),
        }),
    };
    let repl = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "y".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    let result = substitute(&expr, "x", &repl);
    assert_eq!(ast_to_string(&result), "(z : Bool) ");
}

#[test]
fn test_reduce_ast_applicative() {
    set_reduction_order(reduction_order_t::APPLICATIVE);
    let mut table = HashTable::new();
    table.insert("Nat", AstNode::default());

    let body = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    let y = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "y".to_string(),
            type_: "Nat".to_string(),
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
    let app = AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(lambda)),
            argument: Some(Box::new(y)),
        }),
    };
    let reduced = reduce_ast(&mut table, &app);
    assert_eq!(ast_to_string(&reduced), "(y : Nat) ");
}

#[test]
fn test_reduce_ast_normal() {
    set_reduction_order(reduction_order_t::NORMAL);
    let mut table = HashTable::new();
    table.insert("Nat", AstNode::default());

    let body = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    let y = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "y".to_string(),
            type_: "Nat".to_string(),
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
    let app = AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(lambda)),
            argument: Some(Box::new(y)),
        }),
    };
    let reduced = reduce_ast(&mut table, &app);
    assert_eq!(ast_to_string(&reduced), "(y : Nat) ");
}

#[test]
fn test_deepcopy_var_fn() {
    let copy = deepcopy_var("x", "Nat");
    assert_eq!(copy.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(v) = &copy.node {
        assert_eq!(v.name, "x");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_deepcopy_lambda_expr_fn() {
    let body = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "x".to_string(),
            type_: String::new(),
        }),
    };
    let copy = deepcopy_lambda_expr("x", &body, "Nat");
    assert_eq!(copy.type_, AstNodeType::LAMBDA_EXPR);
    assert_eq!(ast_to_string(&copy), "(@x : Nat .(x) ) ");
}

#[test]
fn test_deepcopy_application_fn() {
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
    let copy = deepcopy_application(&f, &a);
    assert_eq!(copy.type_, AstNodeType::APPLICATION);
    assert_eq!(ast_to_string(&copy), "((f : Nat) (a : Nat) ) ");
}

fn main() {}
