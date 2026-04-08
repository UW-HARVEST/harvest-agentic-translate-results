use std::fs::File;
use crate::lex::{Lexer, lex, lexer_getchar, lexer_ungetchar, skip_to_token, ttype_name, TOKEN_PUTBACKS};
use crate::token::{Token, TokenType};
use crate::codegen::{start_main, end_main_custom_return};
use crate::cst::{FunctionDeclaration, ConcreteFileTree, Expression, BlockStatement};

pub fn parse_funcdecl(l: &mut Lexer, fd: &mut FunctionDeclaration) -> i32 {
    // TODO stub - not fully implemented in C either
    0
}

pub fn make_cst(l: &mut Lexer, tree: &mut ConcreteFileTree) -> i32 {
    // TODO stub - not fully implemented in C either
    0
}

pub fn parse_expr(l: &mut Lexer, ex: &mut Expression) -> i32 {
    // TODO stub - not fully implemented in C either
    0
}

pub fn parse_blockstmt(l: &mut Lexer, bs: &mut BlockStatement) -> i32 {
    // TODO stub - not fully implemented in C either
    0
}

pub fn parse_funccall(l: &mut Lexer, ex: &mut Expression) -> i32 {
    // TODO stub - not fully implemented in C either
    0
}

pub fn parse_simple_main_func() -> i32 {
    // Empty in C as well
    0
}

pub fn parse(filename: &str) -> i32 {
    let fp = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("\x1b[31mError: jccc: File {} not found\x1b[0m", filename);
            return 1;
        }
    };

    let mut lexer = Lexer {
        fp: Some(fp),
        current_file: filename.to_string(),
        buffer: [0],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: Default::default(),
        unlexed_count: 0,
    };

    let mut t = Token::default();
    let mut i = 0;
    let mut tokens: Vec<Token> = Vec::new();

    loop {
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        println!("Contents: {:>20}, type: {:>20}, position: {}/{}", t.contents, ttype_name(t.token_type.clone()), t.line, t.column);
        let is_eof = t.token_type == TokenType::TT_EOF;
        tokens.push(Token {
            token_type: std::mem::replace(&mut t.token_type, TokenType::TT_NO_TOKEN),
            contents: std::mem::take(&mut t.contents),
            length: t.length,
            source_file: std::mem::take(&mut t.source_file),
            line: t.line,
            column: t.column,
        });
        i += 1;
        if is_eof {
            break;
        }
    }

    // Main function check
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
                && tokens[6].contents.chars().next().map_or(false, |c| c.is_ascii_digit())
                && tokens[7].token_type == TokenType::TT_SEMI
            {
                if tokens[8].token_type == TokenType::TT_CBRACE {
                    println!();
                    let code_start = start_main();
                    print!("{}", code_start);
                    let val: i32 = tokens[6].contents.parse().unwrap_or(0);
                    let code_end = end_main_custom_return(val);
                    print!("{}", code_end);
                } else {
                    eprintln!("\x1b[31mError: jccc: Wrong closing brace.\n\x1b[0m");
                }
            } else {
                eprintln!("\x1b[31mError: jccc: Return value is wrong.\n\x1b[0m");
            }
        } else {
            eprintln!("\x1b[31mError: jccc: Wrong main function body.\n\x1b[0m");
        }
    } else {
        eprintln!("\x1b[31mError: jccc: Not correct main function.\n\x1b[0m");
    }

    0
}
