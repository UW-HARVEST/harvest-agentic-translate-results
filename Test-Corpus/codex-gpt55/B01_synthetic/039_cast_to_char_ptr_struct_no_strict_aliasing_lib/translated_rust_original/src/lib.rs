use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            printf(c"%02x".as_ptr(), byte as c_uint);
        }
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut raw = [0_u8; size_of::<House>()];

    let floors_offset = std::mem::offset_of!(House, floors);
    let bedrooms_offset = std::mem::offset_of!(House, bedrooms);
    let bathrooms_offset = std::mem::offset_of!(House, bathrooms);

    raw[floors_offset..(floors_offset + size_of::<c_int>())].copy_from_slice(&floors.to_ne_bytes());
    raw[bedrooms_offset..(bedrooms_offset + size_of::<c_int>())]
        .copy_from_slice(&(3 as c_int).to_ne_bytes());
    raw[bathrooms_offset..(bathrooms_offset + size_of::<f64>())]
        .copy_from_slice(&2.0_f64.to_ne_bytes());

    print_hex(&raw);
}
