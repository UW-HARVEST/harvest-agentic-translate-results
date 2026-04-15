use std::io::{self, Read};

fn next_token<'a>(tokens: &mut impl Iterator<Item = &'a str>, err_msg: &str) -> &'a str {
    match tokens.next() {
        Some(s) => s,
        None => {
            eprintln!("{}", err_msg);
            std::process::exit(1);
        }
    }
}

fn parse_token<T: std::str::FromStr>(s: &str, err_msg: &str) -> T {
    match s.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("{}", err_msg);
            std::process::exit(1);
        }
    }
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        std::process::exit(1);
    }
    let mut tokens = input.split_whitespace();

    let s = next_token(&mut tokens, "Error reading flags");
    let flags: u32 = parse_token(s, "Error reading flags");

    let s = next_token(&mut tokens, "Error reading param1");
    let param1: i32 = parse_token(s, "Error reading param1");

    let s = next_token(&mut tokens, "Error reading param2");
    let param2: i32 = parse_token(s, "Error reading param2");

    let s = next_token(&mut tokens, "Error reading length");
    let length: usize = parse_token(s, "Error reading length");

    if length > 256 {
        eprintln!("Error: length {} exceeds maximum 256", length);
        std::process::exit(1);
    }

    let mut buffer = [0u8; 256];
    for i in 0..length {
        let err_msg = format!("Error reading byte {}", i);
        let s = next_token(&mut tokens, &err_msg);
        let byte: u32 = parse_token(s, &err_msg);
        buffer[i] = byte as u8;
    }

    let new_length = driver::process_buffer(&mut buffer, length, flags, param1, param2);

    print!("{}", new_length);
    for i in 0..new_length {
        print!(" {}", buffer[i]);
    }
    println!();
}
