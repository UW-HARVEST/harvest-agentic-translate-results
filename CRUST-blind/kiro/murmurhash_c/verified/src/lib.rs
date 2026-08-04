pub mod murmurhash;

use std::io::{self, BufRead, IsTerminal};

pub fn usage() {
    eprint!("usage: murmur [-hV] [options]\n");
}

pub fn help() {
    eprint!("\noptions:\n");
    eprint!("\n  --seed=[seed]  hash seed (optional)");
    eprint!("\n");
}

pub fn read_stdin() -> Vec<u8> {
    let stdin = io::stdin();
    let mut buf = String::new();
    let mut reader = stdin.lock();
    match reader.read_line(&mut buf) {
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
        if arg.starts_with('-') {
            let rest = &arg[1..];
            if rest.starts_with('-') {
                // long option
                let long = &rest[1..];
                if long.starts_with("seed=") {
                    seed = Some(long["seed=".len()..].to_string());
                }
            } else {
                match rest.chars().next() {
                    Some('h') => {
                        usage();
                        help();
                        return;
                    }
                    Some('V') => {
                        eprintln!("{}", murmurhash::MURMURHASH_VERSION);
                        return;
                    }
                    Some(_) => {
                        eprintln!("unknown option: `{}'", &rest[0..]);
                        usage();
                        std::process::exit(1);
                    }
                    None => {}
                }
            }
        }
        i += 1;
    }

    let seed_val: u32 = seed
        .as_deref()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);

    let stdin = io::stdin();
    if stdin.lock().is_terminal() {
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
