pub mod murmurhash;

use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

pub fn usage() {
    let stderr = io::stderr();
    let mut s = stderr.lock();
    let _ = write!(s, "usage: murmur [-hV] [options]\n");
}

pub fn help() {
    let stderr = io::stderr();
    let mut s = stderr.lock();
    let _ = write!(s, "\noptions:\n");
    let _ = write!(s, "\n  --seed=[seed]  hash seed (optional)");
    let _ = write!(s, "\n");
}

pub fn read_stdin() -> Vec<u8> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();
    match handle.read_line(&mut line) {
        Ok(0) => Vec::new(),
        Ok(_) => line.into_bytes(),
        Err(_) => Vec::new(),
    }
}

fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut seed_str: Option<String> = None;

    // Skip program name (args[0]) and parse remaining flags
    let mut i = 1;
    while i < args.len() {
        let opt = &args[i];
        if opt.starts_with('-') {
            let rest = &opt[1..];
            let mut chars = rest.chars();
            match chars.next() {
                Some('h') => {
                    usage();
                    help();
                    return ExitCode::from(0);
                }
                Some('V') => {
                    let stderr = io::stderr();
                    let mut s = stderr.lock();
                    let _ = writeln!(s, "{}", crate::murmurhash::MURMURHASH_VERSION);
                    return ExitCode::from(0);
                }
                Some('-') => {
                    let long_rest: String = chars.collect();
                    if long_rest.starts_with("seed") {
                        // expect --seed=VALUE
                        let after = &long_rest["seed".len()..];
                        if let Some(stripped) = after.strip_prefix('=') {
                            seed_str = Some(stripped.to_string());
                        } else if !after.is_empty() {
                            seed_str = Some(after.to_string());
                        } else if i + 1 < args.len() {
                            i += 1;
                            seed_str = Some(args[i].clone());
                        }
                    }
                }
                _ => {
                    let stderr = io::stderr();
                    let mut s = stderr.lock();
                    let _ = writeln!(s, "unknown option: `{}'", rest);
                    usage();
                    return ExitCode::from(1);
                }
            }
        }
        i += 1;
    }

    let seed_value: u32 = match seed_str.as_deref() {
        None => 0,
        Some(s) => s.trim().parse::<i64>().unwrap_or(0) as u32,
    };

    let stdin = io::stdin();
    if stdin.is_terminal() {
        return ExitCode::from(1);
    }

    let buf = read_stdin();
    if buf.is_empty() {
        return ExitCode::from(1);
    }

    // Match the C behavior: hash uses strlen(buf), which excludes the
    // trailing nul. fgets keeps the trailing newline, so we hash the line
    // including any newline character but excluding any nul terminator.
    let h = crate::murmurhash::murmurhash(&buf, seed_value);
    println!("{}", h);

    // Continue reading additional lines until EOF.
    loop {
        let key = read_stdin();
        if key.is_empty() {
            break;
        }
        let h = crate::murmurhash::murmurhash(&buf, seed_value);
        println!("{}", h);
    }

    ExitCode::from(0)
}

pub fn main() {
    let _ = run();
}
