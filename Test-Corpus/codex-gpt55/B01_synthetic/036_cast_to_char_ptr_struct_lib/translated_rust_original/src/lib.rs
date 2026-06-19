use std::ffi::{c_char, c_int};
use std::mem::size_of;
use std::slice;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(byte));
        }
    }

    unsafe {
        printf(c"\n".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };

    let bytes = unsafe {
        slice::from_raw_parts((&house as *const House).cast::<u8>(), size_of::<House>())
    };

    print_hex(bytes);
}
