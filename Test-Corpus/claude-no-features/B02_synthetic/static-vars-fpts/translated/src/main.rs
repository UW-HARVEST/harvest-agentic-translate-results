// Translated from c_src/src/main.c

mod analyzer;
mod tokenizer;

use std::cell::RefCell;
use std::io::{Read, Write};

use analyzer::{
    analyze_text, analyzer_init, calculate_complexity_score, find_patterns,
    print_token_distribution, AnalysisResult,
};
use tokenizer::{
    get_tokenizer_ops, TokenType, TokenizerOps, MAX_BUFFER_SIZE,
};

const MAX_INPUT_SIZE: usize = 4096;

// To exactly match C's stdout buffering when redirected (full buffering, flushed
// only on program exit), we collect all stdout writes into a global buffer that
// is flushed once at process exit. stderr remains unbuffered, matching C.
thread_local! {
    static OUT_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(64 * 1024));
}

pub fn out_write(bytes: &[u8]) {
    OUT_BUF.with(|b| b.borrow_mut().extend_from_slice(bytes));
}

pub fn out_flush() {
    OUT_BUF.with(|b| {
        let mut buf = b.borrow_mut();
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(&buf);
        let _ = handle.flush();
        buf.clear();
    });
}

fn print_menu() {
    out_write(b"\n=== Text Analyzer ===\n");
    out_write(b"1. Analyze text\n");
    out_write(b"2. Load text from file\n");
    out_write(b"3. Show token distribution\n");
    out_write(b"4. Calculate complexity score\n");
    out_write(b"5. Find pattern\n");
    out_write(b"6. Interactive tokenizer\n");
    out_write(b"7. Exit\n");
    out_write(b"Choice: ");
}

fn print_analysis_result(result: AnalysisResult) {
    out_write(b"\n=== Analysis Results ===\n");
    out_write(format!("Words/Identifiers: {}\n", result.word_count).as_bytes());
    out_write(format!("Numbers: {}\n", result.number_count).as_bytes());
    out_write(format!("Keywords: {}\n", result.keyword_count).as_bytes());
    out_write(format!("Operators: {}\n", result.operator_count).as_bytes());
    out_write(format!("Comments: {}\n", result.comment_count).as_bytes());
    out_write(format!("Strings: {}\n", result.string_count).as_bytes());
    out_write(format!("Lines: {}\n", result.line_count).as_bytes());
    out_write(format!("Characters: {}\n", result.char_count).as_bytes());
}

/// Read one line from stdin like fgets with a fixed-size buffer of `size` bytes.
/// Returns None on EOF with no data read. Otherwise returns the bytes read,
/// including the trailing newline if encountered.
fn fgets_line(stdin: &mut impl Read, size: usize) -> Option<Vec<u8>> {
    if size <= 1 {
        return Some(Vec::new());
    }
    let mut buf = Vec::with_capacity(size);
    let mut byte = [0u8; 1];
    loop {
        if buf.len() >= size - 1 {
            break;
        }
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

/// Append `src` to `dst`, behaving like C's `strncat(dst, src, n)`.
fn strncat_emulate(dst: &mut Vec<u8>, src: &[u8], n: usize) {
    let mut count = 0usize;
    for &b in src.iter() {
        if count >= n {
            break;
        }
        if b == 0 {
            break;
        }
        dst.push(b);
        count += 1;
    }
}

/// Returns the C-string length: bytes up to first NUL or end of slice.
fn c_strlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// Read text accumulator that mimics:
///   char text[MAX_INPUT_SIZE] = "";
///   while (fgets(line,256,stdin)) { if (line[0]=='\n') break;
///       strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1); }
fn read_text_accumulator(stdin: &mut impl Read) -> Vec<u8> {
    let mut text: Vec<u8> = Vec::with_capacity(MAX_INPUT_SIZE);
    loop {
        let line = match fgets_line(stdin, 256) {
            Some(l) => l,
            None => break,
        };
        if line.is_empty() {
            break;
        }
        if line[0] == b'\n' {
            break;
        }
        let cur_len = c_strlen(&text);
        let n = MAX_INPUT_SIZE.saturating_sub(cur_len).saturating_sub(1);
        strncat_emulate(&mut text, &line, n);
    }
    text
}

/// sscanf "%d" emulation
fn sscanf_int(s: &[u8]) -> (i32, i32) {
    let mut i = 0;
    while i < s.len() && (s[i] as char).is_whitespace() {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let start = i;
    let mut val: i64 = 0;
    while i < s.len() && (b'0'..=b'9').contains(&s[i]) {
        val = val * 10 + (s[i] - b'0') as i64;
        i += 1;
    }
    if i == start {
        return (0, 0);
    }
    let v = (val * sign as i64) as i32;
    (1, v)
}

fn interactive_tokenizer(ops: TokenizerOps, stdin: &mut impl Read) {
    out_write(b"\nEnter text (empty line to stop):\n");

    let input = read_text_accumulator(stdin);

    if (ops.load_text)(&input) != 0 {
        out_write(b"Failed to load text\n");
        return;
    }

    out_write(b"\n=== Tokens ===\n");

    let token_type_names: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        out_write(format!("[{}] '", token_type_names[token.token_type as usize]).as_bytes());
        out_write(&token.value);
        out_write(format!("' (L{}:C{})\n", token.line, token.column).as_bytes());
        count += 1;
        if count > 100 {
            out_write(b"... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let path = OsStr::from_bytes(filename);
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => {
            let mut err = std::io::stderr().lock();
            let _ = err.write_all(b"Error: Could not open file '");
            let _ = err.write_all(filename);
            let _ = err.write_all(b"'\n");
            return None;
        }
    };

    let size = match file.metadata() {
        Ok(m) => m.len() as i64,
        Err(_) => 0,
    };

    if size > MAX_BUFFER_SIZE as i64 {
        let mut err = std::io::stderr().lock();
        let _ = err.write_all(b"Error: File too large\n");
        return None;
    }

    let mut content = Vec::with_capacity(size as usize);
    if file.read_to_end(&mut content).is_err() {
        return None;
    }
    if content.len() > size as usize {
        content.truncate(size as usize);
    }
    Some(content)
}

/// Strip first newline from buffer (matches `input[strcspn(input,"\n")] = 0;`)
fn strip_first_newline(buf: &mut Vec<u8>) {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        buf.truncate(pos);
    }
}

fn main() {
    // Ensure stdout buffer is flushed when main returns or panics.
    struct FlushOnDrop;
    impl Drop for FlushOnDrop {
        fn drop(&mut self) {
            out_flush();
        }
    }
    let _flush_guard = FlushOnDrop;

    let ops = get_tokenizer_ops();
    analyzer_init(ops);

    out_write(b"Text Analysis and Tokenization System\n");
    out_write(b"This system demonstrates function pointers and static globals\n");

    let mut stdin = std::io::stdin();

    loop {
        print_menu();

        let input = match fgets_line(&mut stdin, 256) {
            Some(l) => l,
            None => break,
        };

        let (matched, choice) = sscanf_int(&input);
        if matched != 1 {
            out_write(b"Invalid input\n");
            continue;
        }

        match choice {
            1 => {
                out_write(b"Enter text to analyze (empty line to stop):\n");
                let text = read_text_accumulator(&mut stdin);
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                out_write(b"Enter filename: ");
                let mut input2 = match fgets_line(&mut stdin, 256) {
                    Some(l) => l,
                    None => break,
                };
                strip_first_newline(&mut input2);
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
                out_write(format!("\nComplexity Score: {}\n", score).as_bytes());
                if score < 10 {
                    out_write(b"Complexity: Low\n");
                } else if score < 50 {
                    out_write(b"Complexity: Medium\n");
                } else {
                    out_write(b"Complexity: High\n");
                }
            }
            5 => {
                out_write(b"Enter pattern to search: ");
                let mut input2 = match fgets_line(&mut stdin, 256) {
                    Some(l) => l,
                    None => break,
                };
                strip_first_newline(&mut input2);
                find_patterns(&input2);
            }
            6 => {
                interactive_tokenizer(ops, &mut stdin);
            }
            7 => {
                out_write(b"Goodbye!\n");
                return;
            }
            _ => {
                out_write(b"Invalid choice\n");
            }
        }
    }
}
