use crate::codegen::{end_main_custom_return, start_main};
use crate::cst::{BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration};
use crate::lex::{lex, Lexer, TOKEN_PUTBACKS};
use crate::token::{Token, TokenType, TOKEN_LENGTH};
use std::fs::File;

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    // TODO -- not implemented in the original C either
    0
}

/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    // TODO
    0
}

/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // TODO
    0
}

fn make_empty_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

fn make_unlexed_array() -> [Token; TOKEN_PUTBACKS] {
    [
        make_empty_token(),
        make_empty_token(),
        make_empty_token(),
        make_empty_token(),
        make_empty_token(),
    ]
}

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
        current_file: String::with_capacity(TOKEN_LENGTH),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: make_unlexed_array(),
        unlexed_count: 0,
    };

    let mut tokens: Vec<Token> = Vec::new();
    loop {
        let mut t = make_empty_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents,
            crate::lex::ttype_name(match t.token_type {
                TokenType::TT_LITERAL => TokenType::TT_LITERAL,
                _ => TokenType::TT_LITERAL,
            }),
            t.line,
            t.column
        );
        tokens.push(t);
        if is_eof {
            break;
        }
    }

    // Main function recognition
    if tokens.len() >= 9
        && matches!(tokens[0].token_type, TokenType::TT_INT)
        && matches!(tokens[1].token_type, TokenType::TT_IDENTIFIER)
        && tokens[1].contents == "main"
    {
        if matches!(tokens[2].token_type, TokenType::TT_OPAREN)
            && matches!(tokens[3].token_type, TokenType::TT_CPAREN)
            && matches!(tokens[4].token_type, TokenType::TT_OBRACE)
        {
            if matches!(tokens[5].token_type, TokenType::TT_RETURN)
                && matches!(tokens[6].token_type, TokenType::TT_LITERAL)
                && tokens[6]
                    .contents
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                && matches!(tokens[7].token_type, TokenType::TT_SEMI)
            {
                if matches!(tokens[8].token_type, TokenType::TT_CBRACE) {
                    println!();
                    let code_start = start_main();
                    print!("{}", code_start);
                    let val: i32 = tokens[6].contents.parse().unwrap_or(0);
                    let code_end = end_main_custom_return(val);
                    print!("{}", code_end);
                } else {
                    eprintln!("Error: jccc: Wrong closing brace.");
                }
            } else {
                eprintln!("Error: jccc: Return value is wrong.");
            }
        } else {
            eprintln!("Error: jccc: Wrong main function body.");
        }
    } else {
        eprintln!("Error: jccc: Not correct main function.");
    }

    0
}

/// Parses a simple main function (for testing).
pub fn parse_simple_main_func() -> i32 {
    0
}

/// Parses a block statement from the lexer.
pub fn parse_blockstmt(_l: &mut Lexer, _bs: &mut BlockStatement) -> i32 {
    0
}

/// Parses a function call from the lexer into an Expression object.
pub fn parse_funccall(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    0
}
