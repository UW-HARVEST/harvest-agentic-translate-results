pub mod murmurhash;

use std::io::{self, BufRead, IsTerminal, Write};

pub fn usage() {
    eprintln!("usage: murmur [-hV] [options]");
}

pub fn help() {
    eprintln!();
    eprintln!("options:");
    eprint!("\n  --seed=[seed]  hash seed (optional)");
    eprintln!();
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

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut seed: Option<String> = None;

    // parse opts (skip argv[0])
    let mut i = 1;
    while i < args.len() {
        let opt = &args[i];
        if opt.starts_with('-') {
            let rest = &opt[1..];
            if rest.is_empty() {
                i += 1;
                continue;
            }
            let first = rest.as_bytes()[0] as char;
            match first {
                'h' => {
                    usage();
                    help();
                    return;
                }
                'V' => {
                    eprintln!("{}", crate::murmurhash::MURMURHASH_VERSION);
                    return;
                }
                '-' => {
                    // long option; check for "seed"
                    let long = &rest[1..];
                    if long.starts_with("seed") {
                        // setopt advances past "seed", so the remainder
                        // (typically "=value") is the seed value pointer.
                        let after = &long[4..];
                        seed = Some(after.to_string());
                    }
                }
                _ => {
                    eprintln!("unknown option: `{}'", rest);
                    usage();
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    let seed_str = seed.unwrap_or_else(|| "0".to_string());
    // The C code does atoi(seed); seed values often look like "=N", so
    // strip a leading '=' and parse leading digits like atoi.
    let seed_value = parse_atoi(&seed_str);

    let stdin = io::stdin();
    if stdin.is_terminal() {
        std::process::exit(1);
    }

    let buf = read_stdin();
    if buf.is_empty() {
        // mimics: NULL == buf -> return 1
        // empty read returns empty vec (treated as no input here)
        std::process::exit(1);
    }

    // C uses strlen(buf) so it stops at first NUL; in Rust, just use the bytes.
    let key_bytes = strip_nul(&buf);
    let h = crate::murmurhash::murmurhash(key_bytes, seed_value);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", h);

    loop {
        let key = read_stdin();
        if key.is_empty() {
            break;
        }
        let key_bytes = strip_nul(&key);
        let h = crate::murmurhash::murmurhash(key_bytes, seed_value);
        let _ = writeln!(out, "{}", h);
    }
}

fn strip_nul(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(idx) => &buf[..idx],
        None => buf,
    }
}

fn parse_atoi(s: &str) -> u32 {
    // Mirror C atoi: skip leading whitespace, optional sign, then digits.
    // The seed string passed via setopt may begin with '=' which atoi
    // would treat as non-numeric (returning 0).
    let trimmed = s.trim_start();
    let bytes = trimmed.as_bytes();
    let mut idx = 0;
    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        negative = bytes[idx] == b'-';
        idx += 1;
    }
    let mut val: i64 = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        val = val
            .saturating_mul(10)
            .saturating_add((bytes[idx] - b'0') as i64);
        idx += 1;
    }
    if negative {
        val = -val;
    }
    val as u32
}
