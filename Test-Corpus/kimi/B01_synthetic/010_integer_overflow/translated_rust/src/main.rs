use std::io::{self, Read};

fn print_hex_char_line(char_hex: u8) {
    println!("{:02x}", char_hex);
}

fn main() {
    let mut data: u8 = b' ';
    let mut buffer = [0u8; 1];
    if io::stdin().read_exact(&mut buffer).is_ok() {
        data = buffer[0];
    }
    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}