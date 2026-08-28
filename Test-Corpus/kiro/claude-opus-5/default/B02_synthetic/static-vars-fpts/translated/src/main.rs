//! Translation of main.c

mod analyzer;
mod cio;
mod tokenizer;

use analyzer::{AnalysisResult, Analyzer};
use cio::{cstr, err_bytes, err_str, sscanf_d, In, Out};
use std::io::{Read, Seek, SeekFrom};
use tokenizer::{Tokenizer, MAX_BUFFER_SIZE, TOKEN_EOF};

const MAX_INPUT_SIZE: usize = 4096;
/// Size of main's `char input[256]` / `char line[256]` buffers, i.e. the `n`
/// passed to fgets.
const LINE_BUF: usize = 256;

fn print_menu(out: &mut Out) {
    out.str("\n=== Text Analyzer ===\n");
    out.str("1. Analyze text\n");
    out.str("2. Load text from file\n");
    out.str("3. Show token distribution\n");
    out.str("4. Calculate complexity score\n");
    out.str("5. Find pattern\n");
    out.str("6. Interactive tokenizer\n");
    out.str("7. Exit\n");
    out.str("Choice: ");
}

fn print_analysis_result(out: &mut Out, result: &AnalysisResult) {
    out.str("\n=== Analysis Results ===\n");
    out.str(&format!("Words/Identifiers: {}\n", result.word_count));
    out.str(&format!("Numbers: {}\n", result.number_count));
    out.str(&format!("Keywords: {}\n", result.keyword_count));
    out.str(&format!("Operators: {}\n", result.operator_count));
    out.str(&format!("Comments: {}\n", result.comment_count));
    out.str(&format!("Strings: {}\n", result.string_count));
    out.str(&format!("Lines: {}\n", result.line_count));
    out.str(&format!("Characters: {}\n", result.char_count));
}

/// `strncat(dest, src, MAX_INPUT_SIZE - strlen(dest) - 1)` where `src` is the
/// C string held in the fgets buffer.
fn strncat_bounded(dest: &mut Vec<u8>, src: &[u8]) {
    let n = MAX_INPUT_SIZE - dest.len() - 1;
    let k = if src.len() < n { src.len() } else { n };
    dest.extend_from_slice(&src[..k]);
}

/// Read lines until an empty line or EOF, accumulating into a 4096-byte buffer.
fn read_block(inp: &mut In) -> Vec<u8> {
    let mut text: Vec<u8> = Vec::new();

    while let Some(line) = inp.fgets(LINE_BUF) {
        if line[0] == b'\n' {
            break;
        }
        strncat_bounded(&mut text, cstr(&line));
    }

    text
}

fn interactive_tokenizer(out: &mut Out, inp: &mut In, tk: &mut Tokenizer) {
    out.str("\nEnter text (empty line to stop):\n");

    let input = read_block(inp);

    if tk.load_text(&input) != 0 {
        out.str("Failed to load text\n");
        return;
    }

    out.str("\n=== Tokens ===\n");

    let token_type_names: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count: i32 = 0;

    loop {
        let token = tk.next_token();
        if token.ttype == TOKEN_EOF {
            break;
        }

        out.str(&format!("[{}] '", token_type_names[token.ttype]));
        out.bytes(&token.value);
        out.str(&format!("' (L{}:C{})\n", token.line, token.column));
        count += 1;

        // Checked after the increment, so 101 tokens are printed before this trips.
        if count > 100 {
            out.str("... (truncated, too many tokens)\n");
            break;
        }
    }
}

/// Returns the file contents as a C string (bytes up to the first NUL), or None
/// on any of the error paths the C version reports.
fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;

    let path = std::ffi::OsStr::from_bytes(filename);
    let file = std::fs::File::open(path);
    let mut file = match file {
        Ok(f) => f,
        Err(_) => {
            err_str("Error: Could not open file '");
            err_bytes(filename);
            err_str("'\n");
            return None;
        }
    };

    // fseek(END) / ftell / fseek(SET)
    let size: i64 = match file.seek(SeekFrom::End(0)) {
        Ok(s) => s as i64,
        Err(_) => -1,
    };
    let _ = file.seek(SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as i64 {
        err_str("Error: File too large\n");
        return None;
    }

    // fread(content, 1, size, file); content[read_size] = '\0';
    let mut content: Vec<u8> = Vec::new();
    if size > 0 {
        let mut handle = file.take(size as u64);
        if handle.read_to_end(&mut content).is_err() {
            content.clear();
        }
    }

    Some(cstr(&content).to_vec())
}

fn main() {
    let mut out = Out::new();
    let mut inp = In::new();

    // Tokenizer state (C: file-scope statics) and analyzer initialization.
    let mut tk = Tokenizer::new();
    let mut an = Analyzer::new();
    an.init();

    out.str("Text Analysis and Tokenization System\n");
    out.str("This system demonstrates function pointers and static globals\n");

    loop {
        print_menu(&mut out);

        let input = match inp.fgets(LINE_BUF) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_d(cstr(&input)) {
            Some(c) => c,
            None => {
                out.str("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                out.str("Enter text to analyze (empty line to stop):\n");
                let text = read_block(&mut inp);

                let result = an.analyze_text(&mut tk, &text);
                print_analysis_result(&mut out, &result);
            }

            2 => {
                out.str("Enter filename: ");
                // On EOF the C code does `break`, which only leaves the switch;
                // the menu is printed once more before the loop exits.
                if let Some(line) = inp.fgets(LINE_BUF) {
                    let filename = strip_newline(cstr(&line));

                    if let Some(content) = read_file(filename) {
                        let result = an.analyze_text(&mut tk, &content);
                        print_analysis_result(&mut out, &result);
                    }
                }
            }

            3 => {
                an.print_token_distribution(&mut out);
            }

            4 => {
                let score = an.calculate_complexity_score();
                out.str(&format!("\nComplexity Score: {}\n", score));
                if score < 10 {
                    out.str("Complexity: Low\n");
                } else if score < 50 {
                    out.str("Complexity: Medium\n");
                } else {
                    out.str("Complexity: High\n");
                }
            }

            5 => {
                out.str("Enter pattern to search: ");
                if let Some(line) = inp.fgets(LINE_BUF) {
                    let pattern = strip_newline(cstr(&line)).to_vec();
                    an.find_patterns(&mut tk, &mut out, &pattern);
                }
            }

            6 => {
                interactive_tokenizer(&mut out, &mut inp, &mut tk);
            }

            7 => {
                out.str("Goodbye!\n");
                out.flush();
                return;
            }

            _ => {
                out.str("Invalid choice\n");
            }
        }
    }

    out.flush();
}

/// `s[strcspn(s, "\n")] = 0`
fn strip_newline(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == b'\n') {
        Some(i) => &s[..i],
        None => s,
    }
}
