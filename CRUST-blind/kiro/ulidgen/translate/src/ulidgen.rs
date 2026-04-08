// Import statements
use crate::ulid;
use std::io::{self, BufRead, Write, BufWriter};
// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];
    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse options (simple getopt-style)
    let mut i = 1usize;
    while i < argc as usize {
        match argv[i] {
            "-n" => {
                i += 1;
                if i < argc as usize {
                    n = argv[i].parse().unwrap_or(0);
                }
            }
            "-t" => tflag = true,
            _ => {}
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut had_error = false;

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    let s: String = ulid_buf[..26].iter().collect();
                    if writeln!(out, "{} {}", s, l).is_err() {
                        had_error = true;
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf[..26].iter().collect();
            if writeln!(out, "{}", s).is_err() {
                had_error = true;
                break;
            }
        }
    }

    if out.flush().is_err() {
        had_error = true;
    }

    if had_error { 1 } else { 0 }
}
