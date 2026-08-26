//! Translation of `crypto_pwhash/argon2/pwhash_argon2i.c` plus the constants
//! of `include/sodium/crypto_pwhash_argon2i.h`.
//!
//! Note that `crypto_pwhash_argon2id_str_needs_rehash()` is defined in this C
//! file too (both front-ends share the `_needs_rehash()` helper).
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_ulonglong, c_void};

use crate::common::{calloc, free, memset, strlen, set_errno, EINVAL};
use crate::crypto_pwhash::argon2::argon2::*;
use crate::crypto_pwhash::argon2::argon2_encoding::_sodium_argon2_decode_string;
use crate::randombytes::randombytes_buf;

/// `#define EFBIG 27` (Linux)
const EFBIG: c_int = 27;

/// `#define STR_HASHBYTES 32U`
const STR_HASHBYTES: usize = 32;

// ---------------------------------------------------------------------------
// include/sodium/crypto_pwhash_argon2i.h
// ---------------------------------------------------------------------------

pub const crypto_pwhash_argon2i_ALG_ARGON2I13: c_int = 1;
pub const crypto_pwhash_argon2i_BYTES_MIN: usize = 16;
/// `SODIUM_MIN(SODIUM_SIZE_MAX, 4294967295U)`
pub const crypto_pwhash_argon2i_BYTES_MAX: u64 = 4294967295;
pub const crypto_pwhash_argon2i_PASSWD_MIN: usize = 0;
pub const crypto_pwhash_argon2i_PASSWD_MAX: usize = 4294967295;
pub const crypto_pwhash_argon2i_SALTBYTES: usize = 16;
pub const crypto_pwhash_argon2i_STRBYTES: usize = 128;
pub const crypto_pwhash_argon2i_STRPREFIX: &[u8; 10] = b"$argon2i$\0";
pub const crypto_pwhash_argon2i_OPSLIMIT_MIN: c_ulonglong = 3;
pub const crypto_pwhash_argon2i_OPSLIMIT_MAX: c_ulonglong = 4294967295;
pub const crypto_pwhash_argon2i_MEMLIMIT_MIN: usize = 8192;
/// `((SIZE_MAX >= 4398046510080U) ? 4398046510080U : ...)`
pub const crypto_pwhash_argon2i_MEMLIMIT_MAX: usize = 4398046510080;
pub const crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE: c_ulonglong = 4;
pub const crypto_pwhash_argon2i_MEMLIMIT_INTERACTIVE: usize = 33554432;
pub const crypto_pwhash_argon2i_OPSLIMIT_MODERATE: c_ulonglong = 6;
pub const crypto_pwhash_argon2i_MEMLIMIT_MODERATE: usize = 134217728;
pub const crypto_pwhash_argon2i_OPSLIMIT_SENSITIVE: c_ulonglong = 8;
pub const crypto_pwhash_argon2i_MEMLIMIT_SENSITIVE: usize = 536870912;

/// `#define crypto_pwhash_STRBYTES crypto_pwhash_argon2id_STRBYTES` == 128U
const crypto_pwhash_STRBYTES: usize = 128;

// ---------------------------------------------------------------------------
// pwhash_argon2i.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_alg_argon2i13() -> c_int {
    crypto_pwhash_argon2i_ALG_ARGON2I13
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_min() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_BYTES_MIN >= ARGON2_MIN_OUTLEN); */
    const _: () = assert!(crypto_pwhash_argon2i_BYTES_MIN as u64 >= ARGON2_MIN_OUTLEN as u64);
    crypto_pwhash_argon2i_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_max() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_BYTES_MAX <= ARGON2_MAX_OUTLEN); */
    const _: () = assert!(crypto_pwhash_argon2i_BYTES_MAX <= ARGON2_MAX_OUTLEN as u64);
    crypto_pwhash_argon2i_BYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_min() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_PASSWD_MIN >= ARGON2_MIN_PWD_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2i_PASSWD_MIN as u64 >= ARGON2_MIN_PWD_LENGTH as u64);
    crypto_pwhash_argon2i_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_max() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_PASSWD_MAX <= ARGON2_MAX_PWD_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2i_PASSWD_MAX as u64 <= ARGON2_MAX_PWD_LENGTH as u64);
    crypto_pwhash_argon2i_PASSWD_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_saltbytes() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_SALTBYTES >= ARGON2_MIN_SALT_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2i_SALTBYTES as u64 >= ARGON2_MIN_SALT_LENGTH as u64);
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_SALTBYTES <= ARGON2_MAX_SALT_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2i_SALTBYTES as u64 <= ARGON2_MAX_SALT_LENGTH as u64);
    crypto_pwhash_argon2i_SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_strbytes() -> usize {
    crypto_pwhash_argon2i_STRBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_strprefix() -> *const c_char {
    crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_min() -> c_ulonglong {
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_OPSLIMIT_MIN >= ARGON2_MIN_TIME); */
    const _: () = assert!(crypto_pwhash_argon2i_OPSLIMIT_MIN >= ARGON2_MIN_TIME as c_ulonglong);
    crypto_pwhash_argon2i_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_max() -> c_ulonglong {
    /* COMPILER_ASSERT(crypto_pwhash_argon2i_OPSLIMIT_MAX <= ARGON2_MAX_TIME); */
    const _: () = assert!(crypto_pwhash_argon2i_OPSLIMIT_MAX <= ARGON2_MAX_TIME as c_ulonglong);
    crypto_pwhash_argon2i_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_min() -> usize {
    /* COMPILER_ASSERT((crypto_pwhash_argon2i_MEMLIMIT_MIN / 1024U) >= ARGON2_MIN_MEMORY); */
    const _: () =
        assert!((crypto_pwhash_argon2i_MEMLIMIT_MIN / 1024) as u64 >= ARGON2_MIN_MEMORY as u64);
    crypto_pwhash_argon2i_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_max() -> usize {
    /* COMPILER_ASSERT((crypto_pwhash_argon2i_MEMLIMIT_MAX / 1024U) <= ARGON2_MAX_MEMORY); */
    const _: () = assert!((crypto_pwhash_argon2i_MEMLIMIT_MAX / 1024) as u64 <= ARGON2_MAX_MEMORY);
    crypto_pwhash_argon2i_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_interactive() -> c_ulonglong {
    crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_interactive() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_moderate() -> c_ulonglong {
    crypto_pwhash_argon2i_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_moderate() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_sensitive() -> c_ulonglong {
    crypto_pwhash_argon2i_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_sensitive() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i(
    out: *mut u8,
    outlen: c_ulonglong,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    salt: *const u8,
    opslimit: c_ulonglong,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    unsafe { memset(out, 0, outlen as usize) };
    if outlen > crypto_pwhash_argon2i_BYTES_MAX {
        set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if outlen < crypto_pwhash_argon2i_BYTES_MIN as c_ulonglong {
        set_errno(EINVAL);
        return -1;
    }
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX as c_ulonglong
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN as c_ulonglong
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        set_errno(EINVAL);
        return -1;
    }
    if (out as *const c_void) == (passwd as *const c_void) {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    /* switch (alg) { case crypto_pwhash_argon2i_ALG_ARGON2I13: ... default: ... } */
    if alg == crypto_pwhash_argon2i_ALG_ARGON2I13 {
        if unsafe {
            _sodium_argon2i_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                crypto_pwhash_argon2i_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            )
        } != ARGON2_OK
        {
            return -1; /* LCOV_EXCL_LINE */
        }
        0
    } else {
        set_errno(EINVAL);
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; crypto_pwhash_argon2i_SALTBYTES] = [0u8; crypto_pwhash_argon2i_SALTBYTES];

    unsafe { memset(out as *mut u8, 0, crypto_pwhash_argon2i_STRBYTES) };
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX as c_ulonglong
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN as c_ulonglong
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        set_errno(EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, crypto_pwhash_argon2i_SALTBYTES);
    if unsafe {
        _sodium_argon2i_hash_encoded(
            opslimit as u32,
            (memlimit / 1024) as u32,
            1u32,
            passwd as *const c_void,
            passwdlen as usize,
            salt.as_ptr() as *const c_void,
            crypto_pwhash_argon2i_SALTBYTES,
            STR_HASHBYTES,
            out,
            crypto_pwhash_argon2i_STRBYTES,
        )
    } != ARGON2_OK
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_verify(
    str: *const c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
) -> c_int {
    let verify_ret: c_int;

    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX as c_ulonglong {
        set_errno(EFBIG);
        return -1;
    }
    /* LCOV_EXCL_START */
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN as c_ulonglong {
        set_errno(EINVAL);
        return -1;
    }
    /* LCOV_EXCL_STOP */

    verify_ret =
        unsafe { _sodium_argon2i_verify(str, passwd as *const c_void, passwdlen as usize) };
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        set_errno(EINVAL);
    }
    -1
}

/// `static int _needs_rehash(const char *str, unsigned long long opslimit,
///                          size_t memlimit, argon2_type type)`
unsafe fn _needs_rehash(
    str: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
    type_: argon2_type,
) -> c_int {
    let fodder: *mut u8;
    let mut ctx: argon2_context;
    let fodder_len: usize;
    let mut ret: c_int = -1;
    let mut memlimit: usize = memlimit;

    fodder_len = unsafe { strlen(str) };
    memlimit /= 1024;
    if opslimit > u32::MAX as c_ulonglong
        || memlimit > u32::MAX as usize
        || fodder_len >= crypto_pwhash_STRBYTES
    {
        set_errno(EINVAL);
        return -1;
    }
    ctx = argon2_context::zeroed();
    fodder = unsafe { calloc(fodder_len, 1) } as *mut u8;
    if fodder.is_null() {
        return -1; /* LCOV_EXCL_LINE */
    }
    ctx.salt = fodder;
    ctx.pwd = fodder;
    ctx.out = fodder;
    ctx.saltlen = fodder_len as u32;
    ctx.pwdlen = fodder_len as u32;
    ctx.outlen = fodder_len as u32;
    ctx.secret = core::ptr::null_mut();
    ctx.ad = core::ptr::null_mut();
    ctx.secretlen = 0;
    ctx.adlen = 0;
    if unsafe { _sodium_argon2_decode_string(&mut ctx, str, type_) } != 0 {
        set_errno(EINVAL);
        ret = -1;
    } else if ctx.t_cost != opslimit as u32 || ctx.m_cost != memlimit as u32 {
        ret = 1;
    } else {
        ret = 0;
    }
    unsafe { free(fodder as *mut c_void) };

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_needs_rehash(
    str: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    unsafe { _needs_rehash(str, opslimit, memlimit, Argon2_i) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_needs_rehash(
    str: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    unsafe { _needs_rehash(str, opslimit, memlimit, Argon2_id) }
}
