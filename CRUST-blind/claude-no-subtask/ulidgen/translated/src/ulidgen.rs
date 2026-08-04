// Import statements
use crate::{ulid};
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse command-line arguments similar to getopt(argc, argv, "n:t")
    let argc = argc as usize;
    let mut i = 1usize;
    let mut parse_error = false;
    while i < argc && i < argv.len() {
        let arg = argv[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        if arg == "--" {
            i += 1;
            break;
        }
        // Process each character flag in the arg (after the leading '-')
        let bytes = arg.as_bytes();
        let mut j = 1usize;
        let mut consumed_value = false;
        while j < bytes.len() {
            let c = bytes[j] as char;
            match c {
                'n' => {
                    // Requires an argument: either remainder of this arg or next arg
                    let value: Option<&str> = if j + 1 < bytes.len() {
                        Some(&arg[j + 1..])
                    } else if i + 1 < argc && i + 1 < argv.len() {
                        i += 1;
                        consumed_value = true;
                        Some(argv[i])
                    } else {
                        None
                    };
                    if let Some(v) = value {
                        // atol-like parsing: parse leading integer, default 0 if invalid
                        n = parse_atol(v);
                    } else {
                        parse_error = true;
                    }
                    // 'n:' consumes the rest of the current arg as its value
                    j = bytes.len();
                }
                't' => {
                    tflag = true;
                    j += 1;
                }
                _ => {
                    parse_error = true;
                    j += 1;
                }
            }
        }
        let _ = consumed_value;
        i += 1;
    }
    let _ = parse_error;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut line: Vec<u8> = Vec::new();
        loop {
            line.clear();
            let n_read = match input.read_until(b'\n', &mut line) {
                Ok(n) => n,
                Err(_) => 0,
            };
            if n_read == 0 {
                break;
            }
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid_str = ulid_to_string(&ulid_buf);
            // printf("%s %s", ulid, line);
            if write!(out, "{} ", ulid_str).is_err() {
                break;
            }
            if out.write_all(&line).is_err() {
                break;
            }
            // Line-buffered: flush on newline
            if line.last() == Some(&b'\n') {
                let _ = out.flush();
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid_str = ulid_to_string(&ulid_buf);
            if writeln!(out, "{}", ulid_str).is_err() {
                break;
            }
        }
    }

    let flush_result = out.flush();
    if flush_result.is_err() {
        1
    } else {
        0
    }
}

// Convert the char array (NUL-terminated, like in C) to a String,
// stopping at the first NUL.
fn ulid_to_string(ulid: &[char; ulid::ULID_LENGTH]) -> String {
    let mut s = String::with_capacity(ulid::ULID_LENGTH);
    for &c in ulid.iter() {
        if c == '\0' {
            break;
        }
        s.push(c);
    }
    s
}

// Mimic atol(): parse leading integer, ignoring trailing non-digit characters,
// returning 0 on parse failure for the leading portion.
fn parse_atol(s: &str) -> i64 {
    let bytes = s.as_bytes();
    let mut idx = 0usize;
    // Skip leading whitespace (atol ignores leading whitespace)
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }
    let mut sign: i64 = 1;
    if idx < bytes.len() {
        match bytes[idx] as char {
            '+' => idx += 1,
            '-' => {
                sign = -1;
                idx += 1;
            }
            _ => {}
        }
    }
    let mut result: i64 = 0;
    while idx < bytes.len() {
        let c = bytes[idx] as char;
        if let Some(d) = c.to_digit(10) {
            result = result.saturating_mul(10).saturating_add(d as i64);
            idx += 1;
        } else {
            break;
        }
    }
    sign.saturating_mul(result)
}
