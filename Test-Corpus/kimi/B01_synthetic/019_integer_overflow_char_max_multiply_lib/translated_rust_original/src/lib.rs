use std::ffi::c_char;
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

fn print_line(line: Option<&str>) {
    if let Some(l) = line {
        println!("{}", l);
    }
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
            print_line(Some("data value is too large to perform arithmetic safely."));
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}
