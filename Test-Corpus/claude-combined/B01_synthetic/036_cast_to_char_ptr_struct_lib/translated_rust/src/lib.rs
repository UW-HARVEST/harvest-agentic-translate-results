// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output to the original C.

use std::ffi::c_int;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_hex(p: *const u8, len: c_int) {
    let fmt_byte = b"%02x\0".as_ptr() as *const c_char;
    let fmt_nl = b"\n\0".as_ptr() as *const c_char;
    let mut i: c_int = 0;
    while i < len {
        unsafe {
            let byte = *p.add(i as usize);
            printf(fmt_byte, byte as c_int);
        }
        i += 1;
    }
    unsafe {
        printf(fmt_nl);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut house: HouseT = HouseT {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let p = &house as *const HouseT as *const u8;
    print_hex(p, std::mem::size_of::<HouseT>() as c_int);
}
