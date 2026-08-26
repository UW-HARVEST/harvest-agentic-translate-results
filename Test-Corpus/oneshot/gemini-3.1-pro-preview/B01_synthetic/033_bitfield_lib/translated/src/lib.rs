use std::ffi::{c_int, c_uint};

struct Foo {
    x: c_uint,
    y: c_uint,
    b: bool,
    z: c_int,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b as i32, foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo {
        x: x & 3,
        y: y & 7,
        b,
        z,
    };
    print_foo(&foo);
}
