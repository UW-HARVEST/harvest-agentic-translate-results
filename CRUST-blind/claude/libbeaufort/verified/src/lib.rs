pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    // Match the C `ssize` semantics: byte length up to (but not including) a
    // terminating NUL. If no NUL is present, return the full byte length.
    let bytes = s.as_bytes();
    match bytes.iter().position(|&b| b == 0) {
        Some(idx) => idx,
        None => bytes.len(),
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
        std::str::from_utf8(BEAUFORT_ALPHA).expect("BEAUFORT_ALPHA is valid UTF-8")
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
        Ok(_) => {
            // Strip trailing newline to match `fgets` + `printf("%s\n", ...)`
            // round-trip semantics used by the C `main`.
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            line.into_bytes()
        }
        Err(_) => Vec::new(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    None,
    Encrypt,
    Decrypt,
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        usage();
        std::process::exit(1);
    }

    let mut op = Op::None;
    let mut alpha: Option<String> = None;
    let mut key: Option<String> = None;

    // Skip program name.
    for arg in args.iter().skip(1) {
        let bytes = arg.as_bytes();
        if bytes.is_empty() || bytes[0] != b'-' {
            continue;
        }

        let rest = &arg[1..];
        let rb = rest.as_bytes();
        if rb.is_empty() {
            continue;
        }

        match rb[0] {
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
                let long = &rest[1..];
                if long == "encrypt" {
                    op = Op::Encrypt;
                }
                if long == "decrypt" {
                    op = Op::Decrypt;
                }
                if long.starts_with("key=") {
                    key = Some(long[4..].to_string());
                }
                if long.starts_with("alphabet=") {
                    alpha = Some(long[9..].to_string());
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
        std::str::from_utf8(BEAUFORT_ALPHA)
            .expect("BEAUFORT_ALPHA is valid UTF-8")
            .to_string()
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
        Op::Encrypt => {
            loop {
                let buf = read_stdin();
                if buf.is_empty() {
                    break;
                }
                let out =
                    encrypt::beaufort_encrypt(&buf, key.as_bytes(), &mat_refs);
                println!("{}", String::from_utf8_lossy(&out));
            }
        }
        Op::Decrypt => {
            loop {
                let buf = read_stdin();
                if buf.is_empty() {
                    break;
                }
                let out =
                    decrypt::beaufort_decrypt(&buf, key.as_bytes(), &mat_refs);
                println!("{}", String::from_utf8_lossy(&out));
            }
        }
        Op::None => {
            usage();
            std::process::exit(1);
        }
    }
}
