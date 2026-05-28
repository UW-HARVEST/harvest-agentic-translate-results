// Translation of c_src/src/driver.c to Rust.
// Produces byte-identical stdout for the same inputs.

use std::ffi::{c_char, c_int, CStr};

// On the platforms this is targeting (x86_64 Linux/macOS), `char` is signed and
// CHAR_MAX is 127. Model that exactly.
const CHAR_MAX: i8 = i8::MAX;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Match C's printf("%s\n", line). The C string is read until NUL.
        let cstr = unsafe { CStr::from_ptr(line) };
        // Use stdout write of bytes + newline to mirror printf exactly.
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(cstr.to_bytes());
        let _ = handle.write_all(b"\n");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(char_hex: c_char) {
    // C semantics: printf("%02x\n", charHex)
    // - `charHex` is `char` (signed on x86_64). It undergoes default argument
    //   promotion to `int` (sign-extending).
    // - `%x` reinterprets the int's bits as `unsigned int` for formatting.
    // So for char_hex = -2, the printed value is 0xFFFFFFFE -> "fffffffe".
    let signed: i8 = char_hex as i8;
    let as_int: c_int = signed as c_int;
    let as_uint: u32 = as_int as u32;
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{:02x}\n", as_uint);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        // C: char result = data * 2;
        // Integer promotion to int -> multiply -> assign back to char.
        // For data = 127, result is 254 in int, which wraps to -2 in i8.
        let result: i8 = ((data as c_int) * 2) as i8;
        printHexCharLine(result as c_char);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = ((data as c_int) * 2) as i8;
        printHexCharLine(result as c_char);
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
            printHexCharLine(result as c_char);
        } else {
            let msg = b"data value is too large to perform arithmetic safely.\0";
            unsafe { printLine(msg.as_ptr() as *const c_char) };
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
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
