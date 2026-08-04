use std::ffi::{c_char, c_int};

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    for i in 0..p.len() {
        unsafe {
            libc::printf(b"%02x\0".as_ptr() as *const c_char, p[i] as c_int);
        }
    }
    unsafe {
        libc::printf(b"\n\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let house = HouseT {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let len = std::mem::size_of::<HouseT>();
    let p = &house as *const HouseT as *const u8;
    let slice = unsafe { std::slice::from_raw_parts(p, len) };
    print_hex(slice);
}
