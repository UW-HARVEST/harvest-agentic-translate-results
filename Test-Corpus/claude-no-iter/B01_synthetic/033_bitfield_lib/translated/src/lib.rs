// Translated from C library `driver`. Reproduces the byte-identical output of
// the original implementation, including the C bitfield masking semantics.

use std::ffi::c_int;
use std::os::raw::c_uint;

#[repr(C)]
struct FooT {
    // C bitfield: unsigned int x : 2 — stored masked to 2 bits
    x: c_uint,
    // C bitfield: unsigned int y : 3 — stored masked to 3 bits
    y: c_uint,
    // C bitfield: bool b : 1 — stored as 0 or 1
    b: bool,
    z: c_int,
}

fn print_foo(foo: &FooT) {
    // The C version uses printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z).
    // Invoke libc printf directly so output is byte-identical to the C version.
    let fmt = b"%u %u %d %d\n\0";
    let b_as_int: c_int = if foo.b { 1 } else { 0 };
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const libc::c_char,
            foo.x,
            foo.y,
            b_as_int,
            foo.z,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    // C bitfield assignment semantics: values are masked to the bitfield width.
    // x : 2 bits, y : 3 bits, b : 1 bit (bool already constrains to 0/1)
    let foo = FooT {
        x: x & 0x3,
        y: y & 0x7,
        b,
        z,
    };
    print_foo(&foo);
}
