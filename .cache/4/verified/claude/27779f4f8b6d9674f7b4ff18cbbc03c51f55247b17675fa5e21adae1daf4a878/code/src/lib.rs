// Translation of c_src/src/main.c to Rust.
//
// Original copyright notice from the C source:
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

//! The C translation unit exports three functions with C linkage:
//! `print_foo`, `driver` and `main`.  `print_foo` and `driver` are exported
//! from here with `#[no_mangle] extern "C"`; `main` is exported by the
//! `cdylib` target (`examples/cdylib.rs`, which forwards to [`c_main`])
//! because a `#[no_mangle] fn main` in the library itself would clash with the
//! entry point that rustc generates for the executable target.  The result is
//! a shared object whose dynamic symbol surface matches the C one, built from
//! exactly the code that the executable (`src/main.rs`) runs.

use std::io::{self, Read, Write};
use std::os::raw::{c_int, c_uint};

/// Mirrors the C struct
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
/// As laid out by the platform C ABI (verified against gcc/x86-64):
/// `sizeof == 8`, `_Alignof == 4`, `offsetof(z) == 4`, and all three
/// bit-fields live in byte 0 of the object: `x` in bits 0..=1, `y` in
/// bits 2..=4, `b` in bit 5.  Bits 6..=7 of byte 0 and bytes 1..=3 are
/// padding that the C code neither reads nor writes.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct foo_t {
    /// Bit-field storage byte: `x` = bits 0..=1, `y` = bits 2..=4, `b` = bit 5.
    pub bits: u8,
    /// Padding of the bit-field allocation unit (`unsigned int`).
    pub pad: [u8; 3],
    /// `int z;`
    pub z: c_int,
}

impl foo_t {
    /// Reproduces `foo_t foo = {.x = x, .y = y, .b = b, .z = z};`
    ///
    /// Storing into a bit-field truncates to the field width, exactly what
    /// gcc emits: `x & 3`, `y & 7` and, for the `bool` field, `b & 1`.
    pub fn new(x: c_uint, y: c_uint, b: u8, z: c_int) -> foo_t {
        let bits = ((x & 0x3) as u8) | (((y & 0x7) as u8) << 2) | ((b & 0x1) << 5);
        foo_t {
            bits,
            pad: [0; 3],
            z,
        }
    }

    /// `foo->x` (`unsigned int x : 2`)
    pub fn x(&self) -> c_uint {
        c_uint::from(self.bits & 0x3)
    }

    /// `foo->y` (`unsigned int y : 3`)
    pub fn y(&self) -> c_uint {
        c_uint::from((self.bits >> 2) & 0x7)
    }

    /// `foo->b` (`bool b : 1`), promoted to `int` for `printf("%d", ...)`
    pub fn b(&self) -> c_int {
        c_int::from((self.bits >> 5) & 0x1)
    }
}

/// `void print_foo(const foo_t *foo)`
///
/// ```c
/// printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
/// ```
pub fn print_foo_to(foo: &foo_t, out: &mut dyn Write) {
    let _ = write!(out, "{} {} {} {}\n", foo.x(), foo.y(), foo.b(), foo.z);
}

/// `void driver(unsigned int x, unsigned int y, bool b, int z)`
pub fn driver_to(x: c_uint, y: c_uint, b: u8, z: c_int, out: &mut dyn Write) {
    let foo = foo_t::new(x, y, b, z);
    print_foo_to(&foo, out);
}

/// C `isspace` for the default ("C") locale, which is what `scanf` uses to
/// skip leading white space before a numeric conversion.
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// A byte oriented reader over stdin with a single character pushback slot,
/// used to reproduce `scanf`'s consumption behavior (including the `ungetc`
/// of the first character that does not belong to the conversion).
pub struct Scanner<R: Read> {
    src: R,
    buf: [u8; 4096],
    pos: usize,
    len: usize,
    pushback: Option<u8>,
    at_eof: bool,
}

impl<R: Read> Scanner<R> {
    pub fn new(src: R) -> Scanner<R> {
        Scanner {
            src,
            buf: [0u8; 4096],
            pos: 0,
            len: 0,
            pushback: None,
            at_eof: false,
        }
    }

    /// `getc`: returns `None` on EOF (or read error, which C also treats as a
    /// stream failure).
    fn getc(&mut self) -> Option<u8> {
        if let Some(c) = self.pushback.take() {
            return Some(c);
        }
        if self.pos == self.len {
            if self.at_eof {
                return None;
            }
            loop {
                match self.src.read(&mut self.buf) {
                    Ok(0) => {
                        self.at_eof = true;
                        return None;
                    }
                    Ok(n) => {
                        self.pos = 0;
                        self.len = n;
                        break;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.at_eof = true;
                        return None;
                    }
                }
            }
        }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }

    /// `ungetc`: pushing back EOF is a no-op, as in C.
    fn ungetc(&mut self, c: Option<u8>) {
        if let Some(b) = c {
            self.pushback = Some(b);
        }
    }

    /// Shared scanning of an optionally signed decimal integer, as glibc's
    /// `scanf` does for `%d`/`%u`: skip leading whitespace, accept one sign,
    /// then collect decimal digits; the terminating character is pushed back.
    ///
    /// Returns `None` on input failure (EOF before any character) or matching
    /// failure (no digits), in which case the destination is left unmodified.
    /// Otherwise returns `(negative, magnitude, overflowed)`.
    fn scan_decimal(&mut self) -> Option<(bool, u64, bool)> {
        let mut c = self.getc();
        while let Some(b) = c {
            if is_c_space(b) {
                c = self.getc();
            } else {
                break;
            }
        }
        if c.is_none() {
            // Input failure: EOF while skipping whitespace.
            return None;
        }

        let mut negative = false;
        if let Some(b) = c {
            if b == b'-' || b == b'+' {
                negative = b == b'-';
                c = self.getc();
            }
        }

        let mut digits: usize = 0;
        let mut magnitude: u64 = 0;
        let mut overflow = false;
        while let Some(b) = c {
            if !b.is_ascii_digit() {
                break;
            }
            digits += 1;
            let d = u64::from(b - b'0');
            match magnitude.checked_mul(10).and_then(|m| m.checked_add(d)) {
                Some(v) => magnitude = v,
                None => overflow = true,
            }
            c = self.getc();
        }

        // The character that terminated the number is not part of it.
        self.ungetc(c);

        if digits == 0 {
            // Matching failure (e.g. a lone sign or a non-digit).
            return None;
        }

        Some((negative, magnitude, overflow))
    }

    /// `scanf("%u", &dst)` for a 32-bit `unsigned int`: glibc parses via
    /// `strtoul` (unsigned long, 64-bit here) and then narrows by truncation.
    pub fn scan_u32(&mut self) -> Option<u32> {
        let (negative, magnitude, overflow) = self.scan_decimal()?;
        let value: u64 = if overflow {
            u64::MAX // strtoul saturates to ULONG_MAX on overflow
        } else if negative {
            magnitude.wrapping_neg()
        } else {
            magnitude
        };
        Some(value as u32)
    }

    /// `scanf("%d", &dst)` for a 32-bit `int`: glibc parses via `strtol`
    /// (long, 64-bit here) and then narrows by truncation.
    pub fn scan_i32(&mut self) -> Option<i32> {
        let (negative, magnitude, overflow) = self.scan_decimal()?;
        let value: i64 = if overflow {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else {
            let signed: i128 = if negative {
                -(magnitude as i128)
            } else {
                magnitude as i128
            };
            if signed > i64::MAX as i128 {
                i64::MAX
            } else if signed < i64::MIN as i128 {
                i64::MIN
            } else {
                signed as i64
            }
        };
        Some(value as i32)
    }
}

/// The body of C's `main`, parameterized over the streams so that it can be
/// unit tested; `main` below wires it to the process' stdin/stdout.
pub fn run<R: Read>(input: R, out: &mut dyn Write) -> c_int {
    let mut scanner = Scanner::new(input);

    // unsigned int x = 0, y = 0;
    // int b = 0, z = 0;
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    // Each scanf is attempted regardless of whether earlier ones succeeded;
    // on failure the destination keeps its previous value.
    if let Some(v) = scanner.scan_u32() {
        x = v;
    }
    if let Some(v) = scanner.scan_u32() {
        y = v;
    }
    if let Some(v) = scanner.scan_i32() {
        b = v;
    }
    if let Some(v) = scanner.scan_i32() {
        z = v;
    }

    // driver(x, y, !!b, z)
    driver_to(x, y, u8::from(b != 0), z, out);
    0
}

// ---------------------------------------------------------------------------
// C ABI surface (matches the symbols exported by the C translation unit)
// ---------------------------------------------------------------------------

/// `void print_foo(const foo_t *foo)`
///
/// The C function dereferences `foo` unconditionally (`mov 0x4(%rax),%esi` /
/// `movzbl (%rax),%eax`); a null pointer faults there, so it does here as
/// well.  The two loads are done with `read`/`read_unaligned` on raw pointers
/// instead of through a `&foo_t`, so that — exactly like the C code on x86-64
/// — the behaviour does not depend on the pointer being suitably aligned.
#[no_mangle]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    let base = foo as *const u8;
    let z = std::ptr::read_unaligned(base.wrapping_add(4) as *const c_int);
    let bits = std::ptr::read(base);
    let foo = foo_t {
        bits,
        pad: [0; 3],
        z,
    };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    print_foo_to(&foo, &mut out);
    let _ = out.flush();
}

/// `void driver(unsigned int x, unsigned int y, bool b, int z)`
///
/// `b` is declared `bool` in C.  A C `_Bool` argument is passed in the low
/// byte of a register and gcc masks it with `& 1` when storing it into the
/// one-bit bit-field, so `u8` is used here to be able to reproduce that for
/// every possible byte value that a C caller may pass.
#[no_mangle]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: u8, z: c_int) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver_to(x, y, b, z, &mut out);
    let _ = out.flush();
}

/// `int main()`
///
/// The C-ABI `main` symbol itself is exported by the `cdylib` target
/// (`examples/cdylib.rs`), which simply forwards here; the executable target
/// (`src/main.rs`) uses the very same function, so both artifacts run
/// identical code.
pub fn c_main() -> c_int {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let rc = run(stdin.lock(), &mut out);
    let _ = out.flush();
    rc
}
