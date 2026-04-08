use kairoCompiler::lex_process::*;
use kairoCompiler::compiler::{CompileProcess, Pos};
use kairoCompiler::vector::vector_count;

fn dummy_next(lp: &mut LexProcess) -> char { 'a' }
fn dummy_peek(lp: &mut LexProcess) -> char { 'b' }
fn dummy_push(lp: &mut LexProcess, c: char) {}

#[test]
fn test_lex_process_create() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let lp = lex_process_create(cp, funcs, None);
    assert_eq!(lp.pos.line, 1);
    assert_eq!(lp.pos.col, 1);
    assert!(lp.token_vec.is_some());
    assert!(lp.compiler.is_some());
    assert!(lp.function.is_some());
    assert_eq!(lp.current_expression_count, 0);
}

#[test]
fn test_lex_process_tokens() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let lp = lex_process_create(cp, funcs, None);
    let tv = lex_process_tokens(&lp);
    assert!(tv.is_some());
    assert_eq!(vector_count(tv.unwrap()), 0);
}

#[test]
fn test_lex_process_private() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let lp = lex_process_create(cp, funcs, None);
    assert!(lex_process_private(&lp).is_none());
}

fn main() {}
