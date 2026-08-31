//! Translation of `c_src/src/pcre2_chkdint.c`.
//!
//! Functions to implement checked integer operations.

#![allow(non_snake_case)]

use core::ffi::c_int;

use crate::internal::*;

/* Checked Integer Multiplication.

Arguments:
  r         A pointer to PCRE2_SIZE to store the answer
  a, b      Two integers

Returns:    Bool indicating if the operation overflows

It is modeled after C23's <stdckdint.h> interface. This mirrors the
HAVE_BUILTIN_MUL_OVERFLOW branch of the C source (the compiler builtin is
available under the build configuration). */

pub unsafe fn ckd_smul(r: *mut PCRE2_SIZE, a: c_int, b: c_int) -> BOOL {
    unsafe {
        /* __builtin_mul_overflow computes a * b in the type of *r (PCRE2_SIZE,
        i.e. usize) and reports whether the mathematical result is not
        representable in that type. a and b are ints (possibly negative). */
        let wide = (a as i64).wrapping_mul(b as i64);
        let m = wide as PCRE2_SIZE;
        if (m as i64) != wide {
            return TRUE;
        }
        *r = m;
        FALSE
    }
}

/// Exported as `_pcre2_ckd_smul_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ckd_smul_8(r: *mut PCRE2_SIZE, a: c_int, b: c_int) -> BOOL {
    unsafe { ckd_smul(r, a, b) }
}
