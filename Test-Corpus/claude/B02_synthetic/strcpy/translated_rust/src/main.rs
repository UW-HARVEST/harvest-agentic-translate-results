// Translated from C to Rust. Reproduces C behavior including bugs.

use std::io::{self, Read, Write};

mod lib_strings;

const MAX_BUFFER_SIZE: usize = 1024;

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    // Read all stdin and tokenize on whitespace (mimics scanf %d/%u/%zu)
    let mut input_str = String::new();
    if let Err(_) = io::stdin().read_to_string(&mut input_str) {
        // Read error - treat as EOF
    }
    let mut tokens = input_str.split_ascii_whitespace();

    let mut input_buffer: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];
    let mut ref_buffer: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];

    // Read operation (%d -> i32)
    let operation: i32 = match tokens.next().and_then(|t| t.parse::<i32>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading operation");
            return 1;
        }
    };

    // Read flags (%u -> u32)
    let flags: u32 = match tokens.next().and_then(|t| t.parse::<u32>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading flags");
            return 1;
        }
    };

    // Read input_len (%zu -> usize)
    let input_len: usize = match tokens.next().and_then(|t| t.parse::<usize>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading input length");
            return 1;
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            io::stderr(),
            "Error: input length {} exceeds maximum {}",
            input_len,
            MAX_BUFFER_SIZE
        );
        return 1;
    }

    // Read input bytes (%u each)
    for i in 0..input_len {
        let byte: u32 = match tokens.next().and_then(|t| t.parse::<u32>().ok()) {
            Some(v) => v,
            None => {
                let _ = writeln!(io::stderr(), "Error reading input byte {}", i);
                return 1;
            }
        };
        input_buffer[i] = byte as u8;
    }

    // Read ref_len (%zu)
    let ref_len: usize = match tokens.next().and_then(|t| t.parse::<usize>().ok()) {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading reference length");
            return 1;
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            io::stderr(),
            "Error: reference length {} exceeds maximum {}",
            ref_len,
            MAX_BUFFER_SIZE
        );
        return 1;
    }

    // Read reference bytes
    for i in 0..ref_len {
        let byte: u32 = match tokens.next().and_then(|t| t.parse::<u32>().ok()) {
            Some(v) => v,
            None => {
                let _ = writeln!(io::stderr(), "Error reading reference byte {}", i);
                return 1;
            }
        };
        ref_buffer[i] = byte as u8;
    }

    // Call the library function
    let result = lib_strings::process_strings(
        &mut input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    // Print result to stdout
    println!("{}", result);

    0
}
