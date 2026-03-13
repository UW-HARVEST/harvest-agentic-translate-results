use std::os::raw::{c_int, c_uint};

struct Foo {
    x: c_uint,
    y: c_uint,
    b: c_int,
    z: c_int,
}

fn print_foo(foo: &Foo) {
    // Matches: printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    println!("{} {} {} {}", foo.x, foo.y, foo.b, foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo {
        x: x & 0x3,  // unsigned int x : 2
        y: y & 0x7,  // unsigned int y : 3
        b: (b as c_int) & 0x1, // bool b : 1
        z,
    };
    print_foo(&foo);
}
