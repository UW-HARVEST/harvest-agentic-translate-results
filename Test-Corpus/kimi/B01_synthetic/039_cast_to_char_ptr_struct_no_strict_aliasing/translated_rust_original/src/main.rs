use std::io;

#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    for byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

fn driver(floors: i32) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let raw: [u8; std::mem::size_of::<House>()] = unsafe {
        std::mem::transmute(house)
    };
    print_hex(&raw);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();
    driver(x);
}