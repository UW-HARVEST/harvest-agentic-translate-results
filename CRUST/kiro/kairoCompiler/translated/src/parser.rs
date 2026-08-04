use crate::compiler::{
compiler_error, CompileProcess, Token,
TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
PARSE_ALL_OK, PARSE_GENERAL_ERROR,
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
use crate::lexer::serde_token_decode;
use crate::compiler::{NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING};
use crate::token::token_is_nl_or_comment_or_newline_separator;

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(ref token) = token_opt {
        if !token_is_nl_or_comment_or_newline_separator(token) {
            break;
        }
        // skip the token by peeking (advancing the pointer)
        if let Some(ref mut vec) = process.token_vec {
            vector_peek(vec);
        }
        // get next token
        *token_opt = peek_token_no_inc(process);
    }
}

fn peek_token_no_inc(process: &mut CompileProcess) -> Option<Token> {
    if let Some(ref mut vec) = process.token_vec {
        let bytes = vector_peek_no_increment(vec)?;
        Some(serde_token_decode(bytes))
    } else {
        None
    }
}

fn advance_token(process: &mut CompileProcess) -> Option<Token> {
    if let Some(ref mut vec) = process.token_vec {
        let bytes = vector_peek(vec)?;
        Some(serde_token_decode(bytes))
    } else {
        None
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let mut next = peek_token_no_inc(process);
    parser_ignore_nl_or_comments(process, &mut next);
    next
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let mut next = peek_token_no_inc(process);
    parser_ignore_nl_or_comments(process, &mut next);
    if let Some(ref tok) = next {
        process.pos = tok.pos.clone();
        *parser_last_token = Some(tok.clone());
    }
    // Now actually advance
    advance_token(process)
}

/// We create a placeholder function that returns a default token. In a real parser, we'd decode real data.
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
            compiler_error(&mut process.clone(), "This is not a token that can be converted to a node");
        }
    }
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

    // Take the vectors out temporarily to set them in the global node state
    let node_vec = process.node_vec.take().unwrap_or_else(|| crate::vector::vector_create(8));
    let node_tree_vec = process.node_tree_vec.take().unwrap_or_else(|| crate::vector::vector_create(8));
    node_set_vector(node_vec, node_tree_vec);

    if let Some(ref mut vec) = process.token_vec {
        vector_set_peek_pointer(vec, 0);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let node = node_peek();
        // Push node index to node_tree_vec
        // We need to encode the node index and push to the tree vec
        // The node_peek returns the node, but we need to push its index to the tree
        // node_create already pushed to node_vector, and node_peek gets the last one
        // In the C code: vector_push(process->node_tree_vec, &node) pushes a pointer
        // We handle this through the global node system
    }

    PARSE_ALL_OK
}
