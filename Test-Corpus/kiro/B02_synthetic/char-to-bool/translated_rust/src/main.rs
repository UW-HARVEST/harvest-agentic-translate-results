use std::io::{self, BufRead};

fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    let line = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            eprint!("Error reading operation\n");
            std::process::exit(1);
        }
    };
    let operation: i32 = line.trim().parse().unwrap_or(0);

    let line = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            eprint!("Error reading parameter\n");
            std::process::exit(1);
        }
    };
    let param: i32 = line.trim().parse().unwrap_or(0);

    let line = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            eprint!("Error reading decision string\n");
            std::process::exit(1);
        }
    };

    let mut buf: Vec<u8> = line.into_bytes();
    let len = buf.len();

    let result = driver::process_decisions(&mut buf, len, operation, param);
    println!("{}", result);
}
