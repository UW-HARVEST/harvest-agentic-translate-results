use std::io;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    println!("{:02x}", char_hex as i32);
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
        let result = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: i8 = b' ' as i8;
    data = i8::MAX;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result = data.wrapping_mul(2);
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
    let mut x = 0;
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<i32>() {
                x = parsed;
            }
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
