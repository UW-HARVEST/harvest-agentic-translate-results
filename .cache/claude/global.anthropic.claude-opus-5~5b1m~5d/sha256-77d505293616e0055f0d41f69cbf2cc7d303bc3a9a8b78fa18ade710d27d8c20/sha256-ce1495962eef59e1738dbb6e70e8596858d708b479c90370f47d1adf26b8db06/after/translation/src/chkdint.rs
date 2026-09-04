//! Translated from pcre2_chkdint.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ckd_smul_8(r: *mut PCRE2_SIZE, a: i32, b: i32) -> BOOL {
    /* HAVE_BUILTIN_MUL_OVERFLOW is not defined: portable fallback. */
    let m: i64; /* INT64_OR_DOUBLE */

    /* PCRE2_ASSERT(a >= 0 && b >= 0); */

    m = (a as i64) * (b as i64);

    /* #if defined INT64_MAX || defined int64_t */
    if core::mem::size_of::<i64>() > core::mem::size_of::<PCRE2_SIZE>()
        && m > PCRE2_SIZE_MAX as i64
    {
        return TRUE;
    }
    *r = m as PCRE2_SIZE;

    FALSE
}

