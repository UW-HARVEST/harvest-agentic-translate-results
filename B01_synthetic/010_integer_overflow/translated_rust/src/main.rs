use std::io::Read;

fn print_hex_char_line(char_hex: u8) {
    println!("{:02x}", char_hex);
}

fn main() {
    let mut data: u8 = b' ';
    let mut buf = [0u8; 1];
    if std::io::stdin().read_exact(&mut buf).is_ok() {
        data = buf[0];
    }
    let result: u8 = data.wrapping_add(1);
    print_hex_char_line(result);
}
