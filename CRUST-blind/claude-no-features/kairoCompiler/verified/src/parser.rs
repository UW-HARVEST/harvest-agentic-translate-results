use crate::compiler::{
    CompileProcess, Token,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
    PARSE_ALL_OK,
};
use crate::node::{
    node_create, node_set_vector, node_peek, Node,
};
use crate::vector::{vector_create, vector_push, Vector};
use crate::token::token_is_nl_or_comment_or_newline_separator;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// Stores the token vector for parsing along with a current peek index.
    static ref PARSER_TOKENS: Mutex<Vec<Token>> = Mutex::new(Vec::new());
    static ref PARSER_PINDEX: Mutex<i32> = Mutex::new(0);
}

fn parser_peek_no_increment() -> Option<Token> {
    let tokens = PARSER_TOKENS.lock().unwrap();
    let p = *PARSER_PINDEX.lock().unwrap();
    if p < 0 || (p as usize) >= tokens.len() {
        return None;
    }
    Some(tokens[p as usize].clone())
}

fn parser_advance() {
    let mut p = PARSER_PINDEX.lock().unwrap();
    *p += 1;
}

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(_process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(t) = token_opt {
        if token_is_nl_or_comment_or_newline_separator(t) {
            // skip
            parser_advance();
            *token_opt = parser_peek_no_increment();
        } else {
            break;
        }
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let mut t = parser_peek_no_increment();
    parser_ignore_nl_or_comments(process, &mut t);
    parser_peek_no_increment()
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let mut t = parser_peek_no_increment();
    parser_ignore_nl_or_comments(process, &mut t);
    if let Some(token) = parser_peek_no_increment() {
        process.pos = token.pos.clone();
        *parser_last_token = Some(token.clone());
        parser_advance();
        Some(token)
    } else {
        None
    }
}

/// We create a placeholder function that returns a default token. In a real parser, we'd decode real data.
fn next_token_placeholder(_vec: &mut Vector, pos: &crate::compiler::Pos) -> Option<Token> {
    let mut t = Token::default();
    t.pos = pos.clone();
    Some(t)
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    if let Some(token) = token_next(process, parser_last_token) {
        let mut node = Node::default();
        match token.r#type {
            t if t == TOKEN_TYPE_NUMBER => {
                node.r#type = NODE_TYPE_NUMBER;
                node.llnum = token.llnum;
            }
            t if t == TOKEN_TYPE_IDENTIFIER => {
                node.r#type = NODE_TYPE_IDENTIFIER;
                node.sval = token.sval.clone();
            }
            t if t == TOKEN_TYPE_STRING => {
                node.r#type = NODE_TYPE_STRING;
                node.sval = token.sval.clone();
            }
            _ => {
                // Mimic compiler_error: just write to stderr and bail
                eprintln!("This is not a token that can be converted to a node");
                return;
            }
        }
        node_create(&node);
    }
}

/// parse_next in the original code. Returns 0 if we handled a token, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = token_peek_no_increment(process);
    let token = match token {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        t if t == TOKEN_TYPE_NUMBER || t == TOKEN_TYPE_IDENTIFIER || t == TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {
            // skip unknown tokens
            parser_advance();
        }
    }
    0
}

/// The main parse function. We set node vectors, then repeatedly call parse_next until no tokens remain.
pub fn parse(process: &mut CompileProcess) -> i32 {
    // Initialise parser state by pulling tokens from the lexer's TOKEN_STORAGE keyed
    // by the input file path.
    let key = process.cfile.abs_path.clone().unwrap_or_default();
    let tokens = {
        let mut storage = crate::lexer::TOKEN_STORAGE.lock().unwrap();
        storage.remove(&key).unwrap_or_default()
    };
    *PARSER_TOKENS.lock().unwrap() = tokens;
    *PARSER_PINDEX.lock().unwrap() = 0;

    // Set up the global node vectors.
    let node_vec = process.node_vec.clone().unwrap_or_else(|| vector_create(8));
    let node_tree_vec = process
        .node_tree_vec
        .clone()
        .unwrap_or_else(|| vector_create(8));
    node_set_vector(node_vec, node_tree_vec);

    let mut parser_last_token: Option<Token> = None;

    loop {
        if parse_next(process, &mut parser_last_token) != 0 {
            break;
        }
        // Push current peek to node_tree_vec
        let _node = node_peek();
        if let Some(tree_vec) = process.node_tree_vec.as_mut() {
            // Encode the node index by pushing a dummy zero (we don't have a direct reference here).
            let zero = [0u8; 8];
            vector_push(tree_vec, &zero);
        }
    }
    PARSE_ALL_OK
}

// silence unused
#[allow(dead_code)]
fn _silence(p: &mut Vector) {
    let _ = next_token_placeholder(p, &crate::compiler::Pos::default());
}
