use std::io::{self, Read};

const MAX_BUFFER_SIZE: usize = 1024;

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let mut tokens = input.split_whitespace();

    macro_rules! next {
        ($err:expr) => {
            match tokens.next() {
                Some(t) => t,
                None => {
                    eprint!("{}", $err);
                    std::process::exit(1);
                }
            }
        };
    }

    macro_rules! parse {
        ($tok:expr, $t:ty, $err:expr) => {
            match $tok.parse::<$t>() {
                Ok(v) => v,
                Err(_) => {
                    eprint!("{}", $err);
                    std::process::exit(1);
                }
            }
        };
    }

    let tok = next!("Error reading operation\n");
    let operation: i32 = parse!(tok, i32, "Error reading operation\n");

    let tok = next!("Error reading flags\n");
    let flags: u32 = parse!(tok, u32, "Error reading flags\n");

    let tok = next!("Error reading input length\n");
    let input_len: usize = parse!(tok, usize, "Error reading input length\n");

    if input_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: input length {} exceeds maximum {}\n",
            input_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut input_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..input_len {
        let tok = next!(format!("Error reading input byte {}\n", i));
        let byte: u32 = parse!(tok, u32, format!("Error reading input byte {}\n", i));
        input_buffer[i] = byte as u8;
    }

    let tok = next!("Error reading reference length\n");
    let ref_len: usize = parse!(tok, usize, "Error reading reference length\n");

    if ref_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: reference length {} exceeds maximum {}\n",
            ref_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..ref_len {
        let tok = next!(format!("Error reading reference byte {}\n", i));
        let byte: u32 = parse!(tok, u32, format!("Error reading reference byte {}\n", i));
        ref_buffer[i] = byte as u8;
    }

    let result = strcpy_fun::process_strings(
        &mut input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    println!("{}", result);
}
