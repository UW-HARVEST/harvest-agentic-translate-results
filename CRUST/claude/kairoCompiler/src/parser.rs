use crate::compiler::{
    CompileProcess, Token,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    PARSE_ALL_OK, NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING, Node,
};
use crate::vector::{
    vector_peek, vector_peek_no_increment, vector_set_peek_pointer, vector_push,
};
use crate::node::{
    node_create, node_set_vector, node_peek,
};
use crate::token::token_is_nl_or_comment_or_newline_separator;
use crate::lexer::TOKENS;

fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

fn encode_index(idx: u64) -> [u8; 8] {
    idx.to_le_bytes()
}

fn token_at_index(idx: u64) -> Option<Token> {
    let tokens = TOKENS.lock().unwrap();
    tokens.get(idx as usize).cloned()
}

fn token_vec_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let v = process.token_vec.as_mut()?;
    let bytes = vector_peek_no_increment(v)?;
    let idx = decode_index(bytes)?;
    token_at_index(idx)
}

fn token_vec_peek(process: &mut CompileProcess) -> Option<Token> {
    let v = process.token_vec.as_mut()?;
    let bytes = vector_peek(v)?;
    let idx = decode_index(bytes)?;
    token_at_index(idx)
}

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess) {
    loop {
        let tok = token_vec_peek_no_increment(process);
        match tok {
            Some(t) if token_is_nl_or_comment_or_newline_separator(&t) => {
                // skip the token (advance peek pointer)
                let _ = token_vec_peek(process);
            }
            _ => break,
        }
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_next(process: &mut CompileProcess) -> Option<Token> {
    parser_ignore_nl_or_comments(process);
    token_vec_peek_no_increment(process)
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    parser_ignore_nl_or_comments(process);
    if let Some(t) = token_vec_peek_no_increment(process) {
        process.pos = t.pos.clone();
        *parser_last_token = Some(t);
    }
    token_vec_peek(process)
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    let token = match token_next(process, parser_last_token) {
        Some(t) => t,
        None => return,
    };
    let node = match token.r#type {
        TOKEN_TYPE_NUMBER => Node {
            r#type: NODE_TYPE_NUMBER,
            llnum: token.llnum,
            ..Default::default()
        },
        TOKEN_TYPE_IDENTIFIER => Node {
            r#type: NODE_TYPE_IDENTIFIER,
            sval: token.sval.clone(),
            ..Default::default()
        },
        TOKEN_TYPE_STRING => Node {
            r#type: NODE_TYPE_STRING,
            sval: token.sval.clone(),
            ..Default::default()
        },
        _ => {
            eprintln!("This is not a token that can be converted to a node");
            std::process::exit(1);
        }
    };
    node_create(&node);
}

/// parse_next in the original code. Returns 0 if we handled a token, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = match token_peek_next(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        TOKEN_TYPE_NUMBER | TOKEN_TYPE_IDENTIFIER | TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {
            // Advance past tokens we don't yet handle to prevent infinite loops.
            let _ = token_next(process, parser_last_token);
        }
    }
    0
}

/// The main parse function. We set node vectors, then repeatedly call parse_next until no tokens remain.
pub fn parse(process: &mut CompileProcess) -> i32 {
    let mut parser_last_token: Option<Token> = None;
    // Initialize global node vectors
    let node_vec = process.node_vec.clone().unwrap_or_else(|| crate::vector::vector_create(8));
    let node_tree_vec = process.node_tree_vec.clone().unwrap_or_else(|| crate::vector::vector_create(8));
    node_set_vector(node_vec, node_tree_vec);

    if let Some(v) = process.token_vec.as_mut() {
        vector_set_peek_pointer(v, 0);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let _ = node_peek();
        // The C code pushes node onto node_tree_vec. We approximate by pushing a placeholder.
        // We track via the global node vector.
        if let Some(tree_vec) = process.node_tree_vec.as_mut() {
            // push the index of last node
            let nodes = crate::node::NODES.lock().unwrap();
            let idx = nodes.len() as u64;
            drop(nodes);
            if idx > 0 {
                vector_push(tree_vec, &encode_index(idx - 1));
            }
        }
    }
    PARSE_ALL_OK
}
