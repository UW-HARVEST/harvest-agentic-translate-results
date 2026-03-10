use std::io::{self, Read};

const CHAR_MAX: u8 = 255;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_hex_char_line(c: u8) {
    println!("{:02x}", c);
}

fn bad() {
    let data: u8 = CHAR_MAX;
    if data > 0 {
        let result: u8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: u8 = 2;
    if data > 0 {
        let result: u8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let data: u8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: u8 = data.wrapping_mul(2);
            print_hex_char_line(result);
        } else {
            print_line("data value is too large to perform arithmetic safely.");
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
