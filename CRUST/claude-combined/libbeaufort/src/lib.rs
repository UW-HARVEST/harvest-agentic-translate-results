pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    s.as_bytes().iter().take_while(|&&b| b != 0).count()
}

pub fn usage() {
    eprintln!("usage: beaufort [-hV] [options]");
}

pub fn help() {
    eprint!("\noptions:");
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
    let mut buf = String::new();
    match handle.read_line(&mut buf) {
        Ok(0) => Vec::new(),
        Ok(_) => buf.into_bytes(),
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

    let argv: Vec<String> = std::env::args().collect();
    let argc = argv.len();

    if argc == 1 {
        usage();
        std::process::exit(1);
    }

    let mut op = Op::NoOp;
    let mut key: Option<String> = None;
    let mut alpha: Option<String> = None;

    for opt in argv.iter().skip(1) {
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
                std::process::exit(0);
            }
            b'V' => {
                eprintln!("{}", BEAUFORT_VERSION);
                std::process::exit(0);
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
        .unwrap_or_else(|| std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string());
    let mat = beaufort_tableau(&alpha_str);

    let key = match key {
        Some(k) => k,
        None => {
            eprintln!("error: Expecting cipher key");
            usage();
            std::process::exit(1);
        }
    };

    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();

    match op {
        Op::Encrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = beaufort_encrypt(&buf, key.as_bytes(), &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::Decrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = beaufort_decrypt(&buf, key.as_bytes(), &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::NoOp => {
            usage();
            std::process::exit(1);
        }
    }
}
