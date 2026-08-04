// Main module - port of main.c

mod tokenizer;
mod analyzer;

use std::io::{self, Read, Write};
use std::fs::File;

use tokenizer::{get_tokenizer_ops, Token, TokenType, TokenizerOps, MAX_BUFFER_SIZE};
use analyzer::{analyzer_init, analyze_text, print_token_distribution, calculate_complexity_score, find_patterns, AnalysisResult};

const MAX_INPUT_SIZE: usize = 4096;

// Macro to write formatted output to the global output buffer used by all modules.
// This matches C's stdout (which is fully buffered when redirected).
fn out_write(bytes: &[u8]) {
    crate::io_buf::write_bytes(bytes);
}

fn out_str(s: &str) {
    out_write(s.as_bytes());
}

fn print_menu() {
    out_str("\n=== Text Analyzer ===\n");
    out_str("1. Analyze text\n");
    out_str("2. Load text from file\n");
    out_str("3. Show token distribution\n");
    out_str("4. Calculate complexity score\n");
    out_str("5. Find pattern\n");
    out_str("6. Interactive tokenizer\n");
    out_str("7. Exit\n");
    out_str("Choice: ");
}

fn print_analysis_result(result: AnalysisResult) {
    out_str("\n=== Analysis Results ===\n");
    out_str(&format!("Words/Identifiers: {}\n", result.word_count));
    out_str(&format!("Numbers: {}\n", result.number_count));
    out_str(&format!("Keywords: {}\n", result.keyword_count));
    out_str(&format!("Operators: {}\n", result.operator_count));
    out_str(&format!("Comments: {}\n", result.comment_count));
    out_str(&format!("Strings: {}\n", result.string_count));
    out_str(&format!("Lines: {}\n", result.line_count));
    out_str(&format!("Characters: {}\n", result.char_count));
}

// Read up to (size-1) bytes from stdin until '\n' (inclusive) or EOF.
// Mimic fgets: returns the line as bytes including the '\n' terminator if found,
// or returns None on EOF with no bytes read.
fn fgets(stdin: &mut impl Read, size: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let max_read = size - 1;
    let mut byte = [0u8; 1];
    while buf.len() < max_read {
        match stdin.read(&mut byte) {
            Ok(0) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
        }
    }
    Some(buf)
}

fn strncat(dest: &mut Vec<u8>, src: &[u8], n: usize) {
    let mut i = 0usize;
    while i < n && i < src.len() && src[i] != 0 {
        dest.push(src[i]);
        i += 1;
    }
}

fn strip_newline(s: &mut Vec<u8>) {
    if let Some(pos) = s.iter().position(|&b| b == b'\n') {
        s.truncate(pos);
    }
}

fn interactive_tokenizer<R: Read>(ops: TokenizerOps, stdin: &mut R) {
    out_str("\nEnter text (empty line to stop):\n");

    let mut input: Vec<u8> = Vec::with_capacity(MAX_INPUT_SIZE);

    loop {
        let line_opt = fgets(stdin, 256);
        let line = match line_opt {
            Some(l) => l,
            None => break,
        };
        if !line.is_empty() && line[0] == b'\n' {
            break;
        }
        let avail = MAX_INPUT_SIZE.saturating_sub(input.len()).saturating_sub(1);
        strncat(&mut input, &line, avail);
    }

    if (ops.load_text)(&input) != 0 {
        out_str("Failed to load text\n");
        return;
    }

    out_str("\n=== Tokens ===\n");

    let token_type_names: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE",
        "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count = 0;
    loop {
        let token: Token = (ops.next_token)();
        if token.ttype == TokenType::Eof {
            break;
        }
        let idx = token.ttype as usize;
        let name = if idx < token_type_names.len() {
            token_type_names[idx]
        } else {
            "ERROR"
        };

        let prefix = format!("[{}] '", name);
        out_str(&prefix);
        let value_len = token
            .value
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(token.value.len());
        out_write(&token.value[..value_len]);
        let suffix = format!("' (L{}:C{})\n", token.line, token.column);
        out_str(&suffix);

        count += 1;
        if count > 100 {
            out_str("... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    let filename_str = match std::str::from_utf8(filename) {
        Ok(s) => s,
        Err(_) => {
            // best-effort: still print error to stderr
            let mut stderr = io::stderr();
            let _ = stderr.write_all(b"Error: Could not open file '");
            let _ = stderr.write_all(filename);
            let _ = stderr.write_all(b"'\n");
            return None;
        }
    };

    let file_res = File::open(filename_str);
    let mut file = match file_res {
        Ok(f) => f,
        Err(_) => {
            let mut stderr = io::stderr();
            let _ = stderr.write_all(b"Error: Could not open file '");
            let _ = stderr.write_all(filename);
            let _ = stderr.write_all(b"'\n");
            return None;
        }
    };

    use std::io::Seek;
    let size = match file.seek(io::SeekFrom::End(0)) {
        Ok(s) => s as i64,
        Err(_) => -1,
    };
    let _ = file.seek(io::SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as i64 {
        let _ = writeln!(io::stderr(), "Error: File too large");
        return None;
    }

    let alloc_size = if size < 0 { 0 } else { size as usize };
    let mut content = vec![0u8; alloc_size + 1];
    let mut read_size = 0usize;
    if alloc_size > 0 {
        let mut buf = vec![0u8; alloc_size];
        let mut pos = 0;
        while pos < alloc_size {
            match file.read(&mut buf[pos..]) {
                Ok(0) => break,
                Ok(n) => pos += n,
                Err(_) => break,
            }
        }
        read_size = pos;
        for i in 0..read_size {
            content[i] = buf[i];
        }
    }
    content[read_size] = 0;

    Some(content)
}

pub mod io_buf {
    use std::cell::RefCell;
    use std::io::Write;

    thread_local! {
        // Mimic C's fully-buffered stdout: collect everything until program exit.
        static OUT: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(8192));
    }

    pub fn write_bytes(bytes: &[u8]) {
        OUT.with(|cell| {
            cell.borrow_mut().extend_from_slice(bytes);
        });
    }

    pub fn flush_all() {
        OUT.with(|cell| {
            let mut buf = cell.borrow_mut();
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            let _ = handle.write_all(&buf);
            let _ = handle.flush();
            buf.clear();
        });
    }
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops);

    out_str("Text Analysis and Tokenization System\n");
    out_str("This system demonstrates function pointers and static globals\n");

    let stdin_handle = io::stdin();
    let mut stdin = stdin_handle.lock();

    loop {
        print_menu();

        let input_opt = fgets(&mut stdin, 256);
        let input = match input_opt {
            Some(l) => l,
            None => break,
        };

        let choice = parse_int(&input);
        let choice = match choice {
            Some(n) => n,
            None => {
                out_str("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                out_str("Enter text to analyze (empty line to stop):\n");
                let mut text: Vec<u8> = Vec::with_capacity(MAX_INPUT_SIZE);
                loop {
                    let line_opt = fgets(&mut stdin, 256);
                    let line = match line_opt {
                        Some(l) => l,
                        None => break,
                    };
                    if !line.is_empty() && line[0] == b'\n' {
                        break;
                    }
                    let avail = MAX_INPUT_SIZE.saturating_sub(text.len()).saturating_sub(1);
                    strncat(&mut text, &line, avail);
                }
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                out_str("Enter filename: ");
                let line_opt = fgets(&mut stdin, 256);
                let mut input2 = match line_opt {
                    Some(l) => l,
                    None => break,
                };
                strip_newline(&mut input2);

                if let Some(content) = read_file(&input2) {
                    let result = analyze_text(&content);
                    print_analysis_result(result);
                }
            }
            3 => {
                print_token_distribution();
            }
            4 => {
                let score = calculate_complexity_score();
                out_str(&format!("\nComplexity Score: {}\n", score));
                if score < 10 {
                    out_str("Complexity: Low\n");
                } else if score < 50 {
                    out_str("Complexity: Medium\n");
                } else {
                    out_str("Complexity: High\n");
                }
            }
            5 => {
                out_str("Enter pattern to search: ");
                let line_opt = fgets(&mut stdin, 256);
                let mut input2 = match line_opt {
                    Some(l) => l,
                    None => break,
                };
                strip_newline(&mut input2);
                find_patterns(&input2);
            }
            6 => {
                interactive_tokenizer(ops, &mut stdin);
            }
            7 => {
                out_str("Goodbye!\n");
                io_buf::flush_all();
                return;
            }
            _ => {
                out_str("Invalid choice\n");
            }
        }
    }
    io_buf::flush_all();
}

fn parse_int(input: &[u8]) -> Option<i32> {
    let mut i = 0;
    let n = input.len();
    while i < n && matches!(input[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < n && (input[i] == b'+' || input[i] == b'-') {
        if input[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    if i >= n || !(input[i] >= b'0' && input[i] <= b'9') {
        return None;
    }
    let mut value: i64 = 0;
    while i < n && input[i] >= b'0' && input[i] <= b'9' {
        value = value.wrapping_mul(10).wrapping_add((input[i] - b'0') as i64);
        i += 1;
    }
    value = value.wrapping_mul(sign);
    Some(value as i32)
}
