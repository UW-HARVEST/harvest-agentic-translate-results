use std::io::{self, Read};

fn process_buffer(buffer: &mut [u8], length: usize, flags: u32, param1: i32, param2: i32) -> usize;

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut tokens = input.split_whitespace();
    
    let flags: u32 = tokens.next().unwrap().parse().expect("Error reading flags");
    let param1: i32 = tokens.next().unwrap().parse().expect("Error reading param1");
    let param2: i32 = tokens.next().unwrap().parse().expect("Error reading param2");
    let length: usize = tokens.next().unwrap().parse().expect("Error reading length");
    
    if length > 256 {
        eprintln!("Error: length {} exceeds maximum 256", length);
        std::process::exit(1);
    }
    
    let mut buffer = vec![0u8; 256];
    for i in 0..length {
        let byte: u32 = tokens.next().unwrap().parse().expect(&format!("Error reading byte {}", i));
        buffer[i] = byte as u8;
    }
    
    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);
    
    print!("{}", new_length);
    for i in 0..new_length {
        print!(" {}", buffer[i]);
    }
    println!();
}
