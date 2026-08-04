use crate::ulid::{self, ULID_LENGTH};
use std::io::{self, BufRead, Write};

fn parse_atol(value: &str) -> i64 {
    value.trim().parse::<i64>().unwrap_or(0)
}

fn ulid_to_string(ulid: &[char; ULID_LENGTH]) -> String {
    ulid.iter().take(26).collect()
}

pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let argc = argc.max(0) as usize;
    let args = &argv[..argc.min(argv.len())];

    let mut n = 1_i64;
    let mut tflag = false;
    let mut i = 1usize;

    while i < args.len() {
        let arg = args[i];
        if arg == "-t" {
            tflag = true;
            i += 1;
            continue;
        }

        if arg == "-n" {
            if let Some(value) = args.get(i + 1) {
                n = parse_atol(value);
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        if let Some(value) = arg.strip_prefix("-n") {
            n = parse_atol(value);
        }

        i += 1;
    }

    let mut ulid_buf = ['\0'; ULID_LENGTH];
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut line = String::new();

        loop {
            line.clear();
            match input.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    if write!(handle, "{} {}", ulid_to_string(&ulid_buf), line).is_err() {
                        return 1;
                    }
                }
                Err(_) => return 1,
            }
        }
    } else {
        for _ in 0..n.max(0) as usize {
            ulid::ulidgen_r(&mut ulid_buf);
            if writeln!(handle, "{}", ulid_to_string(&ulid_buf)).is_err() {
                return 1;
            }
        }
    }

    handle.flush().map(|_| 0).unwrap_or(1)
}
