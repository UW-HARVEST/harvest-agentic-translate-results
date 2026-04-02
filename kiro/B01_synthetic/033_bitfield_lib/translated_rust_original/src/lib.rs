use std::os::raw::{c_int, c_uint};

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let x = x & 0x3;
    let y = y & 0x7;
    let b = b as c_int & 0x1;
    unsafe {
        printf(b"%u %u %d %d\n\0".as_ptr(), x, y, b, z);
    }
}
