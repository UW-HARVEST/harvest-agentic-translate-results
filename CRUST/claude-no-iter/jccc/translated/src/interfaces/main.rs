use jccc::lex::{lex, ttype_name, Lexer};
use jccc::parse::parse;
use jccc::token::{Token, TokenType};
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

/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
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
        if is_eof {
            break;
        }
    }

    0
}

/// The main entry point for the program.
pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Skip the executable name.
    let argv: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
    let argc = argv.len();

    if argc == 0 {
        eprintln!("jccc: Usage: --token-dump <filename> to see all tokens");
        return;
    }

    if argc == 1 {
        eprintln!(
            "jccc: default compilation not supported yet -- try 'jccc --token-dump {}' instead.",
            argv[0]
        );
        std::process::exit(1);
    }

    if argc > 2 {
        eprintln!("jccc: expected only two arguments!");
        std::process::exit(1);
    }

    if argv[0] == "--token-dump" {
        let code = lexer_dump(argv[1]);
        std::process::exit(code);
    } else if argv[0] == "--test-parse" {
        parse(argv[1]);
        return;
    }

    eprintln!("Error: jccc: option {} not recognized.", argv[1]);
    std::process::exit(1);
}
