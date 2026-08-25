use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
}

const INTEGER_FORMAT: &[u8] = b"%d\0";
const EMPTY_STRING: &[u8] = b"\0";
const STRING: &[u8] = b"string\0";

fn print_line(line: *const c_char) {
    if !line.is_null() {
        // SAFETY: Callers provide a pointer to a static, NUL-terminated byte string.
        unsafe {
            puts(line);
        }
    }
}

fn bad() {
    // The reference build's indeterminate pointer aliases an empty C string.
    let data = EMPTY_STRING.as_ptr().cast();
    print_line(data);
}

fn good() {
    let data = STRING.as_ptr().cast();
    print_line(data);
}

fn main() {
    let mut x: c_int = 0;

    // SAFETY: The format expects one int pointer, and x is a writable c_int.
    unsafe {
        scanf(INTEGER_FORMAT.as_ptr().cast(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
