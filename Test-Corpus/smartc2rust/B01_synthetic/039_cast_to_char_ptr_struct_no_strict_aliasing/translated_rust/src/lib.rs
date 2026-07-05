

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Read};

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct house_t {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn rust_print_hex(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    println!("{}", hex);
}

fn rust_house_to_bytes(house: &house_t) -> Vec<u8> {
    // Reproduce the C struct memory layout (with padding) in a platform-independent
    // way for typical targets where house_t has size 16 (4 + 4 + 8, naturally aligned).
    let size = std::mem::size_of::<house_t>();
    let mut raw = vec![0u8; size];
    raw[0..4].copy_from_slice(&house.floors.to_ne_bytes());
    raw[4..8].copy_from_slice(&house.bedrooms.to_ne_bytes());
    // bathrooms is 8-byte aligned at offset 8 on all common platforms where
    // sizeof(house_t) == 16.
    if size >= 16 {
        raw[8..16].copy_from_slice(&house.bathrooms.to_ne_bytes());
    }
    raw
}

fn rust_driver(floors: i32) {
    let house = house_t {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let raw = rust_house_to_bytes(&house);
    rust_print_hex(&raw);
}

fn rust_read_int_from_stdin() -> i32 {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return 0;
    }
    // Mimic scanf("%d", ...): skip leading whitespace, then parse optional sign
    // and consecutive digits.
    let mut chars = buf.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
    let mut num_str = String::new();
    if let Some(&c) = chars.peek() {
        if c == '-' || c == '+' {
            num_str.push(c);
            chars.next();
        }
    }
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }
    num_str.parse::<i32>().unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::ffi::c_int {
    let x = rust_read_int_from_stdin();
    rust_driver(x);
    0
}

