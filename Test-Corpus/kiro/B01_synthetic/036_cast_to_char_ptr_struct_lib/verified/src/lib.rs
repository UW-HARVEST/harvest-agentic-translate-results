use std::ffi::c_int;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: *const u8, len: usize) {
    for i in 0..len {
        print!("{:02x}", unsafe { *p.add(i) });
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
    print_hex(
        &house as *const House as *const u8,
        std::mem::size_of::<House>(),
    );
}
