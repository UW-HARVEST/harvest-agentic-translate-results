//! Port of c_src/src/main.c

mod analyzer;
pub mod cio;
mod tokenizer;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;

use analyzer::{
    analyze_text, analyzer_init, calculate_complexity_score, find_patterns,
    print_token_distribution, AnalysisResult,
};
use cio::{cstr, out_bytes, truncate_at_newline, CStdin};
use tokenizer::{get_tokenizer_ops, Token, TokenizerOps, TOKEN_EOF, MAX_BUFFER_SIZE};

const MAX_INPUT_SIZE: usize = 4096;

fn print_menu() {
    cprintf!("\n=== Text Analyzer ===\n");
    cprintf!("1. Analyze text\n");
    cprintf!("2. Load text from file\n");
    cprintf!("3. Show token distribution\n");
    cprintf!("4. Calculate complexity score\n");
    cprintf!("5. Find pattern\n");
    cprintf!("6. Interactive tokenizer\n");
    cprintf!("7. Exit\n");
    cprintf!("Choice: ");
}

fn print_analysis_result(result: AnalysisResult) {
    cprintf!("\n=== Analysis Results ===\n");
    cprintf!("Words/Identifiers: {}\n", result.word_count);
    cprintf!("Numbers: {}\n", result.number_count);
    cprintf!("Keywords: {}\n", result.keyword_count);
    cprintf!("Operators: {}\n", result.operator_count);
    cprintf!("Comments: {}\n", result.comment_count);
    cprintf!("Strings: {}\n", result.string_count);
    cprintf!("Lines: {}\n", result.line_count);
    cprintf!("Characters: {}\n", result.char_count);
}

/// `strncat(dst, src, MAX_INPUT_SIZE - strlen(dst) - 1)` where `dst` is a
/// `char[MAX_INPUT_SIZE]` holding a C string.
fn strncat_bounded(dst: &mut Vec<u8>, src: &[u8]) {
    let n = MAX_INPUT_SIZE - dst.len() - 1;
    let src = cstr(src);
    let take = if src.len() < n { src.len() } else { n };
    dst.extend_from_slice(&src[..take]);
}

/// Reads lines with fgets(line, 256, stdin) until an empty line or EOF,
/// accumulating into a MAX_INPUT_SIZE C-string buffer.
fn read_block(stdin: &mut CStdin) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();

    while let Some(line) = stdin.fgets(256) {
        if line.first() == Some(&b'\n') {
            break;
        }
        strncat_bounded(&mut buffer, &line);
    }

    buffer
}

fn interactive_tokenizer(ops: TokenizerOps, stdin: &mut CStdin) {
    cprintf!("\nEnter text (empty line to stop):\n");

    let input = read_block(stdin);

    if (ops.load_text)(&input) != 0 {
        cprintf!("Failed to load text\n");
        return;
    }

    cprintf!("\n=== Tokens ===\n");

    const TOKEN_TYPE_NAMES: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count: i32 = 0;

    loop {
        let token: Token = (ops.next_token)();
        if token.ttype == TOKEN_EOF {
            break;
        }

        let mut line: Vec<u8> = Vec::new();
        line.extend_from_slice(b"[");
        line.extend_from_slice(TOKEN_TYPE_NAMES[token.ttype].as_bytes());
        line.extend_from_slice(b"] '");
        line.extend_from_slice(&token.value);
        line.extend_from_slice(b"' (L");
        line.extend_from_slice(token.line.to_string().as_bytes());
        line.extend_from_slice(b":C");
        line.extend_from_slice(token.column.to_string().as_bytes());
        line.extend_from_slice(b")\n");
        out_bytes(&line);

        count += 1;

        if count > 100 {
            cprintf!("... (truncated, too many tokens)\n");
            break;
        }
    }
}

/// Returns the raw file contents as a C string (NUL-terminated in the C, so
/// callers see the bytes up to the first NUL).
fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    let path = std::ffi::OsStr::from_bytes(filename);

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            let mut msg: Vec<u8> = Vec::new();
            msg.extend_from_slice(b"Error: Could not open file '");
            msg.extend_from_slice(filename);
            msg.extend_from_slice(b"'\n");
            cio::err_bytes(&msg);
            return None;
        }
    };

    // fseek(file, 0, SEEK_END); size = ftell(file); fseek(file, 0, SEEK_SET);
    let size: i64 = match file.seek(SeekFrom::End(0)) {
        Ok(pos) => pos as i64,
        Err(_) => -1,
    };
    let _ = file.seek(SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as i64 {
        cio::err_bytes(b"Error: File too large\n");
        return None;
    }

    let capacity = if size > 0 { size as usize } else { 0 };
    let mut content = vec![0u8; capacity];

    // fread(content, 1, size, file)
    let mut read_size = 0usize;
    while read_size < capacity {
        match file.read(&mut content[read_size..]) {
            Ok(0) => break,
            Ok(n) => read_size += n,
            Err(_) => break,
        }
    }
    content.truncate(read_size);

    // content[read_size] = '\0'  =>  the C string is the bytes up to the
    // first embedded NUL.
    Some(cstr(&content).to_vec())
}

fn real_main() -> i32 {
    let mut stdin = CStdin::new();

    // Get tokenizer operations (function pointers)
    let ops = get_tokenizer_ops();

    // Initialize analyzer with function pointers
    analyzer_init(ops);

    cprintf!("Text Analysis and Tokenization System\n");
    cprintf!("This system demonstrates function pointers and static globals\n");

    loop {
        print_menu();

        let input = match stdin.fgets(256) {
            Some(line) => line,
            None => break,
        };

        let choice = match cio::sscanf_int(cstr(&input)) {
            Some(v) => v,
            None => {
                cprintf!("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                cprintf!("Enter text to analyze (empty line to stop):\n");
                let text = read_block(&mut stdin);

                let result = analyze_text(&text);
                print_analysis_result(result);
            }

            2 => {
                cprintf!("Enter filename: ");
                // NOTE (faithful to the C): on EOF the C code's `break` leaves
                // the switch, not the while loop, so the menu is shown again.
                if let Some(line) = stdin.fgets(256) {
                    let filename = truncate_at_newline(cstr(&line)).to_vec();

                    if let Some(content) = read_file(&filename) {
                        let result = analyze_text(&content);
                        print_analysis_result(result);
                    }
                }
            }

            3 => {
                print_token_distribution();
            }

            4 => {
                let score = calculate_complexity_score();
                cprintf!("\nComplexity Score: {}\n", score);
                if score < 10 {
                    cprintf!("Complexity: Low\n");
                } else if score < 50 {
                    cprintf!("Complexity: Medium\n");
                } else {
                    cprintf!("Complexity: High\n");
                }
            }

            5 => {
                cprintf!("Enter pattern to search: ");
                // NOTE (faithful to the C): on EOF the C code's `break` leaves
                // the switch, not the while loop, so the menu is shown again.
                if let Some(line) = stdin.fgets(256) {
                    let pattern = truncate_at_newline(cstr(&line)).to_vec();

                    find_patterns(&pattern);
                }
            }

            6 => {
                interactive_tokenizer(ops, &mut stdin);
            }

            7 => {
                cprintf!("Goodbye!\n");
                return 0;
            }

            _ => {
                cprintf!("Invalid choice\n");
            }
        }
    }

    0
}

fn main() {
    let code = real_main();
    cio::out_flush();
    std::process::exit(code);
}
