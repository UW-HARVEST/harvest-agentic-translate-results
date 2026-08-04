use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::io::{Read, Seek, SeekFrom};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    // Open the file. If it cannot be opened, mimic the C behavior:
    // print an error and exit with code 1.
    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    // Compute the (upper-bound) program length to mirror the C code; the
    // C version pre-allocates `numCodes` bytes. We use a growable Vec but
    // still call prog_len for parity (and to leave the file at offset 0).
    let _num_codes = prog_len(&file);

    let mut byte_code: Vec<slothvm::UByte> = Vec::new();

    loop {
        let line = match readline(&file) {
            Some(s) => s,
            None => break,
        };

        let bytes = line.as_bytes();
        let len = bytes.len();
        if len == 0 {
            continue;
        }

        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

        // Walk the line until we hit '#' (comment) or run out of bytes.
        while count < len {
            let c = bytes[count];
            if c == 0 || c == b'#' {
                break;
            }

            let rest = &bytes[count..];

            if rest.len() >= 6 && &rest[..6] == b"slothy" {
                byte_code.push(0x1);
                count += 6;
                had_token = true;
            } else if rest.len() >= 5 && &rest[..5] == b"sloth" {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if rest.len() >= 3 && &rest[..3] == b"and" {
                byte_code.push(current_code);
                current_code = 0;
                count += 3;
                had_token = true;
            } else if rest.len() >= 3 && &rest[..3] == b"nap" {
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
    // Rust drops the program automatically when it goes out of scope.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    // Read one byte at a time until newline or EOF.
    // Returns None when EOF is reached with no characters read.
    // Otherwise returns the line content (without the trailing newline).
    let mut file_ref = file;
    let mut bytes: Vec<u8> = Vec::with_capacity(100);
    let mut buf = [0u8; 1];
    let mut got_any = false;
    loop {
        match file_ref.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                got_any = true;
                if buf[0] == b'\n' {
                    break;
                }
                bytes.push(buf[0]);
            }
            Err(_) => break,
        }
    }
    if !got_any {
        return None;
    }
    // Convert bytes to a String. Sloth source files are ASCII; any
    // non-UTF-8 bytes get replaced (the parser only matches ASCII keywords).
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut file_ref = file;
    let _ = file_ref.seek(SeekFrom::Start(0));
    let mut count: usize = 0;
    let mut buf = [0u8; 1];
    loop {
        match file_ref.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                if buf[0] == b'\n' {
                    count += 1;
                }
            }
            Err(_) => break,
        }
    }
    let _ = file_ref.seek(SeekFrom::Start(0));
    count * 3
}
