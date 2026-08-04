use std::io;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    println!("{:02x}", char_hex as u8);
}

fn bad() {
    let data: i8 = i8::MAX;
    if data > 0 {
        let result = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result = data * 2;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let data: i8 = i8::MAX;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result = data * 2;
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
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
