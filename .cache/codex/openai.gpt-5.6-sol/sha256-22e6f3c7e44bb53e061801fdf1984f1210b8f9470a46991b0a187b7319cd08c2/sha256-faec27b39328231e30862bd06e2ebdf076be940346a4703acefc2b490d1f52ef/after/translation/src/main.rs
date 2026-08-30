use std::ffi::{c_char, c_int};
use std::io::{self, Write};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn c_scanf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut floors: c_int = 0;

    // Using libc here preserves scanf's exact tokenization and conversion behavior.
    unsafe {
        c_scanf(b"%d\0".as_ptr().cast(), &raw mut floors);
    }

    let bedrooms: c_int = 3;
    let bathrooms = 2.0_f64;
    let mut house = Vec::with_capacity(16);
    house.extend_from_slice(&floors.to_ne_bytes());
    house.extend_from_slice(&bedrooms.to_ne_bytes());
    house.extend_from_slice(&bathrooms.to_ne_bytes());

    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = Vec::with_capacity(house.len() * 2 + 1);
    for byte in house {
        output.push(HEX[(byte >> 4) as usize]);
        output.push(HEX[(byte & 0x0f) as usize]);
    }
    output.push(b'\n');

    let _ = io::stdout().lock().write_all(&output);
}
