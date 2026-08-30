// Rust translation of c_src/src/driver.c
//
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

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    /// Use the C library's `printf` rather than Rust's `std::io::stdout` so that
    /// buffering/ordering matches the original translation unit exactly when the
    /// shared object is linked into a C program that also uses `stdio`.
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// ```c
/// typedef struct {
///     unsigned int x : 2;
///     unsigned int y : 3;
///     bool b : 1;
///     int z;
/// } foo_t;
/// ```
///
/// On the SysV / Itanium C++ ABI used by gcc and clang on Linux, the three
/// bit-fields are packed into a single 4-byte allocation unit (least significant
/// bits first on little-endian targets) and `z` follows at offset 4, giving a
/// struct of size 8 and alignment 4. That layout is reproduced here with an
/// explicit `c_uint` holding the packed bits.
#[repr(C)]
pub struct foo_t {
    bits: c_uint,
    z: c_int,
}

/// Bit offsets/widths of the individual bit-fields inside `foo_t::bits`.
const X_SHIFT: u32 = 0;
const X_MASK: c_uint = 0b11;
const Y_SHIFT: u32 = 2;
const Y_MASK: c_uint = 0b111;
const B_SHIFT: u32 = 5;
const B_MASK: c_uint = 0b1;

impl foo_t {
    /// Equivalent of the C designated initializer
    /// `foo_t foo = {.x = x, .y = y, .b = b, .z = z};`
    ///
    /// Assignment into a bit-field truncates to the field width, and assignment
    /// into a `bool` bit-field performs a boolean conversion (any non-zero value
    /// becomes 1).
    fn new(x: c_uint, y: c_uint, b: bool, z: c_int) -> Self {
        let bits = ((x & X_MASK) << X_SHIFT)
            | ((y & Y_MASK) << Y_SHIFT)
            | ((b as c_uint & B_MASK) << B_SHIFT);
        Self { bits, z }
    }

    /// `foo->x` — an `unsigned int : 2` bit-field.
    fn x(&self) -> c_uint {
        (self.bits >> X_SHIFT) & X_MASK
    }

    /// `foo->y` — an `unsigned int : 3` bit-field.
    fn y(&self) -> c_uint {
        (self.bits >> Y_SHIFT) & Y_MASK
    }

    /// `foo->b` — a `bool : 1` bit-field.
    fn b(&self) -> bool {
        ((self.bits >> B_SHIFT) & B_MASK) != 0
    }
}

/// ```c
/// void print_foo(const foo_t *foo) {
///     printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
/// }
/// ```
///
/// `print_foo` is not `static` in the original C, so it is an exported symbol of
/// the shared library and is exported here as well.
///
/// In the variadic call each bit-field undergoes the integer promotions: the
/// 2-bit and 3-bit unsigned fields and the 1-bit `bool` field all fit in `int`,
/// so `int` is what actually reaches `printf`. Because every promoted value is
/// non-negative, the `%u` conversions print the same digits as the `%d` ones
/// would.
///
/// # Safety
///
/// `foo` must be a valid, aligned pointer to an initialized `foo_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    let foo = unsafe { &*foo };
    unsafe {
        c_printf(
            c"%u %u %d %d\n".as_ptr(),
            foo.x() as c_int,
            foo.y() as c_int,
            foo.b() as c_int,
            foo.z,
        )
    };
}

/// ```c
/// void driver(unsigned int x, unsigned int y, bool b, int z) {
///     foo_t foo = {.x = x, .y = y, .b = b, .z = z};
///     print_foo(&foo);
/// }
/// ```
///
/// C's `bool` is passed as a single byte, so the parameter is taken as `u8`
/// here (ABI-identical) and converted with a `!= 0` test. This mirrors C's
/// boolean conversion instead of relying on Rust's `bool`, which would be
/// undefined behaviour for a byte other than 0 or 1.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: u8, z: c_int) {
    let foo = foo_t::new(x, y, b != 0, z);
    unsafe { print_foo(&foo) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_c() {
        assert_eq!(std::mem::size_of::<foo_t>(), 8);
        assert_eq!(std::mem::align_of::<foo_t>(), 4);
    }

    #[test]
    fn bit_fields_truncate() {
        // x keeps 2 bits, y keeps 3 bits, b becomes 0/1.
        let foo = foo_t::new(0xFF, 0xFF, true, -7);
        assert_eq!(foo.x(), 3);
        assert_eq!(foo.y(), 7);
        assert!(foo.b());
        assert_eq!(foo.z, -7);

        let foo = foo_t::new(5, 9, false, 0);
        assert_eq!(foo.x(), 1); // 5 & 0b11
        assert_eq!(foo.y(), 1); // 9 & 0b111
        assert!(!foo.b());
    }
}
