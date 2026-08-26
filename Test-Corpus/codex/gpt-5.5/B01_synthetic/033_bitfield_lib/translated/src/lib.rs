use std::ffi::{c_int, c_uint};

#[repr(C)]
pub struct Foo {
    bits: c_uint,
    z: c_int,
}

impl Foo {
    fn new(x: c_uint, y: c_uint, b: bool, z: c_int) -> Self {
        let mut bits = 0;
        bits |= x & 0x3;
        bits |= (y & 0x7) << 2;
        bits |= (b as c_uint) << 5;

        Self { bits, z }
    }

    fn x(&self) -> c_uint {
        self.bits & 0x3
    }

    fn y(&self) -> c_uint {
        (self.bits >> 2) & 0x7
    }

    fn b(&self) -> c_int {
        ((self.bits >> 5) & 0x1) as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn print_foo(foo: *const Foo) {
    let foo = unsafe { &*foo };
    println!("{} {} {} {}", foo.x(), foo.y(), foo.b(), foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo::new(x, y, b, z);
    print_foo(&foo);
}
