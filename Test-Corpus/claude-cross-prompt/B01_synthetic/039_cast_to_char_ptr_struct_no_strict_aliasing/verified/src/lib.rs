// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Rust translation of c_src/src/main.c

use std::ffi::c_int;
use std::io::{self, Write};

#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        use std::fmt::Write as _;
        let _ = write!(&mut buf, "{:02x}", b);
    }
    buf.push('\n');
    let _ = handle.write_all(buf.as_bytes());
    let _ = handle.flush();
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

    // Copy struct bytes into raw[sizeof(house)] then hex-print.
    let size = std::mem::size_of::<HouseT>();
    let mut raw = vec![0u8; size];
    unsafe {
        std::ptr::copy_nonoverlapping(
            (&house as *const HouseT) as *const u8,
            raw.as_mut_ptr(),
            size,
        );
    }
    print_hex(&raw);
}
