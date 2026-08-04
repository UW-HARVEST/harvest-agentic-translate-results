use kairoCompiler::compiler::{
    CompileProcess, NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER,
};
use kairoCompiler::parser::{parse, parse_get_nodes};
use kairoCompiler::lexer::{lex, lex_get_tokens};
use kairoCompiler::lex_process::{lex_process_create, LexProcess, LexProcessFunctions};
use kairoCompiler::buffer::{Buffer, buffer_create, buffer_printf};

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
            if buf.rindex > 0 {
                buf.rindex -= 1;
                buf.data[buf.rindex as usize] = c as u8;
            }
        }
    });
}

fn lex_input(input: &str) {
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
}

#[test]
fn test_parse_returns_zero() {
    lex_input("$");
    let mut cp = CompileProcess::default();
    let res = parse(&mut cp);
    assert_eq!(res, 0);
}

#[test]
fn test_parse_number_token_makes_number_node() {
    lex_input("12$");
    // Sanity: lex produced 1 number token
    let toks = lex_get_tokens();
    assert_eq!(toks.len(), 1);

    let mut cp = CompileProcess::default();
    let res = parse(&mut cp);
    assert_eq!(res, 0);
    let nodes = parse_get_nodes();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].r#type, NODE_TYPE_NUMBER);
    assert_eq!(nodes[0].llnum, Some(12));
}

#[test]
fn test_parse_identifier_token_makes_identifier_node() {
    lex_input("foo$");
    let mut cp = CompileProcess::default();
    let res = parse(&mut cp);
    assert_eq!(res, 0);
    let nodes = parse_get_nodes();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].r#type, NODE_TYPE_IDENTIFIER);
    assert_eq!(nodes[0].sval, Some("foo".to_string()));
}

#[test]
fn test_parse_multiple_numbers() {
    lex_input("12 34$");
    let mut cp = CompileProcess::default();
    let _ = parse(&mut cp);
    let nodes = parse_get_nodes();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].r#type, NODE_TYPE_NUMBER);
    assert_eq!(nodes[0].llnum, Some(12));
    assert_eq!(nodes[1].r#type, NODE_TYPE_NUMBER);
    assert_eq!(nodes[1].llnum, Some(34));
}

#[test]
fn test_parse_empty_input() {
    lex_input("$");
    let mut cp = CompileProcess::default();
    let _ = parse(&mut cp);
    let nodes = parse_get_nodes();
    assert_eq!(nodes.len(), 0);
}

fn main() {}
