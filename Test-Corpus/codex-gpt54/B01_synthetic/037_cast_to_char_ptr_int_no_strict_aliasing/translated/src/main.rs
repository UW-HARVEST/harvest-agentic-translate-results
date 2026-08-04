use std::ffi::CString;
use std::os::raw::c_int;

fn print_hex(bytes: &[u8]) {
    for byte in bytes {
        print!("{byte:02x}");
    }
    println!();
}

fn driver(x: c_int) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    let mut x: c_int = 0;
    let format = CString::new("%d").expect("format string contains no interior NULs");

    unsafe {
        libc::scanf(format.as_ptr(), &mut x);
    }

    driver(x);
}
