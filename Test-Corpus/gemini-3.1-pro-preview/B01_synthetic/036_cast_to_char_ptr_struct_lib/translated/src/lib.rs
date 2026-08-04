use std::os::raw::c_int;

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut house: HouseT = unsafe { std::mem::zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let p = unsafe {
        std::slice::from_raw_parts(
            &house as *const HouseT as *const u8,
            std::mem::size_of::<HouseT>(),
        )
    };
    print_hex(p);
}