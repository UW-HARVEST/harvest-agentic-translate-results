use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut x: c_int = 0;

    // libc owns the precise conversion behavior that the original program uses.
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    let mut output = [b'0'; 9];
    let digits = b"0123456789abcdef";
    for (index, byte) in x.to_ne_bytes().into_iter().enumerate() {
        output[index * 2] = digits[(byte >> 4) as usize];
        output[index * 2 + 1] = digits[(byte & 0x0f) as usize];
    }
    output[8] = b'\n';

    let _ = io::stdout().write_all(&output);
}
