#![no_main]
use std::io::Read;

fn print_hex_char_line(char_hex: i8) {
    let as_i32 = char_hex as i32;
    let as_u32 = as_i32 as u32;
    println!("{:02x}", as_u32);
}

#[no_mangle]
pub extern "C" fn printHexCharLine(char_hex: i8) {
    print_hex_char_line(char_hex);
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut data: i8 = b' ' as i8;
    let mut buf = [0u8; 1];
    if std::io::stdin().read(&mut buf).unwrap_or(0) > 0 {
        data = buf[0] as i8;
    }
    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(result);
    0
}
