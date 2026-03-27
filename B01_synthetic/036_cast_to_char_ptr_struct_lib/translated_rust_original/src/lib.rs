use std::ffi::c_int;

#[repr(C)]
struct HouseT {
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
    let mut house = HouseT {
        floors: 0,
        bedrooms: 0,
        bathrooms: 0.0,
    };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    print_hex(
        &house as *const HouseT as *const u8,
        std::mem::size_of::<HouseT>(),
    );
}
