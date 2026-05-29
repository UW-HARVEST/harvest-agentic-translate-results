use crate::{ast, token};
use std::cell::RefCell;

pub const SIMPLE_LANG_PARSER_H: bool = true;

// Replicates the C global state: `static Token* tokens; static int pos;`
// In Rust we use thread-local interior mutability to keep parser entry points
// matching the original C signatures.
thread_local! {
    static TOKENS: RefCell<Vec<token::Token>> = RefCell::new(Vec::new());
    static POS: RefCell<usize> = RefCell::new(0);
}

fn current_token() -> token::Token {
    TOKENS.with(|t| {
        POS.with(|p| {
            let pos = *p.borrow();
            let tokens = t.borrow();
            tokens[pos].clone()
        })
    })
}

fn advance() {
    POS.with(|p| {
        *p.borrow_mut() += 1;
    });
}

/// Replicates: Token* lookahead();
pub fn lookahead() -> token::Token {
    current_token()
}

/// Replicates: Token* consume();
pub fn consume() -> token::Token {
    let tok = current_token();
    advance();
    tok
}

/// Replicates: ASTNode* parse_primary();
pub fn parse_primary() -> Box<ast::ASTNode> {
    let tok = consume();
    match tok.token_type {
        token::TokenType::TOKEN_INT => ast::new_ast_node(token::TokenType::TOKEN_INT, &tok.value),
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
            // In C, `new_ast_node(TOKEN_DIS, NULL)` produces a node with NULL value;
            // we represent that as an empty string here.
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
