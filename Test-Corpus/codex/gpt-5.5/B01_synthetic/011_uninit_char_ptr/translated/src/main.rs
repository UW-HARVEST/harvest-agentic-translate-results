use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr().cast::<c_char>(), line);
        }
    }
}

fn bad() {
    let data: *const c_char = unsafe { MaybeUninit::uninit().assume_init() };
    print_line(data);
}

fn good() {
    let data = b"string\0".as_ptr().cast::<c_char>();
    print_line(data);
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
