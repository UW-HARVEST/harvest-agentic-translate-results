use std::mem::size_of;
use std::os::raw::c_int;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    for b in bytes {
        print!("{:02x}", b);
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
    let bytes = unsafe {
        std::slice::from_raw_parts((&house as *const House).cast::<u8>(), size_of::<House>())
    };
    print_hex(bytes);
}
