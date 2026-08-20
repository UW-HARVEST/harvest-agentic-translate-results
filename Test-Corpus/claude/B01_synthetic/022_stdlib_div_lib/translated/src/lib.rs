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

//! Faithful Rust re-implementation of `c_src/src/driver.c`.
//!
//! Public ABI surface (matches `nm -D` on the C shared library exactly):
//!
//! * `driver` — `void driver(int x, int y)`
//!
//! Behaviour notes (bug-for-bug compatible with the C original):
//!
//! * The C code calls `div(x, y)` without validating `y`. When `y == 0`, or
//!   when `x == INT_MIN && y == -1`, the hardware `idiv` instruction raises
//!   `SIGFPE`. That is *not* a bug we are allowed to fix, so the division is
//!   performed with the same `idiv` instruction rather than with Rust's
//!   checked `/` and `%` operators (which would panic with a different
//!   message, or silently wrap if `wrapping_*` were used).
//! * Output is emitted through libc's `printf` against the process-wide
//!   `stdout` `FILE`, so buffering, flush-at-exit semantics and the exact byte
//!   sequence are identical to the C library's.

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// libc `printf`. Used directly so that the emitted bytes and the stdio
    /// buffering behaviour are bit-for-bit identical to the C original.
    #[link_name = "printf"]
    unsafe fn libc_printf(format: *const c_char, ...) -> c_int;
}

/// The result of an integer division, mirroring C's `div_t`.
///
/// The C standard specifies truncation toward zero for `quot`, with `rem`
/// taking the sign of the numerator (`quot * denom + rem == numer`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct DivT {
    quot: c_int,
    rem: c_int,
}

/// Equivalent of glibc's `div(3)`.
///
/// glibc implements `div` as `{ numer / denom, numer % denom }`, which on
/// x86-64 compiles down to a single `idiv`. We emit that `idiv` ourselves so
/// the undefined-behaviour cases (`denom == 0` and `INT_MIN / -1`) trap with
/// `SIGFPE` exactly like the C build does, instead of being turned into a Rust
/// panic.
#[cfg(target_arch = "x86_64")]
#[inline]
fn c_div(numer: c_int, denom: c_int) -> DivT {
    let quot: c_int;
    let rem: c_int;

    // SAFETY: `cdq` sign-extends EAX into EDX:EAX, forming the 64-bit
    // dividend that `idiv` requires; EAX and EDX are declared as explicit
    // register operands so the allocator will not place `denom` in either of
    // them. The block is intentionally *not* marked `pure`, so the optimiser
    // must keep it (and therefore its potential trap) in place.
    unsafe {
        core::arch::asm!(
            "cdq",
            "idiv {denom:e}",
            denom = in(reg) denom,
            inout("eax") numer => quot,
            out("edx") rem,
            options(nomem, nostack),
        );
    }

    DivT { quot, rem }
}

/// Portable fallback for non-x86-64 targets.
///
/// `wrapping_div`/`wrapping_rem` reproduce the C truncating-division results
/// for every well-defined input; the `denom == 0` case still aborts, matching
/// the C original's crash on such platforms.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn c_div(numer: c_int, denom: c_int) -> DivT {
    DivT {
        quot: numer.wrapping_div(denom),
        rem: numer.wrapping_rem(denom),
    }
}

/// `void driver(int x, int y);`
///
/// Divides `x` by `y` and prints the quotient and remainder, exactly as the C
/// implementation does:
///
/// ```text
/// quotient: %d, remainder: %d\n
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = c_div(x, y);

    // SAFETY: the format string is a NUL-terminated literal whose two `%d`
    // conversions are matched by the two `c_int` variadic arguments.
    unsafe {
        libc_printf(
            c"quotient: %d, remainder: %d\n".as_ptr(),
            result.quot,
            result.rem,
        );
    }
}
