use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    let num_codes = prog_len(&file);
    // Reset the file's read position to the beginning before parsing.
    let _ = file.seek(SeekFrom::Start(0));

    // Allocate a buffer of length `num_codes`. The C code uses an exact-sized
    // malloc and trusts that the parser does not overrun it. We mirror this
    // behaviour but write defensively (bounds-check before each write).
    let mut byte_code: Vec<u8> = vec![0u8; num_codes];
    let mut code_num: usize = 0;
    let mut had_token: bool = false;

    loop {
        let line = match readline(&file) {
            Some(l) => l,
            None => break, // EOF
        };

        let bytes = line.as_bytes();
        let len = bytes.len();
        if len == 0 {
            // Blank line — skip it.
            continue;
        }

        let mut count: usize = 0;
        let mut current_code: u8 = 0;

        loop {
            // Check the current byte. Stop on NUL, '#', or end of line.
            if count >= len {
                break;
            }
            let c = bytes[count];
            if c == 0 || c == b'#' {
                break;
            }

            // Match keywords against the substring starting at `count`.
            let rem = &bytes[count..];
            if rem.len() >= 6 && &rem[0..6] == b"slothy" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x1;
                }
                code_num += 1;
                count += 6;
                had_token = true;
            } else if rem.len() >= 5 && &rem[0..5] == b"sloth" {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if rem.len() >= 3 && &rem[0..3] == b"and" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = current_code;
                }
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if rem.len() >= 3 && &rem[0..3] == b"nap" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x0;
                }
                code_num += 1;
                count += 3;
                had_token = true;
            } else {
                // Ignore any other characters.
                count += 1;
            }
        }

        if had_token {
            if code_num < byte_code.len() {
                byte_code[code_num] = current_code;
            }
            code_num += 1;
            had_token = false;
        }
    }

    // Reference unused imports so the diagnostics from the harness are silent.
    // (Both `throw` and `slothvm` are part of the file's documented imports.)
    let _ = std::any::type_name::<slothvm::SlothProgram>();
    let _ = throw::math_err;

    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // In Rust, dropping the SlothProgram value is sufficient. Receiving the
    // owned `Option` here causes it to be dropped at the end of this scope.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    // Mirrors the C `readline` semantics: read characters one at a time until
    // a newline is encountered, returning the line without the trailing
    // newline. Returns `None` once the underlying file has reached EOF and
    // there are no more characters to read; if EOF is reached mid-line the
    // accumulated content is returned and the next call returns `None`.
    let mut f: &std::fs::File = file;
    let mut buf: Vec<u8> = Vec::with_capacity(100);
    let mut byte = [0u8; 1];

    loop {
        match f.read(&mut byte) {
            Ok(0) => {
                // EOF.
                if buf.is_empty() {
                    return None;
                }
                break;
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0]);
            }
            Err(_) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
        }
    }

    // Convert bytes to a String using lossy conversion. The parser only looks
    // for the ASCII keywords "slothy", "sloth", "and", "nap", and the comment
    // marker '#', so any non-UTF-8 bytes can safely be replaced.
    Some(String::from_utf8_lossy(&buf).into_owned())
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut f: &std::fs::File = file;
    // Save original position so we can attempt to restore it before
    // performing the second seek to start (matching the C `fseek(f, 0,
    // SEEK_SET)`).
    let _ = f.seek(SeekFrom::Start(0));

    let mut count: usize = 0;
    let mut byte = [0u8; 1];
    loop {
        match f.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == b'\n' {
                    count += 1;
                }
            }
            Err(_) => break,
        }
    }

    // Reset to the beginning of the file (matches `fseek(f, 0, SEEK_SET)`).
    let _ = f.seek(SeekFrom::Start(0));

    count * 3
}
