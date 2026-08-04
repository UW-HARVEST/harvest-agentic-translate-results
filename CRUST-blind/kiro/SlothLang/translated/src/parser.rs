use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::io::{BufRead, BufReader};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let _file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    let contents = std::fs::read_to_string(filename).unwrap();
    let num_codes = contents.lines().count() * 3;
    let mut byte_code: Vec<u8> = vec![0; num_codes];
    let mut code_num: usize = 0;

    for line_str in contents.lines() {
        let line = line_str.as_bytes();
        let len = line.len();
        if len == 0 { continue; }

        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

        while count <= len {
            if count >= len { break; }
            let c = line[count];
            if c == 0 || c == b'#' { break; }

            if count + 6 <= len && &line[count..count+6] == b"slothy" {
                byte_code[code_num] = 0x1;
                code_num += 1;
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &line[count..count+5] == b"sloth" {
                current_code += 1;
                count += 5;
                had_token = true;
            } else if count + 3 <= len && &line[count..count+3] == b"and" {
                byte_code[code_num] = current_code;
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if count + 3 <= len && &line[count..count+3] == b"nap" {
                byte_code[code_num] = 0x0;
                code_num += 1;
                count += 3;
                had_token = true;
            } else {
                count += 1;
            }
        }

        if had_token {
            byte_code[code_num] = current_code;
            code_num += 1;
        }
    }

    byte_code.truncate(code_num);
    Some(SlothProgram { codes: byte_code, pc: 0 })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // Rust handles deallocation automatically via Drop
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => {
            if line.ends_with('\n') {
                line.pop();
            }
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
