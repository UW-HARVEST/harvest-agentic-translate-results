pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    s.bytes().take_while(|&b| b != 0).count()
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

#[derive(PartialEq)]
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

    for arg in args.iter().skip(1) {
        let bytes = arg.as_bytes();
        if bytes.first() == Some(&b'-') {
            let rest = &arg[1..];
            if rest.is_empty() {
                continue;
            }
            let first = rest.as_bytes()[0];
            match first {
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
                    let opt = &rest[1..];
                    if opt == "encrypt" {
                        op = Op::Encrypt;
                    }
                    if opt == "decrypt" {
                        op = Op::Decrypt;
                    }
                    if opt.starts_with("key=") {
                        key = Some(opt[4..].to_string());
                    }
                    if opt.starts_with("alphabet=") {
                        alpha = Some(opt[9..].to_string());
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

    let alpha_str = alpha.unwrap_or_else(|| {
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

    match op {
        Op::Encrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = encrypt::beaufort_encrypt(&buf, key.as_bytes(), &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::Decrypt => loop {
            let buf = read_stdin();
            if buf.is_empty() {
                break;
            }
            let out = decrypt::beaufort_decrypt(&buf, key.as_bytes(), &mat_refs);
            println!("{}", String::from_utf8_lossy(&out));
        },
        Op::NoOp => {
            usage();
            std::process::exit(1);
        }
    }
}
