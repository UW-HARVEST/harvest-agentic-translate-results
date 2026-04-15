use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let mut tokens = input.split_whitespace();

    let operation: i32 = match tokens.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading operation");
            std::process::exit(1);
        }
    };

    let flags: u32 = match tokens.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading flags");
            std::process::exit(1);
        }
    };

    let input_len: usize = match tokens.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading input length");
            std::process::exit(1);
        }
    };

    if input_len > 1024 {
        eprintln!("Error: input length {} exceeds maximum 1024", input_len);
        std::process::exit(1);
    }

    let mut input_buffer = vec![0u8; input_len];
    for i in 0..input_len {
        input_buffer[i] = match tokens.next().and_then(|s| s.parse::<u32>().ok()) {
            Some(v) => v as u8,
            None => {
                eprintln!("Error reading input byte {}", i);
                std::process::exit(1);
            }
        };
    }

    let ref_len: usize = match tokens.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading reference length");
            std::process::exit(1);
        }
    };

    if ref_len > 1024 {
        eprintln!("Error: reference length {} exceeds maximum 1024", ref_len);
        std::process::exit(1);
    }

    let mut ref_buffer = vec![0u8; ref_len];
    for i in 0..ref_len {
        ref_buffer[i] = match tokens.next().and_then(|s| s.parse::<u32>().ok()) {
            Some(v) => v as u8,
            None => {
                eprintln!("Error reading reference byte {}", i);
                std::process::exit(1);
            }
        };
    }

    let result = driver::process_strings(&input_buffer, &ref_buffer, operation, flags);

    println!("{}", result);
}
