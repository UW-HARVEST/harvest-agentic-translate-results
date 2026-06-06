use jccc::lex::{lex, Lexer, TOKEN_PUTBACKS};
use jccc::token::{Token, TokenType};

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

/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => return 1,
    };

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
    let _ = TOKEN_PUTBACKS;

    loop {
        let mut t = make_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        let is_eof = t.token_type == TokenType::TT_EOF;
        let name = jccc::lex::ttype_name(t.token_type);
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
        return;
    }
    if args.len() == 1 {
        println!(
            "jccc: default compilation not supported yet -- try 'jccc --token-dump {}' instead.",
            args[0]
        );
        return;
    }
    if args.len() > 2 {
        println!("jccc: expected only two arguments!");
        return;
    }
    if args[0] == "--token-dump" {
        lexer_dump(&args[1]);
    } else if args[0] == "--test-parse" {
        jccc::parse::parse(&args[1]);
    }
}
