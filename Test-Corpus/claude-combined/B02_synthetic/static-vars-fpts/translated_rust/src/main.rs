// Rust translation of c_src/src/main.c

use std::io::{Read, Write};

mod analyzer;
mod tokenizer;

use analyzer::{
    analyze_text, analyzer_init, calculate_complexity_score, find_patterns,
    print_token_distribution, AnalysisResult,
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
    let _ = std::io::stdout().flush();
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

/// Read until '\n' (inclusive) or EOF — matches C `fgets(buf, size, stdin)`
/// when there's enough room in the buffer.
///
/// Returns Some(bytes) including the newline if there was one, or None on EOF
/// before any byte was read (mirrors fgets returning NULL on immediate EOF).
fn fgets<R: Read>(reader: &mut R, max_size: usize) -> Option<Vec<u8>> {
    if max_size <= 1 {
        return None;
    }
    let mut buf = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    loop {
        if buf.len() >= max_size - 1 {
            break;
        }
        match reader.read(&mut byte) {
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

/// Mirrors C `sscanf(input, "%d", &choice)` returning 1 on success.
/// Skips leading C-whitespace, then accepts an optional sign and base-10 digits.
fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r'
        || s[i] == 0x0B || s[i] == 0x0C)
    {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return None;
    }
    // Accumulate using i64 to avoid overflow, then cast (C truncates similarly).
    let mut val: i64 = 0;
    for &b in &s[start..i] {
        val = val.wrapping_mul(10).wrapping_add((b - b'0') as i64);
    }
    if neg {
        val = -val;
    }
    Some(val as i32)
}

/// Mirrors C `strncat(dst, line, MAX_INPUT_SIZE - strlen(dst) - 1)`.
/// dst is treated as a null-terminated buffer of capacity MAX_INPUT_SIZE.
/// We model it as a Vec<u8> that holds only the "string" content (no null).
fn strncat_into(dst: &mut Vec<u8>, src: &[u8], max_total: usize) {
    // Available bytes for content (excluding the implicit null terminator).
    if dst.len() + 1 >= max_total {
        return;
    }
    let avail = max_total - dst.len() - 1;
    let take = src.len().min(avail);
    dst.extend_from_slice(&src[..take]);
}

fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    // Convert filename bytes to OsStr; use lossy conversion via String for portability.
    // The C code calls fopen(filename, "r") and the filename comes from fgets minus
    // trailing newline. We assume filenames are valid UTF-8 (matches C behavior on Linux
    // for typical filenames).
    let name = match std::str::from_utf8(filename) {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(
                std::io::stderr(),
                "Error: Could not open file '{}'",
                String::from_utf8_lossy(filename)
            );
            return None;
        }
    };
    let mut file = match std::fs::File::open(name) {
        Ok(f) => f,
        Err(_) => {
            let _ = writeln!(std::io::stderr(), "Error: Could not open file '{}'", name);
            return None;
        }
    };
    // Determine size via metadata (mirrors fseek/ftell behavior for regular files).
    let size = match file.metadata() {
        Ok(m) => m.len() as usize,
        Err(_) => {
            let _ = writeln!(std::io::stderr(), "Error: Could not open file '{}'", name);
            return None;
        }
    };
    if size > MAX_BUFFER_SIZE {
        let _ = writeln!(std::io::stderr(), "Error: File too large");
        return None;
    }
    let mut content = Vec::with_capacity(size + 1);
    if file.read_to_end(&mut content).is_err() {
        let _ = writeln!(std::io::stderr(), "Error: Memory allocation failed");
        return None;
    }
    // C code only reads up to `size` bytes (the size from ftell). Truncate to that.
    if content.len() > size {
        content.truncate(size);
    }
    Some(content)
}

fn interactive_tokenizer<R: Read>(ops: TokenizerOps, handle: &mut R) {
    print!("\nEnter text (empty line to stop):\n");
    let _ = std::io::stdout().flush();

    let mut input: Vec<u8> = Vec::new();

    loop {
        match fgets(handle, 256) {
            Some(line) => {
                if !line.is_empty() && line[0] == b'\n' {
                    break;
                }
                strncat_into(&mut input, &line, MAX_INPUT_SIZE);
            }
            None => break,
        }
    }

    if (ops.load_text)(&input) != 0 {
        print!("Failed to load text\n");
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
        if token.ty == TokenType::Eof {
            break;
        }
        // printf("[%s] '%s' (L%d:C%d)\n", name, value, line, column);
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(b"[");
        let idx = token.ty as usize;
        let name = token_type_names.get(idx).copied().unwrap_or("");
        let _ = out.write_all(name.as_bytes());
        let _ = out.write_all(b"] '");
        let _ = out.write_all(&token.value);
        let _ = out.write_all(format!("' (L{}:C{})\n", token.line, token.column).as_bytes());
        drop(out);
        count += 1;
        if count > 100 {
            print!("... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops);

    print!("Text Analysis and Tokenization System\n");
    print!("This system demonstrates function pointers and static globals\n");

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    loop {
        print_menu();

        let input = match fgets(&mut handle, 256) {
            Some(v) => v,
            None => break,
        };

        let choice = match sscanf_int(&input) {
            Some(c) => c,
            None => {
                print!("Invalid input\n");
                let _ = std::io::stdout().flush();
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter text to analyze (empty line to stop):\n");
                let _ = std::io::stdout().flush();
                let mut text: Vec<u8> = Vec::new();
                loop {
                    match fgets(&mut handle, 256) {
                        Some(line) => {
                            if !line.is_empty() && line[0] == b'\n' {
                                break;
                            }
                            strncat_into(&mut text, &line, MAX_INPUT_SIZE);
                        }
                        None => break,
                    }
                }
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                print!("Enter filename: ");
                let _ = std::io::stdout().flush();
                let line = match fgets(&mut handle, 256) {
                    Some(v) => v,
                    None => break,
                };
                // input[strcspn(input, "\n")] = 0; — strip the trailing newline if any.
                let trimmed = strip_first_newline(&line);
                if let Some(content) = read_file(trimmed) {
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
                let _ = std::io::stdout().flush();
                let line = match fgets(&mut handle, 256) {
                    Some(v) => v,
                    None => break,
                };
                let trimmed = strip_first_newline(&line);
                find_patterns(trimmed);
            }
            6 => {
                interactive_tokenizer(ops, &mut handle);
            }
            7 => {
                print!("Goodbye!\n");
                let _ = std::io::stdout().flush();
                return;
            }
            _ => {
                print!("Invalid choice\n");
            }
        }
    }
}

/// Returns the slice of `s` up to (but not including) the first '\n'.
/// Mirrors `s[strcspn(s, "\n")] = 0` followed by reading `s` as a C string.
fn strip_first_newline(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == b'\n') {
        Some(idx) => &s[..idx],
        None => s,
    }
}
