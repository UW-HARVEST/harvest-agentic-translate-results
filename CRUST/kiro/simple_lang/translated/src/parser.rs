use crate::{token, ast};
pub const SIMPLE_LANG_PARSER_H: bool = true;

use std::cell::RefCell;

thread_local! {
    static TOKENS: RefCell<Vec<token::Token>> = RefCell::new(Vec::new());
    static POS: RefCell<usize> = RefCell::new(0);
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    TOKENS.with(|t| {
        POS.with(|p| {
            let tokens = t.borrow();
            let mut pos = p.borrow_mut();
            let tok = tokens[*pos].clone();
            *pos += 1;
            tok
        })
    })
}
/// Replicates: ASTNode* parse_statement();
pub fn parse_statement() -> Box<ast::ASTNode> {
    let tok = consume();
    if tok.token_type == token::TokenType::TOKEN_LET {
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
    } else if tok.token_type == token::TokenType::TOKEN_IDENTIFIER {
        if consume().token_type != token::TokenType::TOKEN_ASSIGN {
            eprintln!("Expected '=' after identifier");
            std::process::exit(1);
        }
        let expr = parse_expression();
        if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
            eprintln!("Expected ';' at end of statement");
            std::process::exit(1);
        }
        let mut node = ast::new_ast_node(token::TokenType::TOKEN_ASSIGN, &tok.value);
        node.left = Some(expr);
        node
    } else if tok.token_type == token::TokenType::TOKEN_DIS {
        let expr = parse_expression();
        let mut node = ast::new_ast_node(token::TokenType::TOKEN_DIS, "");
        if consume().token_type != token::TokenType::TOKEN_SEMICOLON {
            eprintln!("Expected ';' at end of statement");
            std::process::exit(1);
        }
        node.left = Some(expr);
        node
    } else {
        eprintln!("Unexpected token: {}", tok.value);
        std::process::exit(1);
    }
}
/// Replicates: ASTNode* parse_primary();
pub fn parse_primary() -> Box<ast::ASTNode> {
    let tok = consume();
    if tok.token_type == token::TokenType::TOKEN_INT {
        ast::new_ast_node(token::TokenType::TOKEN_INT, &tok.value)
    } else if tok.token_type == token::TokenType::TOKEN_IDENTIFIER {
        ast::new_ast_node(token::TokenType::TOKEN_IDENTIFIER, &tok.value)
    } else {
        eprintln!("Unexpected token in primary expression: {}", tok.value);
        std::process::exit(1);
    }
}
/// Replicates: ASTNode* parse_expression();
pub fn parse_expression() -> Box<ast::ASTNode> {
    let mut left = parse_primary();
    loop {
        let tok = lookahead();
        if tok.token_type != token::TokenType::TOKEN_PLUS && tok.token_type != token::TokenType::TOKEN_MINUS {
            break;
        }
        let op = consume();
        let right = parse_primary();
        let mut node = ast::new_ast_node(op.token_type, &op.value);
        node.left = Some(left);
        node.right = Some(right);
        left = node;
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

    let mut ast_nodes = Vec::new();
    while lookahead().token_type != token::TokenType::TOKEN_EOF {
        ast_nodes.push(parse_statement());
    }
    ast_nodes
}
/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    TOKENS.with(|t| {
        POS.with(|p| {
            let tokens = t.borrow();
            let pos = p.borrow();
            tokens[*pos].clone()
        })
    })
}
