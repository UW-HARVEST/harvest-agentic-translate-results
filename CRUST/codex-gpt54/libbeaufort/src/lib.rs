pub mod decrypt;
pub mod encrypt;
pub mod tableau;

use std::env;
use std::io::{self, BufRead, Write};

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    s.len()
}

pub fn usage() {
    eprintln!("usage: beaufort [-hV] [options]");
}

pub fn help() {
    eprintln!("\noptions:\n");
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
    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(0) | Err(_) => Vec::new(),
        Ok(_) => buf.into_bytes(),
    }
}

pub fn main() {
    enum Op {
        None,
        Encrypt,
        Decrypt,
    }

    let mut op = Op::None;
    let mut alpha: Option<Vec<u8>> = None;
    let mut key: Option<Vec<u8>> = None;

    let mut args = env::args().skip(1);
    if args.len() == 0 {
        usage();
        return;
    }

    for arg in args.by_ref() {
        match arg.as_str() {
            "-h" => {
                usage();
                help();
                return;
            }
            "-V" => {
                eprintln!("{BEAUFORT_VERSION}");
                return;
            }
            "--encrypt" => op = Op::Encrypt,
            "--decrypt" => op = Op::Decrypt,
            _ if arg.starts_with("--key=") => key = Some(arg[6..].as_bytes().to_vec()),
            _ if arg.starts_with("--alphabet=") => alpha = Some(arg[11..].as_bytes().to_vec()),
            _ => {
                eprintln!("unknown option: `{arg}`");
                usage();
                return;
            }
        }
    }

    let Some(key) = key else {
        eprintln!("error: Expecting cipher key");
        usage();
        return;
    };

    let alpha = alpha.unwrap_or_else(|| BEAUFORT_ALPHA.to_vec());
    let alpha_str = match std::str::from_utf8(&alpha) {
        Ok(value) => value,
        Err(_) => return,
    };
    let mat = tableau::beaufort_tableau(alpha_str);
    let mat_refs = mat.iter().map(Vec::as_slice).collect::<Vec<_>>();

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            return;
        };
        let mut bytes = line.into_bytes();
        let output = match op {
            Op::Encrypt => encrypt::beaufort_encrypt(&bytes, &key, &mat_refs),
            Op::Decrypt => decrypt::beaufort_decrypt(&bytes, &key, &mat_refs),
            Op::None => {
                usage();
                return;
            }
        };
        bytes.clear();
        if stdout.write_all(&output).is_err() || stdout.write_all(b"\n").is_err() {
            return;
        }
    }
}
