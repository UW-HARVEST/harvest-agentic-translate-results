use std::os::raw::{c_int, c_uint};

struct Foo {
    // bit fields in C: x:2, y:3, b:1
    x: c_uint, // 2 bits
    y: c_uint, // 3 bits
    b: bool,   // 1 bit
    z: c_int,
}

fn print_foo(foo: &Foo) {
    // Mimic the C bit-field truncation:
    // unsigned int x : 2  -> mask with 0x3
    // unsigned int y : 3  -> mask with 0x7
    // bool b : 1          -> 0 or 1
    let x = foo.x & 0x3;
    let y = foo.y & 0x7;
    let b: c_int = if foo.b { 1 } else { 0 };
    let z = foo.z;
    // C: printf("%u %u %d %d\n", x, y, b, z);
    println!("{} {} {} {}", x, y, b, z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    // Bit-field assignment truncates to the bit width:
    let foo = Foo {
        x: x & 0x3,
        y: y & 0x7,
        b,
        z,
    };
    print_foo(&foo);
}
