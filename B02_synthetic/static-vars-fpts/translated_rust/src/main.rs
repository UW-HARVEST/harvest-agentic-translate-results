mod tokenizer;
mod analyzer;

use std::io::{self, BufRead, Write};
use tokenizer::*;
use analyzer::*;

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
    io::stdout().flush().unwrap();
}

fn print_analysis_result(result: &AnalysisResult) {
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

fn fgets_line(stdin: &io::Stdin) -> Option<String> {
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(_) => None,
    }
}

fn interactive_tokenizer(ops: &TokenizerOps) {
    println!("\nEnter text (empty line to stop):");
    let stdin = io::stdin();
    let mut input = String::new();
    loop {
        let line = match fgets_line(&stdin) {
            Some(l) => l,
            None => break,
        };
        if line == "\n" || line.is_empty() {
            break;
        }
        let remaining = MAX_INPUT_SIZE - input.len();
        if remaining > 1 {
            let take = std::cmp::min(line.len(), remaining - 1);
            input.push_str(&line[..take]);
        }
    }
    if (ops.load_text)(&input) != 0 {
        println!("Failed to load text");
        return;
    }
    println!("\n=== Tokens ===");
    let token_type_names = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE",
        "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];
    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        println!("[{}] '{}' (L{}:C{})",
            token_type_names[token.token_type as usize],
            token.value,
            token.line,
            token.column);
        count += 1;
        if count > 100 {
            println!("... (truncated, too many tokens)");
            break;
        }
    }
}

fn read_file(filename: &str) -> Option<String> {
    let content = match std::fs::read(filename) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", filename);
            return None;
        }
    };
    if content.len() > MAX_BUFFER_SIZE {
        eprintln!("Error: File too large");
        return None;
    }
    Some(String::from_utf8_lossy(&content).into_owned())
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(get_tokenizer_ops());
    println!("Text Analysis and Tokenization System");
    println!("This system demonstrates function pointers and static globals");
    let stdin = io::stdin();
    loop {
        print_menu();
        let input = match fgets_line(&stdin) {
            Some(l) => l,
            None => break,
        };
        let choice: i32 = match input.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };
        match choice {
            1 => {
                println!("Enter text to analyze (empty line to stop):");
                let mut text = String::new();
                loop {
                    let line = match fgets_line(&stdin) {
                        Some(l) => l,
                        None => break,
                    };
                    if line == "\n" || line.is_empty() {
                        break;
                    }
                    let remaining = MAX_INPUT_SIZE - text.len();
                    if remaining > 1 {
                        let take = std::cmp::min(line.len(), remaining - 1);
                        text.push_str(&line[..take]);
                    }
                }
                let result = analyze_text(&text);
                print_analysis_result(&result);
            }
            2 => {
                print!("Enter filename: ");
                io::stdout().flush().unwrap();
                let input = match fgets_line(&stdin) {
                    Some(l) => l,
                    None => continue,
                };
                let filename = input.trim_end_matches('\n');
                if let Some(content) = read_file(filename) {
                    let result = analyze_text(&content);
                    print_analysis_result(&result);
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
                io::stdout().flush().unwrap();
                let input = match fgets_line(&stdin) {
                    Some(l) => l,
                    None => continue,
                };
                let pattern = input.trim_end_matches('\n');
                find_patterns(pattern);
            }
            6 => {
                interactive_tokenizer(&ops);
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
