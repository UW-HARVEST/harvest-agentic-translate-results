use std::io::{self, Read};

use driver::process_buffer;

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("Error reading input");
        std::process::exit(1);
    }

    let mut parts = input.split_whitespace();

    let flags: u32 = match parts.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading flags");
            std::process::exit(1);
        }
    };

    let param1: i32 = match parts.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading param1");
            std::process::exit(1);
        }
    };

    let param2: i32 = match parts.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading param2");
            std::process::exit(1);
        }
    };

    let length: usize = match parts.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading length");
            std::process::exit(1);
        }
    };

    if length > 256 {
        eprintln!("Error: length {} exceeds maximum 256", length);
        std::process::exit(1);
    }

    let mut buffer = [0u8; 256];
    for (i, slot) in buffer.iter_mut().take(length).enumerate() {
        let byte: u32 = match parts.next().and_then(|s| s.parse().ok()) {
            Some(v) => v,
            None => {
                eprintln!("Error reading byte {}", i);
                std::process::exit(1);
            }
        };
        *slot = byte as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    print!("{}", new_length);
    for b in buffer.iter().take(new_length) {
        print!(" {}", b);
    }
    println!();
}
