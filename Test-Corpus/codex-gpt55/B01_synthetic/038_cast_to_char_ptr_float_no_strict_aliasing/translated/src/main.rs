use std::ffi::{c_char, c_int};
use std::io::{self, Write};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    let mut out = String::with_capacity(bytes.len() * 2 + 1);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    out.push('\n');

    let _ = io::stdout().write_all(out.as_bytes());
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x = 0.0f32;
    unsafe {
        scanf(b"%f\0".as_ptr().cast::<c_char>(), &mut x);
    }
    driver(x);
}
