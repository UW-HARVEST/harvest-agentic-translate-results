use std::ffi::{c_char, c_int};

const CHAR_MAX: i8 = i8::MAX;

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_hex_char_line(char_hex: i8) {
    println!("{:02x}", char_hex as i32);
}

fn bad() {
    let data = CHAR_MAX;
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
    let data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
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
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast::<c_char>(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
