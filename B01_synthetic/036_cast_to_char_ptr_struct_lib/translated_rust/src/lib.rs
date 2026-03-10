use std::ffi::c_int;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: *const u8, len: usize) {
    for i in 0..len {
        let byte = unsafe { *p.add(i) };
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
    let ptr = &house as *const House as *const u8;
    print_hex(ptr, std::mem::size_of::<House>());
}
