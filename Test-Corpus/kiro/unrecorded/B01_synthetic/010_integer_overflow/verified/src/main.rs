use std::io::Read;

fn print_hex_char_line(char_hex: i8) {
    let promoted = char_hex as i32 as u32;
    println!("{:02x}", promoted);
}

fn main() {
    let mut data: i8 = b' ' as i8;
    let mut buf = [0u8; 1];
    if std::io::stdin().read(&mut buf).unwrap_or(0) > 0 {
        data = buf[0] as i8;
    }
    let result: i8 = data.wrapping_add(1);
    print_hex_char_line(result);
}
