use crate::cst::{BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration, NodeType};
use crate::lex::{lex, Lexer, TOKEN_PUTBACKS};
use crate::token::{Token, TokenType, TOKEN_LENGTH};
use crate::codegen::{end_main_custom_return, start_main};
use std::fs::File;

fn empty_token() -> Token {
    Token {
        token_type: TokenType::TT_NO_TOKEN,
        contents: String::new(),
        length: 0,
        source_file: String::new(),
        line: 0,
        column: 0,
    }
}

fn make_lexer(file: File, current_file: &str) -> Lexer {
    let mut unlexed: Vec<Token> = Vec::with_capacity(TOKEN_PUTBACKS);
    for _ in 0..TOKEN_PUTBACKS {
        unlexed.push(empty_token());
    }
    let arr: [Token; TOKEN_PUTBACKS] = match unlexed.try_into() {
        Ok(a) => a,
        Err(_) => unreachable!("filled exactly TOKEN_PUTBACKS slots"),
    };
    Lexer {
        fp: Some(file),
        current_file: current_file.to_string(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: arr,
        unlexed_count: 0,
    }
}

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    // The original C function is a stub (TODO).
    0
}

/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    // The original C function is a stub (TODO).
    0
}

/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // The original C function is a stub (TODO).
    0
}

/// Parses a file and returns a status code.
pub fn parse(filename: &str) -> i32 {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: jccc: File {} not found", filename);
            return 1;
        }
    };

    let mut lexer = make_lexer(file, filename);
    let mut tokens: Vec<Token> = Vec::new();

    loop {
        let mut t = empty_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents,
            crate::lex::ttype_name(clone_ttype(&t.token_type)),
            t.line,
            t.column
        );
        tokens.push(t);
        if is_eof {
            break;
        }
    }

    if tokens.len() < 9 {
        eprintln!("Error: jccc: Not correct main function.");
        return 0;
    }

    let is_int = matches!(tokens[0].token_type, TokenType::TT_INT);
    let is_id = matches!(tokens[1].token_type, TokenType::TT_IDENTIFIER);
    let is_main = is_id && tokens[1].contents == "main";

    if is_int && is_main {
        let is_oparen = matches!(tokens[2].token_type, TokenType::TT_OPAREN);
        let is_cparen = matches!(tokens[3].token_type, TokenType::TT_CPAREN);
        let is_obrace = matches!(tokens[4].token_type, TokenType::TT_OBRACE);
        if is_oparen && is_cparen && is_obrace {
            let is_return = matches!(tokens[5].token_type, TokenType::TT_RETURN);
            let is_lit = matches!(tokens[6].token_type, TokenType::TT_LITERAL);
            let starts_digit = tokens[6]
                .contents
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_digit());
            let is_semi = matches!(tokens[7].token_type, TokenType::TT_SEMI);
            if is_return && is_lit && starts_digit && is_semi {
                let is_cbrace = matches!(tokens[8].token_type, TokenType::TT_CBRACE);
                if is_cbrace {
                    println!();
                    print!("{}", start_main());
                    let val: i32 = tokens[6].contents.parse().unwrap_or(0);
                    print!("{}", end_main_custom_return(val));
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
    // Mark unused.
    let _ = NodeType::NT_FUNCCALL;
    let _ = TOKEN_LENGTH;
    0
}

fn clone_ttype(tt: &TokenType) -> TokenType {
    // Re-use logic from lex.rs by going through ttype_from_string round-trip
    // is brittle, so we duplicate here.
    use crate::token::TokenType::*;
    match tt {
        TT_LITERAL => TT_LITERAL,
        TT_IDENTIFIER => TT_IDENTIFIER,
        TT_OPAREN => TT_OPAREN,
        TT_CPAREN => TT_CPAREN,
        TT_OBRACE => TT_OBRACE,
        TT_CBRACE => TT_CBRACE,
        TT_OBRACKET => TT_OBRACKET,
        TT_CBRACKET => TT_CBRACKET,
        TT_SEMI => TT_SEMI,
        TT_NO_TOKEN => TT_NO_TOKEN,
        TT_EOF => TT_EOF,
        TT_NEWLINE => TT_NEWLINE,
        TT_POUND => TT_POUND,
        TT_PERIOD => TT_PERIOD,
        TT_COMMA => TT_COMMA,
        TT_QMARK => TT_QMARK,
        TT_MINUS => TT_MINUS,
        TT_PLUS => TT_PLUS,
        TT_STAR => TT_STAR,
        TT_SLASH => TT_SLASH,
        TT_ASSIGN => TT_ASSIGN,
        TT_COLON => TT_COLON,
        TT_MOD => TT_MOD,
        TT_BAND => TT_BAND,
        TT_LAND => TT_LAND,
        TT_BOR => TT_BOR,
        TT_LOR => TT_LOR,
        TT_DEC => TT_DEC,
        TT_INC => TT_INC,
        TT_PLUSPLUS => TT_PLUSPLUS,
        TT_MINUSMINUS => TT_MINUSMINUS,
        TT_DIVEQ => TT_DIVEQ,
        TT_MULEQ => TT_MULEQ,
        TT_MODEQ => TT_MODEQ,
        TT_BANDEQ => TT_BANDEQ,
        TT_BOREQ => TT_BOREQ,
        TT_LANDEQ => TT_LANDEQ,
        TT_LOREQ => TT_LOREQ,
        TT_GREATER => TT_GREATER,
        TT_LESS => TT_LESS,
        TT_LESSEQ => TT_LESSEQ,
        TT_GREATEREQ => TT_GREATEREQ,
        TT_LEFTSHIFT => TT_LEFTSHIFT,
        TT_RIGHTSHIFT => TT_RIGHTSHIFT,
        TT_LNOT => TT_LNOT,
        TT_BNOT => TT_BNOT,
        TT_EQUALS => TT_EQUALS,
        TT_NOTEQ => TT_NOTEQ,
        TT_XOR => TT_XOR,
        TT_XOREQ => TT_XOREQ,
        TT_POINT => TT_POINT,
        TT_LEFTSHIFTEQUALS => TT_LEFTSHIFTEQUALS,
        TT_RIGHTSHIFTEQUALS => TT_RIGHTSHIFTEQUALS,
        TT_AUTO => TT_AUTO,
        TT_BREAK => TT_BREAK,
        TT_CHAR => TT_CHAR,
        TT_CONST => TT_CONST,
        TT_CASE => TT_CASE,
        TT_CONTINUE => TT_CONTINUE,
        TT_DOUBLE => TT_DOUBLE,
        TT_DO => TT_DO,
        TT_DEFAULT => TT_DEFAULT,
        TT_ENUM => TT_ENUM,
        TT_ELSE => TT_ELSE,
        TT_EXTERN => TT_EXTERN,
        TT_FLOAT => TT_FLOAT,
        TT_FOR => TT_FOR,
        TT_GOTO => TT_GOTO,
        TT_IF => TT_IF,
        TT_INT => TT_INT,
        TT_LONG => TT_LONG,
        TT_RETURN => TT_RETURN,
        TT_REGISTER => TT_REGISTER,
        TT_STATIC => TT_STATIC,
        TT_SWITCH => TT_SWITCH,
        TT_SHORT => TT_SHORT,
        TT_SIGNED => TT_SIGNED,
        TT_STRUCT => TT_STRUCT,
        TT_SIZEOF => TT_SIZEOF,
        TT_TYPEDEF => TT_TYPEDEF,
        TT_UNSIGNED => TT_UNSIGNED,
        TT_UNION => TT_UNION,
        TT_VOID => TT_VOID,
        TT_VOLATILE => TT_VOLATILE,
        TT_WHILE => TT_WHILE,
    }
}
