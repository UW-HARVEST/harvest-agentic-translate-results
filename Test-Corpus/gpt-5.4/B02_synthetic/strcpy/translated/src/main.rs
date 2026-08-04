use std::io::{self, Read};

use driver::process_strings;

const MAX_BUFFER_SIZE: usize = 1024;

fn main() {
    let mut input_data = String::new();
    if io::stdin().read_to_string(&mut input_data).is_err() {
        eprintln!("Error reading operation");
        std::process::exit(1);
    }

    let mut tokens = input_data.split_whitespace();

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

    if input_len > MAX_BUFFER_SIZE {
        eprintln!(
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut input_buffer = vec![0u8; input_len];
    for (i, byte_ref) in input_buffer.iter_mut().enumerate() {
        let byte: u32 = match tokens.next().and_then(|s| s.parse().ok()) {
            Some(v) => v,
            None => {
                eprintln!("Error reading input byte {}", i);
                std::process::exit(1);
            }
        };
        *byte_ref = byte as u8;
    }

    let ref_len: usize = match tokens.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error reading reference length");
            std::process::exit(1);
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        eprintln!(
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut ref_buffer = vec![0u8; ref_len];
    for (i, byte_ref) in ref_buffer.iter_mut().enumerate() {
        let byte: u32 = match tokens.next().and_then(|s| s.parse().ok()) {
            Some(v) => v,
            None => {
                eprintln!("Error reading reference byte {}", i);
                std::process::exit(1);
            }
        };
        *byte_ref = byte as u8;
    }

    let result = process_strings(
        &mut input_buffer,
        input_len,
        Some(&ref_buffer),
        ref_len,
        operation,
        flags,
    );

    println!("{}", result);
}
