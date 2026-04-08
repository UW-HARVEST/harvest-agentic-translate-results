use kairoCompiler::compiler::{
    CompileProcess, Token, Node,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
    PARSE_ALL_OK,
};
use kairoCompiler::lexer::tokens_build_for_string;
use kairoCompiler::parser::parse;

fn parse_string(s: &str) -> CompileProcess {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, s).expect("lex failed");
    let mut process = *lp.compiler.take().unwrap();
    process.token_vec = lp.token_vec.take();
    process.node_vec = Some(kairoCompiler::vector::vector_create(8));
    process.node_tree_vec = Some(kairoCompiler::vector::vector_create(8));
    let result = parse(&mut process);
    assert_eq!(result, PARSE_ALL_OK);
    process
}

#[test]
fn test_parse_number() {
    let _process = parse_string("42$");
    // parse should complete without error
    // The node is pushed to the global node vector
}

#[test]
fn test_parse_identifier() {
    let _process = parse_string("hello$");
}

#[test]
fn test_parse_string() {
    let _process = parse_string("\"world\"$");
}

#[test]
fn test_parse_multiple() {
    let _process = parse_string("123 abc$");
}

#[test]
fn test_parse_returns_ok() {
    let cp = CompileProcess::default();
    let mut lp = tokens_build_for_string(cp, "42$").expect("lex failed");
    let mut process = *lp.compiler.take().unwrap();
    process.token_vec = lp.token_vec.take();
    process.node_vec = Some(kairoCompiler::vector::vector_create(8));
    process.node_tree_vec = Some(kairoCompiler::vector::vector_create(8));
    let result = parse(&mut process);
    assert_eq!(result, PARSE_ALL_OK);
}

fn main() {}
