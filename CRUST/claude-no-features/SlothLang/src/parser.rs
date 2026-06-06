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

    let _num_codes = prog_len(&file);
    let _ = file.seek(SeekFrom::Start(0));

    let mut byte_code: Vec<u8> = Vec::new();
    let mut current_code: u8 = 0;
    let mut had_token: bool;

    loop {
        let line_opt = readline(&file);
        let line = match line_opt {
            Some(l) => l,
            None => break,
        };

        // Convert to bytes for easier indexing matching the C code semantics
        let bytes = line.as_bytes();
        let len = bytes.len();

        // The C code stores -1 (EOF) into the buffer when end-of-file is hit.
        // In our readline, we return None when EOF reached and no more content,
        // but if there was a partial last line we return the content. So we
        // model EOF using None.

        if len == 0 {
            continue;
        }

        had_token = false;
        current_code = 0;
        let mut count: usize = 0;

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
    // Rust handles memory automatically; dropping the value frees it.
}

pub fn readline(mut file: &std::fs::File) -> Option<String> {
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];
    let mut got_any = false;
    loop {
        match file.read(&mut byte) {
            Ok(0) => {
                // EOF
                if !got_any {
                    return None;
                }
                return Some(String::from_utf8_lossy(&buffer).into_owned());
            }
            Ok(_) => {
                got_any = true;
                if byte[0] == b'\n' {
                    return Some(String::from_utf8_lossy(&buffer).into_owned());
                }
                buffer.push(byte[0]);
            }
            Err(_) => {
                if !got_any {
                    return None;
                }
                return Some(String::from_utf8_lossy(&buffer).into_owned());
            }
        }
    }
}

pub fn prog_len(mut file: &std::fs::File) -> usize {
    let _ = file.seek(SeekFrom::Start(0));
    let mut count: usize = 0;
    let mut buf = [0u8; 4096];
    loop {
        match file.read(&mut buf) {
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
    let _ = file.seek(SeekFrom::Start(0));
    count * 3
}
