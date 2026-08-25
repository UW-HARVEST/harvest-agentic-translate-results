use std::ffi::{c_char, c_int, c_uchar};

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[c_uchar]) {
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
        std::slice::from_raw_parts(
            (&house as *const House).cast::<c_uchar>(),
            size_of::<House>(),
        )
    };
    print_hex(bytes);
}
