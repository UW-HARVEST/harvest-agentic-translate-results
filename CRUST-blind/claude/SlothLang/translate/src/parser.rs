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

    let num_codes = prog_len(&file);
    // Read the whole file into a String for parsing.
    let mut file = file;
    let _ = file.seek(SeekFrom::Start(0));
    let mut contents = String::new();
    if file.read_to_string(&mut contents).is_err() {
        eprint!("[ERROR] File could not be opened.");
        std::process::exit(1);
    }

    let mut byte_code: Vec<u8> = vec![0u8; num_codes];
    let mut code_num: usize = 0;

    // Process line by line, mimicking the C readline behavior.
    // Each readline returns a line (without trailing '\n'), and an EOF
    // sentinel triggers the loop's exit. We replicate that:
    let mut iter = contents.split('\n').peekable();
    let total_lines = contents.matches('\n').count();
    let mut lines_consumed = 0;

    loop {
        // Determine if we have a line that ended in \n, or we've reached EOF.
        let line_opt = iter.next();
        let line = match line_opt {
            Some(s) => s,
            None => break,
        };

        let is_last = iter.peek().is_none();
        // In the C code: if EOF is hit, readline puts -1 at the end and
        // breaks out of the parsing loop.
        if is_last {
            // The final partial line after the last '\n' (or whole content
            // if there is no '\n') is treated as EOF and simply skipped.
            // However if the line only contains EOF (i.e., len == 0 because
            // the last char was '\n'), the loop exits.
            // In C, when EOF is encountered first, readline gives back line[0] = -1,
            // so parse() breaks immediately. So we just break here.
            // But there is one subtle case: if the file's last character is '\n',
            // then split('\n') gives an extra empty trailing string -- which we
            // also want to treat as EOF.
            break;
        }

        lines_consumed += 1;
        let _ = lines_consumed;
        let _ = total_lines;

        let line_bytes = line.as_bytes();
        let len = line_bytes.len();
        if len == 0 {
            continue;
        }

        let mut count: usize = 0;
        let mut current_code: u8 = 0;
        let mut had_token: bool = false;

        // Loop while c is not 0/EOF/'#' and count <= len.
        loop {
            if count > len {
                break;
            }
            let c = if count < len { line_bytes[count] } else { 0 };
            if c == 0 || c == b'#' {
                break;
            }

            if count + 6 <= len && &line_bytes[count..count + 6] == b"slothy" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x1;
                }
                code_num += 1;
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &line_bytes[count..count + 5] == b"sloth" {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if count + 3 <= len && &line_bytes[count..count + 3] == b"and" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = current_code;
                }
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if count + 3 <= len && &line_bytes[count..count + 3] == b"nap" {
                if code_num < byte_code.len() {
                    byte_code[code_num] = 0x0;
                }
                code_num += 1;
                count += 3;
                had_token = true;
            } else {
                // Ignore any other character.
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
    // In Rust, dropping the Option drops the underlying SlothProgram and its Vec.
    // Nothing to do explicitly.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    use std::io::{BufReader, BufRead};
    // Note: BufReader ideally takes ownership, but we're given a borrowed file.
    // We can clone the file handle for use here.
    let cloned = file.try_clone().ok()?;
    let mut reader = BufReader::new(cloned);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => {
            // EOF: place an EOF sentinel character.
            // Use the highest-bit-set byte as the sentinel since we're returning a String.
            // The C code stores -1 at the last position when EOF is hit. We mimic with
            // a special character. Use char value 0xFF as the EOF marker.
            let mut s = String::new();
            s.push('\u{00FF}');
            Some(s)
        }
        Ok(_) => {
            // Strip trailing newline.
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Some(line)
        }
        Err(_) => None,
    }
}

pub fn prog_len(file: &std::fs::File) -> usize {
    use std::io::{BufReader, BufRead};
    let cloned = match file.try_clone() {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let reader = BufReader::new(cloned);
    let mut count = 0usize;
    // Count newline characters in the file.
    for byte_res in reader.bytes() {
        match byte_res {
            Ok(b) => {
                if b == b'\n' {
                    count += 1;
                }
            }
            Err(_) => break,
        }
    }
    count * 3
}

// Suppress unused import warnings.
#[allow(dead_code)]
fn _use_imports() {
    let _ = throw::math_err;
    let _ = slothvm::execute;
}
