use crate::{ast, token};
use crate::token::TokenType;
use std::cell::RefCell;
pub const SIMPLE_LANG_PARSER_H: bool = true;

// Replicate C's static globals: `static Token* tokens; static int pos;`.
// We use a thread-local RefCell to keep mutable parser state without `unsafe`.
thread_local! {
    static PARSER_TOKENS: RefCell<Vec<token::Token>> = RefCell::new(Vec::new());
    static PARSER_POS: RefCell<usize> = RefCell::new(0);
}

/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    PARSER_POS.with(|p| {
        let pos = *p.borrow();
        PARSER_TOKENS.with(|t| t.borrow()[pos].clone())
    })
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    PARSER_POS.with(|p| {
        let pos = *p.borrow();
        let tok = PARSER_TOKENS.with(|t| t.borrow()[pos].clone());
        *p.borrow_mut() = pos + 1;
        tok
    })
}

/// Replicates: ASTNode* parse_primary();
pub fn parse_primary() -> Box<ast::ASTNode> {
    let token = consume();
    match token.token_type {
        TokenType::TOKEN_INT => ast::new_ast_node(TokenType::TOKEN_INT, &token.value),
        TokenType::TOKEN_IDENTIFIER => {
            ast::new_ast_node(TokenType::TOKEN_IDENTIFIER, &token.value)
        }
        _ => {
            eprintln!("Unexpected token in primary expression: {}", token.value);
            std::process::exit(1);
        }
    }
}

/// Replicates: ASTNode* parse_expression();
pub fn parse_expression() -> Box<ast::ASTNode> {
    let mut left = parse_primary();
    let mut token = lookahead();
    while token.token_type == TokenType::TOKEN_PLUS
        || token.token_type == TokenType::TOKEN_MINUS
    {
        let op = consume();
        let right = parse_primary();
        let mut node = ast::new_ast_node(op.token_type.clone(), &op.value);
        node.left = Some(left);
        node.right = Some(right);
        left = node;
        token = lookahead();
    }
    left
}

/// Replicates: ASTNode* parse_statement();
pub fn parse_statement() -> Box<ast::ASTNode> {
    let token = consume();
    match token.token_type {
        TokenType::TOKEN_LET => {
            let identifier = consume();
            if identifier.token_type != TokenType::TOKEN_IDENTIFIER {
                eprintln!("Expected identifier after 'let'");
                std::process::exit(1);
            }
            if consume().token_type != TokenType::TOKEN_ASSIGN {
                eprintln!("Expected '=' after identifier");
                std::process::exit(1);
            }
            let expr = parse_expression();
            if consume().token_type != TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            let mut node = ast::new_ast_node(TokenType::TOKEN_LET, &identifier.value);
            node.left = Some(expr);
            node
        }
        TokenType::TOKEN_IDENTIFIER => {
            let identifier = token;
            if consume().token_type != TokenType::TOKEN_ASSIGN {
                eprintln!("Expected '=' after identifier");
                std::process::exit(1);
            }
            let expr = parse_expression();
            if consume().token_type != TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            let mut node = ast::new_ast_node(TokenType::TOKEN_ASSIGN, &identifier.value);
            node.left = Some(expr);
            node
        }
        TokenType::TOKEN_DIS => {
            let expr = parse_expression();
            let mut node = ast::new_ast_node(TokenType::TOKEN_DIS, "");
            if consume().token_type != TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            node.left = Some(expr);
            node
        }
        _ => {
            eprintln!("Unexpected token: {}", token.value);
            std::process::exit(1);
        }
    }
}

/// Replicates: ASTNode** parse(Token* tokens);
/// In Rust: returns a vector of Box<ASTNode>.
pub fn parse(tokens_array: &[token::Token]) -> Vec<Box<ast::ASTNode>> {
    PARSER_TOKENS.with(|t| {
        *t.borrow_mut() = tokens_array.to_vec();
    });
    PARSER_POS.with(|p| {
        *p.borrow_mut() = 0;
    });

    let mut ast_nodes: Vec<Box<ast::ASTNode>> = Vec::new();

    loop {
        let current = lookahead();
        if current.token_type == TokenType::TOKEN_EOF {
            break;
        }
        ast_nodes.push(parse_statement());
    }

    ast_nodes
}
