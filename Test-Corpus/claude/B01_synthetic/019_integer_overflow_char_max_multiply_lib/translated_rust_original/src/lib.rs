// Translation of c_src/src/driver.c to Rust.
// Produces byte-identical stdout for the same inputs.

use std::ffi::c_int;

// On the platforms this is targeting (x86_64 Linux/macOS), `char` is signed and
// CHAR_MAX is 127. Model that exactly.
const CHAR_MAX: i8 = i8::MAX;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn print_hex_char_line(char_hex: i8) {
    // C semantics: printf("%02x\n", charHex)
    // - `charHex` is `char` (signed). It undergoes default argument promotion
    //   to `int` (sign-extending).
    // - `%x` reinterprets the int's bits as `unsigned int` for formatting.
    // So for char_hex = -2, the printed value is 0xFFFFFFFE -> "fffffffe".
    let as_int: c_int = char_hex as c_int;
    let as_uint: u32 = as_int as u32;
    println!("{:02x}", as_uint);
}

fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        // C: char result = data * 2;
        // Integer promotion to int -> multiply -> assign back to char.
        // For data = 127, result is 254 in int, which wraps to -2 in i8.
        let result: i8 = ((data as c_int) * 2) as i8;
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = ((data as c_int) * 2) as i8;
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    let _ = data; // silence unused warnings; matches C dead-store pattern
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = ((data as c_int) * 2) as i8;
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

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
