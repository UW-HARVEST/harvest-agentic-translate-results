// Translated from pcre2_chkdint.c
use crate::internal::*;
use core::ffi::c_int;

/*************************************************
*        Checked Integer Multiplication          *
*************************************************/

/* HAVE_BUILTIN_MUL_OVERFLOW is not defined, INT64_MAX is available, so the
INT64_OR_DOUBLE branch with int64_t is used. sizeof(int64_t) == sizeof(PCRE2_SIZE)
on this platform, so the size test is false and the value is simply cast. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ckd_smul_8(r: *mut PCRE2_SIZE, a: c_int, b: c_int) -> BOOL {
    let m: i64 = (a as i64) * (b as i64);

    if core::mem::size_of::<i64>() > core::mem::size_of::<PCRE2_SIZE>()
        && m as u64 > PCRE2_SIZE_MAX_LOCAL
    {
        return TRUE;
    }
    *r = m as PCRE2_SIZE;

    FALSE
}

const PCRE2_SIZE_MAX_LOCAL: u64 = usize::MAX as u64;
