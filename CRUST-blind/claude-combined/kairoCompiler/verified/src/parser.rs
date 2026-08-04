use crate::compiler::{
    CompileProcess, Token, TOKEN_TYPE_NUMBER, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_STRING,
    PARSE_ALL_OK, NODE_TYPE_NUMBER, NODE_TYPE_IDENTIFIER, NODE_TYPE_STRING, Pos,
};
use crate::node::{
    node_create, node_set_vector, node_peek, Node,
};
use crate::vector::{vector_create, vector_push};

thread_local! {
    static PARSER_TOKENS: std::cell::RefCell<Vec<Token>> = std::cell::RefCell::new(Vec::new());
    static PARSER_INDEX: std::cell::RefCell<usize> = std::cell::RefCell::new(0);
    static PARSED_NODES: std::cell::RefCell<Vec<Node>> = std::cell::RefCell::new(Vec::new());
}

fn token_is_nl_or_comment_or_newline_separator_local(token: &Token) -> bool {
    use crate::compiler::{TOKEN_TYPE_NEWLINE, TOKEN_TYPE_COMMENT, TOKEN_TYPE_SYMBOL};
    token.r#type == TOKEN_TYPE_NEWLINE
        || token.r#type == TOKEN_TYPE_COMMENT
        || (token.r#type == TOKEN_TYPE_SYMBOL && token.cval == Some('\\'))
}

/// Skips newline/comment tokens from the front.
fn parser_ignore_nl_or_comments(_process: &mut CompileProcess, _token_opt: &mut Option<Token>) {
    PARSER_INDEX.with(|pi| {
        PARSER_TOKENS.with(|tokens| {
            let toks = tokens.borrow();
            let mut idx = pi.borrow_mut();
            while *idx < toks.len() && token_is_nl_or_comment_or_newline_separator_local(&toks[*idx]) {
                *idx += 1;
            }
        });
    });
}

/// Returns the next token without consuming it.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    parser_ignore_nl_or_comments(process, &mut None);
    PARSER_INDEX.with(|pi| {
        PARSER_TOKENS.with(|tokens| {
            let toks = tokens.borrow();
            let idx = *pi.borrow();
            toks.get(idx).cloned()
        })
    })
}

/// Returns the next token with increment.
fn token_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> Option<Token> {
    parser_ignore_nl_or_comments(process, &mut None);
    let result = PARSER_INDEX.with(|pi| {
        PARSER_TOKENS.with(|tokens| {
            let toks = tokens.borrow();
            let idx = *pi.borrow();
            toks.get(idx).cloned()
        })
    });
    if let Some(t) = &result {
        process.pos = t.pos.clone();
        *parser_last_token = Some(t.clone());
    }
    PARSER_INDEX.with(|pi| {
        let mut idx = pi.borrow_mut();
        *idx += 1;
    });
    result
}

/// Placeholder helper.
fn next_token_placeholder(_vec: &mut crate::vector::Vector, _pos: &Pos) -> Option<Token> {
    None
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) {
    let token = match token_next(process, parser_last_token) {
        Some(t) => t,
        None => return,
    };
    let node = match token.r#type {
        x if x == TOKEN_TYPE_NUMBER => Node {
            r#type: NODE_TYPE_NUMBER,
            llnum: token.llnum,
            ..Default::default()
        },
        x if x == TOKEN_TYPE_IDENTIFIER => Node {
            r#type: NODE_TYPE_IDENTIFIER,
            sval: token.sval.clone(),
            ..Default::default()
        },
        x if x == TOKEN_TYPE_STRING => Node {
            r#type: NODE_TYPE_STRING,
            sval: token.sval.clone(),
            ..Default::default()
        },
        _ => return,
    };
    node_create(&node);
}

/// parse_next in the original code.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = token_peek_no_increment(process);
    let token = match token {
        Some(t) => t,
        None => return -1,
    };
    let res = 0;
    match token.r#type {
        x if x == TOKEN_TYPE_NUMBER || x == TOKEN_TYPE_IDENTIFIER || x == TOKEN_TYPE_STRING => {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {
            // Consume non-handled tokens to avoid infinite loop on tokens like SYMBOL/OPERATOR.
            PARSER_INDEX.with(|pi| {
                let mut idx = pi.borrow_mut();
                *idx += 1;
            });
        }
    }
    res
}

/// The main parse function.
pub fn parse(process: &mut CompileProcess) -> i32 {
    // Reset parser state.
    PARSER_TOKENS.with(|tokens| tokens.borrow_mut().clear());
    PARSER_INDEX.with(|pi| *pi.borrow_mut() = 0);
    PARSED_NODES.with(|pn| pn.borrow_mut().clear());

    // Pull all lexed tokens from the lexer into our parser state.
    let lexed = crate::lexer::lex_get_tokens();
    PARSER_TOKENS.with(|tokens| {
        *tokens.borrow_mut() = lexed;
    });

    // Set up node vectors. Element size 8 bytes for u64 indices.
    if process.node_vec.is_none() {
        process.node_vec = Some(vector_create(8));
    }
    if process.node_tree_vec.is_none() {
        process.node_tree_vec = Some(vector_create(8));
    }
    let nv = process.node_vec.clone().unwrap();
    let nvr = process.node_tree_vec.clone().unwrap();
    node_set_vector(nv, nvr);

    let mut parser_last_token: Option<Token> = None;
    while parse_next(process, &mut parser_last_token) == 0 {
        // Read peek of node and push to tree vec — analogous to vector_push(node_tree_vec, &node)
        let n = node_peek();
        PARSED_NODES.with(|pn| pn.borrow_mut().push(n.clone()));
        let idx = PARSED_NODES.with(|pn| pn.borrow().len() as u64 - 1);
        let bytes = idx.to_le_bytes();
        if let Some(tree) = process.node_tree_vec.as_mut() {
            vector_push(tree, &bytes);
        }
    }

    PARSE_ALL_OK
}

/// Returns the parsed nodes from the most recent parse() call (test helper).
pub fn parse_get_nodes() -> Vec<Node> {
    PARSED_NODES.with(|pn| pn.borrow().clone())
}

// Suppress dead-code on helpers that aren't currently used.
#[allow(dead_code)]
fn _unused_marker() {
    let _ = next_token_placeholder;
}
