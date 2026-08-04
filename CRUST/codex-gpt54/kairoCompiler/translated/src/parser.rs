use crate::compiler::{
    compiler_error, CompileProcess, Pos, Token, PARSE_ALL_OK, TOKEN_TYPE_IDENTIFIER,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING,
};
use crate::lexer::get_token;
use crate::node::{current_vectors, last_node_index, node_create, node_peek, node_set_vector};
use crate::token::token_is_nl_or_comment_or_newline_separator;
use crate::vector::{vector_peek, vector_peek_no_increment, vector_push, vector_set_peek_pointer};

fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(raw))
}

fn encode_index(idx: u64) -> [u8; 8] {
    idx.to_le_bytes()
}

fn token_from_slot(bytes: &[u8]) -> Option<Token> {
    get_token(decode_index(bytes)?)
}

fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(token) = token_opt.clone() {
        if !token_is_nl_or_comment_or_newline_separator(&token) {
            break;
        }
        if let Some(vec) = process.token_vec.as_mut() {
            let _ = vector_peek(vec);
            *token_opt = vector_peek_no_increment(vec).and_then(|bytes| token_from_slot(bytes));
        } else {
            *token_opt = None;
        }
    }
}

fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let mut token = process
        .token_vec
        .as_mut()
        .and_then(vector_peek_no_increment)
        .and_then(|bytes| token_from_slot(bytes));
    parser_ignore_nl_or_comments(process, &mut token);
    token
}

fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let mut token = process
        .token_vec
        .as_mut()
        .and_then(vector_peek_no_increment)
        .and_then(|bytes| token_from_slot(bytes));
    parser_ignore_nl_or_comments(process, &mut token);
    if let Some(token) = token.clone() {
        process.pos = token.pos.clone();
        *parser_last_token = Some(token);
    }
    process
        .token_vec
        .as_mut()
        .and_then(vector_peek)
        .and_then(|bytes| token_from_slot(bytes))
}

fn next_token_placeholder(_vec: &mut crate::vector::Vector, pos: &Pos) -> Option<Token> {
    Some(Token {
        pos: pos.clone(),
        ..Token::default()
    })
}

fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    let Some(token) = token_next(process, parser_last_token) else {
        return;
    };

    let node = match token.r#type {
        TOKEN_TYPE_NUMBER => node_create(&crate::node::Node {
            r#type: crate::compiler::NODE_TYPE_NUMBER,
            llnum: token.llnum,
            ..crate::node::Node::default()
        }),
        TOKEN_TYPE_IDENTIFIER => node_create(&crate::node::Node {
            r#type: crate::compiler::NODE_TYPE_IDENTIFIER,
            sval: token.sval,
            ..crate::node::Node::default()
        }),
        TOKEN_TYPE_STRING => node_create(&crate::node::Node {
            r#type: crate::compiler::NODE_TYPE_STRING,
            sval: token.sval,
            ..crate::node::Node::default()
        }),
        _ => {
            compiler_error(process, "This is not a token that can be converted to a node");
            return;
        }
    };

    if let Some(tree_vec) = process.node_tree_vec.as_mut() {
        let index = last_node_index().unwrap_or(0);
        let _ = node;
        vector_push(tree_vec, &encode_index(index));
    }
}

fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let Some(token) = token_peek_no_increment(process) else {
        return -1;
    };

    match token.r#type {
        TOKEN_TYPE_NUMBER | TOKEN_TYPE_IDENTIFIER | TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {}
    }
    0
}

pub fn parse(process: &mut CompileProcess) -> i32 {
    let Some(node_vec) = process.node_vec.clone() else {
        return PARSE_ALL_OK;
    };
    let Some(node_tree_vec) = process.node_tree_vec.clone() else {
        return PARSE_ALL_OK;
    };

    node_set_vector(node_vec, node_tree_vec);
    let mut parser_last_token: Option<Token> = None;
    if let Some(token_vec) = process.token_vec.as_mut() {
        vector_set_peek_pointer(token_vec, 0);
        let _ = next_token_placeholder(token_vec, &process.pos);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let _ = node_peek();
    }

    let (node_vec, node_tree_vec) = current_vectors();
    process.node_vec = node_vec;
    process.node_tree_vec = node_tree_vec;
    PARSE_ALL_OK
}
