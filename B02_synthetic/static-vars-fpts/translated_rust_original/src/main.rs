mod tokenizer;
mod analyzer;

use std::io::{self, BufRead, Write};
use std::fs;

use tokenizer::*;
use analyzer::*;

const MAX_INPUT_SIZE: usize = 4096;

fn print_menu() {
    print!("\n=== Text Analyzer ===\n");
    print!("1. Analyze text\n");
    print!("2. Load text from file\n");
    print!("3. Show token distribution\n");
    print!("4. Calculate complexity score\n");
    print!("5. Find pattern\n");
    print!("6. Interactive tokenizer\n");
    print!("7. Exit\n");
    print!("Choice: ");
    io::stdout().flush().unwrap();
}

fn print_analysis_result(result: &AnalysisResult) {
    print!("\n=== Analysis Results ===\n");
    print!("Words/Identifiers: {}\n", result.word_count);
    print!("Numbers: {}\n", result.number_count);
    print!("Keywords: {}\n", result.keyword_count);
    print!("Operators: {}\n", result.operator_count);
    print!("Comments: {}\n", result.comment_count);
    print!("Strings: {}\n", result.string_count);
    print!("Lines: {}\n", result.line_count);
    print!("Characters: {}\n", result.char_count);
}

fn interactive_tokenizer(ops: &TokenizerOps) {
    print!("\nEnter text (empty line to stop):\n");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    let stdin = io::stdin();

    for line_result in stdin.lock().lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() {
            break;
        }
        // fgets includes the newline; BufRead::lines() strips it. Re-add it.
        let remaining = MAX_INPUT_SIZE - input.len() - 1;
        let to_append = format!("{}\n", line);
        if to_append.len() <= remaining {
            input.push_str(&to_append);
        } else {
            input.push_str(&to_append[..remaining]);
        }
    }

    if (ops.load_text)(&input) != 0 {
        print!("Failed to load text\n");
        return;
    }

    print!("\n=== Tokens ===\n");

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
        print!(
            "[{}] '{}' (L{}:C{})\n",
            token_type_names[token.token_type as usize],
            token.value,
            token.line,
            token.column
        );
        count += 1;
        if count > 100 {
            print!("... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file(filename: &str) -> Option<String> {
    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            eprint!("Error: Could not open file '{}'\n", filename);
            return None;
        }
    };
    if content.len() > MAX_BUFFER_SIZE {
        eprint!("Error: File too large\n");
        return None;
    }
    Some(content)
}

fn read_multiline_input() -> String {
    let mut text = String::new();
    let stdin = io::stdin();
    for line_result in stdin.lock().lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() {
            break;
        }
        let to_append = format!("{}\n", line);
        let remaining = MAX_INPUT_SIZE - text.len() - 1;
        if to_append.len() <= remaining {
            text.push_str(&to_append);
        } else {
            text.push_str(&to_append[..remaining]);
        }
    }
    text
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops.clone());

    print!("Text Analysis and Tokenization System\n");
    print!("This system demonstrates function pointers and static globals\n");

    let stdin = io::stdin();
    loop {
        print_menu();

        let mut input = String::new();
        if stdin.lock().read_line(&mut input).unwrap_or(0) == 0 {
            break;
        }

        let choice: i32 = match input.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                print!("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter text to analyze (empty line to stop):\n");
                io::stdout().flush().unwrap();
                let text = read_multiline_input();
                let result = analyze_text(&text);
                print_analysis_result(&result);
            }
            2 => {
                print!("Enter filename: ");
                io::stdout().flush().unwrap();
                let mut fname = String::new();
                if stdin.lock().read_line(&mut fname).unwrap_or(0) == 0 {
                    break;
                }
                let fname = fname.trim_end_matches('\n').trim_end_matches('\r');
                if let Some(content) = read_file(fname) {
                    let result = analyze_text(&content);
                    print_analysis_result(&result);
                }
            }
            3 => {
                print_token_distribution();
            }
            4 => {
                let score = calculate_complexity_score();
                print!("\nComplexity Score: {}\n", score);
                if score < 10 {
                    print!("Complexity: Low\n");
                } else if score < 50 {
                    print!("Complexity: Medium\n");
                } else {
                    print!("Complexity: High\n");
                }
            }
            5 => {
                print!("Enter pattern to search: ");
                io::stdout().flush().unwrap();
                let mut pat = String::new();
                if stdin.lock().read_line(&mut pat).unwrap_or(0) == 0 {
                    break;
                }
                let pat = pat.trim_end_matches('\n').trim_end_matches('\r');
                find_patterns(pat);
            }
            6 => {
                interactive_tokenizer(&ops);
            }
            7 => {
                print!("Goodbye!\n");
                return;
            }
            _ => {
                print!("Invalid choice\n");
            }
        }
    }
}
