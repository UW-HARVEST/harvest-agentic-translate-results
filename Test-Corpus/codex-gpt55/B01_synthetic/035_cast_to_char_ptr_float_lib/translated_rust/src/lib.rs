use std::ffi::c_float;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let bytes = x.to_ne_bytes();
    let mut output = [0_u8; 9];

    for (i, byte) in bytes.iter().enumerate() {
        output[i * 2] = HEX[(byte >> 4) as usize];
        output[i * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
    output[8] = b'\n';

    let _ = io::stdout().write_all(&output);
}
