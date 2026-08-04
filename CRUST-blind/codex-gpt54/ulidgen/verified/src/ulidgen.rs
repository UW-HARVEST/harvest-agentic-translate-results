// Import statements
use crate::{ulid};
use std::io::{self, BufRead, Write};
// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let argc = argc.max(0) as usize;
    let args = &argv[..argc.min(argv.len())];

    let mut n = 1_i64;
    let mut tflag = false;
    let mut idx = 1usize;
    while idx < args.len() {
        let arg = args[idx];
        if arg == "-t" {
            tflag = true;
        } else if let Some(value) = arg.strip_prefix("-n") {
            let raw = if value.is_empty() {
                idx += 1;
                args.get(idx).copied().unwrap_or("")
            } else {
                value
            };
            n = raw.parse::<i64>().unwrap_or(0);
        }
        idx += 1;
    }

    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut line = Vec::new();

        loop {
            line.clear();
            match input.read_until(b'\n', &mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    let ulid = ulid_to_string(&ulid_buf);
                    if out.write_all(ulid.as_bytes()).is_err() {
                        return 1;
                    }
                    if out.write_all(b" ").is_err() {
                        return 1;
                    }
                    if out.write_all(&line).is_err() {
                        return 1;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid = ulid_to_string(&ulid_buf);
            if out.write_all(ulid.as_bytes()).is_err() {
                return 1;
            }
            if out.write_all(b"\n").is_err() {
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
    ulid.iter().take(26).collect()
}
