//! Translation of `pcre2_chkdint.c`.

use crate::internal::*;
use core::ffi::c_int;

/// `PRIV(ckd_smul)` — checked signed multiplication.
///
/// `HAVE_BUILTIN_MUL_OVERFLOW` is not defined in this configuration, and
/// `INT64_OR_DOUBLE` resolves to `int64_t`. On a 64-bit target
/// `sizeof(int64_t) > sizeof(PCRE2_SIZE)` is false, so the overflow test is
/// never taken and the function always reports success — reproduced here
/// exactly, including the absence of overflow detection.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ckd_smul_8(r: *mut PCRE2_SIZE, a: c_int, b: c_int) -> BOOL {
    let m: i64 = (a as i64).wrapping_mul(b as i64);

    if core::mem::size_of::<i64>() > core::mem::size_of::<PCRE2_SIZE>()
        && m > PCRE2_SIZE_MAX as i64
    {
        return TRUE;
    }
    unsafe { *r = m as PCRE2_SIZE };
    FALSE
}
