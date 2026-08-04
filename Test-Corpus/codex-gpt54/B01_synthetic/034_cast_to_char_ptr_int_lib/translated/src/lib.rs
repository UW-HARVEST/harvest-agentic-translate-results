use std::ffi::c_int;
use std::io::{self, Write};
use std::mem::size_of;
use std::slice;

fn print_hex(p: *const u8, len: usize) {
    let bytes = unsafe { slice::from_raw_parts(p, len) };
    let mut stdout = io::stdout().lock();

    for &byte in bytes {
        let _ = write!(stdout, "{byte:02x}");
    }
    let _ = writeln!(stdout);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let x_ptr = (&x as *const c_int).cast::<u8>();
    print_hex(x_ptr, size_of::<c_int>());
}
