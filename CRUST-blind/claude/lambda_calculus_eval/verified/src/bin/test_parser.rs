use lambda_calculus_eval::common::{
    self, AstNode, AstNodeType, AstNodeUnion, Variable,
};
use lambda_calculus_eval::hash_table::HashTable;
use lambda_calculus_eval::parser::{
    alpha_convert, create_application, create_lambda, create_variable,
    is_uppercase, is_used, is_variable, parse_token,
};

fn placeholder_node() -> AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: String::new(),
            type_: String::new(),
        }),
    }
}

#[test]
fn test_parse_token_l_paren() {
    assert!(matches!(parse_token('('), common::tokens_t::L_PAREN));
}

#[test]
fn test_parse_token_r_paren() {
    assert!(matches!(parse_token(')'), common::tokens_t::R_PAREN));
}

#[test]
fn test_parse_token_lambda() {
    assert!(matches!(parse_token('@'), common::tokens_t::LAMBDA));
}

#[test]
fn test_parse_token_dot() {
    assert!(matches!(parse_token('.'), common::tokens_t::DOT));
}

#[test]
fn test_parse_token_variable() {
    assert!(matches!(parse_token('a'), common::tokens_t::VARIABLE));
    assert!(matches!(parse_token('Z'), common::tokens_t::VARIABLE));
    assert!(matches!(parse_token('_'), common::tokens_t::VARIABLE));
}

#[test]
fn test_parse_token_whitespace_newline() {
    assert!(matches!(parse_token(' '), common::tokens_t::WHITESPACE));
    assert!(matches!(parse_token('\n'), common::tokens_t::NEWLINE));
}

#[test]
fn test_parse_token_eq_quote_colon() {
    assert!(matches!(parse_token('='), common::tokens_t::EQ));
    assert!(matches!(parse_token('"'), common::tokens_t::QUOTE));
    assert!(matches!(parse_token(':'), common::tokens_t::COLON));
}

#[test]
fn test_parse_token_error() {
    assert!(matches!(parse_token('$'), common::tokens_t::ERROR));
    assert!(matches!(parse_token('5'), common::tokens_t::ERROR));
    assert!(matches!(parse_token('-'), common::tokens_t::ERROR));
    assert!(matches!(parse_token('\t'), common::tokens_t::ERROR));
}

#[test]
fn test_is_variable_letters() {
    assert!(is_variable('a'));
    assert!(is_variable('z'));
    assert!(is_variable('A'));
    assert!(is_variable('Z'));
    assert!(is_variable('_'));
}

#[test]
fn test_is_variable_non_letter() {
    assert!(!is_variable('5'));
    assert!(!is_variable('@'));
    assert!(!is_variable('('));
    assert!(!is_variable(')'));
    assert!(!is_variable(' '));
    assert!(!is_variable('\n'));
}

#[test]
fn test_is_uppercase_basic() {
    assert!(is_uppercase('A'));
    assert!(is_uppercase('Z'));
    assert!(!is_uppercase('a'));
    assert!(!is_uppercase('z'));
    assert!(!is_uppercase('_'));
    assert!(!is_uppercase('5'));
}

#[test]
fn test_create_variable_typed() {
    let var = create_variable("test", "Nat");
    assert!(matches!(var.type_, AstNodeType::VAR));
    if let AstNodeUnion::Variable(v) = &var.node {
        assert_eq!(v.name, "test");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("expected variable node");
    }
}

#[test]
fn test_create_variable_no_type() {
    let var = create_variable("test", "");
    assert!(matches!(var.type_, AstNodeType::VAR));
    if let AstNodeUnion::Variable(v) = &var.node {
        assert_eq!(v.name, "test");
        assert_eq!(v.type_, "");
    } else {
        panic!("expected variable node");
    }
}

#[test]
fn test_create_application_basic() {
    let f = create_variable("f", "Nat");
    let a = create_variable("a", "Nat");
    let app = create_application(&f, &a);
    assert!(matches!(app.type_, AstNodeType::APPLICATION));
    if let AstNodeUnion::Application(application) = &app.node {
        assert!(application.function.is_some());
        assert!(application.argument.is_some());
        if let Some(func) = &application.function {
            if let AstNodeUnion::Variable(v) = &func.node {
                assert_eq!(v.name, "f");
            } else {
                panic!("expected variable function");
            }
        }
        if let Some(arg) = &application.argument {
            if let AstNodeUnion::Variable(v) = &arg.node {
                assert_eq!(v.name, "a");
            } else {
                panic!("expected variable argument");
            }
        }
    } else {
        panic!("expected application node");
    }
}

#[test]
fn test_create_lambda_basic() {
    let body = create_variable("x", "");
    let lam = create_lambda("x", &body, "Nat");
    assert!(matches!(lam.type_, AstNodeType::LAMBDA_EXPR));
    if let AstNodeUnion::LambdaExpr(l) = &lam.node {
        assert_eq!(l.parameter, "x");
        assert_eq!(l.type_, "Nat");
        assert!(l.body.is_some());
    } else {
        panic!("expected lambda node");
    }
}

#[test]
fn test_alpha_convert_first_call() {
    // C: alpha_convert("x") -> "x_1" the first time it's called in a process.
    // Each thread has its own counter starting at 1.
    let result = alpha_convert("x");
    assert_eq!(result, "x_1");
}

#[test]
fn test_alpha_convert_second_call() {
    // After test_alpha_convert_first_call ran in this thread, the counter
    // increments. Use a fresh thread to ensure a clean counter, then check
    // both.
    use std::thread;
    let handle = thread::spawn(|| {
        let r1 = alpha_convert("a");
        let r2 = alpha_convert("b");
        let r3 = alpha_convert("foo");
        (r1, r2, r3)
    });
    let (r1, r2, r3) = handle.join().unwrap();
    assert_eq!(r1, "a_1");
    assert_eq!(r2, "b_2");
    assert_eq!(r3, "foo_3");
}

#[test]
fn test_is_used_existing_key() {
    let mut table = HashTable::new();
    table.insert("test", placeholder_node());
    assert!(is_used(&table, "test"));
}

#[test]
fn test_is_used_missing_key() {
    let table = HashTable::new();
    assert!(!is_used(&table, "test"));
    assert!(!is_used(&table, "wrong"));
}

#[test]
fn test_create_lambda_complex() {
    // Lambda with body that's an application
    let var_x = create_variable("x", "");
    let var_y = create_variable("y", "Nat");
    let app = create_application(&var_x, &var_y);
    let lam = create_lambda("x", &app, "Nat");
    let s = common::ast_to_string(&lam);
    assert_eq!(s, "(@x : Nat .((x) (y : Nat) ) ) ");
}

#[test]
fn test_create_variable_ast_to_string() {
    let var = create_variable("a", "Bool");
    let s = common::ast_to_string(&var);
    assert_eq!(s, "(a : Bool) ");
}

fn main() {}
