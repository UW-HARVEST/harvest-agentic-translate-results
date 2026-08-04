

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::BufRead;


fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

/// Emulates C's `atoi`: parses a leading optional sign followed by digits,
/// ignoring leading whitespace and stopping at the first non-digit.
/// Returns 0 if no valid conversion is possible.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }

    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        result = result.saturating_mul(10).saturating_add(digit);
        i += 1;
    }

    result.saturating_mul(sign)
}

/// Emulates `fgets(buf, 14, stdin)`: reads up to 13 bytes or until newline
/// (inclusive), from standard input. Returns `None` on EOF/error with no bytes.
fn read_bounded_line(max_len: usize) -> Option<String> {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut buf = String::new();
    let mut total = 0usize;

    while total + 1 < max_len {
        let available = match handle.fill_buf() {
            Ok(b) if b.is_empty() => break,
            Ok(b) => b,
            Err(_) => return None,
        };

        let remaining = max_len - 1 - total;
        let (chunk, found_nl) = match available.iter().position(|&b| b == b'\n') {
            Some(pos) if pos + 1 <= remaining => (&available[..=pos], true),
            _ => {
                let take = remaining.min(available.len());
                (&available[..take], false)
            }
        };

        buf.push_str(&String::from_utf8_lossy(chunk));
        let consumed = chunk.len();
        handle.consume(consumed);
        total += consumed;

        if found_nl {
            break;
        }
    }

    if total == 0 {
        None
    } else {
        Some(buf)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let data: i32 = match read_bounded_line(14) {
        Some(input) => c_atoi(&input),
        None => {
            print_line(Some("fgets() failed."));
            -1
        }
    };

    // Build `source`: 99 'A's followed by a null terminator (total 100 bytes).
    let mut source = [b'A'; 100];
    source[99] = 0;

    let mut dest = [0u8; 100];

    if data < 100 {
        // In C, a negative `data` passed to strncpy is interpreted as a huge
        // size_t, causing a buffer overflow. In safe Rust we cannot and will
        // not reproduce that undefined behavior; we simply skip the copy for
        // negative values, keeping the program memory-safe.
        if let Ok(n) = usize::try_from(data) {
            let copy_len = n.min(source.len());
            dest[..copy_len].copy_from_slice(&source[..copy_len]);
            if n < dest.len() {
                dest[n] = 0;
            }
        }
    }

    let end = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
    let printable = std::str::from_utf8(&dest[..end]).unwrap_or("");
    print_line(Some(printable));

    0
}
