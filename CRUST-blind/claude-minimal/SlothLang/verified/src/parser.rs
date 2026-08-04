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

    // Match C behavior: progLen counts newlines and multiplies by 3 to get
    // an upper bound on the number of bytecodes.
    let _num_codes = prog_len(&file);

    // Re-open since prog_len consumed the file (the C version uses fseek to
    // rewind; here we just open a fresh reader).
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    let mut byte_code: Vec<u8> = Vec::new();
    let reader = BufReader::new(file);

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.is_empty() {
            continue;
        }

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

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
    // In Rust, dropping the Option will free the program automatically.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    // Read a single \n-delimited line from `file`. Returns None at EOF.
    let mut f = file;
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        match f.read(&mut byte) {
            Ok(0) => {
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
            Err(_) => return None,
        }
    }

    Some(String::from_utf8_lossy(&buf).into_owned())
}

pub fn prog_len(file: &std::fs::File) -> usize {
    // Count newlines in `file` and return that count multiplied by 3, mirroring
    // the C `progLen` function which provides an upper bound on bytecode size.
    let mut f = file;
    let mut buf = [0u8; 4096];
    let mut count: usize = 0;

    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for &b in &buf[..n] {
                    if b == b'\n' {
                        count += 1;
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Rewind so subsequent reads can use the same handle (matches fseek in C).
    let _ = f.seek(SeekFrom::Start(0));

    count * 3
}
