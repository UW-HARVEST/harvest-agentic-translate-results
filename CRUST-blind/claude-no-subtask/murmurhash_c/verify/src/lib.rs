pub mod murmurhash;

use std::io::{self, BufRead, IsTerminal, Read, Write};

pub fn usage() {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(handle, "usage: murmur [-hV] [options]");
}

pub fn help() {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = write!(handle, "\noptions:\n");
    let _ = write!(handle, "\n  --seed=[seed]  hash seed (optional)");
    let _ = write!(handle, "\n");
}

pub fn read_stdin() -> Vec<u8> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf: Vec<u8> = Vec::new();

    // Read up to 1023 bytes or until newline, mimicking fgets(buf, 1024, stdin).
    let mut count = 0usize;
    let max = 1023usize;
    let mut byte = [0u8; 1];
    loop {
        if count >= max {
            break;
        }
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    buf
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut seed_str: Option<String> = None;

    // parse opts (skipping argv[0])
    let mut i = 1;
    while i < args.len() {
        let opt = &args[i];
        i += 1;
        let bytes = opt.as_bytes();
        if !bytes.is_empty() && bytes[0] == b'-' {
            if bytes.len() < 2 {
                // bare '-'
                continue;
            }
            match bytes[1] {
                b'h' => {
                    usage();
                    help();
                    return;
                }
                b'V' => {
                    let stderr = io::stderr();
                    let mut handle = stderr.lock();
                    let _ = writeln!(handle, "{}", murmurhash::MURMURHASH_VERSION);
                    return;
                }
                b'-' => {
                    // long opt: --seed=value
                    let rest = &opt[2..];
                    if rest.starts_with("seed") {
                        // After "seed", expect "=value"
                        let after = &rest[4..];
                        let value = if let Some(stripped) = after.strip_prefix('=') {
                            stripped
                        } else {
                            after
                        };
                        seed_str = Some(value.to_string());
                    }
                }
                _ => {
                    let stderr = io::stderr();
                    let mut handle = stderr.lock();
                    let _ = writeln!(handle, "unknown option: `{}'", &opt[1..]);
                    usage();
                    std::process::exit(1);
                }
            }
        }
    }

    let seed_val: u32 = match seed_str {
        Some(s) => s.trim().parse::<i32>().unwrap_or(0) as u32,
        None => 0,
    };

    // If stdin is a tty, exit 1
    if io::stdin().is_terminal() {
        std::process::exit(1);
    }

    // Read first line
    let buf = read_stdin();
    if buf.is_empty() {
        // mimic returning NULL when EOF on first read
        std::process::exit(1);
    }
    // strlen() in C stops at '\0' — assume no embedded NULs in stdin.
    // Use the bytes up to (but not including) any trailing '\0' if present.
    let key_bytes: &[u8] = match buf.iter().position(|&b| b == 0) {
        Some(p) => &buf[..p],
        None => &buf[..],
    };
    let h = murmurhash::murmurhash(key_bytes, seed_val);
    println!("{}", h);

    // Continue reading until EOF
    loop {
        let key = read_stdin();
        if key.is_empty() {
            break;
        }
        // The C code has a bug: it always re-hashes `buf`, not `key`.
        // Replicate that behavior faithfully.
        let h2 = murmurhash::murmurhash(key_bytes, seed_val);
        println!("{}", h2);
    }
}

// Use BufRead in some configurations to keep the import live.
#[allow(dead_code)]
fn _bufread_anchor<R: BufRead>(_: R) {}
