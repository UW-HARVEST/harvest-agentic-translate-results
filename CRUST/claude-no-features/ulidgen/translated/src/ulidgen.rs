// Import statements
use crate::{ulid};
use std::io::{self, BufRead, Write};

// Function Declarations
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    let mut ulid_buf = ['\0'; ulid::ULID_LENGTH];

    let mut n: i64 = 1;
    let mut tflag = false;

    // Simple getopt-like parsing for "n:t"
    let mut i: usize = 1;
    while (i as i32) < argc && i < argv.len() {
        let arg = argv[i];
        if !arg.starts_with('-') || arg == "-" {
            break;
        }
        if arg == "--" {
            break;
        }
        // process each char after '-'
        let chars: Vec<char> = arg.chars().skip(1).collect();
        let mut j = 0;
        let mut consumed_extra = false;
        while j < chars.len() {
            let c = chars[j];
            match c {
                'n' => {
                    let optarg: String = if j + 1 < chars.len() {
                        chars[j + 1..].iter().collect()
                    } else if i + 1 < argv.len() {
                        i += 1;
                        consumed_extra = true;
                        argv[i].to_string()
                    } else {
                        return 1;
                    };
                    n = optarg.parse::<i64>().unwrap_or(0);
                    break;
                }
                't' => {
                    tflag = true;
                }
                _ => {
                    // unknown option
                }
            }
            j += 1;
            let _ = consumed_extra;
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        let stdin_lock = stdin.lock();
        for line in stdin_lock.lines() {
            match line {
                Ok(l) => {
                    ulid::ulidgen_r(&mut ulid_buf);
                    let s: String = ulid_buf.iter().take_while(|&&c| c != '\0').collect();
                    if writeln!(out, "{} {}", s, l).is_err() {
                        return 1;
                    }
                }
                Err(_) => return 1,
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

    let _ = out.flush();
    0
}
