use crate::codegen::{end_main_custom_return, start_main};
use crate::cst::{BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration};
use crate::lex::{lex, Lexer};
use crate::token::{Token, TokenType, TOKEN_LENGTH};
use std::fs::File;

fn make_default_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

fn make_default_lexer() -> Lexer {
    Lexer {
        fp: None,
        current_file: String::new(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: [
            make_default_token(),
            make_default_token(),
            make_default_token(),
            make_default_token(),
            make_default_token(),
        ],
        unlexed_count: 0,
    }
}

const _UNUSED: usize = TOKEN_LENGTH;

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    // TODO in C
    0
}

/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    // TODO in C
    0
}

/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // TODO in C
    0
}

/// Parses a file and returns a status code.
pub fn parse(filename: &str) -> i32 {
    let mut lexer = make_default_lexer();
    let fp = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    lexer.fp = Some(fp);
    lexer.unlexed_count = 0;
    lexer.column = 1;
    lexer.line = 1;

    let mut tokens: Vec<Token> = Vec::new();
    loop {
        let mut t = make_default_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        tokens.push(t);
        if is_eof {
            break;
        }
    }

    if tokens.len() < 9 {
        return 0;
    }

    if matches!(tokens[0].token_type, TokenType::TT_INT)
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
                    let _code_start = start_main();
                    let val: i32 = tokens[6].contents.parse().unwrap_or(0);
                    let _code_end = end_main_custom_return(val);
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
pub fn parse_blockstmt(_l: &mut Lexer, _bs: &mut BlockStatement) -> i32 {
    0
}

/// Parses a function call from the lexer into an Expression object.
pub fn parse_funccall(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    0
}
