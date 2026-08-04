use std::ffi::c_int;
use std::io::{self, Write};
use std::mem;
use std::slice;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for &byte in bytes {
        let _ = write!(stdout, "{byte:02x}");
    }
    let _ = stdout.write_all(b"\n");
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut house: House = unsafe { mem::zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let raw = unsafe {
        slice::from_raw_parts(
            (&raw const house).cast::<u8>(),
            mem::size_of::<House>(),
        )
    };

    print_hex(raw);
}
