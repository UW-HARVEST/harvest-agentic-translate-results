use std::io::{Read, Write};

fn print_hex_char_line(char_hex: i8) {
    let value = i32::from(char_hex);
    let _ = writeln!(std::io::stdout(), "{value:02x}");
}

fn main() {
    let mut data = b' ' as i8;
    let mut buf = [0_u8; 1];

    if let Ok(1) = std::io::stdin().read(&mut buf) {
        data = buf[0] as i8;
    }

    let result = data.wrapping_add(1);
    print_hex_char_line(result);
}
