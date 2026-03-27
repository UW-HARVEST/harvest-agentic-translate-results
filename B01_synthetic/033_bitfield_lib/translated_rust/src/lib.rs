use std::os::raw::{c_int, c_uint};

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[repr(C)]
pub struct foo_t {
    pub bitfields: c_uint, // x:2 (bits 0-1), y:3 (bits 2-4), b:1 (bit 5)
    pub z: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    let foo = unsafe { &*foo };
    let x = foo.bitfields & 0x3;
    let y = (foo.bitfields >> 2) & 0x7;
    let b = ((foo.bitfields >> 5) & 0x1) as c_int;
    unsafe {
        printf(b"%u %u %d %d\n\0".as_ptr(), x, y, b, foo.z);
    }
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
