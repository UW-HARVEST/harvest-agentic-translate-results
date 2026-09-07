// Rust translation of the C library in c_src/.
//
// Public ABI (matches `nm -D` on the C shared library):
//   T driver
//
// Everything else in the C translation unit (`print_hex`, `house_t`) is
// `static`/file-local and therefore stays private here.

use std::ffi::{c_char, c_double, c_int, c_uchar, c_uint};

extern "C" {
    // Use the platform C library's `printf` so that the output goes through
    // exactly the same `stdout` FILE stream (and buffering behaviour) as the
    // original C library. This keeps interleaving with any other C output
    // byte-identical.
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
    bathrooms: c_double,
}

impl HouseT {
    /// Equivalent of `house_t house = {0};` -- the whole object (including any
    /// padding) is zero initialised.
    const fn zeroed() -> Self {
        HouseT {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }
}

/// static void print_hex(unsigned char *p, int len)
fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // `printf("%02x", p[i]);` -- p[i] is promoted to int/unsigned int.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            printf(b"%02x\0".as_ptr() as *const c_char, byte as c_uint);
        }
        i += 1;
    }
    unsafe {
        printf(b"\n\0".as_ptr() as *const c_char);
    }
}

/// void driver(int floors)
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut house: HouseT = HouseT::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    print_hex(
        &house as *const HouseT as *const c_uchar,
        core::mem::size_of::<HouseT>() as c_int,
    );
}
