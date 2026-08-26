use std::io::{self, Read, Write, BufWriter};

fn main() {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut data: i32 = -1;

    // fgets(inputBuffer, 14, stdin): reads up to 13 bytes, stops at newline (included) or EOF
    {
        let mut buf = [0u8; 14];
        let mut pos = 0usize;
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        while pos < 13 {
            let mut byte = [0u8; 1];
            match handle.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    buf[pos] = byte[0];
                    pos += 1;
                    if byte[0] == b'\n' { break; }
                }
                Err(_) => break,
            }
        }
        if pos > 0 {
            data = atoi(&String::from_utf8_lossy(&buf[..pos]));
        } else {
            let _ = writeln!(out, "fgets() failed.");
        }
    }

    {
        let mut source = [b'A'; 100];
        source[99] = 0;
        let mut dest = [0u8; 100];

        if data < 100 {
            let n = data as usize;
            if n > 99 {
                // Negative data wraps to huge usize (matches C segfault from strncpy overflow)
                std::process::abort();
            }
            dest[..n].copy_from_slice(&source[..n]);
            dest[n] = 0;
        }

        let len = dest.iter().position(|&b| b == 0).unwrap_or(100);
        let _ = writeln!(out, "{}", String::from_utf8_lossy(&dest[..len]));
    }

    let _ = out.flush();
}

/// Mimics C atoi: skip leading whitespace, optional sign, then digits.
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('+') => { chars.next(); false }
        Some('-') => { chars.next(); true }
        _ => false,
    };
    let mut r: i32 = 0;
    while let Some(&c) = chars.peek() {
        if let Some(d) = c.to_digit(10) {
            r = r.wrapping_mul(10).wrapping_add(d as i32);
            chars.next();
        } else { break; }
    }
    if neg { r.wrapping_neg() } else { r }
}
