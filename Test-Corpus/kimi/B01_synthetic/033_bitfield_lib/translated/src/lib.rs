use std::ffi::{c_uint, c_int, c_bool};

#[repr(C)]
struct Foo {
    x: u8,
    y: u8,
    b: bool,
    z: c_int,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b as i32, foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: c_bool, z: c_int) {
    let foo = Foo {
        x: (x & 0x3) as u8,
        y: (y & 0x7) as u8,
        b: b != 0,
        z,
    };
    print_foo(&foo);
}
