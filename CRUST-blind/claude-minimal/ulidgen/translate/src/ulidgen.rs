// Import statements
use crate::ulid;
use std::io::{self, BufRead, Write};
// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let _ = argc;

    let mut n: i64 = 1;
    let mut tflag = false;

    // Parse args mimicking getopt(argc, argv, "n:t").
    // We start at index 1 to skip the program name (argv[0]).
    let mut i = 1usize;
    while i < argv.len() {
        let arg = argv[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }

        // Walk each character of the argument cluster after the '-'.
        let bytes = arg.as_bytes();
        let mut j = 1;
        while j < bytes.len() {
            let c = bytes[j] as char;
            match c {
                'n' => {
                    // Option requires a value: either remainder of this arg
                    // (e.g. "-n5") or the next argument (e.g. "-n 5").
                    let value: &str;
                    if j + 1 < bytes.len() {
                        value = &arg[j + 1..];
                        j = bytes.len();
                    } else if i + 1 < argv.len() {
                        i += 1;
                        value = argv[i];
                        j = bytes.len();
                    } else {
                        // Missing required value; mimic getopt's silent handling.
                        j = bytes.len();
                        continue;
                    }
                    // atol returns 0 on parse failure.
                    n = value.parse::<i64>().unwrap_or(0);
                }
                't' => {
                    tflag = true;
                    j += 1;
                }
                _ => {
                    // Unknown options are silently ignored (matches the C
                    // switch which has no default case).
                    j += 1;
                }
            }
        }
        i += 1;
    }

    let mut ulid_buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(_) => break,
            }
            ulid::ulidgen_r(&mut ulid_buf);
            // Print the ULID (26 chars; ulid[26] is the NUL terminator) then
            // a space and the input line (which already includes the newline).
            let ulid_str: String = ulid_buf[..26].iter().collect();
            if write!(out, "{} {}", ulid_str, line).is_err() {
                return 1;
            }
            if out.flush().is_err() {
                return 1;
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid_str: String = ulid_buf[..26].iter().collect();
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
