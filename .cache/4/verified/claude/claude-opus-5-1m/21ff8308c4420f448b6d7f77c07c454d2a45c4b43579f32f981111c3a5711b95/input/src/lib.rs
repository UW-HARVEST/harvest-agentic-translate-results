// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory driver library).
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

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// C definition (from driver.c):
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
/// On the SysV x86-64 ABI (little endian) GCC/Clang allocate the three
/// bit-fields into the first storage unit of the struct:
///   byte 0: bits 0..1 -> x, bits 2..4 -> y, bit 5 -> b, bits 6..7 -> padding
///   bytes 1..3: padding (for the alignment of `z`)
///   bytes 4..7: `z`
/// yielding sizeof(foo_t) == 8 and _Alignof(foo_t) == 4.
#[repr(C)]
#[repr(align(4))]
pub struct foo_t {
    /// Packed storage unit holding the `x`, `y` and `b` bit-fields.
    bits: u8,
    _pad: [u8; 3],
    z: c_int,
}

// Match the C layout exactly: sizeof(foo_t) == 8, _Alignof(foo_t) == 4.
const _: () = assert!(core::mem::size_of::<foo_t>() == 8);
const _: () = assert!(core::mem::align_of::<foo_t>() == 4);

impl foo_t {
    /// `unsigned int x : 2` — bits 0..1 of the storage unit.
    #[inline]
    fn x(&self) -> c_uint {
        (self.bits & 0x3) as c_uint
    }

    /// `unsigned int y : 3` — bits 2..4 of the storage unit.
    #[inline]
    fn y(&self) -> c_uint {
        ((self.bits >> 2) & 0x7) as c_uint
    }

    /// `bool b : 1` — bit 5 of the storage unit.
    ///
    /// Promoted to `int` when passed through `printf`'s varargs, exactly as
    /// the C code's `%d` conversion of `foo->b` does.
    #[inline]
    fn b(&self) -> c_int {
        ((self.bits >> 5) & 0x1) as c_int
    }

    /// `int z` — full-width member at offset 4.
    #[inline]
    fn z(&self) -> c_int {
        self.z
    }
}

/// ```c
/// void print_foo(const foo_t *foo) {
///     printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
/// }
/// ```
///
/// Non-static in C, therefore part of the shared library's public ABI.
/// The original performs no NULL check, so neither does this translation:
/// `foo` is dereferenced unconditionally.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    let foo = &*foo;
    // Delegate to C's printf so the emitted bytes and the stdio buffering
    // behaviour are identical to the original library's.
    printf(
        b"%u %u %d %d\n\0".as_ptr() as *const c_char,
        foo.x(),
        foo.y(),
        foo.b(),
        foo.z(),
    );
}

/// ```c
/// void driver(unsigned int x, unsigned int y, bool b, int z) {
///     foo_t foo = {.x = x, .y = y, .b = b, .z = z};
///     print_foo(&foo);
/// }
/// ```
///
/// Storing into the bit-fields truncates each value to the field width, which
/// is reproduced here by masking:
///   * `x` keeps its low 2 bits,
///   * `y` keeps its low 3 bits,
///   * `b` keeps its low bit — GCC assumes a `_Bool` argument is already 0 or
///     1 and simply masks bit 0 (`andl $1`) rather than testing for non-zero,
///     so a byte such as 2 lands in the field as 0. `b` is taken as a `u8`
///     here (identical to `_Bool`'s ABI: one byte in the argument register)
///     so that out-of-range values reproduce the C behaviour instead of
///     being undefined as they would be for a Rust `bool`.
///
/// The two padding bits of the storage unit are left as-is by the C
/// initializer (it or-s into whatever the stack held); they are never read
/// back, so their value is immaterial. They are zeroed here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_uint, y: c_uint, b: u8, z: c_int) {
    let bits = ((x as u8) & 0x3) | (((y as u8) & 0x7) << 2) | ((b & 0x1) << 5);
    let foo = foo_t {
        bits,
        _pad: [0; 3],
        z,
    };
    print_foo(&foo as *const foo_t);
}
