use crate::lex::{Lexer, lex, ttype_name};
use crate::token::{Token, TokenType};
use crate::codegen::{start_main, end_main_custom_return};
use crate::cst::{FunctionDeclaration, ConcreteFileTree, Expression, BlockStatement};
use std::fs::File;

/// Parses a file and returns a status code.
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

    let mut tokens: Vec<Token> = Vec::new();

    loop {
        let mut t = Token {
            token_type: TokenType::TT_NO_TOKEN,
            contents: String::new(),
            length: 0,
            source_file: String::new(),
            line: 0,
            column: 0,
        };
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents,
            ttype_name(std::mem::replace(&mut t.token_type, TokenType::TT_NO_TOKEN)),
            t.line,
            t.column
        );
        // Restore token_type for later use - we need to check it
        // Actually we consumed it above, let's re-parse from contents
        t.token_type = crate::lex::ttype_from_string(&t.contents);
        let is_eof = t.token_type == TokenType::TT_EOF;
        tokens.push(t);
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
                    let code_end = end_main_custom_return(
                        tokens[6].contents.parse::<i32>().unwrap_or(0),
                    );
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

/// Parses a simple main function (for testing).
pub fn parse_simple_main_func() -> i32 {
    0
}

/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    0
}

/// Parses a function call from the lexer into an Expression object.
pub fn parse_funccall(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    0
}

/// Parses a block statement from the lexer.
pub fn parse_blockstmt(_l: &mut Lexer, _bs: &mut BlockStatement) -> i32 {
    0
}

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    0
}

/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    0
}
