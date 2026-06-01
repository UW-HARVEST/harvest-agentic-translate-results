// Translated from C, must produce byte-identical output.

use std::os::raw::c_float;

extern "C" {
    fn scanf(fmt: *const u8, ...) -> i32;
}

fn print_hex(p: &[u8]) {
    let mut out = String::with_capacity(p.len() * 2 + 1);
    for byte in p {
        out.push_str(&format!("{:02x}", byte));
    }
    out.push('\n');
    print!("{}", out);
}

fn driver(x: f32) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

fn main() {
    let mut x: c_float = 0.0;
    let fmt = b"%f\0";
    unsafe {
        scanf(fmt.as_ptr(), &mut x as *mut c_float);
    }
    driver(x);
}
