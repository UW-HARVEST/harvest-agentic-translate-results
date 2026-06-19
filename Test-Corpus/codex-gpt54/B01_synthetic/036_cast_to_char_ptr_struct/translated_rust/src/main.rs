use std::mem::{size_of, zeroed};

#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}

fn driver(floors: i32) {
    let mut house: House = unsafe { zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    let bytes = unsafe {
        std::slice::from_raw_parts((&house as *const House).cast::<u8>(), size_of::<House>())
    };
    print_hex(bytes);
}

fn main() {
    let mut x: i32 = 0;
    unsafe {
        libc::scanf(c"%d".as_ptr(), &mut x);
    }
    driver(x);
}
