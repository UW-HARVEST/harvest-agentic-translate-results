use std::ffi::c_int;
use std::io::{self, Write};
use std::mem::{MaybeUninit, size_of};
use std::slice;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    for &byte in bytes {
        let _ = write!(stdout, "{byte:02x}");
    }
    let _ = stdout.write_all(b"\n");
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut house = MaybeUninit::<House>::zeroed();
    let house_ptr = house.as_mut_ptr();

    unsafe {
        (*house_ptr).floors = floors;
        (*house_ptr).bedrooms = 3;
        (*house_ptr).bathrooms = 2.0;

        let bytes = slice::from_raw_parts(house_ptr.cast::<u8>(), size_of::<House>());
        print_hex(bytes);
    }
}
