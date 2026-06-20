/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
    let fp = match std::fs::File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("File {} not found", filename);
            return 1;
        }
    };

    let mut lexer = crate::lex::Lexer {
        fp: Some(fp),
        current_file: filename.to_string(),
        buffer: [0],
        position: 0,
        last_column: 0,
        column: 1,
        line: 1,
        unlexed: std::array::from_fn(|_| crate::lex::empty_token()),
        unlexed_count: 0,
    };

    loop {
        let mut t = crate::lex::empty_token();
        if crate::lex::lex(&mut lexer, &mut t) != 0 {
            return 1;
        }

        println!(
            "Contents: {:>20}, type: {:>20}, position: {}/{}",
            t.contents,
            crate::lex::token_type_name(&t.token_type),
            t.line,
            t.column
        );

        if t.token_type == crate::token::TokenType::TT_EOF {
            break;
        }
    }

    0
}
/// The main entry point for the program.
pub fn main() {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!("Usage: --token-dump <filename> to see all tokens");
        return;
    };

    let second = args.next();
    if second.is_none() {
        eprintln!(
            "default compilation not supported yet -- try 'jccc --token-dump {}' instead.",
            first
        );
        return;
    }

    let third = args.next();
    if third.is_some() {
        eprintln!("expected only two arguments!");
        return;
    }

    let filename = second.expect("checked above");
    if first == "--token-dump" {
        let _ = lexer_dump(&filename);
    } else if first == "--test-parse" {
        let _ = crate::parse::parse(&filename);
    } else {
        eprintln!("option {} not recognized.", filename);
    }
}
