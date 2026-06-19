use std::os::raw::{c_char, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        print!("{:02x}", byte);
    }
    println!();
}

fn driver(x: c_int) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(b"%d\0".as_ptr().cast::<c_char>(), &mut x);
    }

    driver(x);
}
