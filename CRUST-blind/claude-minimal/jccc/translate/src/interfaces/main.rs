/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
    use jccc::lex::{lex, ttype_name, Lexer, TOKEN_PUTBACKS};
    use jccc::token::{Token, TokenType};

    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: jccc: File {} not found", filename);
            return 1;
        }
    };

    let _ = TOKEN_PUTBACKS;
    let mut lexer = Lexer {
        fp: Some(file),
        current_file: filename.to_string(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: std::array::from_fn(|_| Token {
            token_type: TokenType::TT_NO_TOKEN,
            contents: String::new(),
            length: 0,
            source_file: String::new(),
            line: 0,
            column: 0,
        }),
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
            ttype_name(t.token_type.clone()),
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
    use jccc::parse::parse;

    // Skip the executable name.
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

    if args[0] == "--token-dump" {
        let _ = lexer_dump(&args[1]);
        return;
    } else if args[0] == "--test-parse" {
        let _ = parse(&args[1]);
        return;
    }

    eprintln!("Error: jccc: option {} not recognized.", args[1]);
}
