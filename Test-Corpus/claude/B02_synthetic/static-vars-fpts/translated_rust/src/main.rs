// Rust translation of c_src/src/main.c

use std::fs::File;
use std::io::{self, Read, Write};

use driver::analyzer::{
    analyze_text, analyzer_init, calculate_complexity_score, find_patterns, print_token_distribution,
    AnalysisResult,
};
use driver::tokenizer::{get_tokenizer_ops, TokenType, TokenizerOps, MAX_BUFFER_SIZE};

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
    print!("Words/Identifiers: {}\n", result.word_count);
    print!("Numbers: {}\n", result.number_count);
    print!("Keywords: {}\n", result.keyword_count);
    print!("Operators: {}\n", result.operator_count);
    print!("Comments: {}\n", result.comment_count);
    print!("Strings: {}\n", result.string_count);
    print!("Lines: {}\n", result.line_count);
    print!("Characters: {}\n", result.char_count);
}

/// Read up to `cap - 1` bytes including a possible trailing newline.
/// Returns:
///   Ok(Some(bytes)) where bytes is null-terminated like fgets's buffer (excluding the null).
///       The bytes slice may include a trailing '\n' if a newline was encountered.
///   Ok(None) if EOF was reached before any data was read (fgets returns NULL).
fn fgets_bytes(cap: usize) -> io::Result<Option<Vec<u8>>> {
    // fgets reads up to cap-1 bytes from stdin, stopping at newline (inclusive),
    // EOF, or after cap-1 bytes have been read; appends a null terminator.
    let max_read = cap.saturating_sub(1);
    let mut out: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    loop {
        if out.len() >= max_read {
            break;
        }
        let n = handle.read(&mut byte)?;
        if n == 0 {
            // EOF.
            if out.is_empty() {
                return Ok(None);
            }
            break;
        }
        out.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    Ok(Some(out))
}

/// Mimics C's `sscanf(input, "%d", &choice)` returning Some(choice) when one int was read,
/// or None otherwise.
fn sscanf_int(input: &[u8]) -> Option<i32> {
    let mut i = 0;
    // Skip whitespace (matches isspace).
    while i < input.len() && matches!(input[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
        i += 1;
    }
    if i >= input.len() {
        return None;
    }
    let mut sign: i64 = 1;
    if input[i] == b'+' {
        i += 1;
    } else if input[i] == b'-' {
        sign = -1;
        i += 1;
    }
    let start = i;
    let mut val: i64 = 0;
    while i < input.len() && input[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((input[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return None;
    }
    let signed = (sign as i64).wrapping_mul(val);
    Some(signed as i32)
}

/// Like C: input[strcspn(input, "\n")] = 0;
/// Truncate at the first newline. Also drop the rest. Returns &[u8] without trailing newline.
fn trim_newline(buf: &mut Vec<u8>) {
    if let Some(pos) = buf.iter().position(|&c| c == b'\n') {
        buf.truncate(pos);
    }
}

/// Mimic strncat(dest, src, n): appends up to n bytes from src (stopping at first null in src),
/// then appends a null. The "buffer" here has capacity `cap` for the C string content
/// (i.e., `dest` is a `char[cap]` so usable string length is cap-1 plus null).
/// The dest is represented as a Vec<u8> containing the current C string contents (no null).
fn strncat_bounded(dest: &mut Vec<u8>, src: &[u8], cap: usize) {
    // Equivalent to strncat(input, line, MAX_INPUT_SIZE - strlen(input) - 1) in the C code.
    // 'n' there is `cap - dest.len() - 1`, which can underflow in C if dest.len() >= cap.
    if dest.len() + 1 >= cap {
        // No room for any more chars (and not even null terminator). C would have UB,
        // but realistically this won't happen because we break before exceeding.
        return;
    }
    let n = cap - dest.len() - 1;
    // Stop at null terminator in src (treat as C string).
    let src_end = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    let src_eff = &src[..src_end];
    let to_copy = src_eff.len().min(n);
    dest.extend_from_slice(&src_eff[..to_copy]);
}

fn interactive_tokenizer(ops: TokenizerOps) {
    print!("\nEnter text (empty line to stop):\n");
    let _ = io::stdout().flush();

    let mut input: Vec<u8> = Vec::new();
    loop {
        match fgets_bytes(256) {
            Ok(Some(line)) => {
                if !line.is_empty() && line[0] == b'\n' {
                    break;
                }
                strncat_bounded(&mut input, &line, MAX_INPUT_SIZE);
            }
            Ok(None) | Err(_) => break,
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
        if token.ttype == TokenType::Eof {
            break;
        }
        let mut stdout = io::stdout();
        let idx = token.ttype as usize;
        let name = token_type_names.get(idx).copied().unwrap_or("");
        let _ = write!(stdout, "[{}] '", name);
        // Print token.value as a C string (until first null).
        let end = token
            .value
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(token.value.len());
        let _ = stdout.write_all(&token.value[..end]);
        let _ = write!(stdout, "' (L{}:C{})\n", token.line, token.column);
        count += 1;
        if count > 100 {
            print!("... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file(filename: &str) -> Option<Vec<u8>> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", filename);
            return None;
        }
    };

    // Determine size (matches fseek SEEK_END / ftell semantics for regular files).
    let size = match file.metadata() {
        Ok(m) => m.len() as i64,
        Err(_) => {
            eprintln!("Error: Could not open file '{}'", filename);
            return None;
        }
    };

    if size > MAX_BUFFER_SIZE as i64 {
        eprintln!("Error: File too large");
        return None;
    }

    // C: fread up to `size` bytes, then null-terminate at read_size.
    let mut content = Vec::with_capacity(size as usize + 1);
    let mut buf = vec![0u8; size as usize];
    let read_size = match file.read(&mut buf) {
        Ok(n) => n,
        Err(_) => 0,
    };
    content.extend_from_slice(&buf[..read_size]);
    Some(content)
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops);

    print!("Text Analysis and Tokenization System\n");
    print!("This system demonstrates function pointers and static globals\n");

    loop {
        print_menu();

        let input = match fgets_bytes(256) {
            Ok(Some(b)) => b,
            _ => break,
        };

        let choice = match sscanf_int(&input) {
            Some(c) => c,
            None => {
                print!("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter text to analyze (empty line to stop):\n");
                let _ = io::stdout().flush();
                let mut text: Vec<u8> = Vec::new();
                loop {
                    match fgets_bytes(256) {
                        Ok(Some(line)) => {
                            if !line.is_empty() && line[0] == b'\n' {
                                break;
                            }
                            strncat_bounded(&mut text, &line, MAX_INPUT_SIZE);
                        }
                        Ok(None) | Err(_) => break,
                    }
                }
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                print!("Enter filename: ");
                let _ = io::stdout().flush();
                let mut input2 = match fgets_bytes(256) {
                    Ok(Some(b)) => b,
                    _ => break,
                };
                trim_newline(&mut input2);
                let filename = match std::str::from_utf8(&input2) {
                    Ok(s) => s.to_string(),
                    Err(_) => {
                        // Best effort: lossy conversion if filename has non-UTF-8 bytes.
                        String::from_utf8_lossy(&input2).into_owned()
                    }
                };
                if let Some(content) = read_file(&filename) {
                    let result = analyze_text(&content);
                    print_analysis_result(result);
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
                let _ = io::stdout().flush();
                let mut input2 = match fgets_bytes(256) {
                    Ok(Some(b)) => b,
                    _ => break,
                };
                trim_newline(&mut input2);
                find_patterns(&input2);
            }
            6 => {
                interactive_tokenizer(ops);
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
