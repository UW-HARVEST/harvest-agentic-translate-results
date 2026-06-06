pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
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
    eprintln!();
    eprintln!("options:");
    eprintln!();
    eprintln!("  --encrypt           encrypt stdin stream");
    eprintln!("  --decrypt           decrypt stdin stream");
    eprintln!("  --key=[key]         cipher key (required)");
    eprintln!(
        "  --alphabet=[alpha]  cipher tableau alphabet (Default: '{}')",
        std::str::from_utf8(BEAUFORT_ALPHA).unwrap()
    );
    eprintln!();
}

pub fn read_stdin() -> Vec<u8> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut line = String::new();
    let mut handle = stdin.lock();
    match handle.read_line(&mut line) {
        Ok(0) => Vec::new(),
        Ok(_) => line.into_bytes(),
        Err(_) => Vec::new(),
    }
}

pub fn main() {
    use crate::decrypt::beaufort_decrypt;
    use crate::encrypt::beaufort_encrypt;
    use crate::tableau::beaufort_tableau;

    #[derive(PartialEq)]
    enum Op {
        NoOp,
        Encrypt,
        Decrypt,
    }

    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        usage();
        std::process::exit(1);
    }

    let mut op = Op::NoOp;
    let mut key: Option<String> = None;
    let mut alpha: Option<String> = None;

    for opt in args.iter().skip(1) {
        let bytes = opt.as_bytes();
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

    let alpha_str = alpha
        .clone()
        .unwrap_or_else(|| std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string());
    let mat_owned = beaufort_tableau(&alpha_str);
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

    match op {
        Op::Encrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = beaufort_encrypt(&buf, key_bytes, &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::Decrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = beaufort_decrypt(&buf, key_bytes, &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::NoOp => {
            usage();
            std::process::exit(1);
        }
    }
}
