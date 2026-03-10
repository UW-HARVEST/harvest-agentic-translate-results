use std::ffi::{c_char, c_int, CStr};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap());
    }
}

fn print_hex_char_line(char_hex: i8) {
    // C promotes char to int, then %02x prints it as unsigned hex.
    // For a negative signed char, this sign-extends to a full int width.
    let promoted = char_hex as i32;
    print!("{:02x}\n", promoted as u32);
}

fn bad() {
    let data: i8 = i8::MAX; // CHAR_MAX = 127
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
            print_line(
                c"data value is too large to perform arithmetic safely.".as_ptr(),
            );
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
