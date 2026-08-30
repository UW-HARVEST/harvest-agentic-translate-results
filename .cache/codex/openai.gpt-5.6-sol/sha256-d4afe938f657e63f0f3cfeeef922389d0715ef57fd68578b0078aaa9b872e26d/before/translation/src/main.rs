use std::ffi::{c_char, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    for byte in x.to_ne_bytes() {
        print!("{byte:02x}");
    }
    println!();
}
