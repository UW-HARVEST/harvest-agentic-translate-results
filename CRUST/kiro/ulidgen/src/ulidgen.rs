// Import statements
use crate::{ulid};
use std::io::{self, BufRead, Write, BufWriter};
// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];
    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse arguments (simulating getopt for -n N and -t)
    let mut i = 1usize;
    while i < argc as usize {
        match argv[i] {
            "-n" => {
                i += 1;
                if i < argc as usize {
                    n = argv[i].parse().unwrap_or(1);
                }
            }
            "-t" => tflag = true,
            s if s.starts_with("-n") => {
                n = s[2..].parse().unwrap_or(1);
            }
            _ => {}
        }
        i += 1;
    }

    let stdout = io::stdout();

    if tflag {
        let mut out = BufWriter::new(stdout.lock());
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf[..26].iter().collect();
            let _ = writeln!(out, "{} {}", s, line);
            let _ = out.flush();
        }
    } else {
        let mut out = BufWriter::new(stdout.lock());
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf[..26].iter().collect();
            let _ = writeln!(out, "{}", s);
        }
        let _ = out.flush();
    }

    0
}