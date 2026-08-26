//! Translation of c_src/src/main.c

mod analyzer;
mod cio;
mod tokenizer;

use analyzer::{AnalysisResult, Analyzer};
use cio::{err, strncat, sscanf_int, truncate_at_newline, In, Out};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use tokenizer::{TokenType, Tokenizer, MAX_BUFFER_SIZE};

const MAX_INPUT_SIZE: usize = 4096;

fn print_menu(out: &mut Out) {
    out.puts("\n=== Text Analyzer ===\n");
    out.puts("1. Analyze text\n");
    out.puts("2. Load text from file\n");
    out.puts("3. Show token distribution\n");
    out.puts("4. Calculate complexity score\n");
    out.puts("5. Find pattern\n");
    out.puts("6. Interactive tokenizer\n");
    out.puts("7. Exit\n");
    out.puts("Choice: ");
}

fn print_analysis_result(out: &mut Out, result: AnalysisResult) {
    out.puts("\n=== Analysis Results ===\n");
    let _ = write!(out, "Words/Identifiers: {}\n", result.word_count);
    let _ = write!(out, "Numbers: {}\n", result.number_count);
    let _ = write!(out, "Keywords: {}\n", result.keyword_count);
    let _ = write!(out, "Operators: {}\n", result.operator_count);
    let _ = write!(out, "Comments: {}\n", result.comment_count);
    let _ = write!(out, "Strings: {}\n", result.string_count);
    let _ = write!(out, "Lines: {}\n", result.line_count);
    let _ = write!(out, "Characters: {}\n", result.char_count);
}

fn interactive_tokenizer(out: &mut Out, stdin: &mut In, tok: &mut Tokenizer) {
    out.puts("\nEnter text (empty line to stop):\n");

    let mut input: Vec<u8> = Vec::new();

    while let Some(line) = stdin.fgets(256) {
        if line[0] == b'\n' {
            break;
        }
        let room = MAX_INPUT_SIZE.saturating_sub(input.len()).saturating_sub(1);
        strncat(&mut input, &line, room);
    }

    if tok.load_text(&input) != 0 {
        out.puts("Failed to load text\n");
        return;
    }

    out.puts("\n=== Tokens ===\n");

    let token_type_names: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count: i32 = 0;

    loop {
        let token = tok.next_token();
        if token.ttype == TokenType::Eof {
            break;
        }

        let _ = write!(out, "[{}] '", token_type_names[token.ttype.index()]);
        out.put(&token.value);
        let _ = write!(out, "' (L{}:C{})\n", token.line, token.column);
        count += 1;

        if count > 100 {
            out.puts("... (truncated, too many tokens)\n");
            break;
        }
    }
}

/// `read_file`: returns the file contents as a C string (NUL not included).
fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let path = OsStr::from_bytes(filename);
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            let mut msg: Vec<u8> = Vec::new();
            msg.extend_from_slice(b"Error: Could not open file '");
            msg.extend_from_slice(filename);
            msg.extend_from_slice(b"'\n");
            err(&msg);
            return None;
        }
    };

    let size: i64 = match file.seek(SeekFrom::End(0)) {
        Ok(n) => n as i64,
        Err(_) => -1,
    };
    let _ = file.seek(SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as i64 {
        err(b"Error: File too large\n");
        return None;
    }

    let want = if size > 0 { size as u64 } else { 0 };
    let mut content: Vec<u8> = Vec::new();
    let _ = file.take(want).read_to_end(&mut content);

    Some(content)
}

fn main() {
    let mut out = Out::new();
    let mut stdin = In::new();

    // Get tokenizer operations (function pointers)
    let mut tok = Tokenizer::new();

    // Initialize analyzer with function pointers
    let mut an = Analyzer::new();
    an.init();

    out.puts("Text Analysis and Tokenization System\n");
    out.puts("This system demonstrates function pointers and static globals\n");

    loop {
        print_menu(&mut out);

        let input = match stdin.fgets(256) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_int(&input) {
            Some(c) => c,
            None => {
                out.puts("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                out.puts("Enter text to analyze (empty line to stop):\n");
                let mut text: Vec<u8> = Vec::new();

                while let Some(line) = stdin.fgets(256) {
                    if line[0] == b'\n' {
                        break;
                    }
                    let room = MAX_INPUT_SIZE.saturating_sub(text.len()).saturating_sub(1);
                    strncat(&mut text, &line, room);
                }

                let result = an.analyze_text(&mut tok, &text);
                print_analysis_result(&mut out, result);
            }

            2 => {
                out.puts("Enter filename: ");
                let raw = match stdin.fgets(256) {
                    Some(l) => l,
                    None => continue,
                };
                let filename = truncate_at_newline(&raw);

                if let Some(content) = read_file(&filename) {
                    let result = an.analyze_text(&mut tok, &content);
                    print_analysis_result(&mut out, result);
                }
            }

            3 => {
                an.print_token_distribution(&mut out);
            }

            4 => {
                let score = an.calculate_complexity_score();
                let _ = write!(out, "\nComplexity Score: {}\n", score);
                if score < 10 {
                    out.puts("Complexity: Low\n");
                } else if score < 50 {
                    out.puts("Complexity: Medium\n");
                } else {
                    out.puts("Complexity: High\n");
                }
            }

            5 => {
                out.puts("Enter pattern to search: ");
                let raw = match stdin.fgets(256) {
                    Some(l) => l,
                    None => continue,
                };
                let pattern = truncate_at_newline(&raw);

                an.find_patterns(&mut out, &mut tok, &pattern);
            }

            6 => {
                interactive_tokenizer(&mut out, &mut stdin, &mut tok);
            }

            7 => {
                out.puts("Goodbye!\n");
                out.flush_all();
                return;
            }

            _ => {
                out.puts("Invalid choice\n");
            }
        }
    }

    out.flush_all();
}
