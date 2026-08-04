mod analyzer;
mod tokenizer;

use std::fs;
use std::io::{self, Write};

use analyzer::{analyze_text, analyzer_init, calculate_complexity_score, find_patterns, print_token_distribution, AnalysisResult};
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

fn read_multiline_input() -> String {
    let mut input = String::new();
    let stdin = io::stdin();
    loop {
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line == "\n" {
                    break;
                }
                if input.len() + line.len() >= MAX_INPUT_SIZE {
                    let remaining = MAX_INPUT_SIZE.saturating_sub(input.len() + 1);
                    input.push_str(&line[..remaining.min(line.len())]);
                    break;
                }
                input.push_str(&line);
            }
            Err(_) => break,
        }
    }
    input
}

fn interactive_tokenizer(ops: TokenizerOps) {
    println!("\nEnter text (empty line to stop):");
    let input = read_multiline_input();
    if (ops.load_text)(&input) != 0 {
        println!("Failed to load text");
        return;
    }
    println!("\n=== Tokens ===");
    let token_type_names = [
        "EOF",
        "WORD",
        "NUMBER",
        "PUNCT",
        "SPACE",
        "NEWLINE",
        "IDENT",
        "KEYWORD",
        "OPERATOR",
        "STRING",
        "COMMENT",
        "ERROR",
    ];
    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.type_ == TokenType::TokenEof {
            break;
        }
        println!(
            "[{}] '{}' (L{}:C{})",
            token_type_names[token.type_ as usize],
            token.value,
            token.line,
            token.column
        );
        count += 1;
        if count > 100 {
            println!("... (truncated, too many tokens)");
            break;
        }
    }
}

fn read_file(filename: &str) -> Option<String> {
    match fs::read_to_string(filename) {
        Ok(content) => {
            if content.len() > MAX_BUFFER_SIZE {
                eprintln!("Error: File too large");
                None
            } else {
                Some(content)
            }
        }
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", filename);
            None
        }
    }
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops);
    println!("Text Analysis and Tokenization System");
    println!("This system demonstrates function pointers and static globals");
    let stdin = io::stdin();
    loop {
        print_menu();
        let mut input = String::new();
        if stdin.read_line(&mut input).ok().filter(|&n| n > 0).is_none() {
            break;
        }
        let choice = match input.trim().parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };
        match choice {
            1 => {
                println!("Enter text to analyze (empty line to stop):");
                let text = read_multiline_input();
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                print!("Enter filename: ");
                let _ = io::stdout().flush();
                let mut filename = String::new();
                if stdin.read_line(&mut filename).ok().filter(|&n| n > 0).is_none() {
                    continue;
                }
                let filename = filename.trim_end_matches(['\r', '\n']);
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
                let mut pattern = String::new();
                if stdin.read_line(&mut pattern).ok().filter(|&n| n > 0).is_none() {
                    continue;
                }
                let pattern = pattern.trim_end_matches(['\r', '\n']);
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
