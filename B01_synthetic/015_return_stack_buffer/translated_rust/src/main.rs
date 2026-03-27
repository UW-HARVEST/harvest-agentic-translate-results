use std::io::{self, Read};
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> i32;
}

fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

unsafe fn helper_bad() -> *const c_char {
    let char_string: [u8; 17] = *b"helperBad string\0";
    char_string.as_ptr() as *const c_char
}

fn bad() {
    unsafe {
        let ptr = helper_bad();
        printLine(ptr);
    }
}

fn helper_good1() -> *const c_char {
    static CHAR_STRING: &[u8; 19] = b"helperGood1 string\0";
    CHAR_STRING.as_ptr() as *const c_char
}

fn good() {
    printLine(helper_good1());
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);

    let x: i32 = input
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
