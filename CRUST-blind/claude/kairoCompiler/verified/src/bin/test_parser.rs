use kairoCompiler::parser::parse;
use kairoCompiler::lexer::{lex, COMPILER_LEX_FUNCTIONS};
use kairoCompiler::lex_process::lex_process_create;
use kairoCompiler::cprocess::compile_process_create;
use kairoCompiler::compiler::{PARSE_ALL_OK, LEXICAL_ANALYSIS_ALL_OK};
use kairoCompiler::vector::vector_count;
use std::fs;
use std::io::Write;

fn make_compiled_process(input: &str, suffix: &str) -> kairoCompiler::compiler::CompileProcess {
    let path = format!("/tmp/parser_test_{}.txt", suffix);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(input.as_bytes()).unwrap();
    let process = compile_process_create(&path, "", 0).unwrap();
    let mut lp = lex_process_create(process, COMPILER_LEX_FUNCTIONS, None);
    let r = lex(&mut lp);
    assert_eq!(r, LEXICAL_ANALYSIS_ALL_OK);
    let token_vec = lp.token_vec.take();
    // unwrap compiler from box
    let mut compiler = *lp.compiler.take().unwrap();
    compiler.token_vec = token_vec;
    compiler
}

#[test]
fn test_parse_returns_ok_simple() {
    let mut process = make_compiled_process("42$", "simple");
    let r = parse(&mut process);
    assert_eq!(r, PARSE_ALL_OK);
}

#[test]
fn test_parse_returns_ok_empty_dollar() {
    let mut process = make_compiled_process("$", "empty");
    let r = parse(&mut process);
    assert_eq!(r, PARSE_ALL_OK);
    let v = process.token_vec.as_ref().unwrap();
    // No tokens generated for input that immediately ends with $
    assert_eq!(vector_count(v), 0);
}

#[test]
fn test_parse_advances_token_pointer() {
    let mut process = make_compiled_process("42 hello$", "advance");
    let initial_count = vector_count(process.token_vec.as_ref().unwrap());
    assert_eq!(initial_count, 2);
    let r = parse(&mut process);
    assert_eq!(r, PARSE_ALL_OK);
    // After parse, the peek pointer should have advanced through all tokens
    let v = process.token_vec.as_ref().unwrap();
    assert!(v.pindex >= v.rindex);
}

#[test]
fn test_parse_pushes_to_node_tree_vec() {
    let mut process = make_compiled_process("100 200 300$", "tree");
    let r = parse(&mut process);
    assert_eq!(r, PARSE_ALL_OK);
    // The parser should push something into node_tree_vec for each parsed token
    let tree = process.node_tree_vec.as_ref().unwrap();
    // We have 3 number tokens
    assert_eq!(tree.count, 3);
}

#[test]
fn test_parse_no_tokens() {
    let mut process = make_compiled_process("$", "notoken");
    let r = parse(&mut process);
    assert_eq!(r, PARSE_ALL_OK);
    let tree = process.node_tree_vec.as_ref().unwrap();
    assert_eq!(tree.count, 0);
}

#[test]
fn test_parse_single_number() {
    let mut process = make_compiled_process("123$", "single");
    let r = parse(&mut process);
    assert_eq!(r, PARSE_ALL_OK);
    let tree = process.node_tree_vec.as_ref().unwrap();
    assert_eq!(tree.count, 1);
}

fn main() {}
