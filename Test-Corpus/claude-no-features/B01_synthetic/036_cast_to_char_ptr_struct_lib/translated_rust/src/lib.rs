// Translation of c_src/src/driver.c to Rust.
//
// The original C code defines a struct `house_t` containing two `int` fields
// followed by a `double` field, then prints the raw bytes of the struct in
// hexadecimal, one byte at a time, terminated by a newline.

use std::ffi::c_int;

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: *const u8, len: c_int) {
    // Reproduce the C `print_hex` exactly: `%02x` for each byte, then `\n`.
    // Use libc::printf to ensure byte-identical output (including stdout
    // buffering semantics) with the original C implementation.
    let fmt_byte = b"%02x\0".as_ptr() as *const i8;
    let fmt_nl = b"\n\0".as_ptr() as *const i8;
    let mut i: c_int = 0;
    while i < len {
        unsafe {
            let byte = *p.offset(i as isize) as c_int;
            libc::printf(fmt_byte, byte);
        }
        i += 1;
    }
    unsafe {
        libc::printf(fmt_nl);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // Zero-initialize the struct (matches `house_t house = {0};` in C).
    let mut house = HouseT {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let p = &house as *const HouseT as *const u8;
    let len = std::mem::size_of::<HouseT>() as c_int;
    print_hex(p, len);
}
