use crate::{slothvm, throw};
use crate::slothvm::SlothProgram;
use std::fs::File;
use std::io::{BufReader, Read};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    // Read the entire file once
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    if reader.read_to_string(&mut contents).is_err() {
        eprint!("[ERROR] Failed to read file.");
        std::process::exit(1);
    }

    // Determine number of codes (number of newlines * 3)
    let num_codes: usize = contents.matches('\n').count() * 3;

    let mut byte_code: Vec<u8> = vec![0u8; num_codes];
    let mut code_num: usize = 0;

    for line in contents.split('\n') {
        // Skip processing if EOF was reached without a newline; here we just
        // process every split segment that came from a newline-delimited line.
        let len = line.len();
        if len == 0 {
            continue;
        }
        let bytes = line.as_bytes();
        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

        // Iterate while c is non-null, not a comment, and we haven't gone past length
        while count <= len {
            let c = if count < len { bytes[count] } else { 0 };
            if c == 0 || c == b'#' {
                break;
            }
            // Check keywords
            if count + 6 <= len && &bytes[count..count + 6] == b"slothy" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x1;
                }
                code_num += 1;
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &bytes[count..count + 5] == b"sloth" {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count + 3] == b"and" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = current_code;
                }
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count + 3] == b"nap" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x0;
                }
                code_num += 1;
                count += 3;
                had_token = true;
            } else {
                // Ignore any other characters
                count += 1;
            }
        }

        if had_token {
            if code_num < byte_code.len() {
                byte_code[code_num] = current_code;
            }
            code_num += 1;
        }
    }

    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // In Rust, dropping the Option drops the program automatically.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    use std::io::Read;
    let mut f = file;
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match f.read(&mut byte) {
            Ok(0) => {
                // EOF - mirror C behavior of placing -1 sentinel
                buf.push(0xFFu8);
                break;
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0]);
            }
            Err(_) => return None,
        }
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

pub fn prog_len(file: &std::fs::File) -> usize {
    use std::io::Read;
    let mut f = file;
    let mut count: usize = 0;
    let mut byte = [0u8; 1];
    while let Ok(n) = f.read(&mut byte) {
        if n == 0 {
            break;
        }
        if byte[0] == b'\n' {
            count += 1;
        }
    }
    count * 3
}
