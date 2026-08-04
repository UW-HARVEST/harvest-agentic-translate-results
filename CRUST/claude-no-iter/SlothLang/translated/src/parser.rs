#[allow(unused_imports)]
use crate::{throw, slothvm};
use crate::slothvm::SlothProgram;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let path = std::path::Path::new(filename);
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            std::process::exit(1);
        }
    };

    let num_codes = prog_len(&file);
    let mut byte_code: Vec<u8> = vec![0u8; num_codes];

    let mut code_num: usize = 0;
    let mut current_code: u8;
    let mut had_token: bool = false;

    loop {
        current_code = 0;
        let line = match readline(&file) {
            Some(l) => l,
            None => break,
        };

        // The C readline puts -1 (i.e. 0xFF as a signed -1) at the end on EOF.
        // We convert to bytes for indexing.
        let bytes = line.as_bytes();

        // Check the EOF marker - first byte is the EOF sentinel only for
        // an empty trailing line.
        // The Rust readline returns None for EOF without any data, and Some(line)
        // for everything else (including the partial last line).
        if bytes.is_empty() {
            continue;
        }

        let len = bytes.len();
        let mut count: usize = 0;

        while count < len {
            let c = bytes[count];
            if c == 0 || c == b'#' {
                break;
            }

            if count + 6 <= len && &bytes[count..count + 6] == b"slothy" {
                byte_code[code_num] = 0x1;
                code_num += 1;
                count += 6;
                had_token = true;
            } else if count + 5 <= len && &bytes[count..count + 5] == b"sloth" {
                current_code += 1;
                count += 5;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count + 3] == b"and" {
                byte_code[code_num] = current_code;
                code_num += 1;
                current_code = 0;
                count += 3;
                had_token = true;
            } else if count + 3 <= len && &bytes[count..count + 3] == b"nap" {
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
            had_token = false;
        }
    }

    Some(SlothProgram {
        codes: byte_code,
        pc: 0,
    })
}

pub fn free_program(_program: Option<SlothProgram>) {
    // In Rust, dropping the Option will free the contained program.
    // This function exists to mirror the C interface; the program will be
    // freed when this function returns.
}

pub fn readline(file: &std::fs::File) -> Option<String> {
    // Read one line (delimited by '\n') from the file using a shared cursor
    // tracked via the file's seek position. Returns None if there is no more
    // data and we reached EOF before reading any character.
    //
    // The C version returns a buffer containing 0xFF (-1) at the end when it
    // hits EOF. We replicate that by appending a 0xFF byte to the returned
    // buffer when EOF is encountered.
    //
    // Note: Rust's File doesn't natively expose a single-byte read with shared
    // cursor like C's getc, but reading 1 byte at a time using `read` works.
    let mut f = file;
    let mut buf = Vec::with_capacity(100);
    let mut got_any = false;
    loop {
        let mut b = [0u8; 1];
        let n = match f.read(&mut b) {
            Ok(n) => n,
            Err(_) => 0,
        };
        if n == 0 {
            // EOF
            if !got_any {
                return None;
            }
            // Append the EOF sentinel like C does (-1 as unsigned)
            buf.push(0xFFu8);
            break;
        }
        got_any = true;
        if b[0] == b'\n' {
            break;
        }
        buf.push(b[0]);
    }
    // Build a String. The buffer might have a 0xFF byte that's not valid UTF-8,
    // so use lossy conversion. But since later we use as_bytes() to scan, we
    // really want to preserve raw bytes. Use from_utf8_lossy and store as
    // String, but to keep raw bytes we use unsafe wrapping. To stay safe,
    // we'll convert byte-by-byte using std::str::from_utf8 with a fallback.
    //
    // Since the parser only ever scans for ASCII tokens ("slothy", "sloth",
    // "and", "nap", "#") and otherwise advances the cursor, we can safely
    // store the raw bytes by treating non-UTF-8 bytes as Latin-1.
    let s: String = buf.iter().map(|&b| b as char).collect();
    Some(s)
}

pub fn prog_len(file: &std::fs::File) -> usize {
    let mut f = file;
    // Save current position (should be start). We'll seek back to 0 at end.
    let mut count: usize = 0;
    let mut buf = [0u8; 4096];
    loop {
        let n = match f.read(&mut buf) {
            Ok(n) => n,
            Err(_) => 0,
        };
        if n == 0 {
            break;
        }
        for i in 0..n {
            if buf[i] == b'\n' {
                count += 1;
            }
        }
    }
    let _ = f.seek(SeekFrom::Start(0));
    count * 3
}
