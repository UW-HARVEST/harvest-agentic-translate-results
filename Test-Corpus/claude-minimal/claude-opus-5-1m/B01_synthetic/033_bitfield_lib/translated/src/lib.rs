// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::os::raw::{c_int, c_uint};

/// Equivalent of the C `foo_t` struct that uses bit-fields:
///
/// ```c
/// typedef struct {
///     unsigned int x : 2;
///     unsigned int y : 3;
///     bool b : 1;
///     int z;
/// } foo_t;
/// ```
///
/// Rust does not have first-class bit-field support, so we emulate it by
/// masking the values to the appropriate width when stored.
#[derive(Copy, Clone, Debug)]
struct Foo {
    /// 2-bit unsigned field
    x: c_uint,
    /// 3-bit unsigned field
    y: c_uint,
    /// 1-bit boolean field
    b: bool,
    /// Plain int field
    z: c_int,
}

impl Foo {
    fn new(x: c_uint, y: c_uint, b: bool, z: c_int) -> Self {
        Foo {
            // Mask to the width specified by the original C bit-field.
            x: x & 0x3,
            y: y & 0x7,
            b,
            z,
        }
    }
}

fn print_foo(foo: &Foo) {
    // Matches C's `printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);`
    // In C, a `bool` printed with `%d` will print as 0 or 1.
    println!("{} {} {} {}", foo.x, foo.y, foo.b as c_int, foo.z);
}

/// FFI-compatible entry point matching the original C signature:
/// `void driver(unsigned int x, unsigned int y, bool b, int z);`
#[no_mangle]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo::new(x, y, b, z);
    print_foo(&foo);
}
