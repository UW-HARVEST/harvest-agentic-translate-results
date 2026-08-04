use std::io::{self, Read};

#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    for b in p {
        print!("{:02x}", b);
    }
    println!();
}

#[no_mangle]
pub extern "C" fn driver(floors: i32) {
    let mut house = House { floors: 0, bedrooms: 0, bathrooms: 0.0 };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &house as *const House as *const u8,
            std::mem::size_of::<House>(),
        )
    };
    print_hex(bytes);
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);
    driver(x);
    0
}
