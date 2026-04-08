use std::fs::File;
use jccc::lex::{Lexer, lex, ttype_name, TOKEN_PUTBACKS};
use jccc::token::{Token, TokenType};
use jccc::parse::parse;

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

    let mut t = Token::default();
    loop {
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        println!("Contents: {:>20}, type: {:>20}, position: {}/{}", t.contents, ttype_name(t.token_type.clone()), t.line, t.column);
        if t.token_type == TokenType::TT_EOF {
            break;
        }
    }
    0
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() - 1; // skip executable name

    if argc == 0 {
        eprintln!("\x1b[37mjccc: Usage: --token-dump <filename> to see all tokens\x1b[0m");
        return;
    }

    if argc == 1 {
        eprintln!("\x1b[37mjccc: default compilation not supported yet -- try 'jccc --token-dump {}' instead.\x1b[0m", args[1]);
        std::process::exit(1);
    }

    if argc > 2 {
        eprintln!("\x1b[37mjccc: expected only two arguments!\x1b[0m");
        std::process::exit(1);
    }

    if args[1] == "--token-dump" {
        std::process::exit(lexer_dump(&args[2]));
    } else if args[1] == "--test-parse" {
        parse(&args[2]);
        return;
    }

    eprintln!("\x1b[31mError: jccc: option {} not recognized.\x1b[0m", args[2]);
    std::process::exit(1);
}
