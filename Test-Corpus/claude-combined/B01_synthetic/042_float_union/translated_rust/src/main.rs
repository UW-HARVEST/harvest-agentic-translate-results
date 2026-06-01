// Rust translation of c_src/src/main.c
// Uses libc FFI to ensure byte-identical output to C's scanf and printf.

use std::ffi::CString;

extern "C" {
    fn scanf(fmt: *const libc::c_char, ...) -> libc::c_int;
    fn printf(fmt: *const libc::c_char, ...) -> libc::c_int;
}

fn driver(f: f64) {
    let bits: u64 = f.to_bits();
    let fmt = CString::new("%llx %a %.4f\n").unwrap();
    unsafe {
        printf(fmt.as_ptr(), bits, f, f);
    }
}

fn main() {
    let mut f: f64 = 0.0;
    let fmt = CString::new("%lf").unwrap();
    unsafe {
        scanf(fmt.as_ptr(), &mut f as *mut f64);
    }
    driver(f);
}
