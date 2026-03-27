use std::io::{self, Read};

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    // C promotes char to int, then %02x prints it as unsigned hex.
    // Negative char sign-extends to 32-bit, e.g. -2 -> 0xfffffffe.
    let as_int = char_hex as i32;
    let as_uint = as_int as u32;
    println!("{:02x}", as_uint);
}

fn bad() {
    let data: i8 = i8::MAX;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let data: i8 = i8::MAX;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
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
    // scanf("%d", &x) - read all stdin, parse first integer
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();

    if x != 0 {
        good();
    } else {
        bad();
    }
}
