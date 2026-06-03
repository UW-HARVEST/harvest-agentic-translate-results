// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces the byte-identical output of the
// original `driver` C library.

use std::ffi::c_char;
use std::ffi::c_int;
use std::mem::MaybeUninit;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// Mirror of the C `house_t` struct. `#[repr(C)]` ensures the same layout,
// including any padding the C compiler would insert.
#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    // Format string: "%02x" for each byte then a trailing newline.
    let fmt_byte = b"%02x\0".as_ptr() as *const c_char;
    let fmt_nl = b"\n\0".as_ptr() as *const c_char;
    for &b in p {
        unsafe {
            printf(fmt_byte, b as c_int);
        }
    }
    unsafe {
        printf(fmt_nl);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // `house_t house = {0};` zero-initializes the entire struct (including any
    // padding bytes). Use `MaybeUninit::zeroed` to preserve that behavior so
    // the subsequent memcpy reproduces the exact byte pattern.
    let mut house_uninit: MaybeUninit<HouseT> = MaybeUninit::zeroed();
    unsafe {
        // Set fields through the raw pointer so we don't disturb padding bytes.
        let p = house_uninit.as_mut_ptr();
        (*p).floors = floors;
        (*p).bedrooms = 3;
        (*p).bathrooms = 2.0;
    }

    // Equivalent of `char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));`
    let size = std::mem::size_of::<HouseT>();
    let mut raw: Vec<u8> = vec![0u8; size];
    unsafe {
        std::ptr::copy_nonoverlapping(
            house_uninit.as_ptr() as *const u8,
            raw.as_mut_ptr(),
            size,
        );
    }

    print_hex(&raw);
}
