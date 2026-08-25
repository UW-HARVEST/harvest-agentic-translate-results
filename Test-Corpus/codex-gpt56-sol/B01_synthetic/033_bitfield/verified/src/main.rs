use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

#[derive(Clone, Copy)]
struct Foo {
    x: c_uint,
    y: c_uint,
    b: bool,
    z: c_int,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, i32::from(foo.b), foo.z);
}

fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo {
        x: x & 0b11,
        y: y & 0b111,
        b,
        z,
    };
    print_foo(&foo);
}

fn main() {
    let mut x: c_uint = 0;
    let mut y: c_uint = 0;
    let mut b: c_int = 0;
    let mut z: c_int = 0;

    unsafe {
        scanf(b"%u\0".as_ptr().cast(), &mut x);
        scanf(b"%u\0".as_ptr().cast(), &mut y);
        scanf(b"%d\0".as_ptr().cast(), &mut b);
        scanf(b"%d\0".as_ptr().cast(), &mut z);
    }

    driver(x, y, b != 0, z);
}
