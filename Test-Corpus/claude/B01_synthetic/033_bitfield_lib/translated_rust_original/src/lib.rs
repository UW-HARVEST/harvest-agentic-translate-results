use std::ffi::c_int;
use std::os::raw::c_uint;

struct Foo {
    x: c_uint,
    y: c_uint,
    b: bool,
    z: c_int,
}

fn print_foo(foo: &Foo) {
    // Mimic C printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    // foo->b is a 1-bit bool bit-field; %d promotes bool to int
    let b_int: c_int = if foo.b { 1 } else { 0 };
    println!("{} {} {} {}", foo.x, foo.y, b_int, foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    // Bit field truncation: x is 2 bits, y is 3 bits, b is 1 bit
    let foo = Foo {
        x: x & 0x3,
        y: y & 0x7,
        b,
        z,
    };
    print_foo(&foo);
}
