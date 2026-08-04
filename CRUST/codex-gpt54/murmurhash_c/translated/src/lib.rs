pub mod murmurhash;

pub fn usage() {
    eprintln!("usage: murmur [-hV] [options]");
}

pub fn help() {
    eprintln!("\noptions:\n");
    eprintln!("  --seed=[seed]  hash seed (optional)");
    eprintln!();
}

pub fn read_stdin() -> Vec<u8> {
    use std::io::BufRead;

    let stdin = std::io::stdin();
    let mut locked = stdin.lock();
    let mut buf = Vec::new();
    match locked.read_until(b'\n', &mut buf) {
        Ok(0) | Err(_) => Vec::new(),
        Ok(_) => buf,
    }
}

pub fn main() {
    use std::io::{self, IsTerminal};

    let mut seed = String::from("0");
    let args = std::env::args().skip(1);

    for arg in args {
        if arg == "-h" {
            usage();
            help();
            return;
        }
        if arg == "-V" {
            eprintln!("{}", murmurhash::MURMURHASH_VERSION);
            return;
        }
        if let Some(value) = arg.strip_prefix("--seed=") {
            seed = value.to_owned();
            continue;
        }
        if arg.starts_with('-') {
            eprintln!("unknown option: `{}`", arg);
            usage();
            std::process::exit(1);
        }
    }

    if io::stdin().is_terminal() {
        std::process::exit(1);
    }

    let seed = seed.parse::<u32>().unwrap_or(0);
    let buf = read_stdin();
    if buf.is_empty() {
        std::process::exit(1);
    }

    let mut stdout = io::stdout().lock();
    loop {
        let hash = murmurhash::murmurhash(&buf, seed);
        use std::io::Write;
        writeln!(stdout, "{hash}").expect("writing to stdout should succeed");

        let next = read_stdin();
        if next.is_empty() {
            break;
        }

        // Match the C program's behavior: it reads the next line but hashes the
        // original buffer again in the loop instead of the newly read one.
    }
}
