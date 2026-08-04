use crate::compiler::{
    compiler_error, CompileProcess, Token, NODE_TYPE_IDENTIFIER, NODE_TYPE_NUMBER,
    NODE_TYPE_STRING, PARSE_ALL_OK, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING,
};
use crate::lexer::get_token;
use crate::node::{node_create, node_peek, node_set_vector};
use crate::token::token_is_nl_or_comment_or_newline_separator;
use crate::vector::{vector_peek, vector_peek_no_increment, vector_push, vector_set_peek_pointer};

fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(token) = token_opt.clone() {
        if !token_is_nl_or_comment_or_newline_separator(&token) {
            break;
        }

        if let Some(vec) = process.token_vec.as_mut() {
            let _ = vector_peek(vec);
            *token_opt = vector_peek_no_increment(vec).and_then(|bytes| get_token(bytes));
        } else {
            *token_opt = None;
        }
    }
}

fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let mut token_opt = process
        .token_vec
        .as_mut()
        .and_then(vector_peek_no_increment)
        .and_then(|bytes| get_token(bytes));
    parser_ignore_nl_or_comments(process, &mut token_opt);
    token_opt
}

fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let mut next_token = process
        .token_vec
        .as_mut()
        .and_then(vector_peek_no_increment)
        .and_then(|bytes| get_token(bytes));
    parser_ignore_nl_or_comments(process, &mut next_token);

    if let Some(token) = next_token.clone() {
        process.pos = token.pos.clone();
        *parser_last_token = Some(token);
    }

    process
        .token_vec
        .as_mut()
        .and_then(vector_peek)
        .and_then(|bytes| get_token(bytes))
}

fn next_token_placeholder(_vec: &mut crate::vector::Vector, _pos: &crate::compiler::Pos) -> Option<Token> {
    None
}

fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    let Some(token) = token_next(process, parser_last_token) else {
        return;
    };

    let node = match token.r#type {
        TOKEN_TYPE_NUMBER => node_create(&crate::node::Node {
            r#type: NODE_TYPE_NUMBER,
            llnum: token.llnum,
            ..crate::node::Node::default()
        }),
        TOKEN_TYPE_IDENTIFIER => node_create(&crate::node::Node {
            r#type: NODE_TYPE_IDENTIFIER,
            sval: token.sval.clone(),
            ..crate::node::Node::default()
        }),
        TOKEN_TYPE_STRING => node_create(&crate::node::Node {
            r#type: NODE_TYPE_STRING,
            sval: token.sval.clone(),
            ..crate::node::Node::default()
        }),
        _ => {
            compiler_error(process, "This is not a token that can be converted to a node");
            return;
        }
    };

    if let Some(root) = process.node_tree_vec.as_mut() {
        let _ = node;
        let idx_bytes = {
            use crate::node::node_peek_or_null;
            let _ = node_peek_or_null();
            // node_create already pushed the node to the node stack, so use node_peek to recover
            // the actual cloned value and encode a fresh root entry in the process vector below.
            // The node module owns the index bookkeeping.
            None::<[u8; 8]>
        };
        let _ = idx_bytes;
        let current = node_peek();
        let _ = current;
        // Preserve the original parser side effect: tree vector receives a copy of the top node pointer.
        // We model that as another node_create-derived stack entry by reusing node_peek's current top id.
        if let Some(stack_vec) = process.node_vec.as_mut() {
            if let Some(bytes) = crate::vector::vector_back(stack_vec) {
                vector_push(root, bytes);
            }
        }
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
    let mut parser_last_token = None;
    if let (Some(node_vec), Some(node_tree_vec)) = (process.node_vec.clone(), process.node_tree_vec.clone()) {
        node_set_vector(node_vec, node_tree_vec);
    }

    if let Some(token_vec) = process.token_vec.as_mut() {
        vector_set_peek_pointer(token_vec, 0);
        let _ = next_token_placeholder(token_vec, &process.pos);
    }

    while parse_next(process, &mut parser_last_token) == 0 {}
    PARSE_ALL_OK
}
