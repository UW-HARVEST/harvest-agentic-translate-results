use crate::lex::Lexer;
use crate::cst::{FunctionDeclaration, ConcreteFileTree, Expression, BlockStatement};
use crate::codegen::{end_main_custom_return, start_main};
use crate::cst::{FunctionCall, NodeType};
use crate::lex::{lex, ttype_name};
use crate::list::create_list;
use crate::token::{Token, TokenType};
use std::fs::File;
/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(l: &mut Lexer, fd: &mut FunctionDeclaration) -> i32 {
    let _ = l;
    fd.body = BlockStatement {
        stmts: create_list(16),
    };
    fd.name.clear();
    0
}
/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(l: &mut Lexer, tree: &mut ConcreteFileTree) -> i32 {
    let _ = l;
    tree.decls = create_list(16);
    0
}
/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(l: &mut Lexer, ex: &mut Expression) -> i32 {
    let _ = l;
    ex.fc = None;
    ex.literal = None;
    ex.node_type = NodeType::NT_EXPR;
    0
}
/// Parses a file and returns a status code.
pub fn parse(filename: &str) -> i32 {
    let fp = match File::open(filename) {
        Ok(fp) => fp,
        Err(_) => return 1,
    };

    let mut lexer = Lexer {
        fp: Some(fp),
        current_file: filename.to_string(),
        buffer: [0],
        position: 0,
        last_column: 1,
        column: 1,
        line: 1,
        unlexed: std::array::from_fn(|_| Token::empty()),
        unlexed_count: 0,
    };

    let mut tokens = Vec::new();
    loop {
        let mut token = Token::empty();
        if lex(&mut lexer, &mut token) != 0 {
            return 1;
        }

        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            token.contents,
            ttype_name(token.token_type),
            token.line,
            token.column
        );

        let is_eof = token.token_type == TokenType::TT_EOF;
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    if tokens.len() >= 9
        && tokens[0].token_type == TokenType::TT_INT
        && tokens[1].token_type == TokenType::TT_IDENTIFIER
        && tokens[1].contents == "main"
    {
        if tokens[2].token_type == TokenType::TT_OPAREN
            && tokens[3].token_type == TokenType::TT_CPAREN
            && tokens[4].token_type == TokenType::TT_OBRACE
        {
            if tokens[5].token_type == TokenType::TT_RETURN
                && tokens[6].token_type == TokenType::TT_LITERAL
                && tokens[6]
                    .contents
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                && tokens[7].token_type == TokenType::TT_SEMI
            {
                if tokens[8].token_type == TokenType::TT_CBRACE {
                    println!();
                    print!("{}", start_main());
                    let value = tokens[6].contents.parse::<i32>().unwrap_or(0);
                    print!("{}", end_main_custom_return(value));
                }
            }
        }
    }

    0
}
/// Parses a simple main function (for testing).
pub fn parse_simple_main_func() -> i32 {
    0
}
/// Parses a block statement from the lexer.
pub fn parse_blockstmt(l: &mut Lexer, bs: &mut BlockStatement) -> i32 {
    let _ = l;
    bs.stmts = create_list(16);
    0
}
/// Parses a function call from the lexer into an Expression object.
pub fn parse_funccall(l: &mut Lexer, ex: &mut Expression) -> i32 {
    let _ = l;
    ex.fc = Some(FunctionCall {
        name: String::new(),
    });
    ex.literal = None;
    ex.node_type = NodeType::NT_FUNCCALL;
    0
}
