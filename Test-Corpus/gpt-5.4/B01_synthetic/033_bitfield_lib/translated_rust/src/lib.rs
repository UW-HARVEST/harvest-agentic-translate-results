use std::os::raw::{c_int, c_uint};

#[repr(C)]
pub struct foo_t {
    pub x: c_uint,
    pub y: c_uint,
    pub b: bool,
    pub z: c_int,
}

pub fn print_foo(foo: &foo_t) {
    println!("{} {} {} {}", foo.x & 0b11, foo.y & 0b111, if foo.b { 1 } else { 0 }, foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = foo_t {
        x: x & 0b11,
        y: y & 0b111,
        b,
        z,
    };
    print_foo(&foo);
}
