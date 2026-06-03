mod analyzer;
mod tokenizer;

use std::io::{self, BufRead, Read, Write};

use analyzer::{
    analyze_text, analyzer_init, calculate_complexity_score, find_patterns,
    print_token_distribution, AnalysisResult,
};
use tokenizer::{get_tokenizer_ops, TokenType, TokenizerOps, MAX_BUFFER_SIZE};

const MAX_INPUT_SIZE: usize = 4096;

fn print_menu() {
    println!("\n=== Text Analyzer ===");
    println!("1. Analyze text");
    println!("2. Load text from file");
    println!("3. Show token distribution");
    println!("4. Calculate complexity score");
    println!("5. Find pattern");
    println!("6. Interactive tokenizer");
    println!("7. Exit");
    print!("Choice: ");
    let _ = io::stdout().flush();
}

fn print_analysis_result(result: AnalysisResult) {
    println!("\n=== Analysis Results ===");
    println!("Words/Identifiers: {}", result.word_count);
    println!("Numbers: {}", result.number_count);
    println!("Keywords: {}", result.keyword_count);
    println!("Operators: {}", result.operator_count);
    println!("Comments: {}", result.comment_count);
    println!("Strings: {}", result.string_count);
    println!("Lines: {}", result.line_count);
    println!("Characters: {}", result.char_count);
}

// Read a single line from stdin (including the trailing newline if present).
// Returns None on EOF (no bytes read).
fn read_line(stdin: &mut io::StdinLock) -> Option<String> {
    let mut line = String::new();
    match stdin.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(_) => None,
    }
}

fn read_until_blank_line(stdin: &mut io::StdinLock, max_size: usize) -> String {
    let mut input = String::new();
    while let Some(line) = read_line(stdin) {
        if line == "\n" || line == "\r\n" || line.is_empty() {
            break;
        }
        let remaining = max_size.saturating_sub(input.len()).saturating_sub(1);
        if remaining == 0 {
            break;
        }
        let to_take = line.len().min(remaining);
        input.push_str(&line[..to_take]);
    }
    input
}

fn interactive_tokenizer(ops: TokenizerOps) {
    println!("\nEnter text (empty line to stop):");
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let input = read_until_blank_line(&mut handle, MAX_INPUT_SIZE);

    if (ops.load_text)(&input) != 0 {
        println!("Failed to load text");
        return;
    }

    println!("\n=== Tokens ===");

    let token_type_names = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD",
        "OPERATOR", "STRING", "COMMENT", "ERROR",
    ];

    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.type_ == TokenType::Eof {
            break;
        }
        let idx = token.type_ as usize;
        let name = if idx < token_type_names.len() {
            token_type_names[idx]
        } else {
            "?"
        };
        println!(
            "[{}] '{}' (L{}:C{})",
            name, token.value, token.line, token.column
        );
        count += 1;
        if count > 100 {
            println!("... (truncated, too many tokens)");
            break;
        }
    }
}

fn read_file(filename: &str) -> Option<String> {
    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", filename);
            return None;
        }
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => {
            eprintln!("Error: Could not read file metadata");
            return None;
        }
    };

    let size = metadata.len();
    if size as usize > MAX_BUFFER_SIZE {
        eprintln!("Error: File too large");
        return None;
    }

    let mut content = String::new();
    let mut reader = io::BufReader::new(file);
    if reader.read_to_string(&mut content).is_err() {
        eprintln!("Error: Failed to read file");
        return None;
    }
    Some(content)
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops);

    println!("Text Analysis and Tokenization System");
    println!("This system demonstrates function pointers and static globals");

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        print_menu();

        let input_line = match read_line(&mut handle) {
            Some(s) => s,
            None => break,
        };

        let trimmed = input_line.trim();
        let choice: i32 = match trimmed.parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                println!("Enter text to analyze (empty line to stop):");
                let text = read_until_blank_line(&mut handle, MAX_INPUT_SIZE);
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                print!("Enter filename: ");
                let _ = io::stdout().flush();
                let filename_line = match read_line(&mut handle) {
                    Some(s) => s,
                    None => break,
                };
                let filename = filename_line.trim_end_matches(&['\n', '\r'][..]);
                if let Some(content) = read_file(filename) {
                    let result = analyze_text(&content);
                    print_analysis_result(result);
                }
            }
            3 => {
                print_token_distribution();
            }
            4 => {
                let score = calculate_complexity_score();
                println!("\nComplexity Score: {}", score);
                if score < 10 {
                    println!("Complexity: Low");
                } else if score < 50 {
                    println!("Complexity: Medium");
                } else {
                    println!("Complexity: High");
                }
            }
            5 => {
                print!("Enter pattern to search: ");
                let _ = io::stdout().flush();
                let pattern_line = match read_line(&mut handle) {
                    Some(s) => s,
                    None => break,
                };
                let pattern = pattern_line.trim_end_matches(&['\n', '\r'][..]);
                find_patterns(pattern);
            }
            6 => {
                interactive_tokenizer(ops);
            }
            7 => {
                println!("Goodbye!");
                return;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}
