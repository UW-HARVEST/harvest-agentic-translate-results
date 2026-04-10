use std::fs::File;
use jccc::lex::{Lexer, lex, ttype_name};
use jccc::token::{Token, TokenType};
use jccc::parse::parse;

/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
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
        let is_eof = t.token_type == TokenType::TT_EOF;
        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents,
            ttype_name(t.token_type),
            t.line,
            t.column
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
    let argc = args.len() - 1; // skip executable name
    let argv = &args[1..];

    if argc == 0 {
        eprintln!("\x1b[37mjccc: Usage: --token-dump <filename> to see all tokens\x1b[0m");
        return;
    }

    if argc == 1 {
        eprintln!(
            "\x1b[37mjccc: default compilation not supported yet -- try 'jccc --token-dump {}' instead.\x1b[0m",
            argv[0]
        );
        std::process::exit(1);
    }

    if argc > 2 {
        eprintln!("\x1b[37mjccc: expected only two arguments!\x1b[0m");
        std::process::exit(1);
    }

    // Two arguments
    if argv[0] == "--token-dump" {
        std::process::exit(lexer_dump(&argv[1]));
    } else if argv[0] == "--test-parse" {
        parse(&argv[1]);
        return;
    }

    eprintln!("\x1b[31mError: jccc: option {} not recognized.\x1b[0m", argv[1]);
    std::process::exit(1);
}
