//! Translation of `crypto_pwhash/argon2/argon2-encoding.c`.
//!
//! `private/quirks.h` renames `argon2_decode_string` / `argon2_encode_string`
//! to `_sodium_argon2_decode_string` / `_sodium_argon2_encode_string`.
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_ulong};

use crate::common::{memcpy, strlen};
use crate::crypto_pwhash::argon2::argon2::*;
use crate::crypto_pwhash::argon2::argon2_core::{
    _sodium_argon2_validate_inputs, ARGON2_VERSION_NUMBER,
};

/// `#define sodium_base64_VARIANT_ORIGINAL_NO_PADDING 3`
const sodium_base64_VARIANT_ORIGINAL_NO_PADDING: c_int = 3;

unsafe extern "C" {
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
}

/// `strncmp(str, prefix, strlen(prefix)) == 0`, with `prefix` NUL-free.
///
/// Like `strncmp()` this stops at the first difference, so a short `str`
/// (whose terminating NUL differs from `prefix[i]`) is never overread.
#[inline]
unsafe fn strncmp_prefix_eq(str: *const c_char, prefix: &[u8]) -> bool {
    let mut i: usize = 0;
    while i < prefix.len() {
        if (unsafe { *str.add(i) } as u8) != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// ```c
/// static const char *decode_decimal(const char *str, unsigned long *v)
/// ```
///
/// Decode decimal integer from 'str'; the value is written in '*v'.
/// Returned value is a pointer to the next non-decimal character in the
/// string. If there is no digit at all, or the value encoding is not
/// minimal (extra leading zeros), or the value does not fit in an
/// 'unsigned long', then NULL is returned.
unsafe fn decode_decimal(mut str: *const c_char, v: *mut c_ulong) -> *const c_char {
    let orig: *const c_char;
    let mut acc: c_ulong;

    acc = 0;
    orig = str;
    loop {
        let mut c: c_int;

        c = unsafe { *str } as c_int;
        if c < ('0' as c_int) || c > ('9' as c_int) {
            break;
        }
        c -= '0' as c_int;
        if acc > (c_ulong::MAX / 10) {
            return core::ptr::null(); /* LCOV_EXCL_LINE */
        }
        acc = acc.wrapping_mul(10);
        if (c as c_ulong) > (c_ulong::MAX - acc) {
            return core::ptr::null(); /* LCOV_EXCL_LINE */
        }
        acc = acc.wrapping_add(c as c_ulong);
        str = unsafe { str.add(1) };
    }
    if str == orig || (unsafe { *orig } == ('0' as c_char) && str != unsafe { orig.add(1) }) {
        return core::ptr::null(); /* LCOV_EXCL_LINE */
    }
    unsafe { *v = acc };
    str
}

/// `int argon2_decode_string(argon2_context *ctx, const char *str, argon2_type type)`
///
/// Decode an Argon2i hash string into the provided structure 'ctx'.
/// Returned value is ARGON2_OK on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_decode_string(
    ctx: *mut argon2_context,
    str: *const c_char,
    type_: argon2_type,
) -> c_int {
    let mut str: *const c_char = str;

    /* Prefix checking */
    macro_rules! CC {
        ($prefix:expr) => {{
            /* size_t cc_len = strlen(prefix); */
            if !unsafe { strncmp_prefix_eq(str, $prefix) } {
                return ARGON2_DECODING_FAIL;
            }
            str = unsafe { str.add($prefix.len()) };
        }};
    }

    /* Decoding prefix into uint32_t decimal (the value is the macro result) */
    macro_rules! DECIMAL_U32 {
        () => {{
            let mut dec_x: c_ulong = 0;
            str = unsafe { decode_decimal(str, &mut dec_x) };
            if str.is_null() || dec_x > u32::MAX as c_ulong {
                return ARGON2_DECODING_FAIL;
            }
            dec_x as u32
        }};
    }

    /* Decoding base64 into a binary buffer (the length is the macro result) */
    macro_rules! BIN {
        ($buf:expr, $max_len:expr) => {{
            let mut bin_len: usize = $max_len;
            let mut str_end: *const c_char = core::ptr::null();
            if unsafe {
                sodium_base642bin(
                    $buf,
                    $max_len,
                    str,
                    strlen(str),
                    core::ptr::null(),
                    &mut bin_len,
                    &mut str_end,
                    sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
                )
            } != 0
                || bin_len > u32::MAX as usize
            {
                return ARGON2_DECODING_FAIL;
            }
            str = str_end;
            bin_len as u32
        }};
    }

    let maxsaltlen: usize = unsafe { (*ctx).saltlen } as usize;
    let maxoutlen: usize = unsafe { (*ctx).outlen } as usize;
    let validation_result: c_int;
    let version: u32;

    unsafe { (*ctx).saltlen = 0 };
    unsafe { (*ctx).outlen = 0 };

    if type_ == Argon2_id {
        CC!(b"$argon2id");
    } else if type_ == Argon2_i {
        CC!(b"$argon2i");
    } else {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    CC!(b"$v=");
    version = DECIMAL_U32!();
    if version != ARGON2_VERSION_NUMBER {
        return ARGON2_INCORRECT_TYPE;
    }
    CC!(b"$m=");
    unsafe { (*ctx).m_cost = DECIMAL_U32!() };
    if unsafe { (*ctx).m_cost } as u64 > u32::MAX as u64 {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    CC!(b",t=");
    unsafe { (*ctx).t_cost = DECIMAL_U32!() };
    if unsafe { (*ctx).t_cost } as u64 > u32::MAX as u64 {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    CC!(b",p=");
    unsafe { (*ctx).lanes = DECIMAL_U32!() };
    if unsafe { (*ctx).lanes } as u64 > u32::MAX as u64 {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    unsafe { (*ctx).threads = (*ctx).lanes };

    CC!(b"$");
    unsafe { (*ctx).saltlen = BIN!((*ctx).salt, maxsaltlen) };
    CC!(b"$");
    unsafe { (*ctx).outlen = BIN!((*ctx).out, maxoutlen) };
    validation_result = unsafe { _sodium_argon2_validate_inputs(ctx) };
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    if unsafe { *str } == 0 {
        return ARGON2_OK;
    }
    ARGON2_DECODING_FAIL
}

/// `#define U32_STR_MAXSIZE 11U`
const U32_STR_MAXSIZE: usize = 11;

/// `static void u32_to_string(char *str, uint32_t x)`
unsafe fn u32_to_string(str: *mut c_char, mut x: u32) {
    let mut tmp: [u8; U32_STR_MAXSIZE - 1] = [0u8; U32_STR_MAXSIZE - 1];
    let mut i: usize;

    i = U32_STR_MAXSIZE - 1; /* sizeof tmp */
    loop {
        i -= 1;
        tmp[i] = ((x % 10u32) as u8).wrapping_add(b'0');
        x /= 10u32;
        if !(x != 0 && i != 0) {
            break;
        }
    }
    unsafe {
        memcpy(
            str as *mut u8,
            tmp.as_ptr().add(i),
            (U32_STR_MAXSIZE - 1) - i,
        )
    };
    unsafe { *str.add((U32_STR_MAXSIZE - 1) - i) = 0 };
}

/// `int argon2_encode_string(char *dst, size_t dst_len, argon2_context *ctx,
///                          argon2_type type)`
///
/// Encode an argon2i hash string into the provided buffer. 'dst_len'
/// contains the size, in characters, of the 'dst' buffer; if 'dst_len'
/// is less than the number of required characters (including the
/// terminating 0), then this function returns 0.
///
/// On success, ARGON2_OK is returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_encode_string(
    dst: *mut c_char,
    dst_len: usize,
    ctx: *mut argon2_context,
    type_: argon2_type,
) -> c_int {
    let mut dst: *mut c_char = dst;
    let mut dst_len: usize = dst_len;

    /* `SS(str)`: append the NUL-terminated literal `str` */
    macro_rules! SS {
        ($s:expr) => {{
            /* size_t pp_len = strlen(str); */
            let pp_len: usize = $s.len() - 1;
            if pp_len >= dst_len {
                return ARGON2_ENCODING_FAIL;
            }
            unsafe { memcpy(dst as *mut u8, $s.as_ptr(), pp_len + 1) };
            dst = unsafe { dst.add(pp_len) };
            dst_len = dst_len.wrapping_sub(pp_len);
        }};
    }

    /* `SX(x)`: append the decimal representation of the uint32_t `x` */
    macro_rules! SX {
        ($x:expr) => {{
            let mut tmp: [c_char; U32_STR_MAXSIZE] = [0; U32_STR_MAXSIZE];
            unsafe { u32_to_string(tmp.as_mut_ptr(), $x) };
            /* SS(tmp); */
            let pp_len: usize = unsafe { strlen(tmp.as_ptr()) };
            if pp_len >= dst_len {
                return ARGON2_ENCODING_FAIL;
            }
            unsafe { memcpy(dst as *mut u8, tmp.as_ptr() as *const u8, pp_len + 1) };
            dst = unsafe { dst.add(pp_len) };
            dst_len = dst_len.wrapping_sub(pp_len);
        }};
    }

    /* `SB(buf, len)`: append base64(buf[0 .. len]) */
    macro_rules! SB {
        ($buf:expr, $len:expr) => {{
            let sb_len: usize;
            if unsafe {
                sodium_bin2base64(
                    dst,
                    dst_len,
                    $buf,
                    $len,
                    sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
                )
            }
            .is_null()
            {
                return ARGON2_ENCODING_FAIL;
            }
            sb_len = unsafe { strlen(dst) };
            dst = unsafe { dst.add(sb_len) };
            dst_len = dst_len.wrapping_sub(sb_len);
        }};
    }

    let validation_result: c_int;

    if type_ == Argon2_id {
        SS!(b"$argon2id$v=\0");
    } else if type_ == Argon2_i {
        SS!(b"$argon2i$v=\0");
    } else {
        return ARGON2_ENCODING_FAIL; /* LCOV_EXCL_LINE */
    }
    validation_result = unsafe { _sodium_argon2_validate_inputs(ctx) };
    if validation_result != ARGON2_OK {
        return validation_result; /* LCOV_EXCL_LINE */
    }
    SX!(ARGON2_VERSION_NUMBER);
    SS!(b"$m=\0");
    SX!(unsafe { (*ctx).m_cost });
    SS!(b",t=\0");
    SX!(unsafe { (*ctx).t_cost });
    SS!(b",p=\0");
    SX!(unsafe { (*ctx).lanes });

    SS!(b"$\0");
    SB!(unsafe { (*ctx).salt }, unsafe { (*ctx).saltlen } as usize);

    SS!(b"$\0");
    SB!(unsafe { (*ctx).out }, unsafe { (*ctx).outlen } as usize);
    ARGON2_OK
}
