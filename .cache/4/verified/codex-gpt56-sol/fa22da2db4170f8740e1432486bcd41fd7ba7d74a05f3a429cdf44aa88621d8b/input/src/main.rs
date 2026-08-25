mod analyzer;
mod tokenizer;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use analyzer::{AnalysisResult, Analyzer};
use tokenizer::{TokenType, MAX_BUFFER_SIZE};

const MAX_INPUT_SIZE: usize = 4096;

struct Input {
    bytes: Vec<u8>,
    position: usize,
}

impl Input {
    fn from_stdin() -> Self {
        let mut bytes = Vec::new();
        std::io::stdin().read_to_end(&mut bytes).ok();
        Self { bytes, position: 0 }
    }

    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if self.position >= self.bytes.len() {
            return None;
        }

        let start = self.position;
        let limit = start.saturating_add(size.saturating_sub(1));
        while self.position < self.bytes.len() && self.position < limit {
            let c = self.bytes[self.position];
            self.position += 1;
            if c == b'\n' {
                break;
            }
        }
        Some(self.bytes[start..self.position].to_vec())
    }
}

fn c_string(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
    &bytes[..end]
}

fn append_c_string(destination: &mut Vec<u8>, source: &[u8], capacity: usize) {
    let available = capacity.saturating_sub(destination.len() + 1);
    let source = c_string(source);
    destination.extend_from_slice(&source[..source.len().min(available)]);
}

fn print_menu(stdout: &mut Vec<u8>) {
    stdout.extend_from_slice(b"\n=== Text Analyzer ===\n");
    stdout.extend_from_slice(b"1. Analyze text\n");
    stdout.extend_from_slice(b"2. Load text from file\n");
    stdout.extend_from_slice(b"3. Show token distribution\n");
    stdout.extend_from_slice(b"4. Calculate complexity score\n");
    stdout.extend_from_slice(b"5. Find pattern\n");
    stdout.extend_from_slice(b"6. Interactive tokenizer\n");
    stdout.extend_from_slice(b"7. Exit\n");
    stdout.extend_from_slice(b"Choice: ");
}

fn print_analysis_result(result: AnalysisResult, stdout: &mut Vec<u8>) {
    stdout.extend_from_slice(b"\n=== Analysis Results ===\n");
    stdout.extend_from_slice(format!("Words/Identifiers: {}\n", result.word_count).as_bytes());
    stdout.extend_from_slice(format!("Numbers: {}\n", result.number_count).as_bytes());
    stdout.extend_from_slice(format!("Keywords: {}\n", result.keyword_count).as_bytes());
    stdout.extend_from_slice(format!("Operators: {}\n", result.operator_count).as_bytes());
    stdout.extend_from_slice(format!("Comments: {}\n", result.comment_count).as_bytes());
    stdout.extend_from_slice(format!("Strings: {}\n", result.string_count).as_bytes());
    stdout.extend_from_slice(format!("Lines: {}\n", result.line_count).as_bytes());
    stdout.extend_from_slice(format!("Characters: {}\n", result.char_count).as_bytes());
}

fn interactive_tokenizer(
    analyzer: &mut Analyzer,
    input: &mut Input,
    stdout: &mut Vec<u8>,
    stderr: &mut Vec<u8>,
) {
    stdout.extend_from_slice(b"\nEnter text (empty line to stop):\n");

    let mut text = Vec::new();
    while let Some(line) = input.fgets(256) {
        if line.first() == Some(&b'\n') {
            break;
        }
        append_c_string(&mut text, &line, MAX_INPUT_SIZE);
    }

    if !analyzer.tokenizer_mut().load_text(&text, stderr) {
        stdout.extend_from_slice(b"Failed to load text\n");
        return;
    }

    stdout.extend_from_slice(b"\n=== Tokens ===\n");
    const TOKEN_TYPE_NAMES: &[&[u8]] = &[
        b"EOF",
        b"WORD",
        b"NUMBER",
        b"PUNCT",
        b"SPACE",
        b"NEWLINE",
        b"IDENT",
        b"KEYWORD",
        b"OPERATOR",
        b"STRING",
        b"COMMENT",
        b"ERROR",
    ];

    let mut count = 0;
    loop {
        let token = analyzer.tokenizer_mut().next_token();
        if token.token_type == TokenType::Eof {
            break;
        }

        stdout.push(b'[');
        stdout.extend_from_slice(TOKEN_TYPE_NAMES[token.token_type as usize]);
        stdout.extend_from_slice(b"] '");
        stdout.extend_from_slice(&token.value);
        stdout.extend_from_slice(format!("' (L{}:C{})\n", token.line, token.column).as_bytes());
        count += 1;

        if count > 100 {
            stdout.extend_from_slice(b"... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file(filename: &[u8], stderr: &mut Vec<u8>) -> Option<Vec<u8>> {
    let path = Path::new(std::ffi::OsStr::from_bytes(filename));
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            stderr.extend_from_slice(b"Error: Could not open file '");
            stderr.extend_from_slice(filename);
            stderr.extend_from_slice(b"'\n");
            return None;
        }
    };

    let size = file.seek(SeekFrom::End(0)).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as u64 {
        stderr.extend_from_slice(b"Error: File too large\n");
        return None;
    }

    let mut content = Vec::with_capacity(size as usize);
    let mut limited = file.take(size);
    loop {
        let mut chunk = [0u8; 4096];
        match limited.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => content.extend_from_slice(&chunk[..read]),
            Err(_) => break,
        }
    }
    Some(content)
}

fn parse_scanf_int(input: &[u8]) -> Option<i32> {
    let input = c_string(input);
    let mut position = 0;
    while position < input.len() && is_space(input[position]) {
        position += 1;
    }

    let negative = match input.get(position) {
        Some(b'-') => {
            position += 1;
            true
        }
        Some(b'+') => {
            position += 1;
            false
        }
        _ => false,
    };

    let start = position;
    let mut value = 0u64;
    while position < input.len() && input[position].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((input[position] - b'0') as u64);
        position += 1;
    }

    if position == start {
        return None;
    }

    let signed = if negative {
        (value as i64).wrapping_neg()
    } else {
        value as i64
    };
    Some(signed as i32)
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn main() {
    let mut input = Input::from_stdin();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut analyzer = Analyzer::new();

    stdout.extend_from_slice(b"Text Analysis and Tokenization System\n");
    stdout.extend_from_slice(b"This system demonstrates function pointers and static globals\n");

    loop {
        print_menu(&mut stdout);

        let Some(choice_line) = input.fgets(256) else {
            break;
        };
        let Some(choice) = parse_scanf_int(&choice_line) else {
            stdout.extend_from_slice(b"Invalid input\n");
            continue;
        };

        match choice {
            1 => {
                stdout.extend_from_slice(b"Enter text to analyze (empty line to stop):\n");
                let mut text = Vec::new();
                while let Some(line) = input.fgets(256) {
                    if line.first() == Some(&b'\n') {
                        break;
                    }
                    append_c_string(&mut text, &line, MAX_INPUT_SIZE);
                }

                let result = analyzer.analyze_text(&text, &mut stderr);
                print_analysis_result(result, &mut stdout);
            }
            2 => {
                stdout.extend_from_slice(b"Enter filename: ");
                if let Some(filename_line) = input.fgets(256) {
                    let filename = c_string(&filename_line);
                    let newline = filename
                        .iter()
                        .position(|&c| c == b'\n')
                        .unwrap_or(filename.len());
                    let filename = &filename[..newline];

                    if let Some(content) = read_file(filename, &mut stderr) {
                        let result = analyzer.analyze_text(&content, &mut stderr);
                        print_analysis_result(result, &mut stdout);
                    }
                }
            }
            3 => analyzer.print_token_distribution(&mut stdout),
            4 => {
                let score = analyzer.calculate_complexity_score();
                stdout.extend_from_slice(format!("\nComplexity Score: {}\n", score).as_bytes());
                if score < 10 {
                    stdout.extend_from_slice(b"Complexity: Low\n");
                } else if score < 50 {
                    stdout.extend_from_slice(b"Complexity: Medium\n");
                } else {
                    stdout.extend_from_slice(b"Complexity: High\n");
                }
            }
            5 => {
                stdout.extend_from_slice(b"Enter pattern to search: ");
                if let Some(pattern_line) = input.fgets(256) {
                    let pattern = c_string(&pattern_line);
                    let newline = pattern
                        .iter()
                        .position(|&c| c == b'\n')
                        .unwrap_or(pattern.len());
                    analyzer.find_patterns(&pattern[..newline], &mut stdout);
                }
            }
            6 => interactive_tokenizer(&mut analyzer, &mut input, &mut stdout, &mut stderr),
            7 => {
                stdout.extend_from_slice(b"Goodbye!\n");
                break;
            }
            _ => stdout.extend_from_slice(b"Invalid choice\n"),
        }
    }

    let _ = std::io::stdout().write_all(&stdout);
    let _ = std::io::stderr().write_all(&stderr);
}
