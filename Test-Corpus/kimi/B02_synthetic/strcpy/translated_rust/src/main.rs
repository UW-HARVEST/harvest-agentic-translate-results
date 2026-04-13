use std::io::{self, BufRead};

const MAX_BUFFER_SIZE: usize = 1024;

fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    let operation: i32 = lines
        .next()
        .and_then(|l| l.ok())
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Error reading operation");
            std::process::exit(1);
        });

    let flags: u32 = lines
        .next()
        .and_then(|l| l.ok())
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Error reading flags");
            std::process::exit(1);
        });

    let input_len: usize = lines
        .next()
        .and_then(|l| l.ok())
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Error reading input length");
            std::process::exit(1);
        });

    if input_len > MAX_BUFFER_SIZE {
        eprintln!("Error: input length {} exceeds maximum {}", input_len, MAX_BUFFER_SIZE);
        std::process::exit(1);
    }

    let mut input_buffer = vec![0u8; input_len];
    for i in 0..input_len {
        let byte: u32 = lines
            .next()
            .and_then(|l| l.ok())
            .and_then(|l| l.trim().parse().ok())
            .unwrap_or_else(|| {
                eprintln!("Error reading input byte {}", i);
                std::process::exit(1);
            });
        input_buffer[i] = byte as u8;
    }

    let ref_len: usize = lines
        .next()
        .and_then(|l| l.ok())
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Error reading reference length");
            std::process::exit(1);
        });

    if ref_len > MAX_BUFFER_SIZE {
        eprintln!("Error: reference length {} exceeds maximum {}", ref_len, MAX_BUFFER_SIZE);
        std::process::exit(1);
    }

    let mut ref_buffer = vec![0u8; ref_len];
    for i in 0..ref_len {
        let byte: u32 = lines
            .next()
            .and_then(|l| l.ok())
            .and_then(|l| l.trim().parse().ok())
            .unwrap_or_else(|| {
                eprintln!("Error reading reference byte {}", i);
                std::process::exit(1);
            });
        ref_buffer[i] = byte as u8;
    }

    let result = driver::process_strings(
        &mut input_buffer,
        input_len,
        Some(&ref_buffer),
        ref_len,
        operation,
        flags,
    );

    println!("{}", result);
}
