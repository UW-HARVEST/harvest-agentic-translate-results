// Import statements
use crate::{ulid};
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Simple getopt-like parsing: supports `-n N` and `-t` (skip argv[0]).
    let argv_slice: &[&str] = if argc >= 0 && (argc as usize) <= argv.len() {
        &argv[..argc as usize]
    } else {
        argv
    };

    let mut i = 1usize;
    while i < argv_slice.len() {
        let arg = argv_slice[i];
        if arg == "-n" {
            i += 1;
            if i < argv_slice.len() {
                n = argv_slice[i].parse::<i64>().unwrap_or(0);
            }
        } else if arg == "-t" {
            tflag = true;
        } else if let Some(rest) = arg.strip_prefix("-n") {
            // Allow `-nN` style.
            n = rest.parse::<i64>().unwrap_or(0);
        }
        i += 1;
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
            let s: String = ulid_buf.iter().take_while(|&&c| c != '\0').collect();
            if writeln!(out, "{} {}", s, line).is_err() {
                return 1;
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf.iter().take_while(|&&c| c != '\0').collect();
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
