use std::os::raw::c_int;
use std::slice;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    for byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let raw: &[u8] = unsafe {
        slice::from_raw_parts(
            &house as *const House as *const u8,
            std::mem::size_of::<House>(),
        )
    };
    print_hex(raw);
}