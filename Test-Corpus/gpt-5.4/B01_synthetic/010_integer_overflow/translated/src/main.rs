use std::io::{self, Read};

fn print_hex_char_line(char_hex: i8) {
    println!("{:02x}", char_hex as u8);
}

fn main() {
    let mut data: i8 = b' ' as i8;
    let mut buffer = [0u8; 1];
    if io::stdin().read_exact(&mut buffer).is_ok() {
        data = buffer[0] as i8;
    }
    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}
