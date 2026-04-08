use kairoCompiler::compiler::{CompileProcess, LexProcessFunctions, Pos};
use kairoCompiler::lex_process::*;
use kairoCompiler::vector::vector_count;

#[test]
fn test_lex_process_create() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: |_| '\0',
        peek_char: |_| '\0',
        push_char: |_, _| {},
    };
    let lp = lex_process_create(cp, funcs, None);
    assert_eq!(lp.pos.line, 1);
    assert_eq!(lp.pos.col, 1);
    assert!(lp.token_vec.is_some());
    assert!(lp.compiler.is_some());
    assert_eq!(lp.current_expression_count, 0);
}

#[test]
fn test_lex_process_private() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: |_| '\0',
        peek_char: |_| '\0',
        push_char: |_, _| {},
    };
    let lp = lex_process_create(cp, funcs, None);
    assert!(lex_process_private(&lp).is_none());
}

#[test]
fn test_lex_process_tokens() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: |_| '\0',
        peek_char: |_| '\0',
        push_char: |_, _| {},
    };
    let lp = lex_process_create(cp, funcs, None);
    let tv = lex_process_tokens(&lp);
    assert!(tv.is_some());
    assert_eq!(vector_count(tv.unwrap()), 0);
}

#[test]
fn test_lex_process_free() {
    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: |_| '\0',
        peek_char: |_| '\0',
        push_char: |_, _| {},
    };
    let lp = lex_process_create(cp, funcs, None);
    lex_process_free(lp);
}

fn main() {}
