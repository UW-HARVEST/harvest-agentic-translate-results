// Import statements
use crate::{ulid};
use std::io::{BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse arguments mimicking getopt(argc, argv, "n:t").
    // Recognized forms: "-n N" (separate args), "-nN" (combined), "-t".
    let mut i: usize = 1;
    let argc_usize = argc as usize;
    while i < argc_usize && i < argv.len() {
        let arg = argv[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        if arg == "--" {
            break;
        }
        let bytes = arg.as_bytes();
        let mut j = 1;
        let mut consume_next = false;
        while j < bytes.len() {
            match bytes[j] as char {
                't' => {
                    tflag = true;
                    j += 1;
                }
                'n' => {
                    // optarg: rest of this arg, or next arg
                    if j + 1 < bytes.len() {
                        let rest = &arg[j + 1..];
                        n = rest.parse::<i64>().unwrap_or(0);
                        j = bytes.len();
                    } else {
                        consume_next = true;
                        j = bytes.len();
                    }
                }
                _ => {
                    j = bytes.len();
                }
            }
        }
        i += 1;
        if consume_next && i < argc_usize && i < argv.len() {
            n = argv[i].parse::<i64>().unwrap_or(0);
            i += 1;
        }
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut had_error = false;

    if tflag {
        let stdin = std::io::stdin();
        let stdin_lock = stdin.lock();
        for line_res in stdin_lock.lines() {
            match line_res {
                Ok(line) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    let s: String = ulid_buf[..26].iter().collect();
                    if writeln!(out, "{} {}", s, line).is_err() {
                        had_error = true;
                        break;
                    }
                    if out.flush().is_err() {
                        had_error = true;
                        break;
                    }
                }
                Err(_) => {
                    had_error = true;
                    break;
                }
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

    if had_error {
        1
    } else {
        0
    }
}
