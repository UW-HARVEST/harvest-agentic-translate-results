use kairoCompiler::compiler::{
    compile_file, compile_process_create, lex_process_create, lex_process_free,
    lex_process_private, lex_process_tokens, lex,
    token_is_keyword, token_is_symbol, token_is_nl_or_comment_or_newline_separator,
    tokens_build_for_string, parse, node_pop, node_peek, node_peek_or_null,
    node_push, node_set_vector, node_create,
    Pos, Token, TokenNumber, Node, NodeBinded, CompileProcess,
    LexProcess, LexProcessFunctions, CompileProcessInputFile, ClonableFile,
    compile_process_next_char, compile_process_peek_char, compile_process_push_char,
    COMPILER_FILE_COMPILED_OK, COMPILER_FAILED_WITH_ERRORS,
    PARSE_ALL_OK, PARSE_GENERAL_ERROR,
    NODE_TYPE_EXPRESSION, NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
    NODE_TYPE_BLANK,
    LEXICAL_ANALYSIS_ALL_OK, LEXICAL_ANALYSIS_INPUT_ERROR,
    TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_OPERATOR, TOKEN_TYPE_SYMBOL,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING, TOKEN_TYPE_COMMENT, TOKEN_TYPE_NEWLINE,
    NUMBER_TYPE_NORMAL, NUMBER_TYPE_LONG, NUMBER_TYPE_FLOAT, NUMBER_TYPE_DOUBLE,
};
use kairoCompiler::vector::vector_count;
use std::fs;
use std::io::Write;

#[test]
fn test_compile_constants() {
    assert_eq!(COMPILER_FILE_COMPILED_OK, 0);
    assert_eq!(COMPILER_FAILED_WITH_ERRORS, 1);
    assert_eq!(PARSE_ALL_OK, 0);
    assert_eq!(PARSE_GENERAL_ERROR, 1);
    assert_eq!(LEXICAL_ANALYSIS_ALL_OK, 0);
    assert_eq!(LEXICAL_ANALYSIS_INPUT_ERROR, 1);

    assert_eq!(TOKEN_TYPE_IDENTIFIER, 0);
    assert_eq!(TOKEN_TYPE_KEYWORD, 1);
    assert_eq!(TOKEN_TYPE_OPERATOR, 2);
    assert_eq!(TOKEN_TYPE_SYMBOL, 3);
    assert_eq!(TOKEN_TYPE_NUMBER, 4);
    assert_eq!(TOKEN_TYPE_STRING, 5);
    assert_eq!(TOKEN_TYPE_COMMENT, 6);
    assert_eq!(TOKEN_TYPE_NEWLINE, 7);

    assert_eq!(NUMBER_TYPE_NORMAL, 0);
    assert_eq!(NUMBER_TYPE_LONG, 1);
    assert_eq!(NUMBER_TYPE_FLOAT, 2);
    assert_eq!(NUMBER_TYPE_DOUBLE, 3);

    assert_eq!(NODE_TYPE_EXPRESSION, 0);
    assert_eq!(NODE_TYPE_NUMBER, 2);
    assert_eq!(NODE_TYPE_IDENTIFIER, 3);
    assert_eq!(NODE_TYPE_STRING, 4);
    assert_eq!(NODE_TYPE_BLANK, 28);
}

#[test]
fn test_compile_file_success() {
    let in_path = "/tmp/cf_test_in.txt";
    let out_path = "/tmp/cf_test_out.txt";
    let mut f = fs::File::create(in_path).unwrap();
    f.write_all(b"5467 abcd $").unwrap();

    let res = compile_file(in_path, out_path, 0);
    assert_eq!(res, COMPILER_FILE_COMPILED_OK);
    let _ = fs::remove_file(in_path);
    let _ = fs::remove_file(out_path);
}

#[test]
fn test_compile_file_missing_input() {
    let res = compile_file("/nonexistent_file_for_compiler", "/tmp/out.txt", 0);
    assert_eq!(res, COMPILER_FAILED_WITH_ERRORS);
}

#[test]
fn test_compile_process_create_facade_returns_some() {
    let in_path = "/tmp/cpc_facade.txt";
    let out_path = "/tmp/cpc_facade_out.txt";
    let mut f = fs::File::create(in_path).unwrap();
    f.write_all(b"hello$").unwrap();
    let process = compile_process_create(in_path, out_path, 7);
    assert_eq!(process.flags, 7);
    let _ = fs::remove_file(in_path);
    let _ = fs::remove_file(out_path);
}

#[test]
fn test_compile_process_create_facade_invalid_returns_default() {
    let process = compile_process_create("/nonexistent_file_for_facade", "", 0);
    assert_eq!(process.flags, 0);
    assert!(process.cfile.fp.is_none());
}

#[test]
fn test_lex_process_create_compiler_module() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let lp = lex_process_create(cp, funcs, None);
    assert_eq!(lp.pos.line, 1);
    assert_eq!(lp.pos.col, 1);
    assert!(lp.token_vec.is_some());
    assert!(lp.compiler.is_some());
    assert!(lp.function.is_some());
}

#[test]
fn test_lex_process_private_compiler_module() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let lp = lex_process_create(cp, funcs, Some(()));
    assert_eq!(lex_process_private(&lp), Some(()));
}

#[test]
fn test_lex_process_tokens_compiler_module() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let lp = lex_process_create(cp, funcs, None);
    assert!(lex_process_tokens(&lp).is_some());
}

#[test]
fn test_lex_process_free_compiler_module() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let lp = lex_process_create(cp, funcs, None);
    lex_process_free(lp); // shouldn't panic
}

#[test]
fn test_token_is_keyword_compiler_facade_match() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_KEYWORD;
    t.sval = Some("if".to_string());
    let r = token_is_keyword(&t, "if");
    assert_eq!(r, true);
}

#[test]
fn test_token_is_keyword_compiler_facade_mismatch() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_KEYWORD;
    t.sval = Some("if".to_string());
    let r = token_is_keyword(&t, "else");
    assert_eq!(r, false);
}

#[test]
fn test_token_is_symbol_facade() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_SYMBOL;
    t.cval = Some('{');
    assert_eq!(token_is_symbol(&t, '{'), true);
    assert_eq!(token_is_symbol(&t, '}'), false);
}

#[test]
fn test_token_is_nl_or_comment_facade() {
    let mut t = Token::default();
    t.r#type = TOKEN_TYPE_NEWLINE;
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), true);
    t.r#type = TOKEN_TYPE_COMMENT;
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), true);
    t.r#type = TOKEN_TYPE_NUMBER;
    assert_eq!(token_is_nl_or_comment_or_newline_separator(&t), false);
}

#[test]
fn test_pos_default() {
    let p = Pos::default();
    assert_eq!(p.line, 0);
    assert_eq!(p.col, 0);
    assert!(p.filename.is_none());
}

#[test]
fn test_token_default() {
    let t = Token::default();
    assert_eq!(t.r#type, 0);
    assert_eq!(t.flags, 0);
    assert!(t.cval.is_none());
    assert!(t.sval.is_none());
    assert!(t.inum.is_none());
    assert!(t.lnum.is_none());
    assert!(t.llnum.is_none());
    assert_eq!(t.whitespace, false);
    assert!(t.between_brackets.is_none());
    assert_eq!(t.num.r#type, 0);
}

#[test]
fn test_token_number_default() {
    let tn = TokenNumber::default();
    assert_eq!(tn.r#type, 0);
}

#[test]
fn test_node_default() {
    let n = Node::default();
    assert_eq!(n.r#type, 0);
    assert_eq!(n.flags, 0);
    assert!(n.cval.is_none());
    assert!(n.sval.is_none());
}

#[test]
fn test_node_binded_default() {
    let nb = NodeBinded::default();
    assert!(nb.owner.is_none());
    assert!(nb.function.is_none());
}

#[test]
fn test_compile_process_input_file_default() {
    let cf = CompileProcessInputFile::default();
    assert!(cf.fp.is_none());
    assert!(cf.abs_path.is_none());
}

#[test]
fn test_compile_process_default() {
    let cp = CompileProcess::default();
    assert_eq!(cp.flags, 0);
    assert_eq!(cp.pos.line, 0);
    assert!(cp.cfile.fp.is_none());
    assert!(cp.token_vec.is_none());
    assert!(cp.node_vec.is_none());
    assert!(cp.node_tree_vec.is_none());
    assert!(cp.ofile.is_none());
}

#[test]
fn test_lex_process_default() {
    let lp = LexProcess::default();
    assert_eq!(lp.pos.line, 0);
    assert_eq!(lp.current_expression_count, 0);
    assert!(lp.token_vec.is_none());
    assert!(lp.compiler.is_none());
    assert!(lp.parentheses_buffer.is_none());
    assert!(lp.function.is_none());
    assert!(lp.private.is_none());
}

#[test]
fn test_clonable_file_basic() {
    let path = "/tmp/cf_clonable.txt";
    let mut f = fs::File::create(path).unwrap();
    f.write_all(b"abc").unwrap();
    let mut cf = ClonableFile::new(path).unwrap();
    let b1 = cf.read_byte();
    assert_eq!(b1, Some(b'a'));
    let b2 = cf.read_byte();
    assert_eq!(b2, Some(b'b'));

    let cloned = cf.clone();
    drop(cloned);
    let _ = fs::remove_file(path);
}

#[test]
fn test_clonable_file_peek_does_not_consume() {
    let path = "/tmp/cf_peek.txt";
    let mut f = fs::File::create(path).unwrap();
    f.write_all(b"xy").unwrap();
    let mut cf = ClonableFile::new(path).unwrap();
    let p1 = cf.peek_byte();
    assert_eq!(p1, Some(b'x'));
    let r = cf.read_byte();
    assert_eq!(r, Some(b'x'));
    let _ = fs::remove_file(path);
}

#[test]
fn test_compile_process_next_char_compiler_module() {
    let in_path = "/tmp/cpr_nc.txt";
    let mut f = fs::File::create(in_path).unwrap();
    f.write_all(b"hi").unwrap();
    let process = compile_process_create(in_path, "", 0);
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let c1 = compile_process_next_char(&mut lp);
    assert_eq!(c1, 'h');
    let c2 = compile_process_next_char(&mut lp);
    assert_eq!(c2, 'i');
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_compile_process_peek_char_compiler_module() {
    let in_path = "/tmp/cpr_pk.txt";
    let mut f = fs::File::create(in_path).unwrap();
    f.write_all(b"yz").unwrap();
    let process = compile_process_create(in_path, "", 0);
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let p1 = compile_process_peek_char(&mut lp);
    assert_eq!(p1, 'y');
    let p2 = compile_process_peek_char(&mut lp);
    assert_eq!(p2, 'y');
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_compile_process_push_char_compiler_module() {
    let in_path = "/tmp/cpr_push.txt";
    let mut f = fs::File::create(in_path).unwrap();
    f.write_all(b"ab").unwrap();
    let process = compile_process_create(in_path, "", 0);
    let funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let mut lp = lex_process_create(process, funcs, None);
    let c1 = compile_process_next_char(&mut lp);
    assert_eq!(c1, 'a');
    compile_process_push_char(&mut lp, 'a');
    let c2 = compile_process_next_char(&mut lp);
    assert_eq!(c2, 'a');
    let _ = fs::remove_file(in_path);
}

#[test]
fn test_lex_returns_input_error_no_compiler() {
    // When LexProcess has no compiler, lex returns LEXICAL_ANALYSIS_INPUT_ERROR
    let mut lp = LexProcess::default();
    let res = lex(&mut lp);
    assert_eq!(res, LEXICAL_ANALYSIS_INPUT_ERROR);
}

#[test]
fn test_tokens_build_for_string_returns_lex_process() {
    let cp = CompileProcess::default();
    let lp = tokens_build_for_string(cp, "123$");
    // The function returns a lex_process, with token_vec set
    assert!(lp.token_vec.is_some());
}

#[test]
fn test_parse_facade_returns_ok_for_simple() {
    // Build a process via compile_process_create
    let in_path = "/tmp/parse_facade.txt";
    let mut f = fs::File::create(in_path).unwrap();
    f.write_all(b"42$").unwrap();
    let mut process = compile_process_create(in_path, "", 0);

    // Lex first
    let lex_funcs = LexProcessFunctions {
        next_char: compile_process_next_char,
        peek_char: compile_process_peek_char,
        push_char: compile_process_push_char,
    };
    let mut lp = lex_process_create(process.clone(), lex_funcs, None);
    let res = lex(&mut lp);
    assert_eq!(res, LEXICAL_ANALYSIS_ALL_OK);

    // Set token_vec from lex
    process.token_vec = lp.token_vec.take();

    let pres = parse(&mut process);
    assert_eq!(pres, PARSE_ALL_OK);

    let _ = fs::remove_file(in_path);
}

#[test]
fn test_node_facade_lifecycle() {
    let nv = kairoCompiler::vector::vector_create(std::mem::size_of::<usize>());
    let nv_root = kairoCompiler::vector::vector_create(std::mem::size_of::<usize>());
    node_set_vector(nv, nv_root);

    let mut n = Node::default();
    n.r#type = NODE_TYPE_NUMBER;
    n.llnum = Some(99);
    let created = node_create(&n);
    assert_eq!(created.r#type, NODE_TYPE_NUMBER);
    assert_eq!(created.llnum, Some(99));

    // Push another
    let mut n2 = Node::default();
    n2.r#type = NODE_TYPE_IDENTIFIER;
    n2.sval = Some("foo".to_string());
    node_push(n2);

    // peek
    let peeked = node_peek().expect("should have node");
    assert_eq!(peeked.r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(peeked.sval, Some("foo".to_string()));

    // peek_or_null
    let peeked2 = node_peek_or_null().expect("should have node");
    assert_eq!(peeked2.r#type, NODE_TYPE_IDENTIFIER);

    // pop
    let popped = node_pop().expect("should have popped");
    assert_eq!(popped.r#type, NODE_TYPE_IDENTIFIER);
}

#[test]
fn test_vector_count_helper() {
    let v = kairoCompiler::vector::vector_create(8);
    assert_eq!(vector_count(&v), 0);
}

fn main() {}
