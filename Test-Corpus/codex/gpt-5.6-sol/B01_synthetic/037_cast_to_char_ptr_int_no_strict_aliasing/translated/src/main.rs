use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = [0_u8; 2 * size_of::<c_int>() + 1];
    for (index, byte) in bytes.iter().copied().enumerate() {
        output[2 * index] = HEX[usize::from(byte >> 4)];
        output[2 * index + 1] = HEX[usize::from(byte & 0x0f)];
    }
    output[output.len() - 1] = b'\n';

    let _ = io::stdout().write_all(&output);
}

fn driver(x: c_int) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x: c_int = 0;
    let format = b"%d\0";

    // The C parser is retained so scanf's matching and overflow behavior is unchanged.
    unsafe {
        scanf(format.as_ptr().cast(), &mut x as *mut c_int);
    }
    driver(x);
}
