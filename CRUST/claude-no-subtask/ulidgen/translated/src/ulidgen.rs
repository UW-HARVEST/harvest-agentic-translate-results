// Import statements
use crate::ulid;
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf: [char; ulid::ULID_LENGTH] = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Mimic getopt(argc, argv, "n:t") parsing
    let mut i = 1usize;
    let argc = argc as usize;
    while i < argc && i < argv.len() {
        let arg = argv[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        // handle combined short flags like "-n5" or separate "-n 5"
        let bytes = arg.as_bytes();
        let mut j = 1usize;
        while j < bytes.len() {
            let c = bytes[j] as char;
            match c {
                'n' => {
                    let val = if j + 1 < bytes.len() {
                        let v = &arg[j + 1..];
                        j = bytes.len();
                        v.to_string()
                    } else if i + 1 < argc && i + 1 < argv.len() {
                        i += 1;
                        argv[i].to_string()
                    } else {
                        return 1;
                    };
                    n = val.parse::<i64>().unwrap_or(0);
                }
                't' => {
                    tflag = true;
                }
                _ => {
                    // unknown option - ignore (matches getopt default behavior loosely)
                }
            }
            j += 1;
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    let ulid_str: String = ulid_buf.iter().take_while(|&&c| c != '\0').collect();
                    if write!(out, "{} {}", ulid_str, line).is_err() {
                        return 1;
                    }
                    let _ = out.flush();
                }
                Err(_) => return 1,
            }
        }
    } else {
        for _ in 0..n {
            ulid::ulidgen_r(&mut ulid_buf);
            let ulid_str: String = ulid_buf.iter().take_while(|&&c| c != '\0').collect();
            if writeln!(out, "{}", ulid_str).is_err() {
                return 1;
            }
        }
    }

    let _ = out.flush();
    0
}
