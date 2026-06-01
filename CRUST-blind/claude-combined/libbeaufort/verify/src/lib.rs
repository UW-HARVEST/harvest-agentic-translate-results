pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    // Replicate the C `ssize` helper: counts bytes up to (but not including)
    // the first '\0' byte. For Rust strings (which can contain '\0'), we stop
    // at the first NUL byte if present, or return the full length otherwise.
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
    let mut line = String::new();
    let n = stdin.lock().read_line(&mut line).unwrap_or(0);
    if n == 0 {
        return Vec::new();
    }
    line.into_bytes()
}

pub fn main() {
    // Parse command line arguments and execute the encrypt/decrypt operation.
    #[derive(PartialEq)]
    enum Op {
        None,
        Encrypt,
        Decrypt,
    }

    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        usage();
        std::process::exit(1);
    }

    let mut op = Op::None;
    let mut key: Option<String> = None;
    let mut alpha: Option<String> = None;

    for arg in args.iter().skip(1) {
        let bytes = arg.as_bytes();
        if bytes.is_empty() || bytes[0] != b'-' {
            continue;
        }
        // After consuming first '-':
        if bytes.len() >= 2 {
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
    }

    let alpha_string = alpha.unwrap_or_else(|| std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string());
    let mat = tableau::beaufort_tableau(&alpha_string);
    let mat_refs: Vec<&[u8]> = mat.iter().map(|row| row.as_slice()).collect();

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
        Op::None => {
            usage();
            std::process::exit(1);
        }
    }
}
