use std::ffi::{c_char, c_int, CStr};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap());
    }
}

fn print_hex_char_line(char_hex: c_char) {
    // C promotes char to int, then %02x prints it as unsigned hex of that int.
    // For negative char values, sign-extension to i32 then cast to u32 gives e.g. 0xfffffffe.
    let as_int = char_hex as i32;
    println!("{:02x}", as_int as u32);
}

fn bad() {
    let data: i8 = i8::MAX; // CHAR_MAX = 127
    if data > 0 {
        // C: integer promotion makes 127*2=254 as int, then truncated to char = -2
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(result as c_char);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = (data as i32 * 2) as i8;
        print_hex_char_line(result as c_char);
    }
}

fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = i8::MAX; // CHAR_MAX
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result: i8 = (data as i32 * 2) as i8;
            print_hex_char_line(result as c_char);
        } else {
            print_line(
                b"data value is too large to perform arithmetic safely.\0".as_ptr() as *const c_char,
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
