use lambda_calculus_eval::{common, parser, hash_table};

#[test]
fn test_parse_token_l_paren() {
    assert_eq!(parser::parse_token('('), common::tokens_t::L_PAREN);
}

#[test]
fn test_parse_token_r_paren() {
    assert_eq!(parser::parse_token(')'), common::tokens_t::R_PAREN);
}

#[test]
fn test_parse_token_lambda() {
    assert_eq!(parser::parse_token('@'), common::tokens_t::LAMBDA);
}

#[test]
fn test_parse_token_dot() {
    assert_eq!(parser::parse_token('.'), common::tokens_t::DOT);
}

#[test]
fn test_parse_token_variable() {
    assert_eq!(parser::parse_token('t'), common::tokens_t::VARIABLE);
}

#[test]
fn test_parse_token_whitespace() {
    assert_eq!(parser::parse_token(' '), common::tokens_t::WHITESPACE);
}

#[test]
fn test_parse_token_newline() {
    assert_eq!(parser::parse_token('\n'), common::tokens_t::NEWLINE);
}

#[test]
fn test_parse_token_eq() {
    assert_eq!(parser::parse_token('='), common::tokens_t::EQ);
}

#[test]
fn test_parse_token_quote() {
    assert_eq!(parser::parse_token('"'), common::tokens_t::QUOTE);
}

#[test]
fn test_parse_token_colon() {
    assert_eq!(parser::parse_token(':'), common::tokens_t::COLON);
}

#[test]
fn test_parse_token_error() {
    assert_eq!(parser::parse_token('$'), common::tokens_t::ERROR);
}

#[test]
fn test_is_variable_lowercase() {
    assert!(parser::is_variable('a'));
    assert!(parser::is_variable('z'));
}

#[test]
fn test_is_variable_uppercase() {
    assert!(parser::is_variable('A'));
    assert!(parser::is_variable('Z'));
}

#[test]
fn test_is_variable_underscore() {
    assert!(parser::is_variable('_'));
}

#[test]
fn test_is_variable_non_variable() {
    assert!(!parser::is_variable('*'));
    assert!(!parser::is_variable('0'));
    assert!(!parser::is_variable(' '));
    assert!(!parser::is_variable('.'));
}

#[test]
fn test_create_variable() {
    let var = parser::create_variable("test", "Nat");
    assert_eq!(var.type_, common::AstNodeType::VAR);
    if let common::AstNodeUnion::Variable(ref v) = var.node {
        assert_eq!(v.name, "test");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_create_variable_no_type() {
    let var = parser::create_variable("x", "");
    if let common::AstNodeUnion::Variable(ref v) = var.node {
        assert_eq!(v.name, "x");
        assert_eq!(v.type_, "");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_create_application() {
    let f = parser::create_variable("f", "Nat");
    let a = parser::create_variable("a", "Bool");
    let app = parser::create_application(&f, &a);
    assert_eq!(app.type_, common::AstNodeType::APPLICATION);
    if let common::AstNodeUnion::Application(ref ap) = app.node {
        assert!(ap.function.is_some());
        assert!(ap.argument.is_some());
    } else {
        panic!("Expected Application");
    }
}

#[test]
fn test_create_lambda() {
    let body = parser::create_variable("x", "");
    let lambda = parser::create_lambda("x", &body, "Nat");
    assert_eq!(lambda.type_, common::AstNodeType::LAMBDA_EXPR);
    if let common::AstNodeUnion::LambdaExpr(ref le) = lambda.node {
        assert_eq!(le.parameter, "x");
        assert_eq!(le.type_, "Nat");
        assert!(le.body.is_some());
    } else {
        panic!("Expected LambdaExpr");
    }
}

#[test]
fn test_alpha_convert() {
    // alpha_convert uses a global counter, so results depend on call order.
    // We just verify the format is "old_N" where N is some integer.
    let result = parser::alpha_convert("x");
    assert!(result.starts_with("x_"));
    let suffix = &result[2..];
    assert!(suffix.parse::<u64>().is_ok());
}

#[test]
fn test_is_used() {
    let mut table = hash_table::HashTable::new();
    table.insert("test", common::AstNode::default());
    assert!(parser::is_used(&table, "test"));
    assert!(!parser::is_used(&table, "wrong"));
}

#[test]
fn test_is_uppercase() {
    assert!(parser::is_uppercase('A'));
    assert!(parser::is_uppercase('Z'));
    assert!(!parser::is_uppercase('a'));
    assert!(!parser::is_uppercase('z'));
}

fn main() {}
