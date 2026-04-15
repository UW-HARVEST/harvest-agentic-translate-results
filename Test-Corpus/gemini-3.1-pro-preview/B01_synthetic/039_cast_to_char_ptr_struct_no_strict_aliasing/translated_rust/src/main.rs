use std::io::Read;

#[repr(C)]
struct House {
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
    let mut house: House = unsafe { std::mem::zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let raw = unsafe {
        std::slice::from_raw_parts(
            &house as *const House as *const u8,
            std::mem::size_of::<House>(),
        )
    };
    print_hex(raw);
}

fn main() {
    let mut input = String::new();
    let mut x = 0;
    for byte in std::io::stdin().bytes() {
        if let Ok(b) = byte {
            if b.is_ascii_whitespace() {
                if !input.is_empty() {
                    break;
                }
            } else {
                input.push(b as char);
            }
        } else {
            break;
        }
    }
    if let Ok(parsed) = input.parse::<i32>() {
        x = parsed;
    }
    driver(x);
}
