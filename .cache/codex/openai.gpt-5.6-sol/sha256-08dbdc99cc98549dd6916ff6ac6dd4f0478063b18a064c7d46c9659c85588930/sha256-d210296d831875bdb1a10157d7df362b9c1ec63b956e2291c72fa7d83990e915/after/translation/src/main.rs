use std::io::{self, Read};

fn print_hex_char_line(value: i8) {
    println!("{:02x}", i32::from(value));
}

fn main() {
    let mut data = [b' '];
    let _ = io::stdin().read(&mut data);

    let result = (data[0] as i8).wrapping_add(1);
    print_hex_char_line(result);
}
