use std::io::{self, Read};

#[repr(C)]
struct HouseT {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(bytes: &[u8]) {
    for b in bytes {
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
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &house as *const HouseT as *const u8,
            std::mem::size_of::<HouseT>(),
        )
    };
    print_hex(bytes);
}

fn main() {
    // Match scanf("%d", &x): skip leading whitespace, parse integer
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: i32 = input.split_whitespace().next().map_or(0, |s| s.parse().unwrap_or(0));
    driver(x);
}
