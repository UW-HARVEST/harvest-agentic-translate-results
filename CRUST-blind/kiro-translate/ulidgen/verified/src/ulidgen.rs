// Import statements
use crate::ulid;
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let _ = argc;
    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];
    let mut n: i64 = 1;
    let mut tflag = false;

    let mut i = 1usize;
    while i < argv.len() {
        match argv[i] {
            "-n" => {
                i += 1;
                if i < argv.len() {
                    n = argv[i].parse().unwrap_or(0);
                }
            }
            "-t" => tflag = true,
            _ => {}
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    let s: String = ulid_buf[..26].iter().collect();
                    let _ = write!(out, "{} {}\n", s, l);
                    let _ = out.flush();
                }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let s: String = ulid_buf[..26].iter().collect();
            let _ = writeln!(out, "{}", s);
        }
    }

    match out.flush() {
        Ok(_) => 0,
        Err(_) => 1,
    }
}
