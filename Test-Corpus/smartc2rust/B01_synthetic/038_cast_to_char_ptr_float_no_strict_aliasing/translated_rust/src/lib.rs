
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Read, Write};

fn rust_print_hex(bytes: &[u8]) {
    let mut out = io::stdout().lock();
    for b in bytes {
        let _ = write!(out, "{:02x}", b);
    }
    let _ = writeln!(out);
}

fn rust_driver(x: f32) {
    let raw = x.to_ne_bytes();
    rust_print_hex(&raw);
}

fn rust_read_float_from_stdin() -> f32 {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return 0.0f32;
    }
    for token in buf.split_ascii_whitespace() {
        if let Ok(v) = token.parse::<f32>() {
            return v;
        }
    }
    0.0f32
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    let x = rust_read_float_from_stdin();
    rust_driver(x);
    0
}
