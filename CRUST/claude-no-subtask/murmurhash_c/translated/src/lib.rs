pub mod murmurhash;

use std::io::{self, BufRead, IsTerminal, Write};

pub fn usage() {
    eprintln!("usage: murmur [-hV] [options]");
}

pub fn help() {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let _ = write!(stderr, "\noptions:");
    let _ = write!(stderr, "\n  --seed=[seed]  hash seed (optional)");
    let _ = writeln!(stderr);
}

pub fn read_stdin() -> Vec<u8> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf = String::new();
    match handle.read_line(&mut buf) {
        Ok(0) => Vec::new(),
        Ok(_) => buf.into_bytes(),
        Err(_) => Vec::new(),
    }
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut seed: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if let Some(rest) = arg.strip_prefix("-") {
            // remove possible second '-'
            let rest_after = rest.strip_prefix('-').unwrap_or(rest);

            // Match the C parsing: 1st char after '-' decides behavior
            let first = rest.chars().next();
            match first {
                Some('h') => {
                    usage();
                    help();
                    return;
                }
                Some('V') => {
                    eprintln!("{}", murmurhash::MURMURHASH_VERSION);
                    return;
                }
                Some('-') => {
                    // long option
                    if rest_after.starts_with("seed") {
                        // strip "seed" and any '=' prefix
                        let val = &rest_after["seed".len()..];
                        let val = val.strip_prefix('=').unwrap_or(val);
                        seed = Some(val.to_string());
                    }
                }
                _ => {
                    eprintln!("unknown option: `{}'", arg);
                    usage();
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    let seed_str = seed.unwrap_or_else(|| "0".to_string());
    let seed_val: u32 = seed_str.parse().unwrap_or(0);

    if io::stdin().is_terminal() {
        std::process::exit(1);
    }

    let buf = read_stdin();
    if buf.is_empty() {
        std::process::exit(1);
    }
    let h = murmurhash::murmurhash(&buf, seed_val);
    println!("{}", h);

    loop {
        let key = read_stdin();
        if key.is_empty() {
            break;
        }
        let h = murmurhash::murmurhash(&buf, seed_val);
        println!("{}", h);
    }
}
