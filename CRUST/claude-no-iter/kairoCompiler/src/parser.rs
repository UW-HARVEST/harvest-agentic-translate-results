// Wrapper module: the real parsing logic now lives in `crate::compiler`,
// where it can share state with the lexer. This module exposes the original
// public surface area expected by callers.

use crate::compiler::{
    CompileProcess, Token, PARSE_ALL_OK,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING, Node,
    token_is_nl_or_comment_or_newline_separator,
};

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess, _token_opt: &mut Option<Token>) {
    loop {
        match token_peek_no_increment(process) {
            Some(t) if token_is_nl_or_comment_or_newline_separator(&t) => {
                if let Some(vec) = process.token_vec.as_mut() {
                    vec.pindex += 1;
                }
            }
            _ => break,
        }
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let vec = process.token_vec.as_ref()?;
    if vec.pindex < 0 || vec.pindex >= vec.count {
        return None;
    }
    let off = (vec.pindex as usize) * vec.esize;
    if off + std::mem::size_of::<usize>() > vec.data.len() {
        return None;
    }
    let mut idx_bytes = [0u8; std::mem::size_of::<usize>()];
    idx_bytes.copy_from_slice(&vec.data[off..off + std::mem::size_of::<usize>()]);
    let i = usize::from_ne_bytes(idx_bytes);
    crate::compiler::TOKEN_STORAGE.with(|t| t.borrow().get(i).cloned())
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(
    process: &mut CompileProcess,
    parser_last_token: &mut Option<Token>,
) -> Option<Token> {
    let mut tok = token_peek_no_increment(process);
    parser_ignore_nl_or_comments(process, &mut tok);
    let result = token_peek_no_increment(process);
    if let Some(vec) = process.token_vec.as_mut() {
        vec.pindex += 1;
    }
    if let Some(t) = result.as_ref() {
        process.pos = t.pos.clone();
        *parser_last_token = Some(t.clone());
    }
    result
}

/// We create a placeholder function that returns a default token.
fn next_token_placeholder(
    _vec: &mut crate::vector::Vector,
    pos: &crate::compiler::Pos,
) -> Option<Token> {
    let mut t = Token::default();
    t.pos = pos.clone();
    Some(t)
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(
    process: &mut CompileProcess,
    parser_last_token: &mut Option<Token>,
) {
    let token = match token_next(process, parser_last_token) {
        Some(t) => t,
        None => return,
    };
    let node = match token.r#type {
        TOKEN_TYPE_NUMBER => {
            let mut n = Node::default();
            n.r#type = NODE_TYPE_NUMBER;
            n.llnum = token.llnum;
            n
        }
        TOKEN_TYPE_IDENTIFIER => {
            let mut n = Node::default();
            n.r#type = NODE_TYPE_IDENTIFIER;
            n.sval = token.sval.clone();
            n
        }
        TOKEN_TYPE_STRING => {
            let mut n = Node::default();
            n.r#type = NODE_TYPE_STRING;
            n.sval = token.sval.clone();
            n
        }
        _ => return,
    };
    let inner: crate::node::Node = (&node).into();
    crate::node::node_create(&inner);
}

/// parse_next in the original code. Returns 0 if we handled a token, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    parser_ignore_nl_or_comments(process, &mut None);
    let token = match token_peek_no_increment(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        TOKEN_TYPE_NUMBER | TOKEN_TYPE_IDENTIFIER | TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {
            // Skip the token.
            if let Some(vec) = process.token_vec.as_mut() {
                vec.pindex += 1;
            }
        }
    }
    0
}

/// The main parse function. We set node vectors, then repeatedly call parse_next until no tokens remain.
pub fn parse(process: &mut CompileProcess) -> i32 {
    crate::compiler::parse(process);
    PARSE_ALL_OK
}

// Reference unused helpers to suppress dead-code warnings (they remain part of
// the public-ish surface even if the entrypoint is `parse`).
#[allow(dead_code)]
fn _suppress_dead_code(process: &mut CompileProcess) {
    let mut last: Option<Token> = None;
    let _ = parse_next(process, &mut last);
    let _ = next_token_placeholder(
        process.token_vec.as_mut().unwrap(),
        &crate::compiler::Pos::default(),
    );
}
