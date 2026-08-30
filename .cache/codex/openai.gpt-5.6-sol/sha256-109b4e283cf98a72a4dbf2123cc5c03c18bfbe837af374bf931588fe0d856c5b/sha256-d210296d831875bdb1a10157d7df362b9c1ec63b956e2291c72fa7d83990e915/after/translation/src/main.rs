use std::ffi::{c_char, c_int};
use std::io::{self, Write};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut x = 0.0_f32;

    // SAFETY: The format is NUL-terminated and %f expects a valid float pointer.
    unsafe {
        scanf(c"%f".as_ptr(), &mut x);
    }

    let mut output = [0_u8; 9];
    for (byte, chunk) in x.to_ne_bytes().into_iter().zip(output.chunks_exact_mut(2)) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        chunk[0] = HEX[(byte >> 4) as usize];
        chunk[1] = HEX[(byte & 0x0f) as usize];
    }
    output[8] = b'\n';

    let _ = io::stdout().write_all(&output);
}
