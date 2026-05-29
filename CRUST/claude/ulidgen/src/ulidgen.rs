// Import statements
use crate::ulid;
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let _ = argc;

    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];

    // Parse command-line options: -n N | -t
    let mut n: i64 = 1;
    let mut tflag = false;

    let mut i = 1usize;
    while i < argv.len() {
        let arg = argv[i];
        if arg == "-t" {
            tflag = true;
            i += 1;
        } else if arg == "-n" {
            i += 1;
            if i < argv.len() {
                n = argv[i].parse::<i64>().unwrap_or(0);
                i += 1;
            } else {
                break;
            }
        } else if let Some(rest) = arg.strip_prefix("-n") {
            n = rest.parse::<i64>().unwrap_or(0);
            i += 1;
        } else {
            // unknown option / non-option argument: skip
            i += 1;
        }
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let stdin_lock = stdin.lock();
        for line in stdin_lock.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf.iter().take(26).collect();
            if writeln!(out, "{} {}", s, line).is_err() {
                return 1;
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf.iter().take(26).collect();
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
