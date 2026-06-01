use lambda_calculus_eval::common::{
    self, AstNode, AstNodeType, AstNodeUnion,
};
use lambda_calculus_eval::hash_table::HashTable;
use lambda_calculus_eval::parser::{
    alpha_convert, consume, create_application, create_lambda, create_variable, expect,
    free_ast, is_uppercase, is_used, is_variable, p_print_astNode_type, p_print_token,
    parse_token, parse_variable, peek, print_ast,
};

#[test]
fn test_parse_token_l_paren() {
    assert_eq!(parse_token('('), common::tokens_t::L_PAREN);
}

#[test]
fn test_parse_token_r_paren() {
    assert_eq!(parse_token(')'), common::tokens_t::R_PAREN);
}

#[test]
fn test_parse_token_lambda() {
    assert_eq!(parse_token('@'), common::tokens_t::LAMBDA);
}

#[test]
fn test_parse_token_dot() {
    assert_eq!(parse_token('.'), common::tokens_t::DOT);
}

#[test]
fn test_parse_token_variable() {
    assert_eq!(parse_token('t'), common::tokens_t::VARIABLE);
    assert_eq!(parse_token('A'), common::tokens_t::VARIABLE);
    assert_eq!(parse_token('z'), common::tokens_t::VARIABLE);
    assert_eq!(parse_token('Z'), common::tokens_t::VARIABLE);
    assert_eq!(parse_token('_'), common::tokens_t::VARIABLE);
}

#[test]
fn test_parse_token_whitespace() {
    assert_eq!(parse_token(' '), common::tokens_t::WHITESPACE);
}

#[test]
fn test_parse_token_newline() {
    assert_eq!(parse_token('\n'), common::tokens_t::NEWLINE);
}

#[test]
fn test_parse_token_eq() {
    assert_eq!(parse_token('='), common::tokens_t::EQ);
}

#[test]
fn test_parse_token_quote() {
    assert_eq!(parse_token('"'), common::tokens_t::QUOTE);
}

#[test]
fn test_parse_token_colon() {
    assert_eq!(parse_token(':'), common::tokens_t::COLON);
}

#[test]
fn test_parse_token_error() {
    assert_eq!(parse_token('$'), common::tokens_t::ERROR);
    assert_eq!(parse_token('*'), common::tokens_t::ERROR);
    assert_eq!(parse_token('1'), common::tokens_t::ERROR);
}

#[test]
fn test_is_variable_true() {
    assert_eq!(is_variable('t'), true);
    assert_eq!(is_variable('A'), true);
    assert_eq!(is_variable('z'), true);
    assert_eq!(is_variable('Z'), true);
    assert_eq!(is_variable('_'), true);
}

#[test]
fn test_is_variable_false() {
    assert_eq!(is_variable('*'), false);
    assert_eq!(is_variable('1'), false);
    assert_eq!(is_variable(' '), false);
    assert_eq!(is_variable('('), false);
}

#[test]
fn test_create_variable_basic() {
    let var = create_variable("test", "Nat");
    assert_eq!(var.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref v) = var.node {
        assert_eq!(v.name, "test");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable union");
    }
}

#[test]
fn test_create_application_basic() {
    let f = create_variable("f", "Nat");
    let a = create_variable("a", "Nat");
    let app = create_application(&f, &a);
    assert_eq!(app.type_, AstNodeType::APPLICATION);
    if let AstNodeUnion::Application(ref ap) = app.node {
        assert!(ap.function.is_some());
        assert!(ap.argument.is_some());
        let func = ap.function.as_ref().unwrap();
        let arg = ap.argument.as_ref().unwrap();
        assert_eq!(func.type_, AstNodeType::VAR);
        assert_eq!(arg.type_, AstNodeType::VAR);
    } else {
        panic!("Expected Application union");
    }
}

#[test]
fn test_create_lambda_basic() {
    let body = create_variable("x", "Nat");
    let lam = create_lambda("x", &body, "Nat");
    assert_eq!(lam.type_, AstNodeType::LAMBDA_EXPR);
    if let AstNodeUnion::LambdaExpr(ref le) = lam.node {
        assert_eq!(le.parameter, "x");
        assert_eq!(le.type_, "Nat");
        assert!(le.body.is_some());
    } else {
        panic!("Expected LambdaExpr union");
    }
}

// alpha_convert uses a global counter; run all in one #[test] to avoid race
#[test]
fn test_alpha_convert_sequential() {
    // First call returns "x_N" with monotonic N starting from current counter.
    // Second call increments. We just verify monotonic increment relative.
    let r1 = alpha_convert("x");
    let r2 = alpha_convert("y");
    // Format: <name>_<n>
    let n1: i32 = r1.split('_').nth(1).unwrap().parse().unwrap();
    let n2: i32 = r2.split('_').nth(1).unwrap().parse().unwrap();
    assert_eq!(n2, n1 + 1);
    assert!(r1.starts_with("x_"));
    assert!(r2.starts_with("y_"));
}

#[test]
fn test_is_used() {
    let mut table = HashTable::new();
    table.insert_empty("name");
    assert_eq!(is_used(&table, "name"), true);
    assert_eq!(is_used(&table, "missing"), false);
}

#[test]
fn test_is_uppercase_true() {
    assert_eq!(is_uppercase('A'), true);
    assert_eq!(is_uppercase('Z'), true);
    assert_eq!(is_uppercase('M'), true);
}

#[test]
fn test_is_uppercase_false() {
    assert_eq!(is_uppercase('a'), false);
    assert_eq!(is_uppercase('z'), false);
    assert_eq!(is_uppercase('1'), false);
    assert_eq!(is_uppercase('_'), false);
}

#[test]
fn test_peek_no_advance() {
    use std::io::{Seek, SeekFrom, Write};
    let path = format!(
        "/tmp/lc_peek_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    f.write_all(b"abc").unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let c = peek(&mut f);
    assert_eq!(c, 'a');
    let c2 = peek(&mut f);
    assert_eq!(c2, 'a');
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_parse_variable_simple() {
    use std::io::{Seek, SeekFrom, Write};
    let path = format!(
        "/tmp/lc_parsevar_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    f.write_all(b"name1=").unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let v = parse_variable(&mut f);
    assert_eq!(v, "name");
    let _ = std::fs::remove_file(&path);
}

// Smoke tests for printing functions — they should not panic
#[test]
fn test_print_ast_smoke() {
    let v = create_variable("x", "Nat");
    print_ast(&v);
    let lam = create_lambda("x", &v, "Nat");
    print_ast(&lam);
    let app = create_application(&v, &v);
    print_ast(&app);
    let mut def = create_variable("def", "");
    def.type_ = AstNodeType::DEFINITION;
    print_ast(&def);
}

#[test]
fn test_p_print_smoke() {
    p_print_token(common::tokens_t::L_PAREN);
    p_print_token(common::tokens_t::R_PAREN);
    p_print_token(common::tokens_t::LAMBDA);
    p_print_token(common::tokens_t::DOT);
    p_print_token(common::tokens_t::VARIABLE);
    p_print_token(common::tokens_t::WHITESPACE);
    p_print_token(common::tokens_t::NEWLINE);
    p_print_token(common::tokens_t::EQ);
    p_print_token(common::tokens_t::QUOTE);
}

#[test]
fn test_p_print_astnode_type_smoke() {
    let v = create_variable("x", "Nat");
    p_print_astNode_type(&v);
    let lam = create_lambda("x", &v, "Nat");
    p_print_astNode_type(&lam);
    let app = create_application(&v, &v);
    p_print_astNode_type(&app);
    let mut def = create_variable("def", "");
    def.type_ = AstNodeType::DEFINITION;
    p_print_astNode_type(&def);
}

#[test]
fn test_free_ast_no_panic() {
    let mut v = create_variable("x", "Nat");
    free_ast(&mut v);
}

fn main() {}
