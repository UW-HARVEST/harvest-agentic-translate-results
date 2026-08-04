use crate::{ast, token};
use std::cell::RefCell;
pub const SIMPLE_LANG_PARSER_H: bool = true;

thread_local! {
    static TOKENS: RefCell<Vec<token::Token>> = RefCell::new(Vec::new());
    static POS: RefCell<usize> = RefCell::new(0);
}

/// This would represent a global tokens pointer in C:
/// Token* tokens;
/// Rust does not use global mutable state in the same way, so it is omitted.
/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    let p = POS.with(|p| {
        let cur = *p.borrow();
        *p.borrow_mut() = cur + 1;
        cur
    });
    TOKENS.with(|t| t.borrow()[p].clone())
}
/// Replicates: ASTNode* parse_statement();
pub fn parse_statement() -> Box<ast::ASTNode> {
    let token = consume();
    match token.token_type {
        token::TokenType::TOKEN_LET => {
            let identifier = consume();
            if identifier.token_type != token::TokenType::TOKEN_IDENTIFIER {
                eprintln!("Expected identifier after 'let'");
                std::process::exit(1);
            }
            let next = consume();
            if next.token_type != token::TokenType::TOKEN_ASSIGN {
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
            let identifier_value = token.value.clone();
            let next = consume();
            if next.token_type != token::TokenType::TOKEN_ASSIGN {
                eprintln!("Expected '=' after identifier");
                std::process::exit(1);
            }
            let expr = parse_expression();
            let semi = consume();
            if semi.token_type != token::TokenType::TOKEN_SEMICOLON {
                eprintln!("Expected ';' at end of statement");
                std::process::exit(1);
            }
            let mut node = ast::new_ast_node(token::TokenType::TOKEN_ASSIGN, &identifier_value);
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
            eprintln!("Unexpected token: {}", token.value);
            std::process::exit(1);
        }
    }
}
/// Replicates: ASTNode* parse_primary();
pub fn parse_primary() -> Box<ast::ASTNode> {
    let token = consume();
    match token.token_type {
        token::TokenType::TOKEN_INT => {
            ast::new_ast_node(token::TokenType::TOKEN_INT, &token.value)
        }
        token::TokenType::TOKEN_IDENTIFIER => {
            ast::new_ast_node(token::TokenType::TOKEN_IDENTIFIER, &token.value)
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
    TOKENS.with(|t| {
        *t.borrow_mut() = tokens_array.to_vec();
    });
    POS.with(|p| {
        *p.borrow_mut() = 0;
    });

    let mut ast_nodes: Vec<Box<ast::ASTNode>> = Vec::new();

    loop {
        let cur = lookahead();
        if cur.token_type == token::TokenType::TOKEN_EOF {
            break;
        }
        ast_nodes.push(parse_statement());
    }

    ast_nodes
}
/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    let p = POS.with(|p| *p.borrow());
    TOKENS.with(|t| t.borrow()[p].clone())
}
