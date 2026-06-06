/// Dumps lexer output for the specified file.
pub fn lexer_dump(filename: &str) -> i32 {
    use std::fs::File;

    let fp = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: jccc: File {} not found", filename);
            return 1;
        }
    };

    let _ = fp;
    // Stub for compilation; the real implementation would create a Lexer and
    // print every token until EOF.
    0
}

/// The main entry point for the program.
pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() - 1;
    if argc == 0 {
        eprintln!("jccc: Usage: --token-dump <filename> to see all tokens");
        return;
    }
    if argc == 1 {
        eprintln!(
            "jccc: default compilation not supported yet -- try 'jccc --token-dump {}' instead.",
            args[1]
        );
        return;
    }
    if argc > 2 {
        eprintln!("jccc: expected only two arguments!");
        return;
    }
    if args[1] == "--token-dump" {
        lexer_dump(&args[2]);
    } else if args[1] == "--test-parse" {
        // Just stub it.
    } else {
        eprintln!("Error: jccc: option {} not recognized.", args[1]);
    }
}
