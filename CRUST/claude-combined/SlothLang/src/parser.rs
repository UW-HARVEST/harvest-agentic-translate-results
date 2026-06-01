use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let contents = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    // Compute capacity equivalent to (number of '\n' lines) * 3.
    let num_lines = contents.bytes().filter(|&b| b == b'\n').count();
    let capacity = num_lines * 3;
    let mut byte_code: Vec<u8> = Vec::with_capacity(capacity);

    for line in contents.split('\n') {
        // The C parser reads up to a newline; skip blank lines.
        if line.is_empty() {
            continue;
        }

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token = false;

        while count < len {
            let c = bytes[count];
            if c == 0 || c == b'#' {
                break;
            }

            if count + 6 <= len && &bytes[count..count + 6] == b"slothy" {
                byte_code.push(0x1);
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &bytes[count..count + 5] == b"sloth" {
                current_code = current_code.wrapping_add(1);
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
        }

        if had_token {
            byte_code.push(current_code);
        }
    }

    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // Rust handles deallocation automatically when the Option drops here.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    // Read characters from the file until a newline or EOF.
    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    let mut byte = [0u8; 1];
    let mut got_anything = false;
    loop {
        match reader.read(&mut byte) {
            Ok(0) => {
                // EOF
                if !got_anything {
                    return None;
                }
                return Some(buf);
            }
            Ok(_) => {
                got_anything = true;
                if byte[0] == b'\n' {
                    return Some(buf);
                }
                buf.push(byte[0] as char);
            }
            Err(_) => return None,
        }
    }
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut reader = BufReader::new(file);
    let mut count = 0usize;
    let mut byte = [0u8; 1];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == b'\n' {
                    count += 1;
                }
            }
            Err(_) => break,
        }
    }
    count * 3
}
