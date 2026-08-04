use std::io::{self, BufRead, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: i32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

fn main() {
    let stdin = io::stdin();
    let mut input = String::new();
    // Read all input and parse the first integer (matching scanf("%d", &x) semantics)
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                input.push_str(&l);
                input.push('\n');
                // Try parse the first integer token from accumulated input
                if let Some(tok) = input.split_whitespace().next() {
                    if tok.parse::<i32>().is_ok() {
                        break;
                    }
                }
            }
            Err(_) => break,
        }
    }

    let x: i32 = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    driver(x);
}
