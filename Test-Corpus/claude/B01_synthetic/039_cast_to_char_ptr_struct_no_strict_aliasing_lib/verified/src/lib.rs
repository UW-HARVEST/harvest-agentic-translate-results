// Rust translation of the C library in c_src/ (MIT Lincoln Laboratory `driver`).
//
// Public ABI surface (from `nm -D` on the C libdriver.so):
//   T driver
//
// Behavior must be byte-identical to the C original, including the exact
// stdout bytes produced by printf("%02x") over the raw struct bytes.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    /// libc printf, used so that output interleaves with C stdio exactly like
    /// the original library did (same buffering / same bytes).
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
///
/// On the platforms targeted by the C build this lays out as:
///   floors    @ 0 (4 bytes)
///   bedrooms  @ 4 (4 bytes)
///   bathrooms @ 8 (8 bytes)
/// total size 16, alignment 8 -- no padding bytes.
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

impl HouseT {
    /// Equivalent of `house_t house = {0};` -- all bytes (including any
    /// padding) start out zeroed, matching what the C compiler emits.
    const fn zeroed() -> Self {
        HouseT {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }
}

/// static void print_hex(unsigned char *p, int len)
/// (`unsigned char` == u8 == c_uchar on all supported targets)
fn print_hex(p: &[u8], len: c_int) {
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

/// void driver(int floors)
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut house = HouseT::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.;

    // char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));
    let mut raw = [0u8; core::mem::size_of::<HouseT>()];
    let house_bytes: [u8; core::mem::size_of::<HouseT>()] =
        unsafe { core::mem::transmute::<HouseT, [u8; core::mem::size_of::<HouseT>()]>(house) };
    raw.copy_from_slice(&house_bytes);

    print_hex(&raw, raw.len() as c_int);
}
