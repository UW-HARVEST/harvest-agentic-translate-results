// Rust translation of c_src/src/driver.c
//
// Original: Copyright 2025 MIT Lincoln Laboratory (MIT-style license, see c_src).
//
// The C code dumps the raw in-memory representation of a `house_t` struct as
// lowercase hex. To be byte-identical we must reproduce the C ABI layout
// (`#[repr(C)]`), the zero-initialization of `= {0}` (which in practice also
// clears padding bytes), and the exact `printf("%02x")` / trailing newline
// output.

use std::ffi::c_int;
use std::io::Write;
use std::mem::MaybeUninit;

/// typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

/// static void print_hex(unsigned char *p, int len)
fn print_hex(p: &[u8], len: c_int) {
    // Build the output in one buffer so the write to stdout is atomic-ish and
    // the bytes emitted match printf's exactly.
    let mut out = Vec::with_capacity(len as usize * 2 + 1);
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]);
        let _ = write!(out, "{:02x}", p[i as usize]);
        i += 1;
    }
    // printf("\n");
    out.push(b'\n');

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(&out);
    let _ = lock.flush();
}

/// void driver(int floors)
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // house_t house = {0};  -- zero every byte, padding included.
    let mut house: MaybeUninit<HouseT> = MaybeUninit::zeroed();

    // Write the fields through the pointer so padding stays zeroed.
    let ptr = house.as_mut_ptr();
    unsafe {
        (*ptr).floors = floors; // house.floors = floors;
        (*ptr).bedrooms = 3; // house.bedrooms = 3;
        (*ptr).bathrooms = 2.0; // house.bathrooms = 2.;
    }

    // print_hex((unsigned char *)&house, sizeof(house));
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            house.as_ptr() as *const u8,
            std::mem::size_of::<HouseT>(),
        )
    };
    print_hex(bytes, std::mem::size_of::<HouseT>() as c_int);
}
