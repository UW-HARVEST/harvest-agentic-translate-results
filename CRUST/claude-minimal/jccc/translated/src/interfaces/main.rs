/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
    use jccc::lex::{lex, ttype_name, Lexer, TOKEN_PUTBACKS};
    use jccc::token::{Token, TokenType, TOKEN_LENGTH};

    let fp = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("File {} not found", filename);
            return 1;
        }
    };

    fn empty_token() -> Token {
        Token {
            token_type: TokenType::TT_NO_TOKEN,
            contents: String::with_capacity(TOKEN_LENGTH),
            length: 0,
            source_file: String::with_capacity(TOKEN_LENGTH),
            line: 0,
            column: 0,
        }
    }

    let unlexed: [Token; TOKEN_PUTBACKS] = [
        empty_token(),
        empty_token(),
        empty_token(),
        empty_token(),
        empty_token(),
    ];

    let mut lexer = Lexer {
        fp: Some(fp),
        current_file: filename.to_string(),
        buffer: [0u8; 1],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed,
        unlexed_count: 0,
    };

    loop {
        let mut t = empty_token();
        if lex(&mut lexer, &mut t) != 0 {
            return 1;
        }
        // Clone the token type since ttype_name takes by value.
        let tt_name = match &t.token_type {
            TokenType::TT_LITERAL => ttype_name(TokenType::TT_LITERAL),
            TokenType::TT_IDENTIFIER => ttype_name(TokenType::TT_IDENTIFIER),
            TokenType::TT_EOF => ttype_name(TokenType::TT_EOF),
            // Fallback formatter for any other type via debug.
            other => format!("{:?}", other),
        };
        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents, tt_name, t.line, t.column
        );
        if matches!(t.token_type, TokenType::TT_EOF) {
            break;
        }
    }
    0
}

/// The main entry point for the program.
pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argv: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
    let argc = argv.len();

    if argc == 0 {
        println!("Usage: --token-dump <filename> to see all tokens");
        return;
    }
    if argc == 1 {
        println!(
            "default compilation not supported yet -- try 'jccc --token-dump {}' instead.",
            argv[0]
        );
        std::process::exit(1);
    }
    if argc > 2 {
        println!("expected only two arguments!");
        std::process::exit(1);
    }
    if argv[0] == "--token-dump" {
        std::process::exit(lexer_dump(argv[1]));
    } else if argv[0] == "--test-parse" {
        jccc::parse::parse(argv[1]);
        return;
    }
    eprintln!("option {} not recognized.", argv[0]);
    std::process::exit(1);
}
