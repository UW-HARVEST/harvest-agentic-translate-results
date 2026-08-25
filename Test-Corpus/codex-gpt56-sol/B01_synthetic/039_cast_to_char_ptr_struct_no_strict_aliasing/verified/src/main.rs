use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn driver(floors: c_int) {
    let mut raw = [0_u8; 16];
    let bedrooms: c_int = 3;
    raw[..4].copy_from_slice(&floors.to_ne_bytes());
    raw[4..8].copy_from_slice(&bedrooms.to_ne_bytes());
    raw[8..].copy_from_slice(&2_f64.to_ne_bytes());

    let digits = b"0123456789abcdef";
    let mut output = [0_u8; 33];
    for (index, byte) in raw.into_iter().enumerate() {
        output[index * 2] = digits[usize::from(byte >> 4)];
        output[index * 2 + 1] = digits[usize::from(byte & 0x0f)];
    }
    output[32] = b'\n';
    let _ = io::stdout().write_all(&output);
}

fn main() {
    let mut x: c_int = 0;
    unsafe {
        scanf(c"%d".as_ptr(), &raw mut x);
    }
    driver(x);
}
