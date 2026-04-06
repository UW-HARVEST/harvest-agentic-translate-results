use crate::lex::{Lexer, lex, ttype_name, lexer_getchar, lexer_ungetchar};
use crate::token::{Token, TokenType};
use crate::codegen::{start_main, end_main_custom_return};
use crate::cst::{FunctionDeclaration, ConcreteFileTree, Expression, BlockStatement};
use std::fs::File;

/// Parses a file and returns a status code.
pub fn parse(filename: &str) -> i32 {
    let fp = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: jccc: File {} not found", filename);
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
    let mut tokens: Vec<Token> = Vec::new();

    loop {
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        println!("Contents: {:>20}, type: {:>20}, position: {}/{}", t.contents, ttype_name(t.token_type.clone()), t.line, t.column);
        let is_eof = t.token_type == TokenType::TT_EOF;
        tokens.push(t.clone());
        if is_eof {
            break;
        }
    }

    // Main function check
    if tokens[0].token_type == TokenType::TT_INT
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
                    eprintln!("Error: jccc: Wrong closing brace.\n");
                }
            } else {
                eprintln!("Error: jccc: Return value is wrong.\n");
            }
        } else {
            eprintln!("Error: jccc: Wrong main function body.\n");
        }
    } else {
        eprintln!("Error: jccc: Not correct main function.\n");
    }

    0
}

/// Parses a simple main function (for testing).
pub fn parse_simple_main_func() -> i32 {
    0
}

/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // TODO in C code as well - stub
    0
}

/// Parses a function call from the lexer into an Expression object.
pub fn parse_funccall(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // TODO in C code as well - stub
    0
}

/// Parses a block statement from the lexer.
pub fn parse_blockstmt(_l: &mut Lexer, _bs: &mut BlockStatement) -> i32 {
    // TODO in C code as well - stub
    0
}

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    // TODO in C code as well - stub
    0
}

/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    // TODO in C code as well - stub
    0
}
