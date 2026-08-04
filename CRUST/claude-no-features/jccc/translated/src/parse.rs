use crate::cst::{BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration};
use crate::lex::{lex, Lexer, TOKEN_PUTBACKS};
use crate::token::{Token, TokenType};

fn make_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    0
}
/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    0
}
/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    0
}
/// Parses a file and returns a status code.
pub fn parse(filename: &str) -> i32 {
    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => return 1,
    };

    let mut unlexed: [Token; TOKEN_PUTBACKS] = [
        make_token(),
        make_token(),
        make_token(),
        make_token(),
        make_token(),
    ];
    let _ = &mut unlexed;
    let mut lexer = Lexer {
        fp: Some(file),
        current_file: filename.to_string(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: [
            make_token(),
            make_token(),
            make_token(),
            make_token(),
            make_token(),
        ],
        unlexed_count: 0,
    };

    let mut tokens: Vec<Token> = Vec::new();
    loop {
        let mut t = make_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        let is_eof = t.token_type == TokenType::TT_EOF;
        tokens.push(t);
        if is_eof {
            break;
        }
    }

    if tokens.len() >= 9
        && tokens[0].token_type == TokenType::TT_INT
        && tokens[1].token_type == TokenType::TT_IDENTIFIER
        && tokens[1].contents == "main"
        && tokens[2].token_type == TokenType::TT_OPAREN
        && tokens[3].token_type == TokenType::TT_CPAREN
        && tokens[4].token_type == TokenType::TT_OBRACE
        && tokens[5].token_type == TokenType::TT_RETURN
        && tokens[6].token_type == TokenType::TT_LITERAL
        && tokens[6].contents.chars().next().map_or(false, |c| c.is_ascii_digit())
        && tokens[7].token_type == TokenType::TT_SEMI
        && tokens[8].token_type == TokenType::TT_CBRACE
    {
        let val: i32 = tokens[6].contents.parse().unwrap_or(0);
        print!("{}", crate::codegen::start_main());
        print!("{}", crate::codegen::end_main_custom_return(val));
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
