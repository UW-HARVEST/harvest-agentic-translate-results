

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Write, BufRead};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct house_t {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn rust_house_to_bytes(house: &house_t) -> [u8; 16] {
    let mut buf = [0u8; 16];
    buf[0..4].copy_from_slice(&house.floors.to_ne_bytes());
    buf[4..8].copy_from_slice(&house.bedrooms.to_ne_bytes());
    buf[8..16].copy_from_slice(&house.bathrooms.to_ne_bytes());
    buf
}

fn rust_print_hex(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    println!("{}", hex);
}

fn rust_driver(floors: i32) {
    let house = house_t {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let bytes = rust_house_to_bytes(&house);
    rust_print_hex(&bytes);
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let stdin = io::stdin();
    let mut line = String::new();
    let x: i32 = match stdin.lock().read_line(&mut line) {
        Ok(_) => line.trim().parse().unwrap_or(0),
        Err(_) => 0,
    };
    rust_driver(x);
    io::stdout().flush().ok();
    0
}

