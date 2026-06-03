// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    // Match the C `printf("%02x", ...)` output exactly by writing to stdout
    // through libc, then a trailing newline.
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    unsafe {
        libc::fwrite(
            s.as_ptr() as *const libc::c_void,
            1,
            s.len(),
            libc_stdout(),
        );
    }
}

#[cfg(unix)]
fn libc_stdout() -> *mut libc::FILE {
    unsafe extern "C" {
        static stdout: *mut libc::FILE;
    }
    unsafe { stdout }
}

#[cfg(windows)]
fn libc_stdout() -> *mut libc::FILE {
    unsafe { libc::__acrt_iob_func(1) }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // Equivalent to `house_t house = {0};` — zero-initialize all bytes
    // (including any padding) before assigning fields.
    let mut house = HouseT {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };
    // Make sure the entire struct (including any padding) is zeroed,
    // matching `= {0}` in C.
    unsafe {
        std::ptr::write_bytes(&mut house as *mut HouseT as *mut u8, 0, std::mem::size_of::<HouseT>());
    }
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &house as *const HouseT as *const u8,
            std::mem::size_of::<HouseT>(),
        )
    };
    print_hex(bytes);
}
