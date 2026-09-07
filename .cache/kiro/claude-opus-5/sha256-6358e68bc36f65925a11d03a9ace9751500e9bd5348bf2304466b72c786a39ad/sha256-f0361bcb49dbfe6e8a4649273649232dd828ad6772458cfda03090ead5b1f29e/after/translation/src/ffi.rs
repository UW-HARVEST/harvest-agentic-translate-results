//! Declarations for the exact C library entities the original translation
//! unit referenced: `<errno.h>`, `<math.h>` and `<stdio.h>`.
//!
//! We bind libc directly rather than using Rust's standard-library
//! equivalents because the observable behaviour of the C code depends on
//! details that Rust's own facilities do not reproduce:
//!
//! * `f64::powf` lowers to `llvm.pow.f64`, which is explicitly permitted not
//!   to set `errno`; only a real call to libm's `pow` sets it.
//! * Rust's `{:.2}` formatting prints `NaN`/`inf`, whereas C's `%.2f` prints
//!   `nan`/`-nan`/`inf`/`-inf`.
//! * Writes must go to the same `stderr` `FILE` stream the C code used.

use std::ffi::{c_char, c_double, c_int};

/// `EDOM` on Linux/glibc.
pub const EDOM: c_int = 33;
/// `ERANGE` on Linux/glibc.
pub const ERANGE: c_int = 34;

/// Opaque stand-in for C's `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    /// glibc's `stderr` global.
    pub static mut stderr: *mut FILE;

    pub fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;

    /// libm's `pow`. Called through this declaration (rather than via
    /// `f64::powf`) so that `errno` is set just as it is for the C code.
    pub fn pow(base: c_double, exponent: c_double) -> c_double;

    /// glibc's thread-local `errno` accessor; `errno` is a macro expanding to
    /// `(*__errno_location())`.
    pub fn __errno_location() -> *mut c_int;
}

/// Reads `errno`.
#[inline]
pub fn errno() -> c_int {
    // SAFETY: `__errno_location` always returns a valid pointer to the
    // calling thread's `errno`.
    unsafe { *__errno_location() }
}

/// Performs `errno = value`.
#[inline]
pub fn set_errno(value: c_int) {
    // SAFETY: as above.
    unsafe { *__errno_location() = value }
}

/// Optimisation barrier.
///
/// The C code's control flow depends on `pow` writing `errno` as a side
/// effect. LLVM knows `pow` as a library call and, depending on the
/// attributes it infers, could in principle sink the `pow` call past the
/// `errno` load or hoist that load above it. Interposing an empty `asm!`
/// block — which, absent `options(nomem)`, is assumed to read and write
/// memory and has side effects — pins the two operations in source order.
#[inline(always)]
pub fn barrier() {
    // SAFETY: the asm block is empty; it only constrains the optimiser.
    unsafe { std::arch::asm!("", options(nostack, preserves_flags)) }
}
