use std::io::{self, Read};

#[repr(C)]
struct House {
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
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(&house as *const House as *const u8, size_of::<House>()) };
    print_hex(bytes);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);
    driver(x);
}
