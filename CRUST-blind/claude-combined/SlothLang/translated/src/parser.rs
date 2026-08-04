use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    // Read entire file into a string for simpler line processing while
    // mirroring the C parser's tokenization rules exactly.
    let contents = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    // Compute progLen the same way C does: 3 * number of '\n' characters.
    let num_codes = contents.bytes().filter(|&b| b == b'\n').count() * 3;
    let mut byte_code: Vec<u8> = vec![0u8; num_codes];

    let mut code_num: usize = 0;

    // Iterate over each line *without* the trailing '\n' (matches C's readline).
    // We also handle a trailing partial line that the C readline would terminate.
    // Note: C readline appends a -1 byte when EOF is hit *during* a line; here
    // we treat any final non-newline-terminated content as already done (since
    // numCodes is computed from newline count, mirrors C buffer sizing).
    let mut line_iter = contents.split('\n').peekable();
    // To mirror C exactly: keep reading lines until readline detects EOF (line[0] == -1).
    // In our split, the last element after the final '\n' is "" (empty). Skip empty
    // lines like C does (continues), but stop when we run out — equivalent of EOF break.
    while let Some(raw_line) = line_iter.next() {
        // The very last element (after the final '\n') represents the EOF case
        // in C: readline puts a -1 and we break. We approximate by stopping when
        // we've consumed all real lines. If line_iter.peek() is None *and* the
        // current line is empty (no trailing chars after final '\n'), we stop.
        if line_iter.peek().is_none() {
            // This is the after-last-newline segment. C readline would have hit
            // EOF and broken out of the loop. We replicate that here.
            // If the file did not end with a newline, raw_line may contain content,
            // but because num_codes is based on newline count, there is no buffer
            // space allocated for it — match C behavior by simply breaking.
            break;
        }

        let line = raw_line.as_bytes();
        let len = line.len();
        if len == 0 {
            continue;
        }

        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

        loop {
            if count > len {
                break;
            }
            let c = if count < len { line[count] } else { 0 };
            if c == 0 || c == b'#' {
                break;
            }

            // Check keywords with strncmp-style matching.
            if line[count..].starts_with(b"slothy") {
                byte_code[code_num] = 0x1;
                code_num += 1;
                count += 6;
                had_token = true;
            } else if line[count..].starts_with(b"sloth") {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if line[count..].starts_with(b"and") {
                byte_code[code_num] = current_code;
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if line[count..].starts_with(b"nap") {
                byte_code[code_num] = 0x0;
                code_num += 1;
                count += 3;
                had_token = true;
            } else {
                count += 1;
            }
        }

        if had_token {
            byte_code[code_num] = current_code;
            code_num += 1;
        }
    }

    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // In Rust, drop occurs automatically when `_program` goes out of scope.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    // Mirror C readline: read until '\n' or EOF, return string without the
    // newline. If EOF hit, append a 0xFF byte (C uses -1) marker. On a fresh
    // end-of-file read with no content, mark accordingly.
    let mut f = file.try_clone().ok()?;
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    let mut eof = false;
    loop {
        match f.read(&mut byte) {
            Ok(0) => {
                eof = true;
                break;
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0]);
            }
            Err(_) => {
                eof = true;
                break;
            }
        }
    }
    if eof {
        // Append the 0xFF (interpreted as -1 in i8) sentinel byte.
        buf.push(0xFFu8);
    }
    // Convert lossily — preserves the sentinel byte for callers.
    Some(String::from_utf8_lossy(&buf).into_owned())
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut f = match file.try_clone() {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return 0;
    }
    let count = buf.iter().filter(|&&b| b == b'\n').count();
    // Reset position to start, like C's fseek(f, 0, SEEK_SET)
    let _ = f.seek(SeekFrom::Start(0));
    count * 3
}

// Tie the slothvm and throw imports to runtime usage so the warnings stay quiet.
#[allow(dead_code)]
fn _touch_imports() {
    let _ = throw::math_err;
    let _: fn(&mut Option<slothvm::SlothProgram>) -> i32 = slothvm::execute;
}
