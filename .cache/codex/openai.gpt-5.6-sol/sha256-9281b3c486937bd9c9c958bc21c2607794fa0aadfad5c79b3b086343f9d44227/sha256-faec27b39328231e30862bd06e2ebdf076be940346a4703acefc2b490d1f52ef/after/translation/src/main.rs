use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

struct Foo {
    x: c_uint,
    y: c_uint,
    b: bool,
    z: c_int,
}

fn print_foo(foo: &Foo) {
    unsafe {
        printf(
            b"%u %u %d %d\n\0".as_ptr().cast(),
            foo.x,
            foo.y,
            if foo.b { 1 as c_int } else { 0 as c_int },
            foo.z,
        );
    }
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
        scanf(b"%u\0".as_ptr().cast(), &mut x as *mut c_uint);
        scanf(b"%u\0".as_ptr().cast(), &mut y as *mut c_uint);
        scanf(b"%d\0".as_ptr().cast(), &mut b as *mut c_int);
        scanf(b"%d\0".as_ptr().cast(), &mut z as *mut c_int);
    }

    driver(x, y, b != 0, z);
}
