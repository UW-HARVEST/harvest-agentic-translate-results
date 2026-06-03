use crate::cst::{BlockStatement, ConcreteFileTree, Expression, FunctionDeclaration};
use crate::lex::{lex, Lexer, TOKEN_PUTBACKS};
use crate::token::{Token, TokenType};
use std::fs::File;

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

/// Parses a function declaration from the lexer into a FunctionDeclaration object.
pub fn parse_funcdecl(_l: &mut Lexer, _fd: &mut FunctionDeclaration) -> i32 {
    // The C version is a stub (`// TODO`), so this Rust version simply
    // returns success.
    0
}

/// Creates a concrete syntax tree from the lexer.
pub fn make_cst(_l: &mut Lexer, _tree: &mut ConcreteFileTree) -> i32 {
    // C stub.
    0
}

/// Parses an expression from the Lexer into an Expression object.
pub fn parse_expr(_l: &mut Lexer, _ex: &mut Expression) -> i32 {
    // C stub.
    0
}

/// Parses a file and returns a status code.
pub fn parse(filename: &str) -> i32 {
    use crate::codegen::{end_main_custom_return, start_main};
    use crate::lex::ttype_name;

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
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: [
            make_empty_token(),
            make_empty_token(),
            make_empty_token(),
            make_empty_token(),
            make_empty_token(),
        ],
        unlexed_count: 0,
    };

    let _ = TOKEN_PUTBACKS; // keep import live

    let mut tokens: Vec<Token> = Vec::new();

    loop {
        let mut t = make_empty_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        let type_name = ttype_name(clone_ttype(&t.token_type));
        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents, type_name, t.line, t.column
        );
        tokens.push(t);
        if is_eof {
            break;
        }
    }

    if tokens.len() >= 2
        && matches!(tokens[0].token_type, TokenType::TT_INT)
        && matches!(tokens[1].token_type, TokenType::TT_IDENTIFIER)
        && tokens[1].contents == "main"
    {
        if tokens.len() >= 5
            && matches!(tokens[2].token_type, TokenType::TT_OPAREN)
            && matches!(tokens[3].token_type, TokenType::TT_CPAREN)
            && matches!(tokens[4].token_type, TokenType::TT_OBRACE)
        {
            if tokens.len() >= 8
                && matches!(tokens[5].token_type, TokenType::TT_RETURN)
                && matches!(tokens[6].token_type, TokenType::TT_LITERAL)
                && tokens[6]
                    .contents
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_ascii_digit())
                && matches!(tokens[7].token_type, TokenType::TT_SEMI)
            {
                if tokens.len() >= 9 && matches!(tokens[8].token_type, TokenType::TT_CBRACE) {
                    println!();
                    let code_start = start_main();
                    print!("{}", code_start);
                    let v: i32 = tokens[6].contents.parse().unwrap_or(0);
                    let code_end = end_main_custom_return(v);
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

/// Local clone helper because TokenType doesn't implement Clone.
fn clone_ttype(tt: &TokenType) -> TokenType {
    use TokenType::*;
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
