pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    // Mirror the C `ssize` helper: count bytes up to (but not including)
    // a NUL terminator. Rust strings don't include a terminator, but the C
    // implementation effectively returns `strlen`, so we honor that
    // behavior by stopping at the first NUL byte if present.
    let bytes = s.as_bytes();
    let mut size = 0usize;
    while size < bytes.len() && bytes[size] != 0 {
        size += 1;
    }
    size
}

pub fn usage() {
    eprintln!("usage: beaufort [-hV] [options]");
}

pub fn help() {
    eprint!("\noptions:\n");
    eprint!("\n  --encrypt           encrypt stdin stream");
    eprint!("\n  --decrypt           decrypt stdin stream");
    eprint!("\n  --key=[key]         cipher key (required)");
    eprint!(
        "\n  --alphabet=[alpha]  cipher tableau alphabet (Default: '{}')\n",
        std::str::from_utf8(BEAUFORT_ALPHA).unwrap()
    );
    eprint!("\n");
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

#[derive(PartialEq, Eq, Clone, Copy)]
enum Op {
    NoOp,
    Encrypt,
    Decrypt,
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        usage();
        std::process::exit(1);
    }

    let mut alpha: Option<String> = None;
    let mut key: Option<String> = None;
    let mut op: Op = Op::NoOp;

    // Skip program name (args[0]).
    let mut iter = args.iter().skip(1);
    while let Some(opt) = iter.next() {
        let bytes = opt.as_bytes();
        if bytes.is_empty() || bytes[0] != b'-' {
            continue;
        }

        // After consuming the leading '-', look at the next character.
        if bytes.len() < 2 {
            continue;
        }

        match bytes[1] {
            b'h' => {
                usage();
                help();
                return;
            }
            b'V' => {
                eprintln!("{}", BEAUFORT_VERSION);
                return;
            }
            b'-' => {
                // Long option: consume bytes after "--"
                let rest = &opt[2..];
                if rest == "encrypt" {
                    op = Op::Encrypt;
                }
                if rest == "decrypt" {
                    op = Op::Decrypt;
                }
                if let Some(stripped) = rest.strip_prefix("key=") {
                    key = Some(stripped.to_string());
                }
                if let Some(stripped) = rest.strip_prefix("alphabet=") {
                    alpha = Some(stripped.to_string());
                }
            }
            _ => {
                eprintln!("unknown option: `{}'", &opt[1..]);
                usage();
                std::process::exit(1);
            }
        }
    }

    let alpha_str: String = alpha.unwrap_or_else(|| {
        std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string()
    });

    let mat_owned = tableau::beaufort_tableau(&alpha_str);
    let mat_refs: Vec<&[u8]> = mat_owned.iter().map(|r| r.as_slice()).collect();

    let key = match key {
        Some(k) => k,
        None => {
            eprintln!("error: Expecting cipher key");
            usage();
            std::process::exit(1);
        }
    };

    let key_bytes = key.as_bytes();

    let process = |buf: &[u8]| -> Vec<u8> {
        match op {
            Op::Encrypt => encrypt::beaufort_encrypt(buf, key_bytes, &mat_refs),
            Op::Decrypt => decrypt::beaufort_decrypt(buf, key_bytes, &mat_refs),
            Op::NoOp => Vec::new(),
        }
    };

    match op {
        Op::Encrypt | Op::Decrypt => {
            loop {
                let buf = read_stdin();
                if buf.is_empty() {
                    break;
                }
                // Mirror C: trim a single trailing newline before processing
                // because fgets includes it in the buffer, then reprint
                // with our own newline.
                let trimmed = if buf.last() == Some(&b'\n') {
                    &buf[..buf.len() - 1]
                } else {
                    &buf[..]
                };
                let out = process(trimmed);
                // print as a string-like sequence followed by newline
                if let Ok(s) = std::str::from_utf8(&out) {
                    println!("{}", s);
                } else {
                    use std::io::Write;
                    let stdout = std::io::stdout();
                    let mut h = stdout.lock();
                    let _ = h.write_all(&out);
                    let _ = h.write_all(b"\n");
                }
            }
        }
        Op::NoOp => {
            usage();
            std::process::exit(1);
        }
    }
}
