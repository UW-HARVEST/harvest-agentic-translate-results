use crate::compiler::{
    CompileProcess, Token,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    PARSE_ALL_OK,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
};
use crate::vector::{
    vector_peek, vector_peek_no_increment, vector_set_peek_pointer,
    vector_push,
};
use crate::node::{
    node_create, node_set_vector, node_peek, Node,
};
use crate::lexer::get_token;
use crate::token::token_is_nl_or_comment_or_newline_separator;

fn token_at_index(idx: u64) -> Option<Token> {
    get_token(idx)
}

fn bytes_to_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess, _token_opt: &mut Option<Token>) {
    let vec = match process.token_vec.as_mut() {
        Some(v) => v,
        None => return,
    };
    loop {
        let next_bytes = match vector_peek_no_increment(vec) {
            Some(b) => b.to_vec(),
            None => return,
        };
        let idx = match bytes_to_index(&next_bytes) {
            Some(i) => i,
            None => return,
        };
        let t = match token_at_index(idx) {
            Some(t) => t,
            None => return,
        };
        if token_is_nl_or_comment_or_newline_separator(&t) {
            // Consume it
            let _ = vector_peek(vec);
        } else {
            break;
        }
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    parser_ignore_nl_or_comments(process, &mut None);
    let vec = process.token_vec.as_mut()?;
    let bytes = vector_peek_no_increment(vec)?.to_vec();
    let idx = bytes_to_index(&bytes)?;
    token_at_index(idx)
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    parser_ignore_nl_or_comments(process, &mut None);
    let next_token = {
        let vec = process.token_vec.as_mut()?;
        let bytes = vector_peek_no_increment(vec)?.to_vec();
        let idx = bytes_to_index(&bytes)?;
        token_at_index(idx)?
    };
    process.pos = next_token.pos.clone();
    *parser_last_token = Some(next_token.clone());
    // Now consume.
    if let Some(vec) = process.token_vec.as_mut() {
        let _ = vector_peek(vec);
    }
    Some(next_token)
}

/// We create a placeholder function that returns a default token. In a real parser, we'd decode real data.
fn next_token_placeholder(_vec: &mut crate::vector::Vector, pos: &crate::compiler::Pos) -> Option<Token> {
    let mut t = Token::default();
    t.pos = pos.clone();
    Some(t)
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    let token = match token_next(process, parser_last_token) {
        Some(t) => t,
        None => return,
    };
    let mut n = Node::default();
    match token.r#type {
        x if x == TOKEN_TYPE_NUMBER => {
            n.r#type = NODE_TYPE_NUMBER;
            n.llnum = token.llnum;
        }
        x if x == TOKEN_TYPE_IDENTIFIER => {
            n.r#type = NODE_TYPE_IDENTIFIER;
            n.sval = token.sval.clone();
        }
        x if x == TOKEN_TYPE_STRING => {
            n.r#type = NODE_TYPE_STRING;
            n.sval = token.sval.clone();
        }
        _ => {
            return;
        }
    }
    let _ = node_create(&n);
}

/// parse_next in the original code. Returns 0 if we handled a token, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = match token_peek_no_increment(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        x if x == TOKEN_TYPE_NUMBER
            || x == TOKEN_TYPE_IDENTIFIER
            || x == TOKEN_TYPE_STRING =>
        {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {
            // Skip unknown token to make progress.
            if let Some(vec) = process.token_vec.as_mut() {
                let _ = vector_peek(vec);
            }
        }
    }
    0
}

/// The main parse function. We set node vectors, then repeatedly call parse_next until no tokens remain.
pub fn parse(process: &mut CompileProcess) -> i32 {
    let _ = next_token_placeholder; // suppress dead-code warning
    // Reset peek pointer
    if let Some(vec) = process.token_vec.as_mut() {
        vector_set_peek_pointer(vec, 0);
    }
    // Set node vectors (move out of process, will set back).
    let node_vec = process.node_vec.take().unwrap_or_else(|| crate::vector::vector_create(std::mem::size_of::<u64>()));
    let node_tree_vec = process
        .node_tree_vec
        .take()
        .unwrap_or_else(|| crate::vector::vector_create(std::mem::size_of::<u64>()));
    node_set_vector(node_vec, node_tree_vec);

    let mut parser_last_token: Option<Token> = None;
    while parse_next(process, &mut parser_last_token) == 0 {
        let _node = node_peek();
        // Push the index of the last node into the node_tree_vec via the global state.
        // We use the same trick as in node_pop: peek the index from node_vector and push to node_vector_root.
        push_last_node_to_root();
        // Termination: if no more tokens, parse_next returns -1.
        if process
            .token_vec
            .as_mut()
            .and_then(|v| vector_peek_no_increment(v).map(|_| ()))
            .is_none()
        {
            break;
        }
    }
    PARSE_ALL_OK
}

/// Helper that takes the last index from NODE_VECTOR and pushes it to NODE_VECTOR_ROOT.
fn push_last_node_to_root() {
    use crate::node::{__internal_push_root_with_back};
    __internal_push_root_with_back();
}
