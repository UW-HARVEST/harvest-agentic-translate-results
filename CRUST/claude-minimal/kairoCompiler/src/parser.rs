use crate::compiler::{
    CompileProcess, Token, Node,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
    PARSE_ALL_OK,
};
use crate::vector::{
    vector_peek, vector_peek_no_increment, vector_set_peek_pointer,
    vector_push, vector_pop,
};
use crate::node::{
    node_create, node_set_vector, node_peek,
};
use crate::lexer::token_at;

/// Resolve a vector slot containing a token index back into a Token clone.
fn token_from_slot(bytes: &[u8]) -> Option<Token> {
    let arr: [u8; std::mem::size_of::<usize>()] = bytes[..std::mem::size_of::<usize>()].try_into().ok()?;
    let idx = usize::from_le_bytes(arr);
    token_at(idx)
}

/// Skips newline/comment tokens from the front. We assume `token_opt` is the next-peek token,
/// then advance `current_process->token_vec` past any nl/comment tokens, refreshing `token_opt`.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    let vec = match process.token_vec.as_mut() {
        Some(v) => v,
        None => return,
    };
    while let Some(t) = token_opt {
        if !crate::token::token_is_nl_or_comment_or_newline_separator(t) {
            break;
        }
        // Skip the token.
        let _ = vector_peek(vec);
        // Get next peek without increment.
        *token_opt = match vector_peek_no_increment(vec) {
            Some(slot) => token_from_slot(slot),
            None => None,
        };
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let mut next = {
        let vec = process.token_vec.as_mut()?;
        vector_peek_no_increment(vec).and_then(|slot| token_from_slot(slot))
    };
    parser_ignore_nl_or_comments(process, &mut next);
    let vec = process.token_vec.as_mut()?;
    vector_peek_no_increment(vec).and_then(|slot| token_from_slot(slot))
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let mut next = {
        let vec = process.token_vec.as_mut()?;
        vector_peek_no_increment(vec).and_then(|slot| token_from_slot(slot))
    };
    parser_ignore_nl_or_comments(process, &mut next);
    if let Some(t) = next.as_ref() {
        process.pos = t.pos.clone();
    }
    *parser_last_token = next;

    let vec = process.token_vec.as_mut()?;
    vector_peek(vec).and_then(|slot| token_from_slot(slot))
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
            crate::compiler::compiler_error(
                process,
                "This is not a token that can be converted to a node",
            );
            return;
        }
    };
    let _ = node_create(&node);
}

/// parse_next in the original code. Returns 0 if we handled a token, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = match token_peek_no_increment(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        TOKEN_TYPE_NUMBER | TOKEN_TYPE_IDENTIFIER | TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {}
    }
    0
}

/// The main parse function. We set node vectors, then repeatedly call parse_next until no tokens remain.
pub fn parse(process: &mut CompileProcess) -> i32 {
    let mut parser_last_token: Option<Token> = None;
    // Move the node vectors into the global node module state.
    let node_vec = process.node_vec.take().unwrap_or_else(|| {
        crate::vector::vector_create(std::mem::size_of::<usize>())
    });
    let node_tree_vec = process.node_tree_vec.take().unwrap_or_else(|| {
        crate::vector::vector_create(std::mem::size_of::<usize>())
    });
    node_set_vector(node_vec, node_tree_vec);

    if let Some(vec) = process.token_vec.as_mut() {
        vector_set_peek_pointer(vec, 0);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let _node = node_peek();
        // Push node onto node_tree_vec via the global state.
        // node_create already pushed the node onto node_vec; we additionally push
        // a marker into node_tree_vec via the node module's globals.
        // To simulate the C `vector_push(process->node_tree_vec, &node)` line, we need
        // access to the vector. We do this through the node module helper.
        push_to_root_vector(_node);
    }

    // Move the vectors back from node module into the process.
    let (nv, nrv) = crate::node::node_take_vectors();
    process.node_vec = nv;
    process.node_tree_vec = nrv;

    PARSE_ALL_OK
}

/// Pushes the given node's index onto the root vector currently held in the node module's globals.
fn push_to_root_vector(_node: Node) {
    // We need to re-store the node and push its index onto the root vector.
    // Since node_peek already returned a clone, the node already exists in the global registry
    // at the top of the node_vector. Use node_peek_index helper.
    crate::node::node_root_push_top();
}
