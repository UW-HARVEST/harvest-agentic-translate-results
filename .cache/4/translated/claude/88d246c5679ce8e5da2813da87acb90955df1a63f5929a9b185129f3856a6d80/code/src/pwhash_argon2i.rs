//! Translation of `crypto_pwhash/argon2/pwhash_argon2i.c`

use crate::common::*;
use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use core::ptr;

/* ---------------------------------------------------------------- */
/* errno                                                             */
/* ---------------------------------------------------------------- */
const EINVAL: c_int = 22;
const EFBIG: c_int = 27;

/* ---------------------------------------------------------------- */
/* crypto_pwhash_argon2i.h constants                                 */
/* ---------------------------------------------------------------- */
const crypto_pwhash_argon2i_ALG_ARGON2I13: c_int = 1;
const crypto_pwhash_argon2i_BYTES_MIN: usize = 16;
/* SODIUM_MIN(SODIUM_SIZE_MAX, 4294967295U) == 4294967295 on x86-64 */
const crypto_pwhash_argon2i_BYTES_MAX: usize = 4294967295;
const crypto_pwhash_argon2i_PASSWD_MIN: usize = 0;
const crypto_pwhash_argon2i_PASSWD_MAX: usize = 4294967295;
const crypto_pwhash_argon2i_SALTBYTES: usize = 16;
const crypto_pwhash_argon2i_STRBYTES: usize = 128;
const crypto_pwhash_argon2i_STRPREFIX: &[u8; 10] = b"$argon2i$\0";
const crypto_pwhash_argon2i_OPSLIMIT_MIN: c_ulonglong = 3;
const crypto_pwhash_argon2i_OPSLIMIT_MAX: c_ulonglong = 4294967295;
const crypto_pwhash_argon2i_MEMLIMIT_MIN: usize = 8192;
/* SIZE_MAX >= 4398046510080U on x86-64 */
const crypto_pwhash_argon2i_MEMLIMIT_MAX: usize = 4398046510080;
const crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE: c_ulonglong = 4;
const crypto_pwhash_argon2i_MEMLIMIT_INTERACTIVE: usize = 33554432;
const crypto_pwhash_argon2i_OPSLIMIT_MODERATE: c_ulonglong = 6;
const crypto_pwhash_argon2i_MEMLIMIT_MODERATE: usize = 134217728;
const crypto_pwhash_argon2i_OPSLIMIT_SENSITIVE: c_ulonglong = 8;
const crypto_pwhash_argon2i_MEMLIMIT_SENSITIVE: usize = 536870912;

/* crypto_pwhash_STRBYTES == crypto_pwhash_argon2id_STRBYTES */
const crypto_pwhash_STRBYTES: usize = 128;

const STR_HASHBYTES: usize = 32;

/* argon2.h */
const ARGON2_OK: c_int = 0;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

/* enum Argon2_type { Argon2_i = 1, Argon2_id = 2 } */
const Argon2_i: c_uint = 1;
const Argon2_id: c_uint = 2;

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
    /* argon2.c */
    fn _sodium_argon2i_hash_raw(
        t_cost: u32,
        m_cost: u32,
        parallelism: u32,
        pwd: *const c_void,
        pwdlen: usize,
        salt: *const c_void,
        saltlen: usize,
        hash: *mut c_void,
        hashlen: usize,
    ) -> c_int;
    fn _sodium_argon2i_hash_encoded(
        t_cost: u32,
        m_cost: u32,
        parallelism: u32,
        pwd: *const c_void,
        pwdlen: usize,
        salt: *const c_void,
        saltlen: usize,
        hashlen: usize,
        encoded: *mut c_char,
        encodedlen: usize,
    ) -> c_int;
    fn _sodium_argon2i_verify(
        encoded: *const c_char,
        pwd: *const c_void,
        pwdlen: usize,
    ) -> c_int;

    /* argon2-encoding.c */
    fn _sodium_argon2_decode_string(
        ctx: *mut argon2_context,
        str_: *const c_char,
        type_: c_uint,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);

    /* libc */
    fn __errno_location() -> *mut c_int;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_alg_argon2i13() -> c_int {
    crypto_pwhash_argon2i_ALG_ARGON2I13
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_min() -> usize {
    crypto_pwhash_argon2i_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_max() -> usize {
    crypto_pwhash_argon2i_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_min() -> usize {
    crypto_pwhash_argon2i_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_max() -> usize {
    crypto_pwhash_argon2i_PASSWD_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_saltbytes() -> usize {
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
    crypto_pwhash_argon2i_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_max() -> c_ulonglong {
    crypto_pwhash_argon2i_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_min() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_max() -> usize {
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
    memset(out, 0, outlen as usize);
    if outlen > crypto_pwhash_argon2i_BYTES_MAX as c_ulonglong {
        *__errno_location() = EFBIG; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if outlen < crypto_pwhash_argon2i_BYTES_MIN as c_ulonglong {
        *__errno_location() = EINVAL;
        return -1;
    }
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX as c_ulonglong
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        *__errno_location() = EFBIG;
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN as c_ulonglong
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        *__errno_location() = EINVAL;
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        *__errno_location() = EINVAL; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    match alg {
        crypto_pwhash_argon2i_ALG_ARGON2I13 => {
            if _sodium_argon2i_hash_raw(
                opslimit as u32,
                (memlimit / 1024usize) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                crypto_pwhash_argon2i_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            ) != ARGON2_OK
            {
                return -1; /* LCOV_EXCL_LINE */
            }
            0
        }
        _ => {
            *__errno_location() = EINVAL;
            -1
        }
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
    let mut salt: [u8; crypto_pwhash_argon2i_SALTBYTES] = [0; crypto_pwhash_argon2i_SALTBYTES];

    memset(out as *mut u8, 0, crypto_pwhash_argon2i_STRBYTES);
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX as c_ulonglong
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        *__errno_location() = EFBIG;
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN as c_ulonglong
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        *__errno_location() = EINVAL;
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if _sodium_argon2i_hash_encoded(
        opslimit as u32,
        (memlimit / 1024usize) as u32,
        1u32,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        salt.len(),
        STR_HASHBYTES,
        out,
        crypto_pwhash_argon2i_STRBYTES,
    ) != ARGON2_OK
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
) -> c_int {
    let verify_ret: c_int;

    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX as c_ulonglong {
        *__errno_location() = EFBIG;
        return -1;
    }
    /* LCOV_EXCL_START */
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN as c_ulonglong {
        *__errno_location() = EINVAL;
        return -1;
    }
    /* LCOV_EXCL_STOP */

    verify_ret = _sodium_argon2i_verify(str_, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        *__errno_location() = EINVAL;
    }
    -1
}

unsafe fn _needs_rehash(
    str_: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
    type_: c_uint,
) -> c_int {
    let fodder: *mut u8;
    let mut ctx: argon2_context;
    let fodder_len: usize;
    let mut ret: c_int = -1;
    let mut memlimit: usize = memlimit;

    fodder_len = strlen(str_);
    memlimit /= 1024usize;
    if opslimit > 4294967295u64
        || memlimit > 4294967295usize
        || fodder_len >= crypto_pwhash_STRBYTES
    {
        *__errno_location() = EINVAL;
        return -1;
    }
    ctx = core::mem::zeroed();
    fodder = calloc(fodder_len, 1usize) as *mut u8;
    if fodder.is_null() {
        return -1; /* LCOV_EXCL_LINE */
    }
    ctx.salt = fodder;
    ctx.pwd = fodder;
    ctx.out = fodder;
    ctx.saltlen = fodder_len as u32;
    ctx.pwdlen = fodder_len as u32;
    ctx.outlen = fodder_len as u32;
    ctx.secret = ptr::null_mut();
    ctx.ad = ptr::null_mut();
    ctx.secretlen = 0;
    ctx.adlen = 0;
    if _sodium_argon2_decode_string(&mut ctx, str_, type_) != 0 {
        *__errno_location() = EINVAL;
        ret = -1;
    } else if ctx.t_cost != opslimit as u32 || ctx.m_cost != memlimit as u32 {
        ret = 1;
    } else {
        ret = 0;
    }
    free(fodder as *mut c_void);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_needs_rehash(
    str_: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    _needs_rehash(str_, opslimit, memlimit, Argon2_i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_needs_rehash(
    str_: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    _needs_rehash(str_, opslimit, memlimit, Argon2_id)
}
