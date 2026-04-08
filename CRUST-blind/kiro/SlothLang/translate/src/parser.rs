use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::io::{Read, Seek, SeekFrom, BufReader, BufRead};
use std::fs::File;

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let file = File::open(filename).unwrap_or_else(|_| {
        eprintln!("[ERROR] File could not be opened.");
        std::process::exit(1);
    });

    let num_codes = prog_len(&file);
    let mut byte_code: Vec<u8> = vec![0u8; num_codes];
    let mut code_num: usize = 0;

    loop {
        let line = match readline(&file) {
            None => break,
            Some(l) => l,
        };
        if line.is_empty() {
            continue;
        }

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

        while count <= len {
            if count >= len {
                break;
            }
            let c = bytes[count];
            if c == b'#' {
                break;
            }

            if count + 6 <= len && &bytes[count..count+6] == b"slothy" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x1;
                } else {
                    byte_code.push(0x1);
                }
                code_num += 1;
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &bytes[count..count+5] == b"sloth" {
                current_code += 1;
                count += 5;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count+3] == b"and" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = current_code;
                } else {
                    byte_code.push(current_code);
                }
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count+3] == b"nap" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x0;
                } else {
                    byte_code.push(0x0);
                }
                code_num += 1;
                count += 3;
                had_token = true;
            } else {
                count += 1;
            }
        }

        if had_token {
            if code_num < byte_code.len() {
                byte_code[code_num] = current_code;
            } else {
                byte_code.push(current_code);
            }
            code_num += 1;
        }
    }

    byte_code.truncate(code_num);

    Some(SlothProgram { codes: byte_code, pc: 0 })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // In Rust, dropping handles deallocation
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    let mut buf = [0u8; 1];
    let mut line = String::new();
    loop {
        let n = (&*file).read(&mut buf).unwrap_or(0);
        if n == 0 {
            // EOF
            if line.is_empty() {
                return None;
            }
            return Some(line);
        }
        if buf[0] == b'\n' {
            return Some(line);
        }
        line.push(buf[0] as char);
    }
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut count = 0usize;
    let mut buf = [0u8; 4096];
    loop {
        let n = (&*file).read(&mut buf).unwrap_or(0);
        if n == 0 { break; }
        for i in 0..n {
            if buf[i] == b'\n' {
                count += 1;
            }
        }
    }
    (&*file).seek(SeekFrom::Start(0)).ok();
    count * 3
}
