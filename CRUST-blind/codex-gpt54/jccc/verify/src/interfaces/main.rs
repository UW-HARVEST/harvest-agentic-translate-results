/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
    use crate::lex::{lex, ttype_name, Lexer};
    use crate::token::Token;
    use std::fs::File;

    let fp = match File::open(filename) {
        Ok(fp) => fp,
        Err(_) => return 1,
    };

    let mut lexer = Lexer {
        fp: Some(fp),
        current_file: filename.to_string(),
        buffer: [0],
        position: 0,
        last_column: 1,
        column: 1,
        line: 1,
        unlexed: std::array::from_fn(|_| Token::empty()),
        unlexed_count: 0,
    };

    loop {
        let mut token = Token::empty();
        if lex(&mut lexer, &mut token) != 0 {
            return 1;
        }

        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            token.contents,
            ttype_name(token.token_type),
            token.line,
            token.column
        );

        if matches!(token.token_type, crate::token::TokenType::TT_EOF) {
            break;
        }
    }

    0
}
/// The main entry point for the program.
pub fn main() {
    use crate::parse::parse;

    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("jccc: Usage: --token-dump <filename> to see all tokens");
        return;
    }

    if args.len() == 1 {
        eprintln!(
            "jccc: default compilation not supported yet -- try 'jccc --token-dump {}' instead.",
            args[0]
        );
        return;
    }

    if args.len() > 2 {
        eprintln!("jccc: expected only two arguments!");
        return;
    }

    match args[0].as_str() {
        "--token-dump" => {
            let _ = lexer_dump(&args[1]);
        }
        "--test-parse" => {
            let _ = parse(&args[1]);
        }
        _ => {
            eprintln!("Error: jccc: option {} not recognized.", args[1]);
        }
    }
}
