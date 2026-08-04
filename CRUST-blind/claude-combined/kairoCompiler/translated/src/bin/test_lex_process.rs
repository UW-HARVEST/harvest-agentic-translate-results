use kairoCompiler::compiler::CompileProcess;
use kairoCompiler::lex_process::{
    lex_process_create, lex_process_free, lex_process_private, lex_process_tokens,
    LexProcess, LexProcessFunctions,
};

fn dn(_p: &mut LexProcess) -> char { '\0' }
fn dp(_p: &mut LexProcess) -> char { '\0' }
fn dpush(_p: &mut LexProcess, _c: char) {}

#[test]
fn test_lex_process_create_initial_pos() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let lp = lex_process_create(cp, funcs, None);
    assert_eq!(lp.pos.line, 1);
    assert_eq!(lp.pos.col, 1);
}

#[test]
fn test_lex_process_private_default_none() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let lp = lex_process_create(cp, funcs, None);
    assert_eq!(lex_process_private(&lp), None);
}

#[test]
fn test_lex_process_tokens_initial_count() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let lp = lex_process_create(cp, funcs, None);
    let tv = lex_process_tokens(&lp);
    assert!(tv.is_some());
    assert_eq!(tv.unwrap().count, 0);
}

#[test]
fn test_lex_process_free_no_panic() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let lp = lex_process_create(cp, funcs, None);
    lex_process_free(lp);
}

#[test]
fn test_lex_process_compiler_set() {
    let mut cp = CompileProcess::default();
    cp.flags = 7;
    let funcs = LexProcessFunctions { next_char: dn, peek_char: dp, push_char: dpush };
    let lp = lex_process_create(cp, funcs, None);
    let c = lp.compiler.as_ref().unwrap();
    assert_eq!(c.flags, 7);
}

fn main() {}
