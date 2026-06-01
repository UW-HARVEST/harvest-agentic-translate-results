use crate::compiler::{
    CompileProcess, Token, NODE_TYPE_IDENTIFIER, NODE_TYPE_NUMBER, NODE_TYPE_STRING,
    PARSE_ALL_OK, TOKEN_TYPE_IDENTIFIER, TOKEN_TYPE_NUMBER, TOKEN_TYPE_STRING,
};
use crate::node::{node_create, node_peek, node_set_vector, Node as NNode, NodeBinded as NBinded};
use crate::token::token_is_nl_or_comment_or_newline_separator;
use crate::vector::{vector_create, vector_push_node};

fn nnode_default() -> NNode {
    NNode {
        r#type: 0,
        flags: 0,
        pos: crate::compiler::Pos::default(),
        binded: NBinded { owner: None, function: None },
        cval: None,
        sval: None,
        inum: None,
        lnum: None,
        llnum: None,
    }
}

fn nnode_to_compiler(n: &NNode) -> crate::compiler::Node {
    crate::compiler::Node {
        r#type: n.r#type,
        flags: n.flags,
        pos: n.pos.clone(),
        binded: crate::compiler::NodeBinded::default(),
        cval: n.cval,
        sval: n.sval.clone(),
        inum: n.inum,
        lnum: n.lnum,
        llnum: n.llnum,
    }
}

/// Skips newline/comment tokens from the front by advancing the peek pointer.
fn parser_ignore_nl_or_comments(process: &mut CompileProcess, token_opt: &mut Option<Token>) {
    while let Some(t) = token_opt {
        if !token_is_nl_or_comment_or_newline_separator(t) {
            break;
        }
        if let Some(tv) = process.token_vec.as_mut() {
            crate::vector::vector_peek_token(tv);
            *token_opt = crate::vector::vector_peek_token_no_increment(tv).cloned();
        } else {
            *token_opt = None;
        }
    }
}

/// Returns the next token without consuming it, ignoring newlines/comments.
fn token_peek_no_increment(process: &mut CompileProcess) -> Option<Token> {
    let mut next = process
        .token_vec
        .as_ref()
        .and_then(|tv| crate::vector::vector_peek_token_no_increment(tv).cloned());
    parser_ignore_nl_or_comments(process, &mut next);
    process
        .token_vec
        .as_ref()
        .and_then(|tv| crate::vector::vector_peek_token_no_increment(tv).cloned())
}

/// Returns the next token with increment, ignoring newlines/comments.
fn token_next(
    process: &mut CompileProcess,
    parser_last_token: &mut Option<Token>,
) -> Option<Token> {
    let mut next = process
        .token_vec
        .as_ref()
        .and_then(|tv| crate::vector::vector_peek_token_no_increment(tv).cloned());
    parser_ignore_nl_or_comments(process, &mut next);
    if let Some(t) = &next {
        process.pos = t.pos.clone();
    }
    *parser_last_token = next;
    if let Some(tv) = process.token_vec.as_mut() {
        crate::vector::vector_peek_token(tv)
    } else {
        None
    }
}

/// We create a placeholder function that returns a default token. In a real parser, we'd decode real data.
fn next_token_placeholder(_vec: &mut crate::vector::Vector, pos: &crate::compiler::Pos) -> Option<Token> {
    let mut t = Token::default();
    t.pos = pos.clone();
    Some(t)
}

/// Single token -> AST node creation.
fn parse_single_token_to_node(
    process: &mut CompileProcess,
    parser_last_token: &mut Option<Token>,
) {
    let token = token_next(process, parser_last_token);
    if let Some(t) = token {
        let mut tmpl = nnode_default();
        match t.r#type {
            x if x == TOKEN_TYPE_NUMBER => {
                tmpl.r#type = NODE_TYPE_NUMBER;
                tmpl.llnum = t.llnum;
                tmpl.cval = t.cval;
                node_create(&tmpl);
            }
            x if x == TOKEN_TYPE_IDENTIFIER => {
                tmpl.r#type = NODE_TYPE_IDENTIFIER;
                tmpl.sval = t.sval.clone();
                node_create(&tmpl);
            }
            x if x == TOKEN_TYPE_STRING => {
                tmpl.r#type = NODE_TYPE_STRING;
                tmpl.sval = t.sval.clone();
                node_create(&tmpl);
            }
            _ => {}
        }
    }
}

/// parse_next: returns 0 if a token was processed, -1 if none left.
fn parse_next(process: &mut CompileProcess, parser_last_token: &mut Option<Token>) -> i32 {
    let token = token_peek_no_increment(process);
    let Some(t) = token else { return -1; };
    match t.r#type {
        x if x == TOKEN_TYPE_NUMBER
            || x == TOKEN_TYPE_IDENTIFIER
            || x == TOKEN_TYPE_STRING =>
        {
            parse_single_token_to_node(process, parser_last_token);
        }
        _ => {
            // Skip over tokens that we don't currently handle.
            let _ = token_next(process, parser_last_token);
        }
    }
    0
}

/// The main parse function.
pub fn parse(process: &mut CompileProcess) -> i32 {
    let v = vector_create(0);
    let r = vector_create(0);
    node_set_vector(v, r);

    let mut parser_last_token: Option<Token> = None;
    if let Some(tv) = process.token_vec.as_mut() {
        crate::vector::vector_set_peek_pointer(tv, 0);
    }

    while parse_next(process, &mut parser_last_token) == 0 {
        let n = node_peek();
        let cn = nnode_to_compiler(&n);
        if let Some(tree) = process.node_tree_vec.as_mut() {
            vector_push_node(tree, cn);
        }
    }
    PARSE_ALL_OK
}

#[allow(dead_code)]
fn _use_placeholder(p: &crate::compiler::Pos) -> Option<Token> {
    next_token_placeholder(&mut vector_create(0), p)
}
