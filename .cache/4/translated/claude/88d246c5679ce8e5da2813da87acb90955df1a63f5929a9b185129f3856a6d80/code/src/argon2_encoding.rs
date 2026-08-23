//! Translation of `crypto_pwhash/argon2/argon2-encoding.c`
//!
//! Example code for a decoder and encoder of "hash strings", with Argon2
//! parameters.
//!
//! The code was originally written by Thomas Pornin <pornin@bolet.org>.
//! Released under Creative Commons CC0 1.0 Public Domain Dedication.
//!
//! Copyright (c) 2015 Thomas Pornin

use crate::common::*;
use core::ffi::{c_char, c_int, c_uint, c_ulong, c_void};
use core::ptr;

/* ---------------------------------------------------------------- */
/* argon2.h / argon2-core.h constants                                */
/* ---------------------------------------------------------------- */

/* Version of the algorithm */
const ARGON2_VERSION_NUMBER: u32 = 0x13;

/* Error codes (enum Argon2_ErrorCodes) */
const ARGON2_OK: c_int = 0;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_ENCODING_FAIL: c_int = -31;
const ARGON2_DECODING_FAIL: c_int = -32;

/* enum Argon2_type { Argon2_i = 1, Argon2_id = 2 } */
const Argon2_i: c_uint = 1;
const Argon2_id: c_uint = 2;

/* utils.h */
const sodium_base64_VARIANT_ORIGINAL_NO_PADDING: c_int = 3;

const U32_STR_MAXSIZE: usize = 11;

/// `typedef struct Argon2_Context { ... } argon2_context;`
///
/// Exact layout from `argon2.h` (96 bytes, `flags` at offset 92).
#[repr(C)]
pub struct argon2_context {
    pub out: *mut u8,
    pub outlen: u32,

    pub pwd: *mut u8,
    pub pwdlen: u32,

    pub salt: *mut u8,
    pub saltlen: u32,

    pub secret: *mut u8,
    pub secretlen: u32,

    pub ad: *mut u8,
    pub adlen: u32,

    pub t_cost: u32,
    pub m_cost: u32,
    pub lanes: u32,
    pub threads: u32,

    pub flags: u32,
}

extern "C" {
    /* argon2-core.c */
    fn _sodium_argon2_validate_inputs(context: *const argon2_context) -> c_int;

    /* sodium/codecs.c */
    fn sodium_bin2base64(
        b64: *mut c_char,
        b64_maxlen: usize,
        bin: *const u8,
        bin_len: usize,
        variant: c_int,
    ) -> *mut c_char;
    fn sodium_base642bin(
        bin: *mut u8,
        bin_maxlen: usize,
        b64: *const c_char,
        b64_len: usize,
        ignore: *const c_char,
        bin_len: *mut usize,
        b64_end: *mut *const c_char,
        variant: c_int,
    ) -> c_int;

    /* libc */
    fn strlen(s: *const c_char) -> usize;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
}

/* ==================================================================== */

/*
 * Decode decimal integer from 'str'; the value is written in '*v'.
 * Returned value is a pointer to the next non-decimal character in the
 * string. If there is no digit at all, or the value encoding is not
 * minimal (extra leading zeros), or the value does not fit in an
 * 'unsigned long', then NULL is returned.
 */
unsafe fn decode_decimal(str_: *const c_char, v: *mut c_ulong) -> *const c_char {
    let orig: *const c_char;
    let mut acc: c_ulong;
    let mut str_ = str_;

    acc = 0;
    orig = str_;
    loop {
        let mut c: c_int;

        c = *str_ as c_int;
        if c < b'0' as c_int || c > b'9' as c_int {
            break;
        }
        c -= b'0' as c_int;
        if acc > (c_ulong::MAX / 10) {
            return ptr::null(); /* LCOV_EXCL_LINE */
        }
        acc = acc.wrapping_mul(10);
        if (c as c_ulong) > (c_ulong::MAX.wrapping_sub(acc)) {
            return ptr::null(); /* LCOV_EXCL_LINE */
        }
        acc = acc.wrapping_add(c as c_ulong);

        str_ = str_.add(1);
    }
    if str_ == orig || (*orig == b'0' as c_char && str_ != orig.add(1)) {
        return ptr::null(); /* LCOV_EXCL_LINE */
    }
    *v = acc;
    str_
}

/* ==================================================================== */
/*
 * Code specific to Argon2.
 *
 * The code below applies the following format:
 *
 *  $argon2<T>[$v=<num>]$m=<num>,t=<num>,p=<num>$<bin>$<bin>
 */

/* Prefix checking -- the CC() macro */
#[inline]
unsafe fn cc_check(str_: &mut *const c_char, prefix: &[u8]) -> bool {
    let cc_len: usize = prefix.len();
    if strncmp(*str_, prefix.as_ptr() as *const c_char, cc_len) != 0 {
        return false;
    }
    *str_ = (*str_).add(cc_len);
    true
}

/*
 * Decode an Argon2i hash string into the provided structure 'ctx'.
 * Returned value is ARGON2_OK on success.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_decode_string(
    ctx: *mut argon2_context,
    str_: *const c_char,
    type_: c_uint,
) -> c_int {
    let mut str_: *const c_char = str_;

    macro_rules! CC {
        ($prefix:expr) => {
            if !cc_check(&mut str_, $prefix) {
                return ARGON2_DECODING_FAIL;
            }
        };
    }

    /* Decoding prefix into uint32_t decimal */
    macro_rules! DECIMAL_U32 {
        ($x:expr) => {{
            let mut dec_x: c_ulong = 0;
            str_ = decode_decimal(str_, &mut dec_x);
            if str_.is_null() || dec_x > 4294967295u64 {
                return ARGON2_DECODING_FAIL;
            }
            $x = dec_x as u32;
        }};
    }

    /* Decoding base64 into a binary buffer */
    macro_rules! BIN {
        ($buf:expr, $max_len:expr, $len:expr) => {{
            let mut bin_len: usize = $max_len;
            let mut str_end: *const c_char = ptr::null();
            if sodium_base642bin(
                $buf,
                $max_len,
                str_,
                strlen(str_),
                ptr::null(),
                &mut bin_len,
                &mut str_end,
                sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
            ) != 0
                || bin_len > 4294967295usize
            {
                return ARGON2_DECODING_FAIL;
            }
            $len = bin_len as u32;
            str_ = str_end;
        }};
    }

    let maxsaltlen: usize = (*ctx).saltlen as usize;
    let maxoutlen: usize = (*ctx).outlen as usize;
    let validation_result: c_int;
    let mut version: u32 = 0;

    (*ctx).saltlen = 0;
    (*ctx).outlen = 0;

    if type_ == Argon2_id {
        CC!(b"$argon2id");
    } else if type_ == Argon2_i {
        CC!(b"$argon2i");
    } else {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    CC!(b"$v=");
    DECIMAL_U32!(version);
    if version != ARGON2_VERSION_NUMBER {
        return ARGON2_INCORRECT_TYPE;
    }
    CC!(b"$m=");
    DECIMAL_U32!((*ctx).m_cost);
    if (*ctx).m_cost as u64 > 4294967295u64 {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    CC!(b",t=");
    DECIMAL_U32!((*ctx).t_cost);
    if (*ctx).t_cost as u64 > 4294967295u64 {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    CC!(b",p=");
    DECIMAL_U32!((*ctx).lanes);
    if (*ctx).lanes as u64 > 4294967295u64 {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    (*ctx).threads = (*ctx).lanes;

    CC!(b"$");
    BIN!((*ctx).salt, maxsaltlen, (*ctx).saltlen);
    CC!(b"$");
    BIN!((*ctx).out, maxoutlen, (*ctx).outlen);
    validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    if *str_ == 0 {
        return ARGON2_OK;
    }
    ARGON2_DECODING_FAIL
}

unsafe fn u32_to_string(str_: *mut c_char, x: u32) {
    let mut tmp: [c_char; U32_STR_MAXSIZE - 1] = [0; U32_STR_MAXSIZE - 1];
    let mut i: usize;
    let mut x: u32 = x;

    i = core::mem::size_of_val(&tmp);
    loop {
        i -= 1;
        tmp[i] = ((x % 10u32) as u8).wrapping_add(b'0') as c_char;
        x /= 10u32;
        if !(x != 0u32 && i != 0usize) {
            break;
        }
    }
    memcpy(
        str_ as *mut u8,
        tmp.as_ptr().add(i) as *const u8,
        core::mem::size_of_val(&tmp) - i,
    );
    *str_.add(core::mem::size_of_val(&tmp) - i) = 0;
}

/* The SS() macro helper */
#[inline]
unsafe fn ss_append(dst: &mut *mut c_char, dst_len: &mut usize, s: *const c_char) -> c_int {
    let pp_len: usize = strlen(s);
    if pp_len >= *dst_len {
        return ARGON2_ENCODING_FAIL;
    }
    memcpy(*dst as *mut u8, s as *const u8, pp_len.wrapping_add(1));
    *dst = (*dst).add(pp_len);
    *dst_len = (*dst_len).wrapping_sub(pp_len);
    ARGON2_OK
}

/* The SB() macro helper */
#[inline]
unsafe fn sb_append(
    dst: &mut *mut c_char,
    dst_len: &mut usize,
    buf: *const u8,
    len: usize,
) -> c_int {
    let sb_len: usize;
    if sodium_bin2base64(
        *dst,
        *dst_len,
        buf,
        len,
        sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
    )
    .is_null()
    {
        return ARGON2_ENCODING_FAIL;
    }
    sb_len = strlen(*dst);
    *dst = (*dst).add(sb_len);
    *dst_len = (*dst_len).wrapping_sub(sb_len);
    ARGON2_OK
}

/*
 * Encode an argon2i hash string into the provided buffer. 'dst_len'
 * contains the size, in characters, of the 'dst' buffer; if 'dst_len'
 * is less than the number of required characters (including the
 * terminating 0), then this function returns 0.
 *
 * On success, ARGON2_OK is returned.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_encode_string(
    dst: *mut c_char,
    dst_len: usize,
    ctx: *mut argon2_context,
    type_: c_uint,
) -> c_int {
    let mut dst: *mut c_char = dst;
    let mut dst_len: usize = dst_len;

    macro_rules! SS {
        ($s:expr) => {{
            let r = ss_append(&mut dst, &mut dst_len, $s.as_ptr() as *const c_char);
            if r != ARGON2_OK {
                return r;
            }
        }};
    }

    macro_rules! SX {
        ($x:expr) => {{
            let mut tmp: [c_char; U32_STR_MAXSIZE] = [0; U32_STR_MAXSIZE];
            u32_to_string(tmp.as_mut_ptr(), $x);
            let r = ss_append(&mut dst, &mut dst_len, tmp.as_ptr());
            if r != ARGON2_OK {
                return r;
            }
        }};
    }

    macro_rules! SB {
        ($buf:expr, $len:expr) => {{
            let r = sb_append(&mut dst, &mut dst_len, $buf, $len);
            if r != ARGON2_OK {
                return r;
            }
        }};
    }

    let validation_result: c_int;

    match type_ {
        Argon2_id => {
            SS!(b"$argon2id$v=\0");
        }
        Argon2_i => {
            SS!(b"$argon2i$v=\0");
        }
        _ => {
            return ARGON2_ENCODING_FAIL; /* LCOV_EXCL_LINE */
        }
    }
    validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result; /* LCOV_EXCL_LINE */
    }
    SX!(ARGON2_VERSION_NUMBER);
    SS!(b"$m=\0");
    SX!((*ctx).m_cost);
    SS!(b",t=\0");
    SX!((*ctx).t_cost);
    SS!(b",p=\0");
    SX!((*ctx).lanes);

    SS!(b"$\0");
    SB!((*ctx).salt, (*ctx).saltlen as usize);

    SS!(b"$\0");
    SB!((*ctx).out, (*ctx).outlen as usize);
    ARGON2_OK
}
