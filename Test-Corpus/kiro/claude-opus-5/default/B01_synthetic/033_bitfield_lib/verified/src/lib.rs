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

use std::ffi::{c_char, c_int, c_uint};

// The C source declares:
//
//     typedef struct {
//         unsigned int x : 2;
//         unsigned int y : 3;
//         bool b : 1;
//         int z;
//     } foo_t;
//
// On the System V ABI (and every layout GCC/Clang produce for this
// declaration) the three bit-fields share a single 4-byte storage unit and
// `z` follows at offset 4, giving `sizeof(foo_t) == 8` and
// `_Alignof(foo_t) == 4`.  On little-endian targets the first declared
// bit-field occupies the least significant bits, so:
//
//     bit 0..=1 -> x
//     bit 2..=4 -> y
//     bit 5     -> b
//
// The struct is modelled here as the raw storage unit plus `z` so that the
// memory layout (and therefore the ABI of `print_foo`) is preserved exactly.
#[repr(C)]
pub struct foo_t {
    bits: u32,
    z: c_int,
}

const X_SHIFT: u32 = 0;
const X_MASK: u32 = 0b11; // 2 bits
const Y_SHIFT: u32 = 2;
const Y_MASK: u32 = 0b111; // 3 bits
const B_SHIFT: u32 = 5;
const B_MASK: u32 = 0b1; // 1 bit

impl foo_t {
    /// Builds the struct the way the C designated initialiser
    /// `foo_t foo = {.x = x, .y = y, .b = b, .z = z};` does: every value is
    /// simply truncated to the width of its bit-field.
    ///
    /// Note that this includes `b`: the parameter already has type `bool`, so
    /// the compiler performs no `_Bool` normalisation on it and stores the
    /// incoming byte's low bit directly.  A caller that smuggles in a byte
    /// other than 0 or 1 (undefined behaviour in C, but observable) therefore
    /// sees only bit 0 survive -- e.g. `b == 2` prints `0`.  That is
    /// reproduced here rather than corrected.
    fn new(x: c_uint, y: c_uint, b: u8, z: c_int) -> Self {
        let bits = ((x as u32 & X_MASK) << X_SHIFT)
            | ((y as u32 & Y_MASK) << Y_SHIFT)
            | ((b as u32 & B_MASK) << B_SHIFT);
        Self { bits, z }
    }

    /// `foo->x`, an `unsigned int : 2` read back out of the storage unit.
    fn x(&self) -> c_uint {
        ((self.bits >> X_SHIFT) & X_MASK) as c_uint
    }

    /// `foo->y`, an `unsigned int : 3` read back out of the storage unit.
    fn y(&self) -> c_uint {
        ((self.bits >> Y_SHIFT) & Y_MASK) as c_uint
    }

    /// `foo->b`, a `bool : 1` read back out of the storage unit; the value is
    /// promoted to `int` by the variadic call, hence 0 or 1.
    fn b(&self) -> c_int {
        ((self.bits >> B_SHIFT) & B_MASK) as c_int
    }

    /// `foo->z`, a plain `int` with no truncation.
    fn z(&self) -> c_int {
        self.z
    }
}

unsafe extern "C" {
    // Use the platform's C `printf` rather than Rust's own formatting so that
    // the bytes written, and the stdio buffering behaviour, are identical to
    // the original library.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `void print_foo(const foo_t *foo)`
///
/// Non-`static` in the C source, so it is part of the shared library's
/// exported interface and is reproduced here with the same symbol name and
/// signature.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    // Matches `printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);`
    // The C code dereferences `foo` unconditionally, with no NULL check.
    let foo = unsafe { &*foo };
    unsafe {
        printf(
            c"%u %u %d %d\n".as_ptr(),
            foo.x(),
            foo.y(),
            foo.b(),
            foo.z(),
        );
    }
}

/// `void driver(unsigned int x, unsigned int y, bool b, int z)`
///
/// `b` is taken as `u8` because that is how C `_Bool` is passed in this ABI.
/// Doing so keeps the function well defined for every byte a caller might
/// supply, and lets `foo_t::new` mirror the compiler's raw low-bit store.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_uint, y: c_uint, b: u8, z: c_int) {
    let foo = foo_t::new(x, y, b, z);
    unsafe {
        print_foo(&foo);
    }
}
