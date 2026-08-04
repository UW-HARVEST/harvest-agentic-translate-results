use crate::{token, ast};
use std::cell::RefCell;

pub const SIMPLE_LANG_PARSER_H: bool = true;

// Replicates the static globals in parser.c:
// static Token* tokens;
// static int pos;
thread_local! {
    static TOKENS: RefCell<Vec<token::Token>> = RefCell::new(Vec::new());
    static POS: RefCell<usize> = RefCell::new(0);
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    TOKENS.with(|tokens| {
        POS.with(|pos| {
            let p = *pos.borrow();
            let t = tokens.borrow()[p].clone();
            *pos.borrow_mut() = p + 1;
            t
        })
    })
}

/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    TOKENS.with(|tokens| {
        POS.with(|pos| {
            let p = *pos.borrow();
            tokens.borrow()[p].clone()
        })
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
    let mut tok = lookahead();
    while tok.token_type == token::TokenType::TOKEN_PLUS
        || tok.token_type == token::TokenType::TOKEN_MINUS
    {
        let op_tok = consume();
        let right = parse_primary();
        let mut node = ast::new_ast_node(op_tok.token_type, &op_tok.value);
        node.left = Some(left);
        node.right = Some(right);
        left = node;
        tok = lookahead();
    }
    left
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
    } else if tok.token_type == token::TokenType::TOKEN_DIS {
        let expr = parse_expression();
        // In C: new_ast_node(TOKEN_DIS, NULL) -> value is NULL.
        // We use empty string here (represented as Rust String).
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

/// Replicates: ASTNode** parse(Token* tokens);
/// In Rust: returns a vector of Box<ASTNode>.
pub fn parse(tokens_array: &[token::Token]) -> Vec<Box<ast::ASTNode>> {
    TOKENS.with(|tokens| {
        *tokens.borrow_mut() = tokens_array.to_vec();
    });
    POS.with(|pos| {
        *pos.borrow_mut() = 0;
    });

    let mut ast_nodes: Vec<Box<ast::ASTNode>> = Vec::new();
    loop {
        let cur = TOKENS.with(|tokens| {
            POS.with(|pos| {
                let p = *pos.borrow();
                tokens.borrow()[p].token_type.clone()
            })
        });
        if cur == token::TokenType::TOKEN_EOF {
            break;
        }
        ast_nodes.push(parse_statement());
    }
    ast_nodes
}
