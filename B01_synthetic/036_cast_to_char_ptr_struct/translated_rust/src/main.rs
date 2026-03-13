use std::io::{self, Read};

#[repr(C)]
struct HouseT {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    for &b in p {
        print!("{:02x}", b);
    }
    println!();
}

fn driver(floors: i32) {
    let house = HouseT {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let bytes = unsafe {
        std::slice::from_raw_parts(
            &house as *const HouseT as *const u8,
            std::mem::size_of::<HouseT>(),
        )
    };
    print_hex(bytes);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();
    driver(x);
}
