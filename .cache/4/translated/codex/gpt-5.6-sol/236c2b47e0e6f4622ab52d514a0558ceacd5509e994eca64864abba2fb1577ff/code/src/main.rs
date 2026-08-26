use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn c_scanf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = [0_u8; 9];

    for (index, byte) in bytes.iter().copied().enumerate() {
        output[index * 2] = HEX[usize::from(byte >> 4)];
        output[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
    }
    output[8] = b'\n';

    let _ = io::stdout().lock().write_all(&output);
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x = 0.0_f32;

    // Match the C implementation's scanf conversion and leave x unchanged on failure.
    unsafe {
        c_scanf(c"%f".as_ptr(), &mut x);
    }
    driver(x);
}
