use std::io::{self, Read};

fn print_hex_char_line(char_hex: i8) {
    let promoted = char_hex as i32 as u32;
    println!("{:02x}", promoted);
}

fn main() {
    let mut data = b' ' as i8;
    let mut byte = [0_u8; 1];

    if io::stdin().read(&mut byte).unwrap_or(0) == 1 {
        data = byte[0] as i8;
    }

    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}
