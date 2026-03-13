use std::io::{self, Read};
use std::mem;

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
    let bytes: [u8; mem::size_of::<House>()] = unsafe { mem::transmute(house) };
    print_hex(&bytes);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.split_whitespace().next().unwrap().parse().unwrap();
    driver(x);
}
