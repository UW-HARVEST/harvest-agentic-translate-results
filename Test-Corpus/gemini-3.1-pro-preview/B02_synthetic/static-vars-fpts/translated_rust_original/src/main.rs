mod tokenizer;
mod analyzer;

use tokenizer::{get_tokenizer_ops, TokenizerOps, TokenType};
use analyzer::{analyzer_init, analyze_text, print_token_distribution, calculate_complexity_score, find_patterns, AnalysisResult};
use std::io::{self, Write, Read};
use std::fs::File;

const MAX_INPUT_SIZE: usize = 4096;
const MAX_BUFFER_SIZE: usize = 8192;

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

fn interactive_tokenizer(ops: &TokenizerOps) {
    println!("\nEnter text (empty line to stop):");
    
    let mut input = String::new();
    loop {
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() || line == "\n" || line == "\r\n" {
            break;
        }
        input.push_str(&line);
        if input.len() >= MAX_INPUT_SIZE {
            break;
        }
    }
    
    if (ops.load_text)(&input).is_err() {
        println!("Failed to load text");
        return;
    }
    
    println!("\n=== Tokens ===");
    
    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        
        let type_name = match token.token_type {
            TokenType::Eof => "EOF",
            TokenType::Word => "WORD",
            TokenType::Number => "NUMBER",
            TokenType::Punctuation => "PUNCT",
            TokenType::Whitespace => "SPACE",
            TokenType::Newline => "NEWLINE",
            TokenType::Identifier => "IDENT",
            TokenType::Keyword => "KEYWORD",
            TokenType::Operator => "OPERATOR",
            TokenType::String => "STRING",
            TokenType::Comment => "COMMENT",
            TokenType::Error => "ERROR",
        };
        
        println!("[{}] '{}' (L{}:C{})", type_name, token.value, token.line, token.column);
        count += 1;
        
        if count > 100 {
            println!("... (truncated, too many tokens)");
            break;
        }
    }
}

fn read_file(filename: &str) -> Option<String> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", filename);
            return None;
        }
    };
    
    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        eprintln!("Error: Failed to read file");
        return None;
    }
    
    if content.len() > MAX_BUFFER_SIZE {
        eprintln!("Error: File too large");
        return None;
    }
    
    Some(content)
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops.clone());
    
    println!("Text Analysis and Tokenization System");
    println!("This system demonstrates function pointers and static globals");
    
    loop {
        print_menu();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() || input.trim().is_empty() {
            break;
        }
        
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
                    let mut line = String::new();
                    if io::stdin().read_line(&mut line).is_err() || line == "\n" || line == "\r\n" {
                        break;
                    }
                    text.push_str(&line);
                    if text.len() >= MAX_INPUT_SIZE {
                        break;
                    }
                }
                
                let result = analyze_text(&text);
                print_analysis_result(&result);
            }
            2 => {
                print!("Enter filename: ");
                io::stdout().flush().unwrap();
                let mut filename = String::new();
                if io::stdin().read_line(&mut filename).is_ok() {
                    let filename = filename.trim();
                    if let Some(content) = read_file(filename) {
                        let result = analyze_text(&content);
                        print_analysis_result(&result);
                    }
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
                let mut pattern = String::new();
                if io::stdin().read_line(&mut pattern).is_ok() {
                    find_patterns(pattern.trim_end_matches(|c| c == '\n' || c == '\r'));
                }
            }
            6 => {
                interactive_tokenizer(&ops);
            }
            7 => {
                println!("Goodbye!");
                break;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}
