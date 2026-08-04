use kairoCompiler::compiler::{
    compile_file, compile_process_create, compiler_error, compiler_warning,
    lex_process_create as compiler_lex_process_create, lex_process_free, lex_process_private,
    lex_process_tokens, node_create, node_peek, node_peek_or_null, node_pop, node_push,
    node_set_vector, parse, token_is_keyword, token_is_nl_or_comment_or_newline_separator,
    token_is_symbol, tokens_build_for_string, CompileProcess, LexProcessFunctions, Node, Pos,
    Token, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_SYMBOL, TOKEN_TYPE_NUMBER, NODE_TYPE_NUMBER,
    COMPILER_FILE_COMPILED_OK, COMPILER_FAILED_WITH_ERRORS, NODE_TYPE_IDENTIFIER,
};
use kairoCompiler::vector::vector_create;

#[test]
fn test_compile_file_nonexistent() {
    let res = compile_file("/tmp/nonexistent_xyz_file_12345", "/tmp/out.bin", 0);
    assert_eq!(res, COMPILER_FAILED_WITH_ERRORS);
}

#[test]
fn test_compile_process_create_returns_process() {
    std::fs::write("/tmp/test_compiler_in.txt", b"abc").unwrap();
    let cp = compile_process_create("/tmp/test_compiler_in.txt", "/tmp/test_compiler_out.bin", 7);
    assert_eq!(cp.flags, 7);
    assert!(cp.cfile.abs_path.is_some());
}

#[test]
fn test_compiler_error_does_not_panic() {
    let mut cp = CompileProcess::default();
    compiler_error(&mut cp, "some error");
}

#[test]
fn test_compiler_warning_does_not_panic() {
    let mut cp = CompileProcess::default();
    compiler_warning(&mut cp, "some warning");
}

#[test]
fn test_token_is_keyword() {
    let t = Token {
        r#type: TOKEN_TYPE_KEYWORD,
        sval: Some("if".to_string()),
        ..Default::default()
    };
    assert!(token_is_keyword(&t, "if"));
    assert!(!token_is_keyword(&t, "else"));
}

#[test]
fn test_token_is_symbol() {
    let t = Token {
        r#type: TOKEN_TYPE_SYMBOL,
        cval: Some(';'),
        ..Default::default()
    };
    assert!(token_is_symbol(&t, ';'));
    assert!(!token_is_symbol(&t, ','));
}

#[test]
fn test_token_is_nl_or_comment_or_newline_separator() {
    let t = Token {
        r#type: TOKEN_TYPE_NUMBER,
        ..Default::default()
    };
    assert!(!token_is_nl_or_comment_or_newline_separator(&t));
}

#[test]
fn test_node_create_pushes_to_stack() {
    let nv = vector_create(8);
    let nvr = vector_create(8);
    node_set_vector(nv, nvr);
    let n = Node {
        r#type: NODE_TYPE_NUMBER,
        llnum: Some(5),
        ..Default::default()
    };
    let _created = node_create(&n);
    let peeked = node_peek().unwrap();
    assert_eq!(peeked.r#type, NODE_TYPE_NUMBER);
    assert_eq!(peeked.llnum, Some(5));
}

#[test]
fn test_node_peek_or_null_empty() {
    let nv = vector_create(8);
    let nvr = vector_create(8);
    node_set_vector(nv, nvr);
    let p = node_peek_or_null();
    assert!(p.is_none());
}

#[test]
fn test_node_push_and_pop() {
    let nv = vector_create(8);
    let nvr = vector_create(8);
    node_set_vector(nv, nvr);
    let n = Node {
        r#type: NODE_TYPE_IDENTIFIER,
        sval: Some("var".to_string()),
        ..Default::default()
    };
    node_push(n.clone());
    let popped = node_pop().unwrap();
    assert_eq!(popped.r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(popped.sval, Some("var".to_string()));
}

#[test]
fn test_lex_process_create_basic() {
    fn dn(_: &mut kairoCompiler::compiler::LexProcess) -> char { '\0' }
    fn dp(_: &mut kairoCompiler::compiler::LexProcess) -> char { '\0' }
    fn dpush(_: &mut kairoCompiler::compiler::LexProcess, _: char) {}
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let lp = compiler_lex_process_create(cp, funcs, None);
    assert_eq!(lp.pos.line, 1);
    assert_eq!(lp.pos.col, 1);
    assert_eq!(lex_process_private(&lp), None);
    assert!(lex_process_tokens(&lp).is_some());
    lex_process_free(lp);
}

#[test]
fn test_tokens_build_for_string_returns_lex_process() {
    let cp = CompileProcess::default();
    let _ = tokens_build_for_string(cp, "abc$");
    // Currently a placeholder, but should not panic.
}

#[test]
fn test_parse_returns_ok_when_no_tokens() {
    let mut cp = CompileProcess::default();
    let res = parse(&mut cp);
    assert_eq!(res, 0);
}

#[test]
fn test_pos_default() {
    let p = Pos::default();
    assert_eq!(p.line, 0);
    assert_eq!(p.col, 0);
    assert!(p.filename.is_none());
}

#[test]
fn test_compile_file_with_dollar_input() {
    // Write a file containing only "$" so the lexer terminates immediately.
    std::fs::write("/tmp/test_compiler_dollar.txt", b"$").unwrap();
    let res = compile_file(
        "/tmp/test_compiler_dollar.txt",
        "/tmp/test_compiler_dollar_out.bin",
        0,
    );
    assert_eq!(res, COMPILER_FILE_COMPILED_OK);
}

fn main() {}
