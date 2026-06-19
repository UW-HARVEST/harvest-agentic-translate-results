use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

struct FooFields {
    x: c_uint,
    y: c_uint,
    b: bool,
    z: c_int,
}

impl FooFields {
    fn new(x: c_uint, y: c_uint, b: bool, z: c_int) -> Self {
        Self {
            x: x & 0b11,
            y: y & 0b111,
            b,
            z,
        }
    }
}

fn print_foo(foo: &FooFields) {
    static FORMAT: &[u8] = b"%u %u %d %d\n\0";

    unsafe {
        printf(
            FORMAT.as_ptr().cast(),
            foo.x,
            foo.y,
            foo.b as c_int,
            foo.z,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = FooFields::new(x, y, b, z);
    print_foo(&foo);
}
