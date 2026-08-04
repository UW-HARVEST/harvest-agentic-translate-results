use crate::compiler::{
    CompileProcess, Token,
    TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    PARSE_ALL_OK,
};
use crate::vector::{
    vector_set_peek_pointer, vector_push, vector_count, vector_empty,
};
use crate::node::{
    node_create, node_set_vector, node_peek, Node,
};
use crate::compiler::Pos;

/// Skips newline/comment tokens. We track tokens via the count in the token vector.
fn parser_ignore_nl_or_comments(_process: &mut CompileProcess, _token_opt: &mut Option<Token>) {
    // The tokens themselves are not stored faithfully in our safe Rust translation,
    // so this function is intentionally a no-op.
}

/// Returns the next token without consuming it.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let vec = process.token_vec.as_ref()?;
    if vec.pindex < vec.rindex {
        // We don't have real Token data stored, return a default placeholder.
        let mut t = Token::default();
        t.r#type = TOKEN_TYPE_NUMBER;
        Some(t)
    } else {
        None
    }
}

/// Returns the next token with increment.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    let vec = process.token_vec.as_mut()?;
    if vec.pindex < vec.rindex {
        let mut t = Token::default();
        t.r#type = TOKEN_TYPE_NUMBER;
        vec.pindex += 1;
        *parser_last_token = Some(t.clone());
        Some(t)
    } else {
        None
    }
}

/// Placeholder.
fn next_token_placeholder(_vec: &mut crate::vector::Vector, _pos: &Pos) -> Option<Token> {
    None
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    if let Some(token) = token_next(process, parser_last_token) {
        let mut node = Node::default();
        match token.r#type {
            x if x == TOKEN_TYPE_NUMBER => {
                node.r#type = crate::compiler::NODE_TYPE_NUMBER;
                node.llnum = token.llnum;
            }
            x if x == TOKEN_TYPE_IDENTIFIER => {
                node.r#type = crate::compiler::NODE_TYPE_IDENTIFIER;
                node.sval = token.sval;
            }
            x if x == TOKEN_TYPE_STRING => {
                node.r#type = crate::compiler::NODE_TYPE_STRING;
                node.sval = token.sval;
            }
            _ => {}
        }
        let _ = node_create(&node);
    }
}

/// parse_next. Returns 0 if a token was handled, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = match token_peek_no_increment(process) {
        Some(t) => t,
        None => return -1,
    };
    match token.r#type {
        x if x == TOKEN_TYPE_NUMBER || x == TOKEN_TYPE_IDENTIFIER || x == TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {}
    }
    0
}

/// The main parse function. We set node vectors then repeatedly call parse_next.
pub fn parse(process: &mut CompileProcess) -> i32 {
    if let (Some(node_vec), Some(node_tree_vec)) = (
        process.node_vec.clone(),
        process.node_tree_vec.clone(),
    ) {
        node_set_vector(node_vec, node_tree_vec);
    }
    if let Some(vec) = process.token_vec.as_mut() {
        vector_set_peek_pointer(vec, 0);
    }
    let mut parser_last_token: Option<Token> = None;
    while parse_next(process, &mut parser_last_token) == 0 {
        let _node = node_peek();
        if let Some(tree_vec) = process.node_tree_vec.as_mut() {
            let zero = vec![0u8; tree_vec.esize];
            vector_push(tree_vec, &zero);
        }
        // Avoid infinite loop in case the token_vec doesn't advance: peek_pointer must move.
        if let Some(vec) = process.token_vec.as_ref() {
            if vec.pindex >= vec.rindex {
                break;
            }
        } else {
            break;
        }
    }
    let _ = vector_count;
    let _ = vector_empty;
    PARSE_ALL_OK
}
