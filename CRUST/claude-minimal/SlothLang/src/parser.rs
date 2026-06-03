use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    let num_codes = prog_len(&file);
    let mut byte_code: Vec<slothvm::UByte> = Vec::with_capacity(num_codes);

    let mut reader = BufReader::new(file);

    loop {
        let line = match readline_from_reader(&mut reader) {
            Some(l) => l,
            None => break,
        };

        // EOF detection: if line ends with 0xFF (-1 in signed byte) marker — means EOF reached
        // We use a different convention: returned None means EOF. But the C version returns
        // a buffer with -1 inside if EOF reached. We mirror semantics here.
        let is_eof_line = line.is_empty();

        if line.is_empty() && is_eof_line {
            // We need to differentiate truly empty (continue) vs EOF.
            // We'll handle EOF outside this. See below.
        }

        // Parse the line
        let bytes = line.as_bytes();
        if bytes.is_empty() {
            continue;
        }

        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token = false;
        let len = bytes.len();
        let mut c = bytes[0];

        while c != 0 && c != b'#' && count <= len {
            // Check keywords
            if count + 6 <= len && &bytes[count..count + 6] == b"slothy" {
                byte_code.push(0x1);
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &bytes[count..count + 5] == b"sloth" {
                current_code += 1;
                count += 5;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count + 3] == b"and" {
                byte_code.push(current_code);
                current_code = 0;
                count += 3;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count + 3] == b"nap" {
                byte_code.push(0x0);
                count += 3;
                had_token = true;
            } else {
                count += 1;
            }

            if count >= len {
                break;
            }
            c = bytes[count];
        }

        if had_token {
            byte_code.push(current_code);
        }
    }

    let _ = num_codes; // Pre-allocated capacity hint; real size may differ.

    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // Rust drops the program automatically when it goes out of scope.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    let mut reader = BufReader::new(file);
    readline_from_reader(&mut reader)
}

fn readline_from_reader<R: BufRead>(reader: &mut R) -> Option<String> {
    let mut buf = String::new();
    match reader.read_line(&mut buf) {
        Ok(0) => None, // EOF
        Ok(_) => {
            // Strip trailing newline
            if buf.ends_with('\n') {
                buf.pop();
                if buf.ends_with('\r') {
                    buf.pop();
                }
            }
            Some(buf)
        }
        Err(_) => None,
    }
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut f = file;
    let mut count: usize = 0;
    let mut buf = [0u8; 4096];
    let _ = f.seek(SeekFrom::Start(0));
    loop {
        let n = match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        for &b in &buf[..n] {
            if b == b'\n' {
                count += 1;
            }
        }
    }
    let _ = f.seek(SeekFrom::Start(0));
    count * 3
}
