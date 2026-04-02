use std::io::{self, Read};

#[repr(C)]
struct HouseT {
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

// Only export `main` when building the cdylib, not during tests
#[cfg(not(test))]
mod _main_export {
    use super::*;
    #[no_mangle]
    pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap();
        let x: i32 = input.split_whitespace().next().unwrap().parse().unwrap();
        driver(x);
        0
    }
}
