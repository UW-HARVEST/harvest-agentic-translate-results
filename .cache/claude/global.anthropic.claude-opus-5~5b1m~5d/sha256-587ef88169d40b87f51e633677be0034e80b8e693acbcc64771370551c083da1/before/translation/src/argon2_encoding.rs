//! Translated from `c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c`.
//!
//! Encoder/decoder for Argon2 "hash strings" of the form:
//!
//!   $argon2<T>$v=<num>$m=<num>,t=<num>,p=<num>$<bin>$<bin>

#![allow(non_upper_case_globals, non_camel_case_types)]

use core::ffi::{c_char, c_int};

use crate::csys::{memcpy, strlen, strncmp};

// ---------------------------------------------------------------------
// Headers: crypto_pwhash/argon2/argon2.h
// ---------------------------------------------------------------------

/// `argon2_context` — must match `argon2.h` field-for-field (other modules
/// pass pointers to it).
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

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum argon2_type {
    Argon2_i = 1,
    Argon2_id = 2,
}

// A few `Argon2_ErrorCodes` values used in this file.
const ARGON2_OK: c_int = 0;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_ENCODING_FAIL: c_int = -31;
const ARGON2_DECODING_FAIL: c_int = -32;

// crypto_pwhash/argon2/argon2-core.h
const ARGON2_VERSION_NUMBER: u32 = 0x13;

const UINT32_MAX: u64 = u32::MAX as u64;
const ULONG_MAX: u64 = u64::MAX;

// ---------------------------------------------------------------------
// Cross-module calls
// ---------------------------------------------------------------------

extern "C" {
    #[link_name = "_sodium_argon2_validate_inputs"]
    fn argon2_validate_inputs(context: *const argon2_context) -> c_int;

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

    fn sodium_bin2base64(
        b64: *mut c_char,
        b64_maxlen: usize,
        bin: *const u8,
        bin_len: usize,
        variant: c_int,
    ) -> *mut c_char;
}

const sodium_base64_VARIANT_ORIGINAL_NO_PADDING: c_int = 3;

// ---------------------------------------------------------------------
// decode_decimal
// ---------------------------------------------------------------------

/*
 * Decode decimal integer from 'str'; the value is written in '*v'.
 * Returned value is a pointer to the next non-decimal character in the
 * string. If there is no digit at all, or the value encoding is not
 * minimal (extra leading zeros), or the value does not fit in an
 * 'unsigned long', then NULL is returned.
 */
unsafe fn decode_decimal(str_: *const c_char, v: *mut u64) -> *const c_char {
    let orig = str_;
    let mut acc: u64 = 0;
    let mut p = str_;

    loop {
        let c = *p as u8 as c_int; // char is signed in C on x86_64 linux; digits are ASCII so fine
        if !(c >= b'0' as c_int && c <= b'9' as c_int) {
            break;
        }
        let cd = (c - b'0' as c_int) as u64;
        if acc > ULONG_MAX / 10 {
            return core::ptr::null();
        }
        acc = acc.wrapping_mul(10);
        if cd > ULONG_MAX - acc {
            return core::ptr::null();
        }
        acc = acc.wrapping_add(cd);
        p = p.add(1);
    }

    if p == orig || (*orig as u8 == b'0' && p != orig.add(1)) {
        return core::ptr::null();
    }
    *v = acc;
    p
}

// ---------------------------------------------------------------------
// argon2_decode_string
// ---------------------------------------------------------------------

/// `CC(prefix)` macro: `prefix` must be a nul-terminated byte string literal
/// (e.g. `b"$argon2id\0"`). Returns `true` and advances `*str_ptr` past the
/// prefix if `str` starts with it (via `strncmp`, exactly as the C macro
/// does); returns `false` otherwise.
unsafe fn cc(str_ptr: &mut *const c_char, prefix: &[u8]) -> bool {
    let prefix_ptr = prefix.as_ptr() as *const c_char;
    let cc_len = strlen(prefix_ptr);
    if strncmp(*str_ptr, prefix_ptr, cc_len) != 0 {
        return false;
    }
    *str_ptr = (*str_ptr).add(cc_len);
    true
}

/*
 * Decode an Argon2i hash string into the provided structure 'ctx'.
 * Returned value is ARGON2_OK on success.
 */
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_decode_string(
    ctx: *mut argon2_context,
    str_: *const c_char,
    type_: argon2_type,
) -> c_int {
    let mut str_ = str_;

    let maxsaltlen: usize = (*ctx).saltlen as usize;
    let maxoutlen: usize = (*ctx).outlen as usize;
    let mut version: u32 = 0;

    (*ctx).saltlen = 0;
    (*ctx).outlen = 0;

    match type_ {
        argon2_type::Argon2_id => {
            if !cc(&mut str_, b"$argon2id\0") {
                return ARGON2_DECODING_FAIL;
            }
        }
        argon2_type::Argon2_i => {
            if !cc(&mut str_, b"$argon2i\0") {
                return ARGON2_DECODING_FAIL;
            }
        }
    }

    if !cc(&mut str_, b"$v=\0") {
        return ARGON2_DECODING_FAIL;
    }
    // DECIMAL_U32(version)
    {
        let mut dec_x: u64 = 0;
        let np = decode_decimal(str_, &mut dec_x);
        if np.is_null() || dec_x > UINT32_MAX {
            return ARGON2_DECODING_FAIL;
        }
        str_ = np;
        version = dec_x as u32;
    }
    if version != ARGON2_VERSION_NUMBER {
        return ARGON2_INCORRECT_TYPE;
    }

    if !cc(&mut str_, b"$m=\0") {
        return ARGON2_DECODING_FAIL;
    }
    {
        let mut dec_x: u64 = 0;
        let np = decode_decimal(str_, &mut dec_x);
        if np.is_null() || dec_x > UINT32_MAX {
            return ARGON2_DECODING_FAIL;
        }
        str_ = np;
        (*ctx).m_cost = dec_x as u32;
    }
    if (*ctx).m_cost as u64 > UINT32_MAX {
        return ARGON2_INCORRECT_TYPE;
    }

    if !cc(&mut str_, b",t=\0") {
        return ARGON2_DECODING_FAIL;
    }
    {
        let mut dec_x: u64 = 0;
        let np = decode_decimal(str_, &mut dec_x);
        if np.is_null() || dec_x > UINT32_MAX {
            return ARGON2_DECODING_FAIL;
        }
        str_ = np;
        (*ctx).t_cost = dec_x as u32;
    }
    if (*ctx).t_cost as u64 > UINT32_MAX {
        return ARGON2_INCORRECT_TYPE;
    }

    if !cc(&mut str_, b",p=\0") {
        return ARGON2_DECODING_FAIL;
    }
    {
        let mut dec_x: u64 = 0;
        let np = decode_decimal(str_, &mut dec_x);
        if np.is_null() || dec_x > UINT32_MAX {
            return ARGON2_DECODING_FAIL;
        }
        str_ = np;
        (*ctx).lanes = dec_x as u32;
    }
    if (*ctx).lanes as u64 > UINT32_MAX {
        return ARGON2_INCORRECT_TYPE;
    }
    (*ctx).threads = (*ctx).lanes;

    if !cc(&mut str_, b"$\0") {
        return ARGON2_DECODING_FAIL;
    }
    // BIN(ctx->salt, maxsaltlen, ctx->saltlen)
    {
        let mut bin_len: usize = maxsaltlen;
        let mut str_end: *const c_char = core::ptr::null();
        let slen = strlen(str_);
        let rc = sodium_base642bin(
            (*ctx).salt,
            maxsaltlen,
            str_,
            slen,
            core::ptr::null(),
            &mut bin_len,
            &mut str_end,
            sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
        );
        if rc != 0 || bin_len as u64 > UINT32_MAX {
            return ARGON2_DECODING_FAIL;
        }
        (*ctx).saltlen = bin_len as u32;
        str_ = str_end;
    }

    if !cc(&mut str_, b"$\0") {
        return ARGON2_DECODING_FAIL;
    }
    // BIN(ctx->out, maxoutlen, ctx->outlen)
    {
        let mut bin_len: usize = maxoutlen;
        let mut str_end: *const c_char = core::ptr::null();
        let slen = strlen(str_);
        let rc = sodium_base642bin(
            (*ctx).out,
            maxoutlen,
            str_,
            slen,
            core::ptr::null(),
            &mut bin_len,
            &mut str_end,
            sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
        );
        if rc != 0 || bin_len as u64 > UINT32_MAX {
            return ARGON2_DECODING_FAIL;
        }
        (*ctx).outlen = bin_len as u32;
        str_ = str_end;
    }

    let validation_result = argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    if *str_ == 0 {
        return ARGON2_OK;
    }
    ARGON2_DECODING_FAIL
}

// ---------------------------------------------------------------------
// u32_to_string
// ---------------------------------------------------------------------

const U32_STR_MAXSIZE: usize = 11;

unsafe fn u32_to_string(str_: *mut c_char, mut x: u32) {
    // char tmp[U32_STR_MAXSIZE - 1] i.e. tmp[10]
    let mut tmp: [u8; U32_STR_MAXSIZE - 1] = [0; U32_STR_MAXSIZE - 1];
    let mut i: usize = tmp.len();

    loop {
        i -= 1;
        tmp[i] = ((x % 10) as u8) + b'0';
        x /= 10;
        if !(x != 0 && i != 0) {
            break;
        }
    }
    let n = tmp.len() - i;
    memcpy(
        str_ as *mut core::ffi::c_void,
        tmp.as_ptr().add(i) as *const core::ffi::c_void,
        n,
    );
    *str_.add(n) = 0;
}

// ---------------------------------------------------------------------
// argon2_encode_string
// ---------------------------------------------------------------------

/// `SS(str)` macro: copy a nul-terminated string literal into `*dst_ptr`,
/// advancing `*dst_ptr` / shrinking `*dst_len_ptr`. Returns `false` (and the
/// caller must return `ARGON2_ENCODING_FAIL`) if it does not fit.
unsafe fn ss(dst_ptr: &mut *mut c_char, dst_len_ptr: &mut usize, s: &[u8]) -> bool {
    // s does not include the NUL terminator.
    let pp_len = s.len();
    if pp_len >= *dst_len_ptr {
        return false;
    }
    memcpy(
        (*dst_ptr) as *mut core::ffi::c_void,
        s.as_ptr() as *const core::ffi::c_void,
        pp_len,
    );
    *(*dst_ptr).add(pp_len) = 0;
    *dst_ptr = (*dst_ptr).add(pp_len);
    *dst_len_ptr -= pp_len;
    true
}

/// `SX(x)` macro: format `x` as decimal and append via `SS`.
unsafe fn sx(dst_ptr: &mut *mut c_char, dst_len_ptr: &mut usize, x: u32) -> bool {
    let mut tmp: [u8; U32_STR_MAXSIZE] = [0; U32_STR_MAXSIZE];
    u32_to_string(tmp.as_mut_ptr() as *mut c_char, x);
    let len = strlen(tmp.as_ptr() as *const c_char);
    ss(dst_ptr, dst_len_ptr, core::slice::from_raw_parts(tmp.as_ptr(), len))
}

/// `SB(buf, len)` macro: base64-encode `buf[..len]` directly into `*dst`.
unsafe fn sb(
    dst_ptr: &mut *mut c_char,
    dst_len_ptr: &mut usize,
    buf: *const u8,
    len: u32,
) -> bool {
    let r = sodium_bin2base64(
        *dst_ptr,
        *dst_len_ptr,
        buf,
        len as usize,
        sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
    );
    if r.is_null() {
        return false;
    }
    let sb_len = strlen(*dst_ptr as *const c_char);
    *dst_ptr = (*dst_ptr).add(sb_len);
    *dst_len_ptr -= sb_len;
    true
}

/*
 * Encode an argon2i hash string into the provided buffer. 'dst_len'
 * contains the size, in characters, of the 'dst' buffer; if 'dst_len'
 * is less than the number of required characters (including the
 * terminating 0), then this function returns 0.
 *
 * If pp->output_len is 0, then the hash string will be a salt string
 * (no output). if pp->salt_len is also 0, then the string will be a
 * parameter-only string (no salt and no output).
 *
 * On success, ARGON2_OK is returned.
 */
#[no_mangle]
pub unsafe extern "C" fn _sodium_argon2_encode_string(
    dst: *mut c_char,
    dst_len: usize,
    ctx: *mut argon2_context,
    type_: argon2_type,
) -> c_int {
    let mut dst = dst;
    let mut dst_len = dst_len;

    match type_ {
        argon2_type::Argon2_id => {
            if !ss(&mut dst, &mut dst_len, b"$argon2id$v=") {
                return ARGON2_ENCODING_FAIL;
            }
        }
        argon2_type::Argon2_i => {
            if !ss(&mut dst, &mut dst_len, b"$argon2i$v=") {
                return ARGON2_ENCODING_FAIL;
            }
        }
    }

    let validation_result = argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }

    if !sx(&mut dst, &mut dst_len, ARGON2_VERSION_NUMBER) {
        return ARGON2_ENCODING_FAIL;
    }
    if !ss(&mut dst, &mut dst_len, b"$m=") {
        return ARGON2_ENCODING_FAIL;
    }
    if !sx(&mut dst, &mut dst_len, (*ctx).m_cost) {
        return ARGON2_ENCODING_FAIL;
    }
    if !ss(&mut dst, &mut dst_len, b",t=") {
        return ARGON2_ENCODING_FAIL;
    }
    if !sx(&mut dst, &mut dst_len, (*ctx).t_cost) {
        return ARGON2_ENCODING_FAIL;
    }
    if !ss(&mut dst, &mut dst_len, b",p=") {
        return ARGON2_ENCODING_FAIL;
    }
    if !sx(&mut dst, &mut dst_len, (*ctx).lanes) {
        return ARGON2_ENCODING_FAIL;
    }

    if !ss(&mut dst, &mut dst_len, b"$") {
        return ARGON2_ENCODING_FAIL;
    }
    if !sb(&mut dst, &mut dst_len, (*ctx).salt, (*ctx).saltlen) {
        return ARGON2_ENCODING_FAIL;
    }

    if !ss(&mut dst, &mut dst_len, b"$") {
        return ARGON2_ENCODING_FAIL;
    }
    if !sb(&mut dst, &mut dst_len, (*ctx).out, (*ctx).outlen) {
        return ARGON2_ENCODING_FAIL;
    }

    ARGON2_OK
}
