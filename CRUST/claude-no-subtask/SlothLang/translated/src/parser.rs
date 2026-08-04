use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;

#[allow(dead_code)]
fn _unused() {
    let _ = throw::math_err;
    let _ = slothvm::execute;
}

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let contents = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    // Compute prog_len: number of '\n' lines * 3
    let num_lines = contents.bytes().filter(|&b| b == b'\n').count();
    let num_codes = num_lines * 3;

    let mut byte_code: Vec<u8> = vec![0u8; num_codes];
    let mut code_num: usize = 0;

    // Iterate over lines split by '\n'. The C code reads via readline which returns
    // each line up to a '\n' (excluding the '\n'). When EOF is reached the loop breaks.
    // We process each '\n'-terminated line. Since prog_len counts lines by '\n', and
    // contents may not end in '\n', we mirror C behavior: only count complete lines.
    //
    // Use byte-level iteration. We need to split by '\n' exactly, and only process
    // lines that end with '\n' (consistent with C reading until '\n' or EOF).

    let bytes = contents.as_bytes();
    let mut start = 0usize;
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'\n' {
            // Process line bytes[start..idx]
            process_line(&bytes[start..idx], &mut byte_code, &mut code_num);
            start = idx + 1;
        }
        idx += 1;
    }
    // Per C readline+parse: when EOF is encountered the loop breaks before processing
    // the last (unterminated) line, so we don't process it.

    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

fn process_line(line: &[u8], byte_code: &mut Vec<u8>, code_num: &mut usize) {
    if line.is_empty() {
        return;
    }
    let len = line.len();
    let mut count: usize = 0;
    let mut current_code: u8 = 0;
    let mut had_token: bool = false;

    let mut c = line[0];
    while c != 0 && c != b'#' && count <= len {
        if count >= len {
            break;
        }
        if starts_with(&line[count..], b"slothy") {
            byte_code[*code_num] = 0x1;
            *code_num += 1;
            count += 6;
            had_token = true;
        } else if starts_with(&line[count..], b"sloth") {
            current_code = current_code.wrapping_add(1);
            count += 5;
            had_token = true;
        } else if starts_with(&line[count..], b"and") {
            byte_code[*code_num] = current_code;
            *code_num += 1;
            current_code = 0;
            count += 3;
            had_token = true;
        } else if starts_with(&line[count..], b"nap") {
            byte_code[*code_num] = 0x0;
            *code_num += 1;
            count += 3;
            had_token = true;
        } else {
            count += 1;
        }

        if count < len {
            c = line[count];
        } else {
            c = 0;
        }
    }

    if had_token {
        byte_code[*code_num] = current_code;
        *code_num += 1;
    }
}

fn starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && &haystack[..needle.len()] == needle
}

pub fn free_program(_program: Option<SlothProgram>) {
    // Dropped automatically when this function returns.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    use std::io::Read;
    let mut f = file;
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match f.read(&mut byte) {
            Ok(0) => {
                // EOF: mirror C behavior — append -1 (0xFF) to indicate EOF
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
    let mut buf = [0u8; 4096];
    let mut count = 0usize;
    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for i in 0..n {
                    if buf[i] == b'\n' {
                        count += 1;
                    }
                }
            }
            Err(_) => break,
        }
    }
    count * 3
}
