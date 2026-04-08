pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    s.len()
}

pub fn usage() {
    eprint!("usage: beaufort [-hV] [options]\n");
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
    let mut line = String::new();
    match std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut line) {
        Ok(0) => Vec::new(),
        Ok(_) => line.into_bytes(),
        Err(_) => Vec::new(),
    }
}

pub fn main() {
    use std::io::IsTerminal;

    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        usage();
        std::process::exit(1);
    }

    const NO_OP: u8 = 0;
    const ENCRYPT_OP: u8 = 1;
    const DECRYPT_OP: u8 = 2;

    let mut op = NO_OP;
    let mut key: Option<String> = None;
    let mut alpha: Option<String> = None;

    for arg in &args[1..] {
        if arg.starts_with('-') {
            let rest = &arg[1..];
            if rest.starts_with('-') {
                // long option
                let opt = &rest[1..];
                if opt == "encrypt" { op = ENCRYPT_OP; }
                else if opt == "decrypt" { op = DECRYPT_OP; }
                else if let Some(k) = opt.strip_prefix("key=") { key = Some(k.to_string()); }
                else if let Some(a) = opt.strip_prefix("alphabet=") { alpha = Some(a.to_string()); }
            } else {
                match rest.chars().next() {
                    Some('h') => { usage(); help(); std::process::exit(0); }
                    Some('V') => { eprintln!("{}", BEAUFORT_VERSION); std::process::exit(0); }
                    _ => {
                        eprintln!("unknown option: `{}`", rest);
                        usage();
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    let alpha_str = alpha.unwrap_or_else(|| std::str::from_utf8(BEAUFORT_ALPHA).unwrap().to_string());
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

    match op {
        ENCRYPT_OP => {
            if std::io::stdin().is_terminal() { std::process::exit(1); }
            loop {
                let buf = read_stdin();
                if buf.is_empty() { break; }
                let out = encrypt::beaufort_encrypt(&buf, key_bytes, &mat_refs);
                println!("{}", String::from_utf8_lossy(&out));
            }
        }
        DECRYPT_OP => {
            if std::io::stdin().is_terminal() { std::process::exit(1); }
            loop {
                let buf = read_stdin();
                if buf.is_empty() { break; }
                let out = decrypt::beaufort_decrypt(&buf, key_bytes, &mat_refs);
                println!("{}", String::from_utf8_lossy(&out));
            }
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
