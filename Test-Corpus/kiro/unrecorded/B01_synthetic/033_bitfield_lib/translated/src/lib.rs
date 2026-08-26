use std::ffi::{c_int, c_uint};
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

struct FooT {
    x: c_uint,
    y: c_uint,
    b: bool,
    z: c_int,
}

fn print_foo(foo: &FooT) {
    unsafe {
        printf(
            b"%u %u %d %d\n\0".as_ptr() as *const c_char,
            foo.x,
            foo.y,
            foo.b as c_int,
            foo.z,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = FooT {
        x: x & 0x3,
        y: y & 0x7,
        b: (b as c_uint & 0x1) != 0,
        z,
    };
    print_foo(&foo);
}
