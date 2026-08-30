use std::ffi::{c_char, c_int, c_uint};

#[repr(C)]
pub struct Foo {
    bit_fields: c_uint,
    z: c_int,
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const FORMAT: &[u8] = b"%u %u %d %d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const Foo) {
    let foo = unsafe { &*foo };
    let x = foo.bit_fields & 0x3;
    let y = (foo.bit_fields >> 2) & 0x7;
    let b = (foo.bit_fields >> 5) & 0x1;

    unsafe {
        printf(FORMAT.as_ptr().cast(), x, y, b as c_int, foo.z);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo {
        bit_fields: (x & 0x3) | ((y & 0x7) << 2) | ((b as c_uint) << 5),
        z,
    };

    unsafe {
        print_foo(&foo);
    }
}
