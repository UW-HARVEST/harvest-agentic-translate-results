use crate::{ast, token};
use std::cell::RefCell;

pub const SIMPLE_LANG_PARSER_H: bool = true;

// Replicates the C static globals:
//   static Token* tokens;
//   static int pos;
// In Rust we use a thread-local RefCell to keep the same API.
thread_local! {
    static PARSER_STATE: RefCell<ParserState> = RefCell::new(ParserState::default());
}

#[derive(Default)]
struct ParserState {
    tokens: Vec<token::Token>,
    pos: usize,
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    PARSER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let tok = state.tokens[state.pos].clone();
        state.pos += 1;
        tok
    })
}

/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    PARSER_STATE.with(|state| {
        let state = state.borrow();
        state.tokens[state.pos].clone()
    })
}

/// Replicates: ASTNode* parse_primary();
pub fn parse_primary() -> Box<ast::ASTNode> {
    let tok = consume();
    match tok.token_type {
        token::TokenType::TOKEN_INT => {
            ast::new_ast_node(token::TokenType::TOKEN_INT, &tok.value)
        }
        token::TokenType::TOKEN_IDENTIFIER => {
            ast::new_ast_node(token::TokenType::TOKEN_IDENTIFIER, &tok.value)
        }
        _ => {
            eprintln!("Unexpected token in primary expression: {}", tok.value);
            std::process::exit(1);
        }
    }
}

/// Replicates: ASTNode* parse_expression();
pub fn parse_expression() -> Box<ast::ASTNode> {
    let mut left = parse_primary();
    let mut next = lookahead();
    while matches!(
        next.token_type,
        token::TokenType::TOKEN_PLUS | token::TokenType::TOKEN_MINUS
    ) {
        let op = consume();
        let right = parse_primary();
        let mut node = ast::new_ast_node(op.token_type.clone(), &op.value);
        node.left = Some(left);
        node.right = Some(right);
        left = node;
        next = lookahead();
    }
    left
}

/// Replicates: ASTNode* parse_statement();
pub fn parse_statement() -> Box<ast::ASTNode> {
    let tok = consume();
    match tok.token_type {
        token::TokenType::TOKEN_LET => {
            let identifier = consume();
            if identifier.token_type != token::TokenType::TOKEN_IDENTIFIER {
                eprintln!("Expected identifier after 'let'");
                std::process::exit(1);
            }
            if consume().token_type != token::TokenType::TOKEN_ASSIGN {
                eprintln!("Expected '=' after identifier");
                std::process::exit(1);
            }
            let expr = parse_expression();
            if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            let mut node = ast::new_ast_node(token::TokenType::TOKEN_LET, &identifier.value);
            node.left = Some(expr);
            node
        }
        token::TokenType::TOKEN_IDENTIFIER => {
            let identifier = tok;
            if consume().token_type != token::TokenType::TOKEN_ASSIGN {
                eprintln!("Expected '=' after identifier");
                std::process::exit(1);
            }
            let expr = parse_expression();
            if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            let mut node = ast::new_ast_node(token::TokenType::TOKEN_ASSIGN, &identifier.value);
            node.left = Some(expr);
            node
        }
        token::TokenType::TOKEN_DIS => {
            let expr = parse_expression();
            // The C version creates the node with NULL value before checking the
            // semicolon; in Rust we emulate the empty value.
            let mut node = ast::new_ast_node(token::TokenType::TOKEN_DIS, "");
            if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            node.left = Some(expr);
            node
        }
        _ => {
            eprintln!("Unexpected token: {}", tok.value);
            std::process::exit(1);
        }
    }
}

/// Replicates: ASTNode** parse(Token* tokens);
/// In Rust: returns a vector of Box<ASTNode>.
pub fn parse(tokens_array: &[token::Token]) -> Vec<Box<ast::ASTNode>> {
    PARSER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.tokens = tokens_array.to_vec();
        state.pos = 0;
    });

    let mut ast_nodes: Vec<Box<ast::ASTNode>> = Vec::new();

    loop {
        let at_eof = PARSER_STATE.with(|state| {
            let state = state.borrow();
            state.tokens[state.pos].token_type == token::TokenType::TOKEN_EOF
        });
        if at_eof {
            break;
        }
        ast_nodes.push(parse_statement());
    }

    ast_nodes
}
