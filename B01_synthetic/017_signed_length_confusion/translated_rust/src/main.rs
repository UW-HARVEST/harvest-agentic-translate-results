use std::io::{self, BufRead, Write, BufWriter};

fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let neg = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else {
        if i < bytes.len() && bytes[i] == b'+' {
            i += 1;
        }
        false
    };
    let mut val: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i32);
        i += 1;
    }
    if neg { val.wrapping_neg() } else { val }
}

fn main() {
    // Use BufWriter to match C's fully-buffered stdout behavior when piped
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut data: i32 = -1;

    let stdin = io::stdin();
    let mut input_buffer = String::new();
    if stdin.lock().read_line(&mut input_buffer).unwrap_or(0) > 0 {
        data = c_atoi(&input_buffer);
    } else {
        writeln!(out, "fgets() failed.").unwrap();
    }

    let mut source = [b'A'; 100];
    source[99] = 0;
    let mut dest = [0u8; 100];

    if data < 100 {
        let n = data as usize;
        let src_len = source.iter().position(|&b| b == 0).unwrap_or(source.len());
        let copy_len = src_len.min(n).min(dest.len());
        dest[..copy_len].copy_from_slice(&source[..copy_len]);

        // dest[data] = '\0' — negative index is UB in C (segfault)
        if data < 0 || data as usize >= dest.len() {
            std::process::abort();
        }
        dest[data as usize] = 0;
    }

    let end = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
    writeln!(out, "{}", std::str::from_utf8(&dest[..end]).unwrap_or("")).unwrap();
    out.flush().unwrap();
}
