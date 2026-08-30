use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut x = 0.0_f32;

    // Use the C conversion directly so locale, accepted spellings, overflow,
    // and failed-conversion behavior match scanf("%f", &x).
    unsafe {
        scanf(c"%f".as_ptr(), &mut x);
    }

    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = [0_u8; 9];
    for (index, byte) in x.to_ne_bytes().into_iter().enumerate() {
        output[index * 2] = HEX[(byte >> 4) as usize];
        output[index * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
    output[8] = b'\n';

    io::stdout().write_all(&output).unwrap();
}
