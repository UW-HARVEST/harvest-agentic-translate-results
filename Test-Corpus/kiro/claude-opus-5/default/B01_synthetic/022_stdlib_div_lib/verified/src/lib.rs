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

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C implementation is:
//!
//! ```c
//! void driver(int x, int y) {
//!     div_t result = div(x, y);
//!     printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
//! }
//! ```
//!
//! `div()` truncates the quotient toward zero and computes the remainder as
//! `x - quot * y`, which is exactly what Rust's `/` and `%` operators do for
//! signed integers. The degenerate inputs (`y == 0`, and `x == INT_MIN` with
//! `y == -1`) are undefined behaviour in C; see [`c_div`] for how they are
//! reproduced here.

use std::ffi::c_int;
use std::io::Write;

/// Result of the C `div()` function.
struct DivT {
    quot: c_int,
    rem: c_int,
}

/// Reproduces C's `div(numer, denom)`.
///
/// For the two input combinations that C leaves undefined we deliberately
/// mirror the behaviour of the hardware instruction that a C compiler emits
/// rather than Rust's checked semantics, so that no Rust panic message is ever
/// written to stderr:
///
/// * `denom == 0` and `INT_MIN / -1` trap on x86-64 (`SIGFPE`), so the raw
///   `idiv` instruction is used there.
/// * Elsewhere we fall back to wrapping arithmetic, which matches the
///   non-trapping division instructions of those targets.
#[cfg(target_arch = "x86_64")]
fn c_div(numer: c_int, denom: c_int) -> DivT {
    let quot: i32;
    let rem: i32;
    // SAFETY: `idiv` is the instruction a C compiler emits for this division.
    // It faithfully reproduces the C behaviour, including the hardware trap on
    // a zero divisor or on `INT_MIN / -1`.
    unsafe {
        std::arch::asm!(
            "cdq",
            "idiv {denom:e}",
            denom = in(reg) denom,
            inlateout("eax") numer => quot,
            lateout("edx") rem,
            // Deliberately not `pure`: the trap on a degenerate divisor is an
            // observable side effect that must not be optimised away.
            options(nomem, nostack),
        );
    }
    DivT { quot, rem }
}

#[cfg(not(target_arch = "x86_64"))]
fn c_div(numer: c_int, denom: c_int) -> DivT {
    DivT {
        quot: numer.wrapping_div(denom),
        rem: numer.wrapping_rem(denom),
    }
}

/// Translation of the C `driver` function. The header declares no namespacing
/// macro, so the exported linker symbol is plain `driver`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let result = c_div(x, y);
    let out = format!("quotient: {}, remainder: {}\n", result.quot, result.rem);
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    // Ignore write errors, exactly as the C code ignores printf's return value.
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}
