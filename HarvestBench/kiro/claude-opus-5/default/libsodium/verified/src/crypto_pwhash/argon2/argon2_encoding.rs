//! Translation of `crypto_pwhash/argon2/argon2-encoding.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::common::memcpy;
use crate::sodium_codecs::{sodium_base642bin, sodium_bin2base64};

use super::argon2_core::*;

/* sodium_base64_VARIANT_ORIGINAL_NO_PADDING == 3 (utils.h) */
const SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING: c_int = 3;

/* ULONG_MAX on LP64 */
const ULONG_MAX: u64 = u64::MAX;

/* ---- local C string helpers ---- */
unsafe fn strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/* strncmp semantics on signed char (target has signed char). */
unsafe fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int {
    let mut i: usize = 0;
    while i < n {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 != c2 {
            /* compare as unsigned char, per C standard */
            return (c1 as u8 as c_int) - (c2 as u8 as c_int);
        }
        if c1 == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/*
 * Decode decimal integer from 'str'; the value is written in '*v'.
 * Returns a pointer to the next non-decimal character, or NULL on failure.
 */
unsafe fn decode_decimal(str: *const c_char, v: *mut u64) -> *const c_char {
    let orig: *const c_char;
    let mut acc: u64;
    let mut s = str;

    acc = 0;
    orig = s;
    loop {
        let c_char_val = *s;
        let mut c: c_int = c_char_val as c_int;
        if c < ('0' as c_int) || c > ('9' as c_int) {
            break;
        }
        c -= '0' as c_int;
        if acc > (ULONG_MAX / 10) {
            return core::ptr::null();
        }
        acc = acc.wrapping_mul(10);
        if (c as u64) > (ULONG_MAX - acc) {
            return core::ptr::null();
        }
        acc = acc.wrapping_add(c as u64);
        s = s.add(1);
    }
    if s == orig || (*orig == ('0' as c_char) && s != orig.add(1)) {
        return core::ptr::null();
    }
    *v = acc;
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_decode_string(
    ctx: *mut argon2_context,
    str: *const c_char,
    type_: argon2_type,
) -> c_int {
    let mut str = str;

    /* CC(prefix): compare literal prefix. */
    macro_rules! cc {
        ($prefix:expr) => {{
            let prefix: &[u8] = $prefix;
            let cc_len = prefix.len();
            if strncmp(str, prefix.as_ptr() as *const c_char, cc_len) != 0 {
                return ARGON2_DECODING_FAIL;
            }
            str = str.add(cc_len);
        }};
    }

    /* DECIMAL_U32(x) */
    macro_rules! decimal_u32 {
        ($x:expr) => {{
            let mut dec_x: u64 = 0;
            str = decode_decimal(str, &mut dec_x);
            if str.is_null() || dec_x > u32::MAX as u64 {
                return ARGON2_DECODING_FAIL;
            }
            $x = dec_x as u32;
        }};
    }

    /* BIN(buf, max_len, len) */
    macro_rules! bin {
        ($buf:expr, $max_len:expr, $len:expr) => {{
            let mut bin_len: usize = $max_len;
            let mut str_end: *const c_char = core::ptr::null();
            if sodium_base642bin(
                $buf,
                $max_len,
                str,
                strlen(str),
                core::ptr::null(),
                &mut bin_len,
                &mut str_end,
                SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING,
            ) != 0
                || bin_len > u32::MAX as usize
            {
                return ARGON2_DECODING_FAIL;
            }
            $len = bin_len as u32;
            str = str_end;
        }};
    }

    let maxsaltlen: usize = (*ctx).saltlen as usize;
    let maxoutlen: usize = (*ctx).outlen as usize;
    let validation_result: c_int;
    let mut version: u32 = 0;

    (*ctx).saltlen = 0;
    (*ctx).outlen = 0;

    if type_ == Argon2_id {
        cc!(b"$argon2id");
    } else if type_ == Argon2_i {
        cc!(b"$argon2i");
    } else {
        return ARGON2_INCORRECT_TYPE;
    }
    cc!(b"$v=");
    decimal_u32!(version);
    if version != ARGON2_VERSION_NUMBER {
        return ARGON2_INCORRECT_TYPE;
    }
    cc!(b"$m=");
    decimal_u32!((*ctx).m_cost);
    if (*ctx).m_cost > u32::MAX {
        return ARGON2_INCORRECT_TYPE;
    }
    cc!(b",t=");
    decimal_u32!((*ctx).t_cost);
    if (*ctx).t_cost > u32::MAX {
        return ARGON2_INCORRECT_TYPE;
    }
    cc!(b",p=");
    decimal_u32!((*ctx).lanes);
    if (*ctx).lanes > u32::MAX {
        return ARGON2_INCORRECT_TYPE;
    }
    (*ctx).threads = (*ctx).lanes;

    cc!(b"$");
    bin!((*ctx).salt, maxsaltlen, (*ctx).saltlen);
    cc!(b"$");
    bin!((*ctx).out, maxoutlen, (*ctx).outlen);
    validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    if *str == 0 {
        return ARGON2_OK;
    }
    ARGON2_DECODING_FAIL
}

const U32_STR_MAXSIZE: usize = 11;

unsafe fn u32_to_string(str: *mut c_char, mut x: u32) {
    let mut tmp: [c_char; U32_STR_MAXSIZE - 1] = [0; U32_STR_MAXSIZE - 1];
    let mut i: usize;

    i = core::mem::size_of_val(&tmp); /* sizeof tmp == 10 */
    loop {
        i -= 1;
        tmp[i] = ((x % 10u32) as u8 as c_char).wrapping_add('0' as c_char);
        x /= 10u32;
        if !(x != 0 && i != 0) {
            break;
        }
    }
    memcpy(
        str as *mut u8,
        tmp.as_ptr().add(i) as *const u8,
        core::mem::size_of_val(&tmp) - i,
    );
    *str.add(core::mem::size_of_val(&tmp) - i) = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_encode_string(
    dst: *mut c_char,
    dst_len: usize,
    ctx: *mut argon2_context,
    type_: argon2_type,
) -> c_int {
    let mut dst = dst;
    let mut dst_len = dst_len;

    /* SS(str): append NUL-terminated string. */
    macro_rules! ss {
        ($s:expr) => {{
            let s: &[u8] = $s;
            let pp_len = s.len(); /* strlen of the literal (no trailing NUL in slice) */
            if pp_len >= dst_len {
                return ARGON2_ENCODING_FAIL;
            }
            /* memcpy(dst, str, pp_len + 1): copies the terminating NUL too. */
            memcpy(dst as *mut u8, s.as_ptr(), pp_len);
            *dst.add(pp_len) = 0;
            dst = dst.add(pp_len);
            dst_len -= pp_len;
        }};
    }

    /* SX(x) */
    macro_rules! sx {
        ($x:expr) => {{
            let mut tmp: [c_char; U32_STR_MAXSIZE] = [0; U32_STR_MAXSIZE];
            u32_to_string(tmp.as_mut_ptr(), $x);
            /* SS(tmp): tmp is NUL-terminated; compute its length. */
            let pp_len = strlen(tmp.as_ptr());
            if pp_len >= dst_len {
                return ARGON2_ENCODING_FAIL;
            }
            memcpy(dst as *mut u8, tmp.as_ptr() as *const u8, pp_len + 1);
            dst = dst.add(pp_len);
            dst_len -= pp_len;
        }};
    }

    /* SB(buf, len) */
    macro_rules! sb {
        ($buf:expr, $len:expr) => {{
            let sb_len: usize;
            if sodium_bin2base64(
                dst,
                dst_len,
                $buf,
                $len,
                SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING,
            )
            .is_null()
            {
                return ARGON2_ENCODING_FAIL;
            }
            sb_len = strlen(dst);
            dst = dst.add(sb_len);
            dst_len -= sb_len;
        }};
    }

    let validation_result: c_int;

    match type_ {
        x if x == Argon2_id => ss!(b"$argon2id$v="),
        x if x == Argon2_i => ss!(b"$argon2i$v="),
        _ => return ARGON2_ENCODING_FAIL,
    }
    validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    sx!(ARGON2_VERSION_NUMBER);
    ss!(b"$m=");
    sx!((*ctx).m_cost);
    ss!(b",t=");
    sx!((*ctx).t_cost);
    ss!(b",p=");
    sx!((*ctx).lanes);

    ss!(b"$");
    sb!((*ctx).salt, (*ctx).saltlen as usize);

    ss!(b"$");
    sb!((*ctx).out, (*ctx).outlen as usize);
    ARGON2_OK
}
