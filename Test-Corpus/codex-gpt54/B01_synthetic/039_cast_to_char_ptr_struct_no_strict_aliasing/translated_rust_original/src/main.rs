use std::os::raw::{c_char, c_int};

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}

fn driver(floors: c_int) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let raw = unsafe {
        std::slice::from_raw_parts(
            (&house as *const House).cast::<u8>(),
            std::mem::size_of::<House>(),
        )
    };
    print_hex(raw);
}

fn main() {
    let mut x: c_int = 0;
    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
    }
    driver(x);
}
