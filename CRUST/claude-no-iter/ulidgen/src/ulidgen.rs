// Import statements
use crate::{ulid};
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse arguments in a getopt-like manner: support `-n N`, `-nN`, and `-t`.
    let mut i: usize = 1;
    while (i as i32) < argc && i < argv.len() {
        let arg = argv[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        if arg == "--" {
            i += 1;
            break;
        }

        // Iterate over each option char in this argv
        let opts: Vec<char> = arg[1..].chars().collect();
        let mut j = 0;
        while j < opts.len() {
            let c = opts[j];
            match c {
                't' => {
                    tflag = true;
                    j += 1;
                }
                'n' => {
                    // optarg is the rest of this arg, or the next argv
                    let optarg: String;
                    if j + 1 < opts.len() {
                        optarg = opts[j + 1..].iter().collect();
                        j = opts.len();
                    } else {
                        i += 1;
                        if (i as i32) >= argc || i >= argv.len() {
                            return 1;
                        }
                        optarg = argv[i].to_string();
                        j = opts.len();
                    }
                    // Parse like atol: best effort, default to 0 on failure.
                    n = parse_atol(&optarg);
                }
                _ => {
                    // Unknown option — getopt would print an error; just skip.
                    j += 1;
                }
            }
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let stdin_locked = stdin.lock();
        for line in stdin_locked.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => return 1,
            };
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid_str = ulid_to_string(&ulid_buf);
            if writeln!(out, "{} {}", ulid_str, line).is_err() {
                return 1;
            }
            if out.flush().is_err() {
                return 1;
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid_str = ulid_to_string(&ulid_buf);
            if writeln!(out, "{}", ulid_str).is_err() {
                return 1;
            }
        }
    }

    if out.flush().is_err() {
        return 1;
    }
    0
}

fn ulid_to_string(ulid: &[char; ulid::ULID_LENGTH]) -> String {
    // The first 26 characters are the printable ULID; index 26 is the
    // null terminator (kept to mirror the C interface).
    ulid.iter().take(26).collect()
}

fn parse_atol(s: &str) -> i64 {
    // Mimic C's atol: parse leading optional sign and as many digits as possible.
    let bytes = s.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
        idx += 1;
    }
    let mut sign: i64 = 1;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        if bytes[idx] == b'-' {
            sign = -1;
        }
        idx += 1;
    }
    let mut value: i64 = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((bytes[idx] - b'0') as i64);
        idx += 1;
    }
    sign.saturating_mul(value)
}
