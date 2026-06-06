use crate::{ast, token};
use std::cell::RefCell;

pub const SIMPLE_LANG_PARSER_H: bool = true;

/// Replicates the C statics:
/// static Token* tokens;
/// static int pos;
thread_local! {
    static PARSER_TOKENS: RefCell<Vec<token::Token>> = RefCell::new(Vec::new());
    static PARSER_POS: RefCell<usize> = RefCell::new(0);
}

fn with_token_at<R>(idx: usize, f: impl FnOnce(&token::Token) -> R) -> R {
    PARSER_TOKENS.with(|tokens| {
        let tokens = tokens.borrow();
        f(&tokens[idx])
    })
}

/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    let pos = PARSER_POS.with(|p| *p.borrow());
    with_token_at(pos, |t| t.clone())
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    let pos = PARSER_POS.with(|p| {
        let cur = *p.borrow();
        *p.borrow_mut() = cur + 1;
        cur
    });
    with_token_at(pos, |t| t.clone())
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
            eprintln!(
                "Unexpected token in primary expression: {}",
                token.value
            );
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
        let op = consume();
        let right = parse_primary();
        let mut node = ast::new_ast_node(op.token_type.clone(), &op.value);
        node.left = Some(left);
        node.right = Some(right);
        left = node;
        tok = lookahead();
    }
    left
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
            let mut node =
                ast::new_ast_node(token::TokenType::TOKEN_LET, &identifier.value);
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
            let mut node =
                ast::new_ast_node(token::TokenType::TOKEN_ASSIGN, &identifier_value);
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

/// Replicates: ASTNode** parse(Token* tokens);
/// In Rust: returns a vector of Box<ASTNode>.
pub fn parse(tokens_array: &[token::Token]) -> Vec<Box<ast::ASTNode>> {
    PARSER_TOKENS.with(|t| *t.borrow_mut() = tokens_array.to_vec());
    PARSER_POS.with(|p| *p.borrow_mut() = 0);

    let mut nodes: Vec<Box<ast::ASTNode>> = Vec::new();
    loop {
        let cur = lookahead();
        if cur.token_type == token::TokenType::TOKEN_EOF {
            break;
        }
        nodes.push(parse_statement());
    }
    nodes
}
