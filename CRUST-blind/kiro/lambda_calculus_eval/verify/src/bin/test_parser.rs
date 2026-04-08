use lambda_calculus_eval::common::*;
use lambda_calculus_eval::parser::*;
use lambda_calculus_eval::hash_table::*;

#[test]
fn test_parse_token_l_paren() {
    assert_eq!(parse_token('('), tokens_t::L_PAREN);
}

#[test]
fn test_parse_token_r_paren() {
    assert_eq!(parse_token(')'), tokens_t::R_PAREN);
}

#[test]
fn test_parse_token_lambda() {
    assert_eq!(parse_token('@'), tokens_t::LAMBDA);
}

#[test]
fn test_parse_token_dot() {
    assert_eq!(parse_token('.'), tokens_t::DOT);
}

#[test]
fn test_parse_token_variable() {
    assert_eq!(parse_token('t'), tokens_t::VARIABLE);
}

#[test]
fn test_parse_token_whitespace() {
    assert_eq!(parse_token(' '), tokens_t::WHITESPACE);
}

#[test]
fn test_parse_token_newline() {
    assert_eq!(parse_token('\n'), tokens_t::NEWLINE);
}

#[test]
fn test_parse_token_eq() {
    assert_eq!(parse_token('='), tokens_t::EQ);
}

#[test]
fn test_parse_token_quote() {
    assert_eq!(parse_token('"'), tokens_t::QUOTE);
}

#[test]
fn test_parse_token_colon() {
    assert_eq!(parse_token(':'), tokens_t::COLON);
}

#[test]
fn test_parse_token_error() {
    assert_eq!(parse_token('$'), tokens_t::ERROR);
}

#[test]
fn test_is_variable_lowercase() {
    assert!(is_variable('a'));
    assert!(is_variable('z'));
}

#[test]
fn test_is_variable_uppercase() {
    assert!(is_variable('A'));
    assert!(is_variable('Z'));
}

#[test]
fn test_is_variable_underscore() {
    assert!(is_variable('_'));
}

#[test]
fn test_is_variable_digit() {
    assert!(!is_variable('0'));
}

#[test]
fn test_is_variable_space() {
    assert!(!is_variable(' '));
}

#[test]
fn test_is_variable_dollar() {
    assert!(!is_variable('$'));
}

#[test]
fn test_is_uppercase_true() {
    assert!(is_uppercase('A'));
    assert!(is_uppercase('Z'));
}

#[test]
fn test_is_uppercase_false() {
    assert!(!is_uppercase('a'));
    assert!(!is_uppercase('z'));
}

#[test]
fn test_create_variable() {
    let var = create_variable("test", "Nat");
    assert_eq!(var.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(v) = &var.node {
        assert_eq!(v.name, "test");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_create_application() {
    let f = create_variable("f", "Nat");
    let a = create_variable("a", "Nat");
    let app = create_application(&f, &a);
    assert_eq!(app.type_, AstNodeType::APPLICATION);
    if let AstNodeUnion::Application(ap) = &app.node {
        assert!(ap.function.is_some());
        assert!(ap.argument.is_some());
    } else {
        panic!("Expected Application");
    }
}

#[test]
fn test_create_lambda() {
    let body = create_variable("x", "");
    let lambda = create_lambda("x", &body, "Nat");
    assert_eq!(lambda.type_, AstNodeType::LAMBDA_EXPR);
    if let AstNodeUnion::LambdaExpr(le) = &lambda.node {
        assert_eq!(le.parameter, "x");
        assert_eq!(le.type_, "Nat");
        assert!(le.body.is_some());
    } else {
        panic!("Expected LambdaExpr");
    }
}

#[test]
fn test_is_used() {
    let mut table = HashTable::new();
    table.insert("test", AstNode::default());
    assert!(is_used(&table, "test"));
    assert!(!is_used(&table, "wrong"));
}

#[test]
fn test_peek() {
    let path = "/tmp/test_parser_peek.txt";
    std::fs::write(path, "AB").unwrap();
    let mut f = std::fs::File::open(path).unwrap();
    let c = peek(&mut f);
    assert_eq!(c, 'A');
    // peek should not advance
    let c2 = peek(&mut f);
    assert_eq!(c2, 'A');
    std::fs::remove_file(path).unwrap();
}

#[test]
fn test_parse_variable_from_file() {
    let path = "/tmp/test_parser_pv.txt";
    std::fs::write(path, "hello ").unwrap();
    let mut f = std::fs::File::open(path).unwrap();
    let name = parse_variable(&mut f);
    assert_eq!(name, "hello");
    std::fs::remove_file(path).unwrap();
}

fn main() {}
