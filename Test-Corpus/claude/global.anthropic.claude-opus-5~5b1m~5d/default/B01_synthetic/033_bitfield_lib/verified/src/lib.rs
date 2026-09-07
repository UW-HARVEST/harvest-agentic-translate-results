// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
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

use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    // C's printf, used so that output formatting *and* stdio buffering
    // behaviour are byte-for-byte identical to the original library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Translation of
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
/// On the SysV/ELF ABI (gcc & clang) the three bit-fields share the first
/// storage unit: `x` occupies bits 0..=1, `y` bits 2..=4 and `b` bit 5 of
/// byte 0.  Bits 6..=7 of byte 0 and bytes 1..=3 are padding, `z` lives at
/// offset 4.  `sizeof(foo_t) == 8`, `_Alignof(foo_t) == 4`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct foo_t {
    /// Packed bit-field storage unit (x: bits 0-1, y: bits 2-4, b: bit 5).
    bits: u8,
    /// int z; (offset 4 thanks to the repr(C) alignment padding)
    z: c_int,
}

impl foo_t {
    #[inline]
    fn new(x: c_uint, y: c_uint, b: u8, z: c_int) -> Self {
        // Exactly what the C compiler emits for
        // `foo_t foo = {.x = x, .y = y, .b = b, .z = z};`
        let bits = ((x as u8) & 0x3) | (((y as u8) & 0x7) << 2) | ((b & 0x1) << 5);
        foo_t { bits, z }
    }

    /// `foo->x` — 2-bit unsigned bit-field, promoted to `int` for the
    /// variadic call.
    #[inline]
    fn x(self) -> c_int {
        (self.bits & 0x3) as c_int
    }

    /// `foo->y` — 3-bit unsigned bit-field, promoted to `int`.
    #[inline]
    fn y(self) -> c_int {
        ((self.bits >> 2) & 0x7) as c_int
    }

    /// `foo->b` — 1-bit `bool` bit-field, promoted to `int`.
    #[inline]
    fn b(self) -> c_int {
        ((self.bits >> 5) & 0x1) as c_int
    }
}

/// ```c
/// void print_foo(const foo_t *foo) {
///     printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    // The C code dereferences unconditionally; do the same.
    let foo = unsafe { std::ptr::read(foo) };
    unsafe {
        printf(
            b"%u %u %d %d\n\0".as_ptr() as *const c_char,
            foo.x(),
            foo.y(),
            foo.b(),
            foo.z,
        );
    }
}

/// ```c
/// void driver(unsigned int x, unsigned int y, bool b, int z) {
///     foo_t foo = {.x = x, .y = y, .b = b, .z = z};
///     print_foo(&foo);
/// }
/// ```
///
/// `b` is taken as a `u8` rather than a Rust `bool` so that the (technically
/// out-of-contract) case of a caller passing a byte other than 0/1 behaves
/// like the C code does — the bit-field assignment keeps only bit 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_uint, y: c_uint, b: u8, z: c_int) {
    let foo = foo_t::new(x, y, b, z);
    unsafe { print_foo(&foo as *const foo_t) };
}
