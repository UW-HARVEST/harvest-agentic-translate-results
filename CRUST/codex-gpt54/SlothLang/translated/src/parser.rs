use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
pub fn parse(filename: &str) -> Option<SlothProgram> {
    let bytes = match std::fs::read(filename) {
        Ok(bytes) => bytes,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    let mut byte_code = Vec::new();
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }

        let mut count = 0usize;
        let len = line.len();
        let mut current_code = 0u8;
        let mut had_token = false;

        while count < len {
            let c = line[count];
            if c == 0 || c == 0xFF || c == b'#' {
                break;
            }

            if line[count..].starts_with(b"slothy") {
                byte_code.push(0x01);
                count += 6;
                had_token = true;
            } else if line[count..].starts_with(b"sloth") {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if line[count..].starts_with(b"and") {
                byte_code.push(current_code);
                current_code = 0;
                count += 3;
                had_token = true;
            } else if line[count..].starts_with(b"nap") {
                byte_code.push(0x00);
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

    Some(SlothProgram { codes: byte_code, pc: 0 })
}
pub fn free_program(_program: Option<SlothProgram>) {
    drop(_program);
}
pub fn readline(file: &std::fs::File) -> Option<String> {
    use std::io::Read;

    let mut handle = file;
    let mut line = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        match handle.read(&mut byte) {
            Ok(0) => {
                line.push(0xFF);
                break;
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                line.push(byte[0]);
            }
            Err(_) => return None,
        }
    }

    Some(line.into_iter().map(char::from).collect())
}
pub fn prog_len(file: &std::fs::File) -> usize {
    use std::io::{Read, Seek, SeekFrom};

    let mut handle = file;
    let mut count = 0usize;
    let mut buf = [0u8; 4096];

    loop {
        match handle.read(&mut buf) {
            Ok(0) => break,
            Ok(read) => {
                count += buf[..read].iter().filter(|byte| **byte == b'\n').count();
            }
            Err(_) => return 0,
        }
    }

    let _ = handle.seek(SeekFrom::Start(0));
    count * 3
}
