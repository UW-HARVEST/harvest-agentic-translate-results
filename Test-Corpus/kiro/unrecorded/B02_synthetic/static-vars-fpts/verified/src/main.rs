use std::io::{self, BufRead, Write, Read};
use std::fs;
use text_analyzer::tokenizer::*;
use text_analyzer::analyzer::*;

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
    io::stdout().flush().unwrap_or(());
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

fn read_multiline(stdin: &io::Stdin) -> String {
    let mut text = String::new();
    let lock = stdin.lock();
    for line_result in lock.lines() {
        match line_result {
            Ok(line) => {
                if line.is_empty() {
                    break;
                }
                // strncat behavior: append line + newline, respecting max size
                let remaining = MAX_INPUT_SIZE.saturating_sub(text.len()).saturating_sub(1);
                let to_append = format!("{}\n", line);
                if remaining == 0 {
                    break;
                }
                let take = to_append.len().min(remaining);
                text.push_str(&to_append[..take]);
            }
            Err(_) => break,
        }
    }
    text
}

fn interactive_tokenizer() {
    print!("\nEnter text (empty line to stop):\n");
    io::stdout().flush().unwrap_or(());

    let stdin = io::stdin();
    let input = read_multiline(&stdin);

    if tokenizer_load_text_rs(&input) != 0 {
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
        let token = tokenizer_next_token_rs();
        if token.token_type == TokenType::Eof {
            break;
        }
        print!("[{}] '{}' (L{}:C{})\n",
            token_type_names[token.token_type as usize],
            token.value,
            token.line,
            token.column);
        count += 1;
        if count > 100 {
            print!("... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file(filename: &str) -> Option<String> {
    let metadata = match fs::metadata(filename) {
        Ok(m) => m,
        Err(_) => {
            eprint!("Error: Could not open file '{}'\n", filename);
            return None;
        }
    };

    if metadata.len() as usize > MAX_BUFFER_SIZE {
        eprint!("Error: File too large\n");
        return None;
    }

    let mut file = match fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprint!("Error: Could not open file '{}'\n", filename);
            return None;
        }
    };

    let mut content = String::new();
    match file.read_to_string(&mut content) {
        Ok(_) => Some(content),
        Err(_) => {
            eprint!("Error: Memory allocation failed\n");
            None
        }
    }
}

fn main() {
    analyzer_init_rs();

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
            Ok(v) => v,
            Err(_) => {
                print!("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter text to analyze (empty line to stop):\n");
                io::stdout().flush().unwrap_or(());
                let text = read_multiline(&stdin);
                let result = analyze_text_rs(&text);
                print_analysis_result(&result);
            }
            2 => {
                print!("Enter filename: ");
                io::stdout().flush().unwrap_or(());
                let mut fname = String::new();
                if stdin.lock().read_line(&mut fname).unwrap_or(0) == 0 {
                    break;
                }
                let fname = fname.trim_end_matches('\n').trim_end_matches('\r');
                if let Some(content) = read_file(fname) {
                    let result = analyze_text_rs(&content);
                    print_analysis_result(&result);
                }
            }
            3 => {
                print_token_distribution_rs();
            }
            4 => {
                let score = calculate_complexity_score_rs();
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
                io::stdout().flush().unwrap_or(());
                let mut pat = String::new();
                if stdin.lock().read_line(&mut pat).unwrap_or(0) == 0 {
                    break;
                }
                let pat = pat.trim_end_matches('\n').trim_end_matches('\r');
                find_patterns_rs(pat);
            }
            6 => {
                interactive_tokenizer();
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
