pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    s.len()
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
    let mut line = String::new();
    let mut handle = stdin.lock();
    match handle.read_line(&mut line) {
        Ok(0) => Vec::new(),
        Ok(_) => {
            // strip trailing newline to mirror fgets-style line semantics? The C code
            // uses fgets which keeps the newline. We'll keep it to match behavior.
            line.into_bytes()
        }
        Err(_) => Vec::new(),
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
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

    let mut op = Op::NoOp;
    let mut alpha: Option<String> = None;
    let mut key: Option<String> = None;

    // Skip program name
    for arg in args.iter().skip(1) {
        let bytes = arg.as_bytes();
        if bytes.is_empty() || bytes[0] != b'-' {
            continue;
        }
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
                let rest = &arg[2..];
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
                eprintln!("unknown option: `{}'", &arg[1..]);
                usage();
                std::process::exit(1);
            }
        }
    }

    let alpha_str = alpha.unwrap_or_else(|| {
        std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string()
    });

    let mat_owned = tableau::beaufort_tableau(&alpha_str);
    let mat_refs: Vec<&[u8]> = mat_owned.iter().map(|v| v.as_slice()).collect();

    let key = match key {
        Some(k) => k,
        None => {
            eprintln!("error: Expecting cipher key");
            usage();
            std::process::exit(1);
        }
    };

    match op {
        Op::Encrypt => {
            loop {
                let buf = read_stdin();
                if buf.is_empty() {
                    break;
                }
                // Strip trailing newline before encrypting (it's not in alphabet anyway,
                // but keep behavior similar to C).
                let input: &[u8] = if buf.ends_with(b"\n") {
                    &buf[..buf.len() - 1]
                } else {
                    &buf[..]
                };
                let out = encrypt::beaufort_encrypt(input, key.as_bytes(), &mat_refs);
                println!("{}", String::from_utf8_lossy(&out));
            }
        }
        Op::Decrypt => {
            loop {
                let buf = read_stdin();
                if buf.is_empty() {
                    break;
                }
                let input: &[u8] = if buf.ends_with(b"\n") {
                    &buf[..buf.len() - 1]
                } else {
                    &buf[..]
                };
                let out = decrypt::beaufort_decrypt(input, key.as_bytes(), &mat_refs);
                println!("{}", String::from_utf8_lossy(&out));
            }
        }
        Op::NoOp => {
            usage();
            std::process::exit(1);
        }
    }
}
