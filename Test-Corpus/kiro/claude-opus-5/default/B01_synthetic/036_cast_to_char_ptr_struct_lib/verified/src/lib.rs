// Rust translation of c_src/src/driver.c
//
// The C library exposes a single function, `driver`, which fills in a
// `house_t` struct and then dumps the struct's raw bytes as lowercase hex.
// Output must be byte-identical, so the struct layout (`#[repr(C)]`) and the
// use of C `stdio` (for identical buffering behaviour) are both preserved.

use std::ffi::{c_char, c_double, c_int, c_uchar};

unsafe extern "C" {
    // Variadic C `printf` from libc; used instead of Rust's `print!` so that
    // stream buffering and flushing behave exactly as in the original.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Mirrors the anonymous struct typedef'd as `house_t` in driver.c.
///
/// On the target ABI this is 16 bytes: `floors` at offset 0, `bedrooms` at
/// offset 4, and `bathrooms` (8-byte aligned) at offset 8.
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Kept as a private helper (it has internal linkage in C, so it is not part
/// of the exported ABI).
fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // "%02x" promotes the unsigned char argument to int.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            printf(c"%02x".as_ptr(), byte as c_int);
        }
        i += 1;
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}

/// `void driver(int floors)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // `house_t house = {0};` zero-initialises every byte, including any
    // padding, before the individual fields are assigned.
    let mut house: HouseT = unsafe { std::mem::zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.;

    print_hex(
        (&raw const house) as *const HouseT as *const c_uchar,
        std::mem::size_of::<HouseT>() as c_int,
    );
}
