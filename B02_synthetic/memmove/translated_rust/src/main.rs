use std::io::{self, Read};
use std::process;

fn scan_tokens(input: &str) -> Vec<&str> {
    input.split_whitespace().collect()
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let tokens: Vec<&str> = scan_tokens(&input);
    let mut ti = 0;

    macro_rules! next_token {
        ($err:expr) => {{
            if ti >= tokens.len() {
                eprint!("{}\n", $err);
                process::exit(1);
            }
            let t = tokens[ti];
            ti += 1;
            t
        }};
    }

    let flags: u32 = next_token!("Error reading flags")
        .parse()
        .unwrap_or_else(|_| { eprint!("Error reading flags\n"); process::exit(1); });

    let param1: i32 = next_token!("Error reading param1")
        .parse()
        .unwrap_or_else(|_| { eprint!("Error reading param1\n"); process::exit(1); });

    let param2: i32 = next_token!("Error reading param2")
        .parse()
        .unwrap_or_else(|_| { eprint!("Error reading param2\n"); process::exit(1); });

    let length: usize = next_token!("Error reading length")
        .parse()
        .unwrap_or_else(|_| { eprint!("Error reading length\n"); process::exit(1); });

    if length > 256 {
        eprint!("Error: length {} exceeds maximum 256\n", length);
        process::exit(1);
    }

    let mut buffer = [0u8; 256];
    for i in 0..length {
        let byte: u32 = next_token!(format!("Error reading byte {}", i))
            .parse()
            .unwrap_or_else(|_| { eprint!("Error reading byte {}\n", i); process::exit(1); });
        buffer[i] = byte as u8;
    }

    let new_length = driver::process_buffer(&mut buffer, length, flags, param1, param2);

    print!("{}", new_length);
    for i in 0..new_length {
        print!(" {}", buffer[i]);
    }
    println!();
}
