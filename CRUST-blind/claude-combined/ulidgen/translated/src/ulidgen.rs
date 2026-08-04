// Import statements
use crate::{ulid};
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Simple getopt-like parsing for "-n N" and "-t"
    let argc = argc as usize;
    let mut i: usize = 1;
    while i < argc && i < argv.len() {
        let arg = argv[i];
        if arg == "-t" {
            tflag = true;
            i += 1;
        } else if arg == "-n" {
            if i + 1 < argv.len() {
                n = argv[i + 1].parse::<i64>().unwrap_or(0);
                i += 2;
            } else {
                i += 1;
            }
        } else if let Some(rest) = arg.strip_prefix("-n") {
            // -nN form
            n = rest.parse::<i64>().unwrap_or(0);
            i += 1;
        } else {
            i += 1;
        }
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf[..26].iter().collect();
            if writeln!(out, "{} {}", s, line).is_err() {
                return 1;
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf[..26].iter().collect();
            if writeln!(out, "{}", s).is_err() {
                return 1;
            }
        }
    }

    if out.flush().is_err() {
        return 1;
    }
    0
}
