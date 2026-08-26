use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{line}");
    }
}

fn print_hex_char_line(char_hex: i8) {
    println!("{:02x}", (char_hex as i32) as u32);
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

#[allow(unused_assignments)]
fn good_b2g() {
    let mut data: i8 = b' ' as i8;
    data = i8::MAX;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result = data.wrapping_mul(2);
            print_hex_char_line(result);
        } else {
            print_line(Some(
                "data value is too large to perform arithmetic safely.",
            ));
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
        scanf(c"%d".as_ptr(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
