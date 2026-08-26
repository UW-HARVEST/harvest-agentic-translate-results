use std::ffi::{CStr, c_int};
use std::io::{self, Write};
use std::mem;
use std::slice;

fn print_hex(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    for &byte in bytes {
        write!(stdout, "{byte:02x}").expect("write to stdout failed");
    }
    writeln!(stdout).expect("write to stdout failed");
}

fn driver(x: c_int) {
    let bytes = unsafe {
        slice::from_raw_parts((&x as *const c_int).cast::<u8>(), mem::size_of_val(&x))
    };
    print_hex(bytes);
}

fn main() {
    let mut x: c_int = 0;
    let format = CStr::from_bytes_with_nul(b"%d\0").expect("format string must be nul-terminated");

    unsafe {
        libc::scanf(format.as_ptr(), &mut x);
    }

    driver(x);
}
