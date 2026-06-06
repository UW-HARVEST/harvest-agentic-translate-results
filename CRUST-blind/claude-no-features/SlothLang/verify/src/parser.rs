use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::io::{Read, Seek, SeekFrom};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    // Read full file contents to preserve "newline" semantics like the C code.
    let mut content = String::new();
    let mut f = file;
    f.read_to_string(&mut content).ok()?;

    // Calculate prog_len equivalent: number of '\n' * 3
    let num_newlines = content.matches('\n').count();
    let num_codes = num_newlines * 3;
    let mut byte_code: Vec<u8> = vec![0u8; num_codes];

    let mut code_num: usize = 0;

    // Process by lines (mimic readline behavior).
    // The C readline reads up to a '\n' (consumed) and returns the buffer.
    // It also detects EOF and pushes -1 marker. We'll iterate lines directly.
    //
    // The C code: if line[0] == -1, break (i.e. EOF reached at start of a line)
    // if len == 0, continue (empty line — but in C empty line still has len > 0 if just '\n' was consumed?
    // Actually in C readline returns "" for an empty line "\n" (count was 0, c was '\n', loop breaks before writing).
    // So len == 0 corresponds to a blank line — they "continue".
    //
    // We want to replicate: for each line until EOF, scan tokens.

    let mut iter = content.split('\n').peekable();
    // The C code sees EOF only after the last line. If file ends with '\n', the last
    // split element will be "" and we'd skip it. If file does NOT end with '\n', the
    // last element is partial line content and we should still process tokens within.
    // In the C code, when readline encounters EOF mid-line, it writes -1 and the
    // outer loop processes that line, then on next iteration breaks (because line[0]
    // == -1 after EOF-only). We'll replicate by processing all lines, but skipping
    // empties.

    while let Some(line) = iter.next() {
        // Determine if this is the trailing "" produced by a final '\n'.
        // If line is empty, we treat it like the C code's "empty line — continue"
        if line.is_empty() {
            continue;
        }

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

        // The C loop condition is: c != 0 && c != -1 && c != '#' && count <= len
        // c starts as line[0]; loop increments count. We translate accordingly.
        loop {
            if count > len {
                break;
            }
            // Determine current character (or pseudo-NUL at end).
            let c: u8 = if count < len { bytes[count] } else { 0 };
            if c == 0 || c == b'#' {
                break;
            }

            // Check keywords by remaining slice
            let rest = &bytes[count..];
            if rest.starts_with(b"slothy") {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x1;
                } else {
                    byte_code.push(0x1);
                }
                code_num += 1;
                count += 6;
                had_token = true;
            } else if rest.starts_with(b"sloth") {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if rest.starts_with(b"and") {
                if code_num < byte_code.len() {
                    byte_code[code_num] = current_code;
                } else {
                    byte_code.push(current_code);
                }
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if rest.starts_with(b"nap") {
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

    // Note: we keep byte_code at its allocated num_codes length (which may include
    // trailing zeros that act as EXIT opcodes). The C code does the same thing.
    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // Rust automatically drops the program when it goes out of scope.
    // No-op.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    // Read characters one at a time until '\n' or EOF, mimicking the C readline.
    let mut f = file;
    let mut buf = Vec::with_capacity(100);
    let mut byte = [0u8; 1];
    loop {
        match f.read(&mut byte) {
            Ok(0) => {
                // EOF: append -1 marker as in C (0xFF as i8 == -1)
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
    // Convert to lossy String (preserving the 0xFF byte if present).
    Some(String::from_utf8_lossy(&buf).into_owned())
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut f = file;
    let mut count: usize = 0;
    let mut byte = [0u8; 1];
    loop {
        match f.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == b'\n' {
                    count += 1;
                }
            }
            Err(_) => break,
        }
    }
    let _ = f.seek(SeekFrom::Start(0));
    count * 3
}
