// Import statements
use crate::ulid;
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut n: i64 = 1;
    let mut tflag = false;

    // Tiny `getopt(argc, argv, "n:t")` clone — handles `-n N`, `-nN`, and `-t`.
    let args: Vec<&str> = argv.iter().take(argc.max(0) as usize).copied().collect();
    let mut i = 1usize;
    while i < args.len() {
        let arg = args[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        let bytes = arg.as_bytes();
        let mut j = 1;
        while j < bytes.len() {
            match bytes[j] as char {
                't' => {
                    tflag = true;
                    j += 1;
                }
                'n' => {
                    let value: &str = if j + 1 < bytes.len() {
                        &arg[j + 1..]
                    } else {
                        i += 1;
                        if i >= args.len() {
                            break;
                        }
                        args[i]
                    };
                    n = value.parse::<i64>().unwrap_or(0);
                    j = bytes.len();
                }
                _ => {
                    j += 1;
                }
            }
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut ulid = ['\0'; ulid::ULID_LENGTH];

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => return 1,
            };
            ulid::ulidgen_r(&mut ulid);
            let ulid_str: String = ulid.iter().take(26).collect();
            if writeln!(out, "{} {}", ulid_str, line).is_err() {
                return 1;
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid);
            let ulid_str: String = ulid.iter().take(26).collect();
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
