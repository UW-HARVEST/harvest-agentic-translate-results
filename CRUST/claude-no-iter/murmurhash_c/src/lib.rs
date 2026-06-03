pub mod murmurhash;

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
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut buf = String::new();
    match handle.read_line(&mut buf) {
        Ok(0) => Vec::new(),
        Ok(_) => buf.into_bytes(),
        Err(_) => Vec::new(),
    }
}

pub fn main() {
    use std::io::IsTerminal;
    let args: Vec<String> = std::env::args().collect();
    let mut seed: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let opt = &args[i];
        if opt.starts_with('-') {
            let rest = &opt[1..];
            if let Some(c) = rest.chars().next() {
                match c {
                    'h' => {
                        usage();
                        help();
                        return;
                    }
                    'V' => {
                        eprintln!("{}", murmurhash::MURMURHASH_VERSION);
                        return;
                    }
                    '-' => {
                        let long_opt = &rest[1..];
                        if let Some(stripped) = long_opt.strip_prefix("seed=") {
                            seed = Some(stripped.to_string());
                        } else if long_opt == "seed" {
                            // next arg is the seed value
                            if i + 1 < args.len() {
                                i += 1;
                                seed = Some(args[i].clone());
                            }
                        }
                    }
                    _ => {
                        eprintln!("unknown option: `{}'", rest);
                        usage();
                        std::process::exit(1);
                    }
                }
            }
        }
        i += 1;
    }

    let seed_str = seed.unwrap_or_else(|| "0".to_string());
    let seed_value: u32 = seed_str.parse().unwrap_or(0);

    if std::io::stdin().is_terminal() {
        std::process::exit(1);
    }

    let buf = read_stdin();
    if buf.is_empty() {
        let h = murmurhash::murmurhash(&[], seed_value);
        println!("{}", h);
        return;
    }
    // Match C: hash uses strlen(buf), so exclude trailing null/newline like C strlen would.
    // C reads with fgets which keeps the newline, then strlen counts up to '\0'.
    let h = murmurhash::murmurhash(&buf, seed_value);
    println!("{}", h);

    loop {
        let key = read_stdin();
        if key.is_empty() {
            break;
        }
        let h = murmurhash::murmurhash(&buf, seed_value);
        println!("{}", h);
    }
}
