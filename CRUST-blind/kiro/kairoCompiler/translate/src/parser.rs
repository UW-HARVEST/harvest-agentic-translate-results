use crate::compiler::{
    compiler_error, CompileProcess, Token,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    PARSE_ALL_OK, PARSE_GENERAL_ERROR,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
};
use crate::vector::{
    vector_peek, vector_peek_no_increment, vector_set_peek_pointer,
    vector_peek_ptr, vector_push, vector_pop, vector_back, vector_count, vector_empty,
};
use crate::node::{
    node_create, node_set_vector, node_peek, node_peek_or_null, node_pop, node_push as node_stack_push,
    Node,
};
use crate::compiler::Pos;
use crate::lexer::TOKENS;

fn deserialize_token(bytes: &[u8]) -> Token {
    if bytes.len() >= 8 {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[..8]);
        let idx = u64::from_le_bytes(arr) as usize;
        let tokens = TOKENS.lock().unwrap();
        if idx < tokens.len() {
            return tokens[idx].clone();
        }
    }
    Token::default()
}

fn token_is_nl_or_comment_or_newline_separator(token: &Token) -> bool {
    crate::token::token_is_nl_or_comment_or_newline_separator(token)
}

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(ref token) = token_opt {
        if !token_is_nl_or_comment_or_newline_separator(token) {
            break;
        }
        // Skip this token by peeking (consuming it)
        if let Some(ref mut tv) = process.token_vec {
            vector_peek(tv);
        }
        // Get next token
        *token_opt = peek_token_from_vec(process);
    }
}

fn peek_token_from_vec(process: &mut CompileProcess) -> Option<Token> {
    if let Some(ref mut tv) = process.token_vec {
        if let Some(bytes) = vector_peek_no_increment(tv) {
            return Some(deserialize_token(bytes));
        }
    }
    None
}

fn consume_token_from_vec(process: &mut CompileProcess) -> Option<Token> {
    if let Some(ref mut tv) = process.token_vec {
        if let Some(bytes) = vector_peek(tv) {
            return Some(deserialize_token(bytes));
        }
    }
    None
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let mut next_token = peek_token_from_vec(process);
    parser_ignore_nl_or_comments(process, &mut next_token);
    peek_token_from_vec(process)
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let mut next_token = peek_token_from_vec(process);
    parser_ignore_nl_or_comments(process, &mut next_token);
    // Update position
    if let Some(ref t) = next_token {
        process.pos = t.pos.clone();
    }
    *parser_last_token = next_token;
    consume_token_from_vec(process)
}

/// We create a placeholder function that returns a default token.
fn next_token_placeholder(_vec: &mut crate::vector::Vector, pos: &Pos) -> Option<Token> {
    None
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    let token = match token_next(process, parser_last_token) {
        Some(t) => t,
        None => return,
    };
    match token.r#type {
        TOKEN_TYPE_NUMBER => {
            node_create(&Node {
                r#type: NODE_TYPE_NUMBER,
                llnum: token.llnum,
                ..Node::default()
            });
        }
        TOKEN_TYPE_IDENTIFIER => {
            node_create(&Node {
                r#type: NODE_TYPE_IDENTIFIER,
                sval: token.sval.clone(),
                ..Node::default()
            });
        }
        TOKEN_TYPE_STRING => {
            node_create(&Node {
                r#type: NODE_TYPE_STRING,
                sval: token.sval.clone(),
                ..Node::default()
            });
        }
        _ => {
            compiler_error(process, "This is not a token that can be converted to a node");
        }
    }
}

/// parse_next: Returns 0 if we handled a token, -1 if none left.
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

/// The main parse function.
pub fn parse(process: &mut CompileProcess) -> i32 {
    let mut parser_last_token: Option<Token> = None;

    // Set node vectors
    let node_vec = process.node_vec.take().unwrap_or_else(|| crate::vector::vector_create(8));
    let node_tree_vec = process.node_tree_vec.take().unwrap_or_else(|| crate::vector::vector_create(8));
    node_set_vector(node_vec, node_tree_vec);

    if let Some(ref mut tv) = process.token_vec {
        vector_set_peek_pointer(tv, 0);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let node = node_peek();
        // Encode node index and push to node_tree_vec
        // Since node_tree_vec is now in the global, we push via node module
        // The C code does: vector_push(process->node_tree_vec, &node);
        // But node_tree_vec is now in the global NODE_VECTOR_ROOT
        // Actually in C, node_create already pushes to node_vector.
        // Then parse pushes to node_tree_vec separately.
        // We need to push to the root vec too.
        // node_tree_vec is the root vec set via node_set_vector.
        // Let's push to it via the global.
        {
            let mut nvr = crate::node::NODE_VECTOR_ROOT.lock().unwrap();
            if let Some(ref mut rv) = *nvr {
                // Encode the node - find its index in NODES
                let nodes = crate::node::NODES.lock().unwrap();
                let idx = if nodes.is_empty() { 0 } else { nodes.len() - 1 };
                let encoded = (idx as u64).to_le_bytes();
                vector_push(rv, &encoded);
            }
        }
    }

    PARSE_ALL_OK
}
