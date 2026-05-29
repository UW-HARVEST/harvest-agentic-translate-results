// Import statements
use crate::{ulid};
use std::io::{BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse arguments similar to getopt(argc, argv, "n:t")
    let mut i = 1;
    while i < argc as usize && i < argv.len() {
        let arg = argv[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        // Process each character after '-' as an option
        let chars: Vec<char> = arg.chars().skip(1).collect();
        let mut j = 0;
        let mut consumed_next = false;
        while j < chars.len() {
            let c = chars[j];
            match c {
                'n' => {
                    // Get the option argument
                    let optarg: String = if j + 1 < chars.len() {
                        chars[j + 1..].iter().collect()
                    } else if i + 1 < argv.len() {
                        consumed_next = true;
                        argv[i + 1].to_string()
                    } else {
                        // missing argument - getopt would print an error
                        String::new()
                    };
                    // atol behavior: parse leading integer, returns 0 on parse failure
                    n = parse_atol(&optarg);
                    break;
                }
                't' => {
                    tflag = true;
                }
                _ => {
                    // unknown option - getopt prints error but continues
                }
            }
            j += 1;
        }
        if consumed_next {
            i += 2;
        } else {
            i += 1;
        }
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    if tflag {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        let mut line = String::new();
        loop {
            line.clear();
            match input.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    let ulid_str: String = ulid_buf
                        .iter()
                        .take_while(|&&c| c != '\0')
                        .collect();
                    if write!(out, "{} {}", ulid_str, line).is_err() {
                        had_error = true;
                        break;
                    }
                    if out.flush().is_err() {
                        had_error = true;
                        break;
                    }
                }
                Err(_) => {
                    had_error = true;
                    break;
                }
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid_str: String = ulid_buf
                .iter()
                .take_while(|&&c| c != '\0')
                .collect();
            if writeln!(out, "{}", ulid_str).is_err() {
                had_error = true;
                break;
            }
        }
    }

    if out.flush().is_err() {
        had_error = true;
    }

    if had_error {
        1
    } else {
        0
    }
}

// Mimics C's atol: parse leading optional sign and digits, ignoring trailing non-digit characters.
// Returns 0 on parse failure.
fn parse_atol(s: &str) -> i64 {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading whitespace
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }
    let mut result: i64 = 0;
    let mut found_digit = false;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        found_digit = true;
        let d = (bytes[i] - b'0') as i64;
        result = result.saturating_mul(10).saturating_add(d);
        i += 1;
    }
    if !found_digit {
        return 0;
    }
    if negative {
        -result
    } else {
        result
    }
}
