// Translation of c_src/src/driver.c to Rust.
// Reproduces byte-identical output with the original C library.

use std::ffi::{c_char, c_int, c_uint};
use std::io::Write;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        // Mimic: printf("%s\n", line);
        // Read bytes up to NUL terminator.
        unsafe {
            let bytes = std::ffi::CStr::from_ptr(line).to_bytes();
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            let _ = handle.write_all(bytes);
            let _ = handle.write_all(b"\n");
        }
    }
}

fn print_hex_char_line(char_hex: c_char) {
    // Mimic: printf("%02x\n", charHex);
    // In C, a `char` argument to a variadic function is promoted to `int`
    // (sign-extension if `char` is signed). The %x conversion then
    // reinterprets that int as unsigned int.
    let promoted: c_int = char_hex as c_int;
    let as_uint: c_uint = promoted as c_uint;
    println!("{:02x}", as_uint);
}

fn bad() {
    let data: c_char;
    data = c_char::MAX;
    if data > 0 {
        // In C, `data * 2` promotes to int, multiplies, then assignment
        // to `char` truncates back. On the targets we care about, this
        // is equivalent to wrapping multiplication at the c_char width.
        let result: c_char = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: c_char;
    data = 2;
    if data > 0 {
        let result: c_char = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: c_char;
    // Preserve the C source exactly: `data = ' ';` is dead-stored before
    // being overwritten with CHAR_MAX. Do not "fix" this.
    #[allow(unused_assignments)]
    {
        data = b' ' as c_char;
    }
    data = c_char::MAX;
    if data > 0 {
        if data < (c_char::MAX / 2) {
            let result: c_char = data.wrapping_mul(2);
            print_hex_char_line(result);
        } else {
            let msg = b"data value is too large to perform arithmetic safely.\0";
            print_line(msg.as_ptr() as *const c_char);
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
