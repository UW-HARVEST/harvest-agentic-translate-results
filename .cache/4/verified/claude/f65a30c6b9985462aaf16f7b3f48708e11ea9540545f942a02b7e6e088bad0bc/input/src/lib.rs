// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// Public ABI of the C shared library (from `nm -D libdriver.so`):
//     driver
//
// The translation deliberately reproduces the original behaviour byte for
// byte, including dumping the raw in-memory representation of the `house_t`
// struct (which is platform/ABI dependent) and writing through C `stdio` so
// that buffering and interleaving with any C code in the same process is
// preserved exactly.

use std::os::raw::{c_char, c_int, c_uchar};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

/// static void print_hex(unsigned char *p, int len)
fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]);
        unsafe {
            printf(
                b"%02x\0".as_ptr() as *const c_char,
                *p.offset(i as isize) as c_int,
            );
        }
        i += 1;
    }
    // printf("\n");
    unsafe {
        printf(b"\n\0".as_ptr() as *const c_char);
    }
}

/// void driver(int floors)
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // house_t house = {0};
    let mut house: HouseT = HouseT {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };
    // Ensure any ABI padding bytes are zeroed, matching `= {0}` as emitted by
    // the C compiler for this aggregate initialization.
    unsafe {
        std::ptr::write_bytes(&mut house as *mut HouseT as *mut u8, 0, size_of::<HouseT>());
    }

    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.;

    // print_hex((unsigned char *)&house, sizeof(house));
    print_hex(
        &house as *const HouseT as *const c_uchar,
        size_of::<HouseT>() as c_int,
    );
}
