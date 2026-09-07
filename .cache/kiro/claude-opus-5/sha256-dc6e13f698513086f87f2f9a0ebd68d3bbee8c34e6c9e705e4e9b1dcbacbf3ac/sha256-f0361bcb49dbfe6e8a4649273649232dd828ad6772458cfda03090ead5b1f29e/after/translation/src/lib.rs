// Rust translation of c_src/src/driver.c + c_src/include/driver.h
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

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_uint};

// The C library formats its output with the C standard library's `printf`.
// Calling straight through to libc keeps both the formatting and the stdout
// buffering behaviour byte-for-byte identical to the original.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
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
/// Verified layout as produced by gcc on x86-64 (sizeof == 8, alignof == 4):
///
/// * byte 0, bits 0..=1 -> `x`
/// * byte 0, bits 2..=4 -> `y`
/// * byte 0, bit  5     -> `b`
/// * bytes 1..=3        -> padding (never read)
/// * bytes 4..=7        -> `z`
///
/// `bits` is a byte array rather than a `u32` so that the bit positions match
/// the C compiler's allocation-unit layout independently of host endianness.
#[repr(C)]
pub struct foo_t {
    bits: [u8; 4],
    z: c_int,
}

// Compile-time confirmation of the C layout (gcc x86-64: sizeof 8, alignof 4,
// `z` at offset 4).
const _: () = {
    assert!(core::mem::size_of::<foo_t>() == 8);
    assert!(core::mem::align_of::<foo_t>() == 4);
    assert!(core::mem::offset_of!(foo_t, bits) == 0);
    assert!(core::mem::offset_of!(foo_t, z) == 4);
};

impl foo_t {
    const X_SHIFT: u32 = 0;
    const X_MASK: u8 = 0x03; // 2 bits
    const Y_SHIFT: u32 = 2;
    const Y_MASK: u8 = 0x07; // 3 bits
    const B_SHIFT: u32 = 5;
    const B_MASK: u8 = 0x01; // 1 bit

    #[inline]
    fn get_x(&self) -> c_uint {
        c_uint::from((self.bits[0] >> Self::X_SHIFT) & Self::X_MASK)
    }

    #[inline]
    fn get_y(&self) -> c_uint {
        c_uint::from((self.bits[0] >> Self::Y_SHIFT) & Self::Y_MASK)
    }

    /// The `bool b : 1` member. gcc materialises this as
    /// `(byte0 >> 5) & 1`, i.e. an `int` valued 0 or 1.
    #[inline]
    fn get_b(&self) -> c_int {
        c_int::from((self.bits[0] >> Self::B_SHIFT) & Self::B_MASK)
    }

    #[inline]
    fn set_x(&mut self, x: c_uint) {
        let v = (x as u8) & Self::X_MASK;
        self.bits[0] = (self.bits[0] & !(Self::X_MASK << Self::X_SHIFT)) | (v << Self::X_SHIFT);
    }

    #[inline]
    fn set_y(&mut self, y: c_uint) {
        let v = (y as u8) & Self::Y_MASK;
        self.bits[0] = (self.bits[0] & !(Self::Y_MASK << Self::Y_SHIFT)) | (v << Self::Y_SHIFT);
    }

    /// gcc masks the incoming `_Bool` byte with `and $0x1` before shifting it
    /// into place, so only the low bit of the argument byte is significant.
    #[inline]
    fn set_b(&mut self, b: u8) {
        let v = b & Self::B_MASK;
        self.bits[0] = (self.bits[0] & !(Self::B_MASK << Self::B_SHIFT)) | (v << Self::B_SHIFT);
    }
}

/// ```c
/// void print_foo(const foo_t *foo) {
///     printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
/// }
/// ```
///
/// # Safety
/// `foo` must point at a valid, readable `foo_t` (the C original dereferences
/// it unconditionally and does the same).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    let foo = unsafe { &*foo };
    // Matches the C integer promotions: the `unsigned int` bit-fields of width
    // 2 and 3 and the `bool` bit-field of width 1 all promote to `int`, and are
    // therefore passed as 32-bit values.
    unsafe {
        printf(
            c"%u %u %d %d\n".as_ptr(),
            foo.get_x() as c_int,
            foo.get_y() as c_int,
            foo.get_b(),
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
/// `b` is declared as `u8` rather than `bool`: it has the identical C ABI
/// (a single byte passed in the low 8 bits of the third argument register) but
/// also faithfully reproduces gcc's `& 1` masking for byte values other than
/// 0 and 1, without incurring Rust UB.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: u8, z: c_int) {
    // The C original leaves the struct's padding bits uninitialised; they are
    // never read back, so zeroing them here is unobservable.
    let mut foo = foo_t {
        bits: [0u8; 4],
        z: 0,
    };
    foo.set_x(x);
    foo.set_y(y);
    foo.set_b(b);
    foo.z = z;

    unsafe { print_foo(&foo as *const foo_t) };
}
