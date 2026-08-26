#[repr(C)]
struct House {
    floors: std::ffi::c_int,
    bedrooms: std::ffi::c_int,
    bathrooms: std::ffi::c_double,
}

fn print_hex(p: &[u8]) {
    for &byte in p {
        print!("{:02x}", byte);
    }
    println!();
}

fn driver(floors: std::ffi::c_int) {
    let mut house: House = unsafe { std::mem::zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let bytes = unsafe {
        std::slice::from_raw_parts(
            &house as *const House as *const u8,
            std::mem::size_of::<House>(),
        )
    };
    print_hex(bytes);
}

fn main() {
    let mut x: std::ffi::c_int = 0;
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<std::ffi::c_int>() {
                x = parsed;
            }
        }
    }
    driver(x);
}
