mod analyzer;
mod tokenizer;

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;

use analyzer::{AnalysisResult, Analyzer};
use tokenizer::{TokenType, MAX_BUFFER_SIZE};

const MAX_INPUT_SIZE: usize = 4096;

struct CInput<R> {
    reader: R,
}

impl<R: BufRead> CInput<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn fgets(&mut self, capacity: usize) -> io::Result<Option<Vec<u8>>> {
        let mut result = Vec::with_capacity(capacity.saturating_sub(1));

        while result.len() < capacity.saturating_sub(1) {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return if result.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(result))
                };
            }

            let room = capacity - 1 - result.len();
            let considered = available.len().min(room);
            let take = match available[..considered]
                .iter()
                .position(|byte| *byte == b'\n')
            {
                Some(index) => index + 1,
                None => considered,
            };
            let found_newline = available[take - 1] == b'\n';
            result.extend_from_slice(&available[..take]);
            self.reader.consume(take);

            if found_newline {
                break;
            }
        }

        Ok(Some(result))
    }
}

fn c_string(bytes: &[u8]) -> &[u8] {
    let length = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    &bytes[..length]
}

fn parse_decimal_int(bytes: &[u8]) -> Option<i32> {
    let bytes = c_string(bytes);
    let mut index = 0;
    while index < bytes.len()
        && matches!(bytes[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = if bytes.get(index) == Some(&b'-') {
        index += 1;
        true
    } else {
        if bytes.get(index) == Some(&b'+') {
            index += 1;
        }
        false
    };

    let start = index;
    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut magnitude = 0u64;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|value| value.checked_add((bytes[index] - b'0') as u64))
            .unwrap_or(u64::MAX)
            .min(limit);
        index += 1;
    }

    if index == start {
        None
    } else {
        let value = if negative {
            if magnitude == (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }
}

fn print_menu<W: Write>(out: &mut W) -> io::Result<()> {
    write!(
        out,
        "\n=== Text Analyzer ===\n\
         1. Analyze text\n\
         2. Load text from file\n\
         3. Show token distribution\n\
         4. Calculate complexity score\n\
         5. Find pattern\n\
         6. Interactive tokenizer\n\
         7. Exit\n\
         Choice: "
    )
}

fn print_analysis_result<W: Write>(result: &AnalysisResult, out: &mut W) -> io::Result<()> {
    writeln!(out, "\n=== Analysis Results ===")?;
    writeln!(out, "Words/Identifiers: {}", result.word_count)?;
    writeln!(out, "Numbers: {}", result.number_count)?;
    writeln!(out, "Keywords: {}", result.keyword_count)?;
    writeln!(out, "Operators: {}", result.operator_count)?;
    writeln!(out, "Comments: {}", result.comment_count)?;
    writeln!(out, "Strings: {}", result.string_count)?;
    writeln!(out, "Lines: {}", result.line_count)?;
    writeln!(out, "Characters: {}", result.char_count)
}

fn collect_text<R: BufRead>(input: &mut CInput<R>) -> io::Result<Vec<u8>> {
    let mut text = Vec::with_capacity(MAX_INPUT_SIZE);

    while let Some(line) = input.fgets(256)? {
        if line.first() == Some(&b'\n') {
            break;
        }

        let source = c_string(&line);
        let room = MAX_INPUT_SIZE - 1 - text.len();
        text.extend_from_slice(&source[..source.len().min(room)]);
    }

    Ok(text)
}

fn interactive_tokenizer<R: BufRead, W: Write>(
    analyzer: &mut Analyzer,
    input: &mut CInput<R>,
    out: &mut W,
) -> io::Result<()> {
    writeln!(out, "\nEnter text (empty line to stop):")?;
    out.flush()?;
    let text = collect_text(input)?;

    if analyzer.tokenizer_mut().load_text(&text) != 0 {
        writeln!(out, "Failed to load text")?;
        return Ok(());
    }

    writeln!(out, "\n=== Tokens ===")?;
    const TOKEN_TYPE_NAMES: [&str; 12] = [
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE", "NEWLINE", "IDENT", "KEYWORD", "OPERATOR",
        "STRING", "COMMENT", "ERROR",
    ];

    let mut count = 0i32;
    loop {
        let token = analyzer.tokenizer_mut().next_token();
        if token.token_type == TokenType::Eof {
            break;
        }

        write!(out, "[{}] '", TOKEN_TYPE_NAMES[token.token_type as usize])?;
        out.write_all(&token.value)?;
        writeln!(out, "' (L{}:C{})", token.line, token.column)?;
        count = count.wrapping_add(1);

        if count > 100 {
            writeln!(out, "... (truncated, too many tokens)")?;
            break;
        }
    }

    Ok(())
}

fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    let path = OsStr::from_bytes(filename);
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            let mut err = io::stderr().lock();
            let _ = write!(err, "Error: Could not open file '");
            let _ = err.write_all(filename);
            let _ = writeln!(err, "'");
            return None;
        }
    };

    let size = match file.seek(SeekFrom::End(0)) {
        Ok(size) => size,
        Err(_) => return None,
    };
    let _ = file.seek(SeekFrom::Start(0));

    if size > MAX_BUFFER_SIZE as u64 {
        let _ = writeln!(io::stderr().lock(), "Error: File too large");
        return None;
    }

    let mut content = Vec::with_capacity(size as usize);
    let mut limited = file.take(size);
    let _ = limited.read_to_end(&mut content);
    Some(content)
}

fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = CInput::new(stdin.lock());
    let mut out = stdout.lock();
    let mut analyzer = Analyzer::new();
    analyzer.init();

    writeln!(out, "Text Analysis and Tokenization System")?;
    writeln!(
        out,
        "This system demonstrates function pointers and static globals"
    )?;

    loop {
        print_menu(&mut out)?;
        out.flush()?;

        let line = match input.fgets(256)? {
            Some(line) => line,
            None => break,
        };

        let choice = match parse_decimal_int(&line) {
            Some(choice) => choice,
            None => {
                writeln!(out, "Invalid input")?;
                continue;
            }
        };

        match choice {
            1 => {
                writeln!(out, "Enter text to analyze (empty line to stop):")?;
                out.flush()?;
                let text = collect_text(&mut input)?;
                let result = analyzer.analyze_text(&text);
                print_analysis_result(&result, &mut out)?;
            }
            2 => {
                write!(out, "Enter filename: ")?;
                out.flush()?;
                let Some(line) = input.fgets(256)? else {
                    continue;
                };
                let filename = c_string(&line);
                let filename = match filename.iter().position(|byte| *byte == b'\n') {
                    Some(index) => &filename[..index],
                    None => filename,
                };

                if let Some(content) = read_file(filename) {
                    let result = analyzer.analyze_text(&content);
                    print_analysis_result(&result, &mut out)?;
                }
            }
            3 => {
                analyzer.print_token_distribution(&mut out)?;
            }
            4 => {
                let score = analyzer.calculate_complexity_score();
                writeln!(out, "\nComplexity Score: {}", score)?;
                if score < 10 {
                    writeln!(out, "Complexity: Low")?;
                } else if score < 50 {
                    writeln!(out, "Complexity: Medium")?;
                } else {
                    writeln!(out, "Complexity: High")?;
                }
            }
            5 => {
                write!(out, "Enter pattern to search: ")?;
                out.flush()?;
                let Some(line) = input.fgets(256)? else {
                    continue;
                };
                let pattern = c_string(&line);
                let pattern = match pattern.iter().position(|byte| *byte == b'\n') {
                    Some(index) => &pattern[..index],
                    None => pattern,
                };
                analyzer.find_patterns(pattern, &mut out)?;
            }
            6 => {
                interactive_tokenizer(&mut analyzer, &mut input, &mut out)?;
            }
            7 => {
                writeln!(out, "Goodbye!")?;
                return Ok(());
            }
            _ => {
                writeln!(out, "Invalid choice")?;
            }
        }
    }

    Ok(())
}

fn main() {
    let _ = run();
}
