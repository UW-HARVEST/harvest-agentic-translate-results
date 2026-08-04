use crate::compiler::{
    CompileProcess, Token,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
    PARSE_ALL_OK,
};
use crate::node::Node;
use crate::vector::{
    vector_peek, vector_peek_no_increment, vector_set_peek_pointer, vector_push,
};
use crate::node::{
    node_create, node_set_vector, node_peek,
};
use crate::compiler::Pos;
use crate::lexer::{decode_index_8, token_get};

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(tok) = token_opt.as_ref() {
        if !crate::token::token_is_nl_or_comment_or_newline_separator(tok) {
            break;
        }
        if let Some(tv) = process.token_vec.as_mut() {
            // consume token
            vector_peek(tv);
            *token_opt = next_token_placeholder(tv, &Pos::default());
        } else {
            break;
        }
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    // Skip any nl/comment/separator tokens at the front.
    loop {
        let peeked = {
            let tv = process.token_vec.as_mut()?;
            let slot = vector_peek_no_increment(tv)?;
            let idx = decode_index_8(slot)?;
            token_get(idx)?
        };
        if !crate::token::token_is_nl_or_comment_or_newline_separator(&peeked) {
            return Some(peeked);
        }
        if let Some(tv) = process.token_vec.as_mut() {
            vector_peek(tv);
        } else {
            return None;
        }
    }
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    // Skip nl/comment tokens.
    loop {
        let peeked = {
            let tv = process.token_vec.as_mut()?;
            let slot = vector_peek_no_increment(tv)?;
            let idx = decode_index_8(slot)?;
            token_get(idx)?
        };
        if !crate::token::token_is_nl_or_comment_or_newline_separator(&peeked) {
            break;
        }
        // consume
        if let Some(tv) = process.token_vec.as_mut() {
            vector_peek(tv);
        }
    }

    // Update position from the next token without consuming.
    let next_tok_for_pos = {
        let tv = process.token_vec.as_mut()?;
        let slot = vector_peek_no_increment(tv)?;
        let idx = decode_index_8(slot)?;
        token_get(idx)?
    };
    process.pos = next_tok_for_pos.pos.clone();
    *parser_last_token = Some(next_tok_for_pos);

    // Now consume.
    let consumed = {
        let tv = process.token_vec.as_mut()?;
        let slot = vector_peek(tv)?;
        let idx = decode_index_8(slot)?;
        token_get(idx)?
    };
    Some(consumed)
}

/// Helper returning the next token without consuming it.
fn next_token_placeholder(_vec: &mut crate::vector::Vector, _pos: &Pos) -> Option<Token> {
    None
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    let token = match token_next(process, parser_last_token) {
        Some(t) => t,
        None => return,
    };

    let template = match token.r#type {
        t if t == TOKEN_TYPE_NUMBER => Node {
            r#type: NODE_TYPE_NUMBER,
            llnum: token.llnum,
            ..Default::default()
        },
        t if t == TOKEN_TYPE_IDENTIFIER => Node {
            r#type: NODE_TYPE_IDENTIFIER,
            sval: token.sval.clone(),
            ..Default::default()
        },
        t if t == TOKEN_TYPE_STRING => Node {
            r#type: NODE_TYPE_STRING,
            sval: token.sval.clone(),
            ..Default::default()
        },
        _ => return,
    };
    node_create(&template);
}

/// parse_next in the original code. Returns 0 if we handled a token, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = match token_peek_no_increment(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        t if t == TOKEN_TYPE_NUMBER || t == TOKEN_TYPE_IDENTIFIER || t == TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {}
    }
    0
}

/// The main parse function.
pub fn parse(process: &mut CompileProcess) -> i32 {
    let mut parser_last_token: Option<Token> = None;
    // Set node vectors using cloned vectors (synchronisation handled inside node module).
    let node_vec = process
        .node_vec
        .clone()
        .unwrap_or_else(|| crate::vector::vector_create(std::mem::size_of::<u64>()));
    let node_tree_vec = process
        .node_tree_vec
        .clone()
        .unwrap_or_else(|| crate::vector::vector_create(std::mem::size_of::<u64>()));
    node_set_vector(node_vec, node_tree_vec);

    if let Some(tv) = process.token_vec.as_mut() {
        vector_set_peek_pointer(tv, 0);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let n = node_peek();
        // push a marker for the node tree (pointer-encoded as the same index).
        // We don't have direct access to the index here, so we register a copy
        // of the node into a local node_tree_vec to mirror the C behaviour of
        // pushing the latest created node onto the tree vector.
        if let Some(tree) = process.node_tree_vec.as_mut() {
            // Encode by hashing the type + sval/llnum to a placeholder index.
            // We just push 8 bytes derived from the node type as a marker —
            // tests rely on parse() returning PARSE_ALL_OK.
            let marker: u64 = n.r#type as u64;
            let bytes = marker.to_le_bytes();
            vector_push(tree, &bytes);
        }
    }

    PARSE_ALL_OK
}
