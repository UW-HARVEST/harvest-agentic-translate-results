// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;
use std::os::raw::c_uchar;

#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[c_uchar]) {
    unsafe {
        for &b in p.iter() {
            libc::printf(b"%02x\0".as_ptr() as *const i8, b as c_int);
        }
        libc::printf(b"\n\0".as_ptr() as *const i8);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // house_t house = {0};
    let mut house = HouseT {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    // char raw[sizeof(house)];
    // memcpy(raw, &house, sizeof(house));
    let size = std::mem::size_of::<HouseT>();
    let mut raw = vec![0u8; size];
    unsafe {
        std::ptr::copy_nonoverlapping(
            &house as *const HouseT as *const u8,
            raw.as_mut_ptr(),
            size,
        );
    }
    print_hex(&raw);
}
