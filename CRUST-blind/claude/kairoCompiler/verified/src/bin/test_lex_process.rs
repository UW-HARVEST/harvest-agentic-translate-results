use kairoCompiler::lex_process::{
    lex_process_create, lex_process_free, lex_process_private,
    lex_process_tokens, LexProcess, LexProcessFunctions,
};
use kairoCompiler::compiler::CompileProcess;

fn dummy_next(_p: &mut LexProcess) -> char { '\u{FFFF}' }
fn dummy_peek(_p: &mut LexProcess) -> char { '\u{FFFF}' }
fn dummy_push(_p: &mut LexProcess, _c: char) {}

#[test]
fn test_lex_process_create_initial_state() {
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
    assert!(lp.private.is_none());
}

#[test]
fn test_lex_process_private_returns_none() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let lp = lex_process_create(cp, funcs, None);
    let p = lex_process_private(&lp);
    assert!(p.is_none());
}

#[test]
fn test_lex_process_private_returns_unit() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let lp = lex_process_create(cp, funcs, Some(()));
    let p = lex_process_private(&lp);
    assert_eq!(p, Some(()));
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
    let tokens = lex_process_tokens(&lp);
    assert!(tokens.is_some());
    let v = tokens.unwrap();
    assert_eq!(v.count, 0);
    // Should be initialized with esize equal to size_of::<usize>()
    assert_eq!(v.esize, std::mem::size_of::<usize>());
}

#[test]
fn test_lex_process_token_vec_empty_initially() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let lp = lex_process_create(cp, funcs, None);
    let v = lp.token_vec.as_ref().unwrap();
    assert_eq!(v.count, 0);
    assert_eq!(v.rindex, 0);
    assert_eq!(v.pindex, 0);
}

#[test]
fn test_lex_process_free() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: dummy_next,
        peek_char: dummy_peek,
        push_char: dummy_push,
    };
    let lp = lex_process_create(cp, funcs, None);
    lex_process_free(lp); // Should not panic
}

fn main() {}
