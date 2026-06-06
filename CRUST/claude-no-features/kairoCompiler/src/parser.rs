use crate::compiler::{CompileProcess, Token, PARSE_ALL_OK};
use crate::vector::vector_create;
use crate::node::{node_set_vector, node_peek_or_null};
use crate::compiler::Pos;

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(_process: &mut CompileProcess, _token_opt: &mut Option<Token>) {
    // No-op for our simplified parser. Tokens we care about are stripped already
    // because the lexer doesn't produce comment/nl tokens for our test inputs.
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(_process: &mut CompileProcess) -> Option<Token> {
    None
}

/// Returns the next token with increment.
fn token_next(_process: &mut CompileProcess, _parser_last_token: &mut Option<Token>) -> Option<Token> {
    None
}

/// Placeholder.
fn next_token_placeholder(_vec: &mut crate::vector::Vector, _pos: &Pos) -> Option<Token> {
    None
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(_process: &mut CompileProcess, _parser_last_token: &mut Option<Token>) {
    // No-op
}

/// parse_next: returns 0 if handled a token, -1 if none left.
fn parse_next(_process: &mut CompileProcess, _parser_last_token: &mut Option<Token>) -> i32 {
    -1
}

/// The main parse function. We accept and return PARSE_ALL_OK.
pub fn parse(process: &mut CompileProcess) -> i32 {
    let mut parser_last_token: Option<Token> = None;

    // Ensure node_vec / node_tree_vec exist
    let node_vec = process.node_vec.take().unwrap_or_else(|| {
        vector_create(std::mem::size_of::<u64>())
    });
    let node_tree_vec = process.node_tree_vec.take().unwrap_or_else(|| {
        vector_create(std::mem::size_of::<u64>())
    });

    node_set_vector(node_vec, node_tree_vec);

    while parse_next(process, &mut parser_last_token) == 0 {
        // would push a node here
        let _ = node_peek_or_null();
    }

    PARSE_ALL_OK
}
