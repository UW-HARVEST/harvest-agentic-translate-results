use std::ffi::c_char;

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> i32;
}

fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}

fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    let mut x = 0.0f32;

    unsafe {
        scanf(c"%f".as_ptr(), &mut x);
    }

    driver(x);
}
