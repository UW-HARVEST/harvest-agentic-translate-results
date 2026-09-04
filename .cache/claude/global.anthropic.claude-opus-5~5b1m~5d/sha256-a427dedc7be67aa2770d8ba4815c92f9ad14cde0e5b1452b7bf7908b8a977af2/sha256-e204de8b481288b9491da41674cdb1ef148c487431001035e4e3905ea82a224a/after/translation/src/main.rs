//! Translation of `main.c`.

mod analyzer;
mod cio;
mod tokenizer;

use std::io::{Read, Seek, SeekFrom};

use analyzer::{
    analyze_text, analyzer_init, calculate_complexity_score, find_patterns,
    print_token_distribution, AnalysisResult,
};
use cio::{c_str, err_bytes, err_str, fgets, out_bytes, out_flush, out_str, sscanf_int};
use tokenizer::{get_tokenizer_ops, TokenType, TokenizerOps, MAX_BUFFER_SIZE};

const MAX_INPUT_SIZE: usize = 4096;

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

/// `strncat(dest, src, n)` where `dest` is a C string of capacity `capacity`.
fn strncat(dest: &mut Vec<u8>, src: &[u8], n: usize) {
    let src = c_str(src);
    let take = src.len().min(n);
    dest.extend_from_slice(&src[..take]);
}

fn interactive_tokenizer(ops: TokenizerOps) {
    out_str("\nEnter text (empty line to stop):\n");

    let mut input: Vec<u8> = Vec::new();

    while let Some(line) = fgets(256) {
        if line[0] == b'\n' {
            break;
        }
        let remaining = MAX_INPUT_SIZE - input.len() - 1;
        strncat(&mut input, &line, remaining);
    }

    if (ops.load_text)(&input) != 0 {
        out_str("Failed to load text\n");
        return;
    }

    out_str("\n=== Tokens ===\n");

    const TOKEN_TYPE_NAMES: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count = 0;

    loop {
        let token = (ops.next_token)();
        if token.ttype == TokenType::Eof {
            break;
        }

        out_str(&format!("[{}] '", TOKEN_TYPE_NAMES[token.ttype.index()]));
        out_bytes(&token.value);
        out_str(&format!("' (L{}:C{})\n", token.line, token.column));
        count += 1;

        if count > 100 {
            out_str("... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;

    let path = std::path::Path::new(std::ffi::OsStr::from_bytes(filename));
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            err_str("Error: Could not open file '");
            err_bytes(filename);
            err_str("'\n");
            return None;
        }
    };

    // fseek(file, 0, SEEK_END); size = ftell(file); fseek(file, 0, SEEK_SET);
    //
    // The C code ignores the return value of `fseek`.  When seeking to the end
    // fails (procfs files reject SEEK_END, for instance) the stream position is
    // left untouched and the following `ftell` reports that position instead --
    // it does not report an error.  Only a stream on which `ftell` itself fails
    // yields -1.
    let size: i64 = match file.seek(SeekFrom::End(0)) {
        Ok(pos) => pos as i64,
        Err(_) => match file.stream_position() {
            Ok(pos) => pos as i64,
            Err(_) => -1,
        },
    };
    let _ = file.seek(SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as i64 {
        err_str("Error: File too large\n");
        return None;
    }

    // `malloc(size + 1)`.  When `ftell` failed (a non-seekable stream such as a
    // pipe or a FIFO) `size` is -1, so this is `malloc(0)`, which succeeds and
    // hands back a zero-byte block.  The subsequent
    // `fread(content, 1, (size_t)-1, file)` then asks the kernel to read into
    // that empty block, `read` fails with EFAULT and `fread` reports 0 bytes.
    // The observable result is an empty string, never an allocation failure.
    let alloc_len = if size < 0 { 0usize } else { size as usize };
    let readable = size >= 0;

    // fread(content, 1, size, file)
    let mut content = vec![0u8; alloc_len];
    if !readable {
        return Some(Vec::new());
    }
    let mut read_size = 0usize;
    while read_size < content.len() {
        match file.read(&mut content[read_size..]) {
            Ok(0) => break,
            Ok(n) => read_size += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    content.truncate(read_size);

    // content[read_size] = '\0'; the value seen by the callers is the C string.
    Some(c_str(&content).to_vec())
}

fn main() {
    cio::restore_sigpipe();

    // Get tokenizer operations (function pointers)
    let ops = get_tokenizer_ops();

    // Initialize analyzer with function pointers
    analyzer_init(ops);

    out_str("Text Analysis and Tokenization System\n");
    out_str("This system demonstrates function pointers and static globals\n");

    loop {
        print_menu();

        let input = match fgets(256) {
            Some(line) => line,
            None => break,
        };

        let choice = match sscanf_int(c_str(&input)) {
            Some(value) => value,
            None => {
                out_str("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                out_str("Enter text to analyze (empty line to stop):\n");
                let mut text: Vec<u8> = Vec::new();

                while let Some(line) = fgets(256) {
                    if line[0] == b'\n' {
                        break;
                    }
                    let remaining = MAX_INPUT_SIZE - text.len() - 1;
                    strncat(&mut text, &line, remaining);
                }

                let result = analyze_text(&text);
                print_analysis_result(result);
            }

            2 => {
                out_str("Enter filename: ");
                let input = match fgets(256) {
                    Some(line) => line,
                    // `break` in the C source leaves the switch, not the loop.
                    None => continue,
                };
                let input = truncate_at_newline(&input);

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
                let input = match fgets(256) {
                    Some(line) => line,
                    // `break` in the C source leaves the switch, not the loop.
                    None => continue,
                };
                let input = truncate_at_newline(&input);

                find_patterns(&input);
            }

            6 => {
                interactive_tokenizer(ops);
            }

            7 => {
                out_str("Goodbye!\n");
                out_flush();
                return;
            }

            _ => {
                out_str("Invalid choice\n");
            }
        }
    }

    out_flush();
}

/// `input[strcspn(input, "\n")] = 0;`
fn truncate_at_newline(buf: &[u8]) -> Vec<u8> {
    let s = c_str(buf);
    match s.iter().position(|&b| b == b'\n') {
        Some(i) => s[..i].to_vec(),
        None => s.to_vec(),
    }
}
