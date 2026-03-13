use std::ffi::c_int;

#[repr(C)]
struct HouseT {
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
    let house = HouseT {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let p = &house as *const HouseT as *const u8;
    print_hex(p, std::mem::size_of::<HouseT>());
}
