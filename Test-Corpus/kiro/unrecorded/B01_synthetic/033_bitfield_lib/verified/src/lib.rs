use std::ffi::{c_int, c_uint};
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[repr(C)]
pub struct FooT {
    _bitfield: c_uint,
    z: c_int,
}

impl FooT {
    fn x(&self) -> c_uint {
        self._bitfield & 0x3
    }
    fn y(&self) -> c_uint {
        (self._bitfield >> 2) & 0x7
    }
    fn b(&self) -> bool {
        (self._bitfield >> 5) & 0x1 != 0
    }
    fn new(x: c_uint, y: c_uint, b: bool, z: c_int) -> Self {
        let bf = (x & 0x3) | ((y & 0x7) << 2) | (((b as c_uint) & 0x1) << 5);
        FooT { _bitfield: bf, z }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn print_foo(foo: *const FooT) {
    unsafe {
        let foo = &*foo;
        printf(
            b"%u %u %d %d\n\0".as_ptr() as *const c_char,
            foo.x(),
            foo.y(),
            foo.b() as c_int,
            foo.z,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = FooT::new(x, y, b, z);
    print_foo(&foo as *const FooT);
}
