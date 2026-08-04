use std::io::{self, Read};

fn print_hex_char_line(char_hex: u8) {
    println!("{:02x}", char_hex);
}

fn main() {
    let mut data: u8 = b' ';
    let mut buffer = [0; 1];
    if let Ok(1) = io::stdin().read(&mut buffer) {
        data = buffer[0];
    }
    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}
