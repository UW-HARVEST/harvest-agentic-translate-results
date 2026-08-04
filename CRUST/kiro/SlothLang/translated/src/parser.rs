use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::io::{BufRead, BufReader};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let file = std::fs::File::open(filename).unwrap_or_else(|_| {
        eprintln!("[ERROR] File could not be opened.");
        std::process::exit(1);
    });
    let reader = BufReader::new(&file);
    let num_codes = prog_len(&std::fs::File::open(filename).unwrap());
    let mut byte_code: Vec<u8> = vec![0; num_codes];
    let mut code_num: usize = 0;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() { continue; }

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token = false;

        while count <= len {
            if count >= len { break; }
            let c = bytes[count];
            if c == b'#' { break; }
            if count + 6 <= len && &bytes[count..count+6] == b"slothy" {
                if code_num < byte_code.len() { byte_code[code_num] = 0x01; }
                code_num += 1;
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &bytes[count..count+5] == b"sloth" {
                current_code += 1;
                count += 5;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count+3] == b"and" {
                if code_num < byte_code.len() { byte_code[code_num] = current_code; }
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count+3] == b"nap" {
                if code_num < byte_code.len() { byte_code[code_num] = 0x00; }
                code_num += 1;
                count += 3;
                had_token = true;
            } else {
                count += 1;
            }
        }

        if had_token {
            if code_num < byte_code.len() { byte_code[code_num] = current_code; }
            code_num += 1;
        }
    }

    byte_code.truncate(code_num);
    Some(SlothProgram { codes: byte_code, pc: 0 })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // Drop happens automatically in Rust
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => {
            if line.ends_with('\n') { line.pop(); }
            Some(line)
        }
        Err(_) => None,
    }
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let reader = BufReader::new(file);
    let count = reader.lines().count();
    count * 3
}
