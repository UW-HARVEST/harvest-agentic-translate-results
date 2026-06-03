pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    // Mirrors the C helper `ssize`, which counts bytes until '\0'.
    // For Rust &str (no embedded NULs), this is just the byte length;
    // if a NUL is present we stop at it to mirror C semantics.
    match s.as_bytes().iter().position(|&b| b == 0) {
        Some(i) => i,
        None => s.len(),
    }
}

pub fn usage() {
    eprintln!("usage: beaufort [-hV] [options]");
}

pub fn help() {
    eprintln!();
    eprintln!("options:");
    eprint!("\n  --encrypt           encrypt stdin stream");
    eprint!("\n  --decrypt           decrypt stdin stream");
    eprint!("\n  --key=[key]         cipher key (required)");
    eprint!(
        "\n  --alphabet=[alpha]  cipher tableau alphabet (Default: '{}')\n",
        std::str::from_utf8(BEAUFORT_ALPHA).unwrap()
    );
    eprintln!();
}

pub fn read_stdin() -> Vec<u8> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();
    match handle.read_line(&mut line) {
        Ok(0) => Vec::new(),
        Ok(_) => line.into_bytes(),
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
    use crate::decrypt::beaufort_decrypt;
    use crate::encrypt::beaufort_encrypt;
    use crate::tableau::beaufort_tableau;

    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        usage();
        std::process::exit(1);
    }

    let mut alpha: Option<String> = None;
    let mut key: Option<String> = None;
    let mut op: Op = Op::NoOp;

    // Skip program name.
    let mut iter = args.iter().skip(1);
    while let Some(opt) = iter.next() {
        if !opt.starts_with('-') {
            continue;
        }
        let rest = &opt[1..];
        if rest.is_empty() {
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
                eprintln!("{}", BEAUFORT_VERSION);
                return;
            }
            '-' => {
                let long = &rest[1..];
                if long == "encrypt" {
                    op = Op::Encrypt;
                }
                if long == "decrypt" {
                    op = Op::Decrypt;
                }
                if let Some(stripped) = long.strip_prefix("key=") {
                    key = Some(stripped.to_string());
                }
                if let Some(stripped) = long.strip_prefix("alphabet=") {
                    alpha = Some(stripped.to_string());
                }
            }
            _ => {
                eprintln!("unknown option: `{}'", rest);
                usage();
                std::process::exit(1);
            }
        }
    }

    let alpha_str = alpha.unwrap_or_else(|| {
        std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string()
    });

    let mat_owned = beaufort_tableau(&alpha_str);
    let mat: Vec<&[u8]> = mat_owned.iter().map(|r| r.as_slice()).collect();

    let key = match key {
        Some(k) => k,
        None => {
            eprintln!("error: Expecting cipher key");
            usage();
            std::process::exit(1);
        }
    };

    match op {
        Op::Encrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = beaufort_encrypt(&buf, key.as_bytes(), &mat);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::Decrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = beaufort_decrypt(&buf, key.as_bytes(), &mat);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::NoOp => {
            usage();
            std::process::exit(1);
        }
    }
}
