// Translated from c_src/src/main.c
//
// This binary reproduces the C executable's behavior byte-for-byte,
// including the original program's quirks (e.g., the `case 4` default
// branch on a failed read continues the loop instead of exiting because
// the `break` only escapes the `match`).

mod analyzer;
mod tokenizer;

use std::io::{self, Read, Write};

use analyzer::{
    analyze_text, analyzer_init, calculate_complexity_score, find_patterns, print_token_distribution,
    AnalysisResult,
};
use tokenizer::{get_tokenizer_ops, TokenType, TokenizerOps, MAX_BUFFER_SIZE};

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
    let _ = io::stdout().flush();
}

fn print_analysis_result(result: AnalysisResult) {
    print!("\n=== Analysis Results ===\n");
    println!("Words/Identifiers: {}", result.word_count);
    println!("Numbers: {}", result.number_count);
    println!("Keywords: {}", result.keyword_count);
    println!("Operators: {}", result.operator_count);
    println!("Comments: {}", result.comment_count);
    println!("Strings: {}", result.string_count);
    println!("Lines: {}", result.line_count);
    println!("Characters: {}", result.char_count);
}

/// Reads up to `buf_size - 1` bytes (plus the terminating '\0' that C
/// would write) from stdin into `buf`. Returns true if at least one byte
/// was read (or a newline was encountered) -- i.e., mirrors C's `fgets`
/// returning a non-NULL pointer.
fn fgets(stdin: &mut StdinReader, buf: &mut Vec<u8>, buf_size: usize) -> bool {
    buf.clear();
    if buf_size <= 1 {
        return false;
    }
    let mut got_any = false;
    while buf.len() < buf_size - 1 {
        let mut byte = [0u8; 1];
        match stdin.read_exact(&mut byte) {
            Ok(()) => {
                got_any = true;
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    got_any
}

/// Wraps stdin so we can buffer one byte at a time without losing data
/// across reads (matching libc's stdio buffering well enough for our
/// purposes).
struct StdinReader {
    inner: io::Stdin,
}

impl StdinReader {
    fn new() -> Self {
        StdinReader {
            inner: io::stdin(),
        }
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let mut handle = self.inner.lock();
        handle.read_exact(buf)
    }
}

/// C's `strncat(dst, src, n)` appends as many bytes of src as possible
/// such that the resulting string is at most n bytes (excluding the
/// terminator) longer than the original dst. We model dst as a Vec<u8>
/// without a NUL byte and operate accordingly.
fn strncat(dst: &mut Vec<u8>, src: &[u8], max_append: usize) {
    // Stop at the first embedded NUL in src to mimic C semantics.
    let src_end = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let take = std::cmp::min(src_end, max_append);
    dst.extend_from_slice(&src[..take]);
}

/// Simulates `sscanf(buf, "%d", &choice)` returning 1 on success. Skips
/// leading whitespace (including '\n') and then parses an optional sign
/// followed by digits. Returns Some(value) on success.
fn sscanf_int(buf: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < buf.len() && (buf[i] == b' ' || buf[i] == b'\t' || buf[i] == b'\n'
        || buf[i] == b'\r' || buf[i] == 0x0b || buf[i] == 0x0c)
    {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < buf.len() && (buf[i] == b'+' || buf[i] == b'-') {
        if buf[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    if i >= buf.len() || !buf[i].is_ascii_digit() {
        return None;
    }
    let mut value: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((buf[i] - b'0') as i64);
        i += 1;
    }
    let result = value.wrapping_mul(sign);
    Some(result as i32)
}

fn interactive_tokenizer(ops: TokenizerOps, stdin: &mut StdinReader) {
    print!("\nEnter text (empty line to stop):\n");
    let _ = io::stdout().flush();

    let mut input: Vec<u8> = Vec::new();
    let mut line: Vec<u8> = Vec::new();

    while fgets(stdin, &mut line, 256) {
        if line.first() == Some(&b'\n') {
            break;
        }
        // Append, leaving room for the implicit terminator (matches C's
        // `MAX_INPUT_SIZE - strlen(input) - 1`).
        let remaining = MAX_INPUT_SIZE.saturating_sub(input.len()).saturating_sub(1);
        strncat(&mut input, &line, remaining);
    }

    if (ops.load_text)(&input) != 0 {
        println!("Failed to load text");
        return;
    }

    print!("\n=== Tokens ===\n");

    let token_type_names = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if matches!(token.typ, TokenType::Eof) {
            break;
        }
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = write!(handle, "[{}] '", token_type_names[token.typ as usize]);
        let _ = handle.write_all(&token.value);
        let _ = writeln!(handle, "' (L{}:C{})", token.line, token.column);
        count += 1;
        if count > 100 {
            println!("... (truncated, too many tokens)");
            break;
        }
    }
}

fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    use std::fs::File;
    use std::io::SeekFrom;

    // Convert filename bytes to an OsStr/str for std::fs. Mirror C: pass
    // raw bytes (treat as UTF-8 for simplicity; Amazon dev hosts use
    // UTF-8 by default).
    let path = match std::str::from_utf8(filename) {
        Ok(s) => s,
        Err(_) => {
            eprintln!(
                "Error: Could not open file '{}'",
                String::from_utf8_lossy(filename)
            );
            return None;
        }
    };

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", path);
            return None;
        }
    };

    let mut file = file;
    use std::io::Seek;
    let size = match file.seek(SeekFrom::End(0)) {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", path);
            return None;
        }
    };
    let _ = file.seek(SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as u64 {
        eprintln!("Error: File too large");
        return None;
    }

    let mut content = vec![0u8; size as usize];
    let mut read_total = 0usize;
    while read_total < content.len() {
        match file.read(&mut content[read_total..]) {
            Ok(0) => break,
            Ok(n) => read_total += n,
            Err(_) => break,
        }
    }
    content.truncate(read_total);
    Some(content)
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops);

    print!("Text Analysis and Tokenization System\n");
    print!("This system demonstrates function pointers and static globals\n");
    let _ = io::stdout().flush();

    let mut stdin = StdinReader::new();
    let mut input: Vec<u8> = Vec::new();

    loop {
        print_menu();

        if !fgets(&mut stdin, &mut input, 256) {
            break;
        }

        let choice = match sscanf_int(&input) {
            Some(c) => c,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter text to analyze (empty line to stop):\n");
                let _ = io::stdout().flush();
                let mut text: Vec<u8> = Vec::new();
                let mut line: Vec<u8> = Vec::new();
                while fgets(&mut stdin, &mut line, 256) {
                    if line.first() == Some(&b'\n') {
                        break;
                    }
                    let remaining =
                        MAX_INPUT_SIZE.saturating_sub(text.len()).saturating_sub(1);
                    strncat(&mut text, &line, remaining);
                }
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                print!("Enter filename: ");
                let _ = io::stdout().flush();
                if !fgets(&mut stdin, &mut input, 256) {
                    break;
                }
                // strcspn(input, "\n"): find first '\n' and turn it into '\0'
                // (i.e., truncate). We model "\0" by truncating the Vec.
                if let Some(pos) = input.iter().position(|&b| b == b'\n') {
                    input.truncate(pos);
                }
                if let Some(content) = read_file(&input) {
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
                if !fgets(&mut stdin, &mut input, 256) {
                    break;
                }
                if let Some(pos) = input.iter().position(|&b| b == b'\n') {
                    input.truncate(pos);
                }
                find_patterns(&input);
            }
            6 => {
                interactive_tokenizer(ops, &mut stdin);
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
