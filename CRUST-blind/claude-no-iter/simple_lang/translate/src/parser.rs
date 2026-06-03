use crate::{token, ast};
use std::cell::RefCell;
pub const SIMPLE_LANG_PARSER_H: bool = true;
/// This would represent a global tokens pointer in C:
/// Token* tokens;
/// Rust does not use global mutable state in the same way, so it is omitted.
thread_local! {
    static PARSER_STATE: RefCell<ParserState> = RefCell::new(ParserState::new());
}

struct ParserState {
    tokens: Vec<token::Token>,
    pos: usize,
}

impl ParserState {
    fn new() -> Self {
        ParserState {
            tokens: Vec::new(),
            pos: 0,
        }
    }
}

fn set_tokens(tokens: &[token::Token]) {
    PARSER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.tokens = tokens.to_vec();
        state.pos = 0;
    });
}

fn current_token() -> token::Token {
    PARSER_STATE.with(|state| {
        let state = state.borrow();
        if state.pos < state.tokens.len() {
            state.tokens[state.pos].clone()
        } else {
            // Mirror EOF behavior if we somehow run past the end.
            token::Token {
                token_type: token::TokenType::TOKEN_EOF,
                value: String::new(),
            }
        }
    })
}

fn advance_pos() {
    PARSER_STATE.with(|state| {
        state.borrow_mut().pos += 1;
    });
}

fn is_eof() -> bool {
    PARSER_STATE.with(|state| {
        let state = state.borrow();
        state.pos >= state.tokens.len()
            || state.tokens[state.pos].token_type == token::TokenType::TOKEN_EOF
    })
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    let tok = current_token();
    advance_pos();
    tok
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
            let assign = consume();
            if assign.token_type != token::TokenType::TOKEN_ASSIGN {
                eprintln!("Expected '=' after identifier");
                std::process::exit(1);
            }
            let expr = parse_expression();
            let semi = consume();
            if semi.token_type != token::TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            let mut node = ast::new_ast_node(token::TokenType::TOKEN_LET, &identifier.value);
            node.left = Some(expr);
            node
        }
        token::TokenType::TOKEN_IDENTIFIER => {
            let identifier = tok;
            let assign = consume();
            if assign.token_type != token::TokenType::TOKEN_ASSIGN {
                eprintln!("Expected '=' after identifier");
                std::process::exit(1);
            }
            let expr = parse_expression();
            let semi = consume();
            if semi.token_type != token::TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            let mut node = ast::new_ast_node(token::TokenType::TOKEN_ASSIGN, &identifier.value);
            node.left = Some(expr);
            node
        }
        token::TokenType::TOKEN_DIS => {
            let expr = parse_expression();
            let mut node = ast::new_ast_node(token::TokenType::TOKEN_DIS, "");
            let semi = consume();
            if semi.token_type != token::TokenType::TOKEN_SEMICOLON {
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
    let mut tok = lookahead();
    while tok.token_type == token::TokenType::TOKEN_PLUS
        || tok.token_type == token::TokenType::TOKEN_MINUS
    {
        let op_tok = consume();
        let right = parse_primary();
        let mut node = ast::new_ast_node(op_tok.token_type.clone(), &op_tok.value);
        node.left = Some(left);
        node.right = Some(right);
        left = node;
        tok = lookahead();
    }
    left
}
/// Replicates: ASTNode** parse(Token* tokens);
/// In Rust: returns a vector of Box<ASTNode>.
pub fn parse(tokens_array: &[token::Token]) -> Vec<Box<ast::ASTNode>> {
    set_tokens(tokens_array);
    let mut ast_nodes: Vec<Box<ast::ASTNode>> = Vec::new();
    while !is_eof() {
        ast_nodes.push(parse_statement());
    }
    ast_nodes
}
/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    current_token()
}
