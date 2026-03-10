use std::os::raw::{c_int, c_uint};

struct FooT {
    x: c_uint,
    y: c_uint,
    b: c_int,
    z: c_int,
}

fn print_foo(foo: &FooT) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b, foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = FooT {
        x: x & 0x3,
        y: y & 0x7,
        b: (b as c_int) & 0x1,
        z,
    };
    print_foo(&foo);
}
