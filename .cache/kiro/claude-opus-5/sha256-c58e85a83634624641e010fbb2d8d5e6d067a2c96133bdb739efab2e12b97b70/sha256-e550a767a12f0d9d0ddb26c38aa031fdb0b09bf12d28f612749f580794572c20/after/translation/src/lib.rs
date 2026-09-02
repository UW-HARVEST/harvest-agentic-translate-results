// Rust translation of c_src/src/driver.c
//
// Public ABI: `void driver(int x)` (see c_src/include/driver.h).
//
// Behaviour is reproduced exactly, including the use of C `stdio` `printf` so
// that buffering/interleaving with any C caller's own stdio output matches the
// original library byte for byte.

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// C `printf` from libc, used so output goes through the very same FILE
    /// buffer the original library wrote to.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
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
/// `repr(C)` gives the identical layout (offsets 0, 4, 8; size 16, align 8 on
/// the LP64 targets the C library is built for), so the raw byte dump below is
/// the same as the C version's.
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

/// Translation of the file-static `print_hex`. Kept private, exactly like the
/// C `static` function, so it contributes no dynamic symbol.
fn print_hex(p: &[u8]) {
    // `for (int i = 0; i < len; i++) printf("%02x", p[i]);`
    for &b in p {
        unsafe {
            c_printf(c"%02x".as_ptr(), c_int::from(b));
        }
    }
    // `printf("\n");`
    unsafe {
        c_printf(c"\n".as_ptr());
    }
}

/// `void driver(int floors)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // house_t house = {0};
    let mut house = HouseT {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };

    // house.floors = floors;
    house.floors = floors;
    // house.bedrooms = 3;
    house.bedrooms = 3;
    // house.bathrooms = 2.;
    house.bathrooms = 2.0;

    // char raw[sizeof(house)];
    // memcpy(raw, &house, sizeof(house));
    let mut raw = [0u8; core::mem::size_of::<HouseT>()];
    unsafe {
        core::ptr::copy_nonoverlapping(
            (&raw const house).cast::<u8>(),
            raw.as_mut_ptr(),
            core::mem::size_of::<HouseT>(),
        );
    }

    // print_hex((unsigned char *)&raw, sizeof(raw));
    print_hex(&raw);
}
