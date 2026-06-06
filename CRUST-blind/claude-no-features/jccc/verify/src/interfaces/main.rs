use jccc::lex::{lex, ttype_name, Lexer, TOKEN_PUTBACKS};
use jccc::parse::parse;
use jccc::token::{Token, TokenType};

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

/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: jccc: File {} not found", filename);
            return 1;
        }
    };

    let mut unlexed: Vec<Token> = Vec::with_capacity(TOKEN_PUTBACKS);
    for _ in 0..TOKEN_PUTBACKS {
        unlexed.push(empty_token());
    }
    let arr: [Token; TOKEN_PUTBACKS] = unlexed.try_into().unwrap_or_else(|_| unreachable!());

    let mut lexer = Lexer {
        fp: Some(file),
        current_file: filename.to_string(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: arr,
        unlexed_count: 0,
    };

    loop {
        let mut t = empty_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        let is_eof = matches!(t.token_type, TokenType::TT_EOF);
        // We can't move t.token_type so we go via formatting helpers; simulate
        // the C printf line.
        let name = ttype_name(match t.token_type {
            TokenType::TT_LITERAL => TokenType::TT_LITERAL,
            TokenType::TT_IDENTIFIER => TokenType::TT_IDENTIFIER,
            TokenType::TT_OPAREN => TokenType::TT_OPAREN,
            TokenType::TT_CPAREN => TokenType::TT_CPAREN,
            TokenType::TT_OBRACE => TokenType::TT_OBRACE,
            TokenType::TT_CBRACE => TokenType::TT_CBRACE,
            TokenType::TT_OBRACKET => TokenType::TT_OBRACKET,
            TokenType::TT_CBRACKET => TokenType::TT_CBRACKET,
            TokenType::TT_SEMI => TokenType::TT_SEMI,
            TokenType::TT_NO_TOKEN => TokenType::TT_NO_TOKEN,
            TokenType::TT_EOF => TokenType::TT_EOF,
            TokenType::TT_NEWLINE => TokenType::TT_NEWLINE,
            TokenType::TT_POUND => TokenType::TT_POUND,
            TokenType::TT_PERIOD => TokenType::TT_PERIOD,
            TokenType::TT_COMMA => TokenType::TT_COMMA,
            TokenType::TT_QMARK => TokenType::TT_QMARK,
            TokenType::TT_MINUS => TokenType::TT_MINUS,
            TokenType::TT_PLUS => TokenType::TT_PLUS,
            TokenType::TT_STAR => TokenType::TT_STAR,
            TokenType::TT_SLASH => TokenType::TT_SLASH,
            TokenType::TT_ASSIGN => TokenType::TT_ASSIGN,
            TokenType::TT_COLON => TokenType::TT_COLON,
            TokenType::TT_MOD => TokenType::TT_MOD,
            TokenType::TT_BAND => TokenType::TT_BAND,
            TokenType::TT_LAND => TokenType::TT_LAND,
            TokenType::TT_BOR => TokenType::TT_BOR,
            TokenType::TT_LOR => TokenType::TT_LOR,
            TokenType::TT_DEC => TokenType::TT_DEC,
            TokenType::TT_INC => TokenType::TT_INC,
            TokenType::TT_PLUSPLUS => TokenType::TT_PLUSPLUS,
            TokenType::TT_MINUSMINUS => TokenType::TT_MINUSMINUS,
            TokenType::TT_DIVEQ => TokenType::TT_DIVEQ,
            TokenType::TT_MULEQ => TokenType::TT_MULEQ,
            TokenType::TT_MODEQ => TokenType::TT_MODEQ,
            TokenType::TT_BANDEQ => TokenType::TT_BANDEQ,
            TokenType::TT_BOREQ => TokenType::TT_BOREQ,
            TokenType::TT_LANDEQ => TokenType::TT_LANDEQ,
            TokenType::TT_LOREQ => TokenType::TT_LOREQ,
            TokenType::TT_GREATER => TokenType::TT_GREATER,
            TokenType::TT_LESS => TokenType::TT_LESS,
            TokenType::TT_LESSEQ => TokenType::TT_LESSEQ,
            TokenType::TT_GREATEREQ => TokenType::TT_GREATEREQ,
            TokenType::TT_LEFTSHIFT => TokenType::TT_LEFTSHIFT,
            TokenType::TT_RIGHTSHIFT => TokenType::TT_RIGHTSHIFT,
            TokenType::TT_LNOT => TokenType::TT_LNOT,
            TokenType::TT_BNOT => TokenType::TT_BNOT,
            TokenType::TT_EQUALS => TokenType::TT_EQUALS,
            TokenType::TT_NOTEQ => TokenType::TT_NOTEQ,
            TokenType::TT_XOR => TokenType::TT_XOR,
            TokenType::TT_XOREQ => TokenType::TT_XOREQ,
            TokenType::TT_POINT => TokenType::TT_POINT,
            TokenType::TT_LEFTSHIFTEQUALS => TokenType::TT_LEFTSHIFTEQUALS,
            TokenType::TT_RIGHTSHIFTEQUALS => TokenType::TT_RIGHTSHIFTEQUALS,
            TokenType::TT_AUTO => TokenType::TT_AUTO,
            TokenType::TT_BREAK => TokenType::TT_BREAK,
            TokenType::TT_CHAR => TokenType::TT_CHAR,
            TokenType::TT_CONST => TokenType::TT_CONST,
            TokenType::TT_CASE => TokenType::TT_CASE,
            TokenType::TT_CONTINUE => TokenType::TT_CONTINUE,
            TokenType::TT_DOUBLE => TokenType::TT_DOUBLE,
            TokenType::TT_DO => TokenType::TT_DO,
            TokenType::TT_DEFAULT => TokenType::TT_DEFAULT,
            TokenType::TT_ENUM => TokenType::TT_ENUM,
            TokenType::TT_ELSE => TokenType::TT_ELSE,
            TokenType::TT_EXTERN => TokenType::TT_EXTERN,
            TokenType::TT_FLOAT => TokenType::TT_FLOAT,
            TokenType::TT_FOR => TokenType::TT_FOR,
            TokenType::TT_GOTO => TokenType::TT_GOTO,
            TokenType::TT_IF => TokenType::TT_IF,
            TokenType::TT_INT => TokenType::TT_INT,
            TokenType::TT_LONG => TokenType::TT_LONG,
            TokenType::TT_RETURN => TokenType::TT_RETURN,
            TokenType::TT_REGISTER => TokenType::TT_REGISTER,
            TokenType::TT_STATIC => TokenType::TT_STATIC,
            TokenType::TT_SWITCH => TokenType::TT_SWITCH,
            TokenType::TT_SHORT => TokenType::TT_SHORT,
            TokenType::TT_SIGNED => TokenType::TT_SIGNED,
            TokenType::TT_STRUCT => TokenType::TT_STRUCT,
            TokenType::TT_SIZEOF => TokenType::TT_SIZEOF,
            TokenType::TT_TYPEDEF => TokenType::TT_TYPEDEF,
            TokenType::TT_UNSIGNED => TokenType::TT_UNSIGNED,
            TokenType::TT_UNION => TokenType::TT_UNION,
            TokenType::TT_VOID => TokenType::TT_VOID,
            TokenType::TT_VOLATILE => TokenType::TT_VOLATILE,
            TokenType::TT_WHILE => TokenType::TT_WHILE,
        });
        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents, name, t.line, t.column
        );
        if is_eof {
            break;
        }
    }

    0
}

/// The main entry point for the program.
pub fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        println!("jccc: Usage: --token-dump <filename> to see all tokens");
        std::process::exit(0);
    }
    if args.len() == 1 {
        println!(
            "jccc: default compilation not supported yet -- try 'jccc --token-dump {}' instead.",
            args[0]
        );
        std::process::exit(1);
    }
    if args.len() > 2 {
        println!("jccc: expected only two arguments!");
        std::process::exit(1);
    }
    if args[0] == "--token-dump" {
        std::process::exit(lexer_dump(&args[1]));
    } else if args[0] == "--test-parse" {
        parse(&args[1]);
        std::process::exit(0);
    }
    eprintln!("Error: jccc: option {} not recognized.", args[1]);
    std::process::exit(1);
}
