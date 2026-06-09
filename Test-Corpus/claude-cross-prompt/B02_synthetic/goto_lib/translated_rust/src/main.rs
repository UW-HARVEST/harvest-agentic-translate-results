// Translation of c_src/src/goto.c to Rust producing byte-identical output.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::ExitCode;

fn forward_goto_example(x: i32) -> i32 {
    if x < 0 {
        // goto error
        let stderr = io::stderr();
        let mut stderr = stderr.lock();
        let _ = stderr.write_all(b"Error: negative input\n");
        return -1;
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let _ = write!(stdout, "Processing: {}\n", x);
    x.wrapping_mul(2)
}

/// Mimics fgets(buffer, 100, fp): reads up to 99 bytes, stops at newline (included) or EOF.
/// Returns Ok(Some(bytes)) when bytes were read (analogous to fgets returning the buffer),
/// Ok(None) on EOF with no bytes (analogous to fgets returning NULL),
/// Err on read error (sets ferror).
fn fgets_99<R: Read>(reader: &mut R, buf: &mut Vec<u8>) -> io::Result<bool> {
    buf.clear();
    let cap: usize = 99;
    while buf.len() < cap {
        let mut byte = [0u8; 1];
        let n = reader.read(&mut byte)?;
        if n == 0 {
            // EOF
            break;
        }
        buf.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    Ok(!buf.is_empty())
}

/// Mimics open_with_cleanup. Returns Some(File) on success or None on error.
fn open_with_cleanup(filename: &str) -> Option<File> {
    let fp_res = File::open(filename);
    let fp = match fp_res {
        Ok(f) => f,
        Err(_) => {
            // goto cleanup with fp == NULL
            let stderr = io::stderr();
            let mut stderr = stderr.lock();
            let _ = write!(stderr, "Error: opening or processing file {}\n", filename);
            return None;
        }
    };

    // Read with a BufReader for efficiency (still emits same output bytes).
    let mut reader = BufReader::new(fp);
    let mut buf: Vec<u8> = Vec::with_capacity(99);
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut had_error = false;

    loop {
        match fgets_99(&mut reader, &mut buf) {
            Ok(true) => {
                let _ = stdout.write_all(&buf);
            }
            Ok(false) => break, // EOF, fgets returned NULL
            Err(_) => {
                had_error = true;
                break;
            }
        }
    }

    drop(stdout);

    let fp = reader.into_inner();

    if had_error {
        let stderr = io::stderr();
        let mut stderr = stderr.lock();
        let _ = write!(stderr, "Error: opening or processing file {}\n", filename);
        // C: if(fp) fclose(fp); -- fp is non-null here, drop handles it
        drop(fp);
        return None;
    }

    Some(fp)
}

fn driver(num: i32, filename: &str) -> i32 {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        let _ = write!(stdout, "Goto output: {}\n", res);
    }

    let out = open_with_cleanup(filename);
    if out.is_none() {
        return -2;
    }
    // else: fclose(out); drop handles it
    drop(out);

    0
}

/// Read the next whitespace-delimited token from stdin,
/// matching scanf's whitespace-skipping behavior.
fn read_token<R: BufRead>(reader: &mut R) -> Option<String> {
    let mut s = Vec::new();
    // Skip leading whitespace
    loop {
        let mut tmp = [0u8; 1];
        let n = reader.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        if !tmp[0].is_ascii_whitespace() {
            s.push(tmp[0]);
            break;
        }
    }
    // Read until whitespace or EOF
    loop {
        let mut tmp = [0u8; 1];
        let n = reader.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        if tmp[0].is_ascii_whitespace() {
            break;
        }
        s.push(tmp[0]);
    }
    String::from_utf8(s).ok()
}

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    // Read int from stdin (scanf %d behavior).
    let num_str = match read_token(&mut reader) {
        Some(s) => s,
        None => return ExitCode::from(0),
    };
    let num: i32 = match num_str.parse() {
        Ok(n) => n,
        Err(_) => return ExitCode::from(0),
    };

    // Read filename token (scanf %s behavior).
    let filename = match read_token(&mut reader) {
        Some(s) => s,
        None => String::new(),
    };

    let rc = driver(num, &filename);
    // Use the lower 8 bits as the exit code, matching C's main return convention.
    let code = (rc & 0xFF) as u8;
    ExitCode::from(code)
}
