use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[derive(Clone, Copy)]
struct Foo {
    x: c_uint,
    y: c_uint,
    b: bool,
    z: c_int,
}

fn print_foo(foo: &Foo) {
    let fmt = b"%u %u %d %d\n\0";
    let b = if foo.b { 1 as c_int } else { 0 as c_int };

    unsafe {
        printf(
            fmt.as_ptr().cast::<c_char>(),
            foo.x,
            foo.y,
            b,
            foo.z,
        );
    }
}

fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo {
        x: x & 0x3,
        y: y & 0x7,
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
        scanf(b"%u\0".as_ptr().cast::<c_char>(), &mut x);
        scanf(b"%u\0".as_ptr().cast::<c_char>(), &mut y);
        scanf(b"%d\0".as_ptr().cast::<c_char>(), &mut b);
        scanf(b"%d\0".as_ptr().cast::<c_char>(), &mut z);
    }

    driver(x, y, b != 0, z);
}
