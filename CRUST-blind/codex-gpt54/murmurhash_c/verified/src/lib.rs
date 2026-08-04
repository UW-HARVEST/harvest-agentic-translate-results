pub mod murmurhash;

pub fn usage() {
    eprint!("usage: murmur [-hV] [options]\n");
}

pub fn help() {
    eprint!("\noptions:\n");
    eprint!("\n  --seed=[seed]  hash seed (optional)");
    eprint!("\n");
}

pub fn read_stdin() -> Vec<u8> {
    use std::io::{self, BufRead};

    let mut buf = Vec::new();
    match io::stdin().lock().read_until(b'\n', &mut buf) {
        Ok(0) | Err(_) => Vec::new(),
        Ok(_) => {
            if buf.len() > 1023 {
                buf.truncate(1023);
            }
            buf
        }
    }
}

pub fn main() {
    run_main(std::env::args().skip(1));
}

fn run_main<I>(args: I)
where
    I: IntoIterator<Item = String>,
{
    use std::io::{self, IsTerminal, Write};

    let mut seed: Option<String> = None;

    for arg in args {
        if let Some(stripped) = arg.strip_prefix('-') {
            let mut chars = stripped.chars();
            match chars.next() {
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
                    if let Some(value) = stripped.strip_prefix("-seed=") {
                        seed = Some(value.to_string());
                    }
                }
                _ => {
                    eprintln!("unknown option: `{}`", stripped);
                    usage();
                    return;
                }
            }
        }
    }

    let parsed_seed = seed
        .as_deref()
        .unwrap_or("0")
        .parse::<u32>()
        .unwrap_or(0);

    let stdin = io::stdin();
    if stdin.is_terminal() {
        return;
    }

    let mut reader = stdin.lock();
    let mut stdout = io::stdout().lock();
    let mut buf = Vec::new();

    if read_line_like_c(&mut reader, &mut buf).is_none() {
        return;
    }

    let first_hash = murmurhash::murmurhash(&buf, parsed_seed);
    let _ = writeln!(stdout, "{}", first_hash);

    loop {
        let mut key = Vec::new();
        if read_line_like_c(&mut reader, &mut key).is_none() {
            break;
        }

        let repeated_hash = murmurhash::murmurhash(&buf, parsed_seed);
        let _ = writeln!(stdout, "{}u", repeated_hash);
    }
}

fn read_line_like_c<R: std::io::BufRead>(reader: &mut R, buf: &mut Vec<u8>) -> Option<()> {
    buf.clear();
    match reader.read_until(b'\n', buf) {
        Ok(0) | Err(_) => None,
        Ok(_) => {
            if buf.len() > 1023 {
                buf.truncate(1023);
            }
            Some(())
        }
    }
}
