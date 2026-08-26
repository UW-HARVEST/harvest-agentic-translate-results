// Translated from c_src/src/pcre2_chkdint.c
use crate::internal::*;

/* This file contains functions to implement checked integer operation */

/*************************************************
*        Checked Integer Multiplication          *
*************************************************/

/*
Arguments:
  r         A pointer to PCRE2_SIZE to store the answer
  a, b      Two integers

Returns:    Bool indicating if the operation overflows

It is modeled after C23's <stdckdint.h> interface
The INT64_OR_DOUBLE type is a 64-bit integer type when available,
otherwise double. */

/* HAVE_BUILTIN_MUL_OVERFLOW is not defined, so the fallback code is used.
INT64_OR_DOUBLE is int64_t (INT64_MAX / int64_t are available). */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ckd_smul_8(r: *mut PCRE2_SIZE, a: c_int, b: c_int) -> BOOL {
    let m: i64;

    /* PCRE2_ASSERT(a >= 0 && b >= 0); -- a no-op without PCRE2_DEBUG */

    m = (a as i64) * (b as i64);

    /* #if defined INT64_MAX || defined int64_t */
    if size_of::<i64>() > size_of::<PCRE2_SIZE>() && m > PCRE2_SIZE_MAX as i64 {
        return TRUE;
    }
    *r = m as PCRE2_SIZE;

    FALSE
}

/* End of pcre2_chkdint.c */
