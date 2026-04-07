use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn print_hex_char_line(char_hex: c_char) {
    unsafe {
        // C promotes char to int; replicate sign-extension for %02x
        printf(b"%02x\n\0".as_ptr() as *const c_char, char_hex as c_int);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_char) {
    let result: c_char = data.wrapping_add(1);
    print_hex_char_line(result);
}
