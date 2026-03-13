use std::io::{self, Read, Write, BufWriter};

fn c_atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let neg = if i < s.len() && s[i] == b'-' {
        i += 1;
        true
    } else {
        if i < s.len() && s[i] == b'+' {
            i += 1;
        }
        false
    };
    let mut result: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((s[i] - b'0') as i32);
        i += 1;
    }
    if neg { result.wrapping_neg() } else { result }
}

fn c_fgets(buf: &mut Vec<u8>, max_size: usize) -> bool {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let max_chars = max_size - 1;
    let mut byte = [0u8; 1];
    loop {
        if buf.len() >= max_chars {
            break;
        }
        match handle.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    !buf.is_empty()
}

fn main() {
    // Use a BufWriter so output is buffered like C's fully-buffered stdout on pipes.
    // This means if we abort(), unflushed output is lost — matching C behavior.
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut data: i32 = -1;

    {
        let mut input_buffer = Vec::new();
        if c_fgets(&mut input_buffer, 14) {
            data = c_atoi(&input_buffer);
        } else {
            let _ = writeln!(out, "fgets() failed.");
        }
    }

    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];
        for i in 0..99 {
            source[i] = b'A';
        }
        source[99] = 0;

        if data < 100 {
            if data < 0 {
                // C: strncpy with wrapped huge size + dest[negative] = UB -> segfault.
                // Buffered output is lost. Abort to match.
                std::process::abort();
            }
            let n = data as usize;
            let src_null = source.iter().position(|&b| b == 0).unwrap_or(source.len());
            for i in 0..n {
                dest[i] = if i < src_null { source[i] } else { 0 };
            }
            dest[n] = 0;
        }

        let end = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
        let s = std::str::from_utf8(&dest[..end]).unwrap_or("");
        let _ = writeln!(out, "{}", s);
    }

    let _ = out.flush();
}
