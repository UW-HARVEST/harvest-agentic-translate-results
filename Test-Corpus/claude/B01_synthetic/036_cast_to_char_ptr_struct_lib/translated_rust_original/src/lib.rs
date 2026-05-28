// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;
use std::io::{self, Write};
use std::mem::MaybeUninit;

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let mut stdout = io::stdout();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for byte in p {
        buf.push_str(&format!("{:02x}", byte));
    }
    buf.push('\n');
    let _ = stdout.write_all(buf.as_bytes());
    let _ = stdout.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // Mirror C: house_t house = {0}; — zeroes the entire struct including
    // any padding bytes, then individual fields are assigned.
    let mut house: HouseT = unsafe { MaybeUninit::<HouseT>::zeroed().assume_init() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let len = std::mem::size_of::<HouseT>();
    let p = &house as *const HouseT as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(p, len) };
    print_hex(slice);
}
