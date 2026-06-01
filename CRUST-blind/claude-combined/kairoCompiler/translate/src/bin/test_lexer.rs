use kairoCompiler::compiler::{
    CompileProcess, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_KEYWORD, TOKEN_TYPE_NUMBER,
    TOKEN_TYPE_OPERATOR, TOKEN_TYPE_STRING, TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT,
};
use kairoCompiler::lex_process::{lex_process_create, LexProcess, LexProcessFunctions};
use kairoCompiler::buffer::{Buffer, buffer_create, buffer_printf};
use kairoCompiler::lexer::{lex, lex_get_tokens};

// Tests use string-buffer-backed lex by overriding the function table.
thread_local! {
    static BUF: std::cell::RefCell<Option<Buffer>> = std::cell::RefCell::new(None);
}

fn buf_next(_p: &mut LexProcess) -> char {
    BUF.with(|b| {
        let mut bm = b.borrow_mut();
        match bm.as_mut() {
            Some(buf) => kairoCompiler::buffer::buffer_read(buf),
            None => '\0',
        }
    })
}

fn buf_peek(_p: &mut LexProcess) -> char {
    BUF.with(|b| {
        let bm = b.borrow();
        match bm.as_ref() {
            Some(buf) => kairoCompiler::buffer::buffer_peek(buf),
            None => '\0',
        }
    })
}

fn buf_push(_p: &mut LexProcess, c: char) {
    BUF.with(|b| {
        let mut bm = b.borrow_mut();
        if let Some(buf) = bm.as_mut() {
            // Decrement rindex like ungetc
            if buf.rindex > 0 {
                buf.rindex -= 1;
                buf.data[buf.rindex as usize] = c as u8;
            }
        }
    });
}

fn run_lex(input: &str) -> Vec<kairoCompiler::compiler::Token> {
    let mut buf = buffer_create();
    buffer_printf(&mut buf, input);
    BUF.with(|b| *b.borrow_mut() = Some(buf));

    let cp = CompileProcess::default();
    let funcs = LexProcessFunctions {
        next_char: buf_next,
        peek_char: buf_peek,
        push_char: buf_push,
    };
    let mut lp = lex_process_create(cp, funcs, None);

    let _ = lex(&mut lp);
    lex_get_tokens()
}

#[test]
fn test_lex_identifier() {
    let toks = run_lex("foo$");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_IDENTIFIER);
    assert_eq!(toks[0].sval, Some("foo".to_string()));
}

#[test]
fn test_lex_keyword() {
    let toks = run_lex("if$");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_KEYWORD);
    assert_eq!(toks[0].sval, Some("if".to_string()));
}

#[test]
fn test_lex_number() {
    let toks = run_lex("12345$");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(toks[0].llnum, Some(12345));
}

#[test]
fn test_lex_two_keywords() {
    let toks = run_lex("if while$");
    assert_eq!(toks.len(), 2);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_KEYWORD);
    assert_eq!(toks[0].sval, Some("if".to_string()));
    assert_eq!(toks[1].r#type, TOKEN_TYPE_KEYWORD);
    assert_eq!(toks[1].sval, Some("while".to_string()));
}

#[test]
fn test_lex_string() {
    let toks = run_lex("\"hi\"$");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_STRING);
    assert_eq!(toks[0].sval, Some("hi".to_string()));
}

#[test]
fn test_lex_quote_char() {
    let toks = run_lex("'a'$");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(toks[0].cval, Some('a'));
}

#[test]
fn test_lex_number_plus_number() {
    let toks = run_lex("12+34$");
    assert_eq!(toks.len(), 3);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(toks[0].llnum, Some(12));
    assert_eq!(toks[1].r#type, TOKEN_TYPE_OPERATOR);
    assert_eq!(toks[2].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(toks[2].llnum, Some(34));
}

#[test]
fn test_lex_comment_then_number() {
    let toks = run_lex("// hi\n12$");
    // C says: COMMENT, NEWLINE, NUMBER -> 3 tokens
    assert_eq!(toks.len(), 3);
    assert_eq!(toks[0].r#type, TOKEN_TYPE_COMMENT);
    assert_eq!(toks[1].r#type, TOKEN_TYPE_NEWLINE);
    assert_eq!(toks[2].r#type, TOKEN_TYPE_NUMBER);
    assert_eq!(toks[2].llnum, Some(12));
}

#[test]
fn test_lex_empty_just_dollar() {
    let toks = run_lex("$");
    assert_eq!(toks.len(), 0);
}

fn main() {}
