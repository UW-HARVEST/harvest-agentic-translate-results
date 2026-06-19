use std::ffi::c_char;

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> i32;
}

fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        print!("{:02x}", byte);
    }
    println!();
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x = 0.0_f32;
    unsafe {
        scanf(c"%f".as_ptr(), &mut x);
    }
    driver(x);
}
