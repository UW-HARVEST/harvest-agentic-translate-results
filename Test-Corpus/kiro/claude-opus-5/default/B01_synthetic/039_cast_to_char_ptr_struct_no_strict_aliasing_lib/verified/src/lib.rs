// Rust translation of c_src/src/driver.c
//
// Original copyright 2025 MIT Lincoln Laboratory (MIT-style license, see c_src).
//
// Behavior must be byte-identical to the C original: the raw in-memory bytes of
// a `house_t` value are printed as lowercase two-digit hex, followed by a
// newline. Output goes through libc's `printf` so that it shares the C stdout
// stream/buffering with any C code in the same process.

use std::ffi::{c_char, c_double, c_int, c_uchar};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Mirror of the C `house_t`:
///
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// `repr(C)` gives the same field order, offsets, alignment and total size
/// (including any tail/interior padding) as the C compiler produces.
#[repr(C)]
#[derive(Copy, Clone)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// Translation of the C `static void print_hex(unsigned char *p, int len)`.
///
/// Kept private, matching the `static` linkage of the original.
fn print_hex(p: &[c_uchar], len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(p[i as usize]));
        }
        i += 1;
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}

/// Translation of the C `void driver(int floors)`.
///
/// The header `driver.h` declares plain `driver` with no namespace/rename
/// macros, so the exported linker symbol is `driver`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // house_t house = {0};  -> every byte, including padding, starts as zero.
    let mut house: House = unsafe { std::mem::zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.;

    // char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));
    let mut raw = [0u8; std::mem::size_of::<House>()];
    let src = unsafe {
        std::slice::from_raw_parts(
            (&raw const house).cast::<u8>(),
            std::mem::size_of::<House>(),
        )
    };
    raw.copy_from_slice(src);

    print_hex(&raw, std::mem::size_of::<House>() as c_int);
}
