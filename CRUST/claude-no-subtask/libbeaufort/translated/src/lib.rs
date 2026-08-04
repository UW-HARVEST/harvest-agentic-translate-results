pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    // Match C `strlen` semantics: count bytes until first NUL.
    let bytes = s.as_bytes();
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
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
    let mut buf = String::new();
    match handle.read_line(&mut buf) {
        Ok(0) => Vec::new(),
        Ok(_) => buf.into_bytes(),
        Err(_) => Vec::new(),
    }
}

pub fn main() {
    use std::env;

    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        usage();
        std::process::exit(1);
    }

    #[derive(PartialEq)]
    enum Op {
        NoOp,
        Encrypt,
        Decrypt,
    }
    let mut op = Op::NoOp;
    let mut alpha: Option<String> = None;
    let mut key: Option<String> = None;

    // Skip program name (args[0])
    let mut iter = args.iter().skip(1);
    while let Some(opt) = iter.next() {
        if let Some(rest) = opt.strip_prefix('-') {
            if let Some(longopt) = rest.strip_prefix('-') {
                if longopt == "encrypt" {
                    op = Op::Encrypt;
                } else if longopt == "decrypt" {
                    op = Op::Decrypt;
                } else if let Some(k) = longopt.strip_prefix("key=") {
                    key = Some(k.to_string());
                } else if let Some(a) = longopt.strip_prefix("alphabet=") {
                    alpha = Some(a.to_string());
                }
            } else {
                let c = rest.chars().next();
                match c {
                    Some('h') => {
                        usage();
                        help();
                        return;
                    }
                    Some('V') => {
                        eprintln!("{}", BEAUFORT_VERSION);
                        return;
                    }
                    _ => {
                        eprintln!("unknown option: `{}'", rest);
                        usage();
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    let alpha_owned = alpha.unwrap_or_else(|| {
        std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string()
    });
    let mat = tableau::beaufort_tableau(&alpha_owned);

    let key = match key {
        Some(k) => k,
        None => {
            eprintln!("error: Expecting cipher key");
            usage();
            std::process::exit(1);
        }
    };

    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let key_bytes = key.as_bytes();

    match op {
        Op::Encrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = encrypt::beaufort_encrypt(&buf, key_bytes, &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::Decrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = decrypt::beaufort_decrypt(&buf, key_bytes, &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::NoOp => {
            usage();
            std::process::exit(1);
        }
    }
}
