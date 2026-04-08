use crate::compiler::{
    compiler_error, CompileProcess, Token, Node,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING,
    PARSE_ALL_OK,
};
use crate::vector::{
    vector_peek, vector_peek_no_increment, vector_set_peek_pointer,
};
use crate::node::{
    node_create, node_set_vector, node_peek,
};
use crate::compiler::Pos;
use crate::lexer::{token_from_bytes_pub};

fn get_token(bytes: &[u8]) -> Token {
    token_from_bytes_pub(bytes)
}

fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(ref t) = token_opt {
        if !crate::token::token_is_nl_or_comment_or_newline_separator(t) {
            break;
        }
        if let Some(ref mut tv) = process.token_vec {
            vector_peek(tv);
            match vector_peek_no_increment(tv) {
                Some(bytes) => *token_opt = Some(get_token(bytes)),
                None => { *token_opt = None; break; }
            }
        } else {
            break;
        }
    }
}

fn token_peek_next(process: &mut CompileProcess) -> Option<Token> {
    let mut token_opt = if let Some(ref mut tv) = process.token_vec {
        vector_peek_no_increment(tv).map(|b| get_token(b))
    } else {
        None
    };
    parser_ignore_nl_or_comments(process, &mut token_opt);
    token_opt
}

fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let mut next_token = if let Some(ref mut tv) = process.token_vec {
        vector_peek_no_increment(tv).map(|b| get_token(b))
    } else {
        None
    };
    parser_ignore_nl_or_comments(process, &mut next_token);
    if let Some(ref t) = next_token {
        process.pos = t.pos.clone();
        *parser_last_token = Some(t.clone());
    }
    if let Some(ref mut tv) = process.token_vec {
        vector_peek(tv);
    }
    next_token
}

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
                ..Default::default()
            });
        }
        TOKEN_TYPE_IDENTIFIER => {
            node_create(&Node {
                r#type: NODE_TYPE_IDENTIFIER,
                sval: token.sval.clone(),
                ..Default::default()
            });
        }
        TOKEN_TYPE_STRING => {
            node_create(&Node {
                r#type: NODE_TYPE_STRING,
                sval: token.sval.clone(),
                ..Default::default()
            });
        }
        _ => {
            compiler_error(process, "This is not a token that can be converted to a node");
        }
    }
}

fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = match token_peek_next(process) {
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

pub fn parse(process: &mut CompileProcess) -> i32 {
    let mut parser_last_token: Option<Token> = None;

    let node_vec = process.node_vec.take().unwrap_or_else(|| crate::vector::vector_create(8));
    let node_tree_vec = process.node_tree_vec.take().unwrap_or_else(|| crate::vector::vector_create(8));
    node_set_vector(node_vec, node_tree_vec);

    if let Some(ref mut tv) = process.token_vec {
        vector_set_peek_pointer(tv, 0);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let _node = node_peek();
    }

    PARSE_ALL_OK
}
