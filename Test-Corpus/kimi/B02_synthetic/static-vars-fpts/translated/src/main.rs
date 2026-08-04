mod tokenizer;
mod analyzer;

use std::io::{self, Write};
use std::fs;
use tokenizer::{get_tokenizer_ops, TokenizerOps, TokenType};
use analyzer::{analyzer_init, analyze_text, AnalysisResult, print_token_distribution, calculate_complexity_score, find_patterns};

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
    let mut line = String::new();
    
    loop {
        line.clear();
        io::stdin().read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        if input.len() + line.len() < MAX_INPUT_SIZE {
            input.push_str(&line);
        }
    }
    
    if ops.load_text(&input) != 0 {
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
        let token = ops.next_token();
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
    
    let mut input = String::new();
    
    loop {
        print_menu();
        
        input.clear();
        if io::stdin().read_line(&mut input).is_err() {
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
                let mut line = String::new();
                
                loop {
                    line.clear();
                    io::stdin().read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    if text.len() + line.len() < MAX_INPUT_SIZE {
                        text.push_str(&line);
                    }
                }
                
                let result = analyze_text(&text);
                print_analysis_result(&result);
            }
            
            2 => {
                print!("Enter filename: ");
                io::stdout().flush().unwrap();
                input.clear();
                io::stdin().read_line(&mut input).unwrap();
                let filename = input.trim();
                
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
                input.clear();
                io::stdin().read_line(&mut input).unwrap();
                let pattern = input.trim();
                
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
