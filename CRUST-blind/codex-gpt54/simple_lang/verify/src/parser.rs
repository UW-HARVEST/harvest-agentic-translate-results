use crate::{token, ast};
use std::cell::RefCell;

pub const SIMPLE_LANG_PARSER_H: bool = true;
/// This would represent a global tokens pointer in C:
/// Token* tokens;
/// Rust does not use global mutable state in the same way, so it is omitted.

#[derive(Default)]
struct ParserState {
    tokens: Vec<token::Token>,
    pos: usize,
}

thread_local! {
    static PARSER_STATE: RefCell<ParserState> = RefCell::new(ParserState::default());
}

fn with_state<R>(f: impl FnOnce(&mut ParserState) -> R) -> R {
    PARSER_STATE.with(|state| f(&mut state.borrow_mut()))
}

fn parser_error(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    with_state(|state| {
        let token = state.tokens.get(state.pos).cloned().unwrap_or_else(|| token::Token {
            token_type: token::TokenType::TOKEN_EOF,
            value: String::new(),
        });
        state.pos = state.pos.saturating_add(1);
        token
    })
}
/// Replicates: ASTNode* parse_statement();
pub fn parse_statement() -> Box<ast::ASTNode> {
    let token = consume();
    if token.token_type == token::TokenType::TOKEN_LET {
        let identifier = consume();
        if identifier.token_type != token::TokenType::TOKEN_IDENTIFIER {
            parser_error("Expected identifier after 'let'");
        }
        if consume().token_type != token::TokenType::TOKEN_ASSIGN {
            parser_error("Expected '=' after identifier");
        }
        let expr = parse_expression();
        if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
            parser_error("Expected ';' at end of statement");
        }
        let mut node = ast::new_ast_node(token::TokenType::TOKEN_LET, &identifier.value);
        node.left = Some(expr);
        node
    } else if token.token_type == token::TokenType::TOKEN_IDENTIFIER {
        let identifier = token;
        if consume().token_type != token::TokenType::TOKEN_ASSIGN {
            parser_error("Expected '=' after identifier");
        }
        let expr = parse_expression();
        if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
            parser_error("Expected ';' at end of statement");
        }
        let mut node = ast::new_ast_node(token::TokenType::TOKEN_ASSIGN, &identifier.value);
        node.left = Some(expr);
        node
    } else if token.token_type == token::TokenType::TOKEN_DIS {
        let expr = parse_expression();
        let mut node = ast::new_ast_node(token::TokenType::TOKEN_DIS, "");
        if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
            parser_error("Expected ';' at end of statement");
        }
        node.left = Some(expr);
        node
    } else {
        parser_error(&format!("Unexpected token: {}", token.value));
    }
}
/// Replicates: ASTNode* parse_primary();
pub fn parse_primary() -> Box<ast::ASTNode> {
    let token = consume();
    if token.token_type == token::TokenType::TOKEN_INT {
        ast::new_ast_node(token::TokenType::TOKEN_INT, &token.value)
    } else if token.token_type == token::TokenType::TOKEN_IDENTIFIER {
        ast::new_ast_node(token::TokenType::TOKEN_IDENTIFIER, &token.value)
    } else {
        parser_error(&format!(
            "Unexpected token in primary expression: {}",
            token.value
        ));
    }
}
/// Replicates: ASTNode* parse_expression();
pub fn parse_expression() -> Box<ast::ASTNode> {
    let mut left = parse_primary();
    let mut token = lookahead();
    while token.token_type == token::TokenType::TOKEN_PLUS
        || token.token_type == token::TokenType::TOKEN_MINUS
    {
        token = consume();
        let right = parse_primary();
        let mut node = ast::new_ast_node(token.token_type.clone(), &token.value);
        node.left = Some(left);
        node.right = Some(right);
        left = node;
        token = lookahead();
    }
    left
}
/// Replicates: ASTNode** parse(Token* tokens);
/// In Rust: returns a vector of Box<ASTNode>.
pub fn parse(tokens_array: &[token::Token]) -> Vec<Box<ast::ASTNode>> {
    with_state(|state| {
        state.tokens = tokens_array.to_vec();
        state.pos = 0;
    });

    let mut ast_nodes = Vec::new();
    while lookahead().token_type != token::TokenType::TOKEN_EOF {
        ast_nodes.push(parse_statement());
    }
    ast_nodes
}
/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    with_state(|state| {
        state.tokens.get(state.pos).cloned().unwrap_or_else(|| token::Token {
            token_type: token::TokenType::TOKEN_EOF,
            value: String::new(),
        })
    })
}
