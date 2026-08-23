//! Translation of `crypto_pwhash/argon2/pwhash_argon2id.c` plus the constants
//! of `include/sodium/crypto_pwhash_argon2id.h`.
//!
//! `crypto_pwhash_argon2id_str_needs_rehash()` is *not* here: the C source
//! defines it in `pwhash_argon2i.c`.

use core::ffi::{c_char, c_int, c_ulonglong, c_void};

use crate::common::{memset, set_errno, EINVAL};
use crate::crypto_pwhash::argon2::argon2::*;
use crate::randombytes::randombytes_buf;

/// `#define EFBIG 27` (Linux)
const EFBIG: c_int = 27;

/// `#define STR_HASHBYTES 32U`
const STR_HASHBYTES: usize = 32;

// ---------------------------------------------------------------------------
// include/sodium/crypto_pwhash_argon2id.h
// ---------------------------------------------------------------------------

pub const crypto_pwhash_argon2id_ALG_ARGON2ID13: c_int = 2;
pub const crypto_pwhash_argon2id_BYTES_MIN: usize = 16;
/// `SODIUM_MIN(SODIUM_SIZE_MAX, 4294967295U)`
pub const crypto_pwhash_argon2id_BYTES_MAX: u64 = 4294967295;
pub const crypto_pwhash_argon2id_PASSWD_MIN: usize = 0;
pub const crypto_pwhash_argon2id_PASSWD_MAX: usize = 4294967295;
pub const crypto_pwhash_argon2id_SALTBYTES: usize = 16;
pub const crypto_pwhash_argon2id_STRBYTES: usize = 128;
pub const crypto_pwhash_argon2id_STRPREFIX: &[u8; 11] = b"$argon2id$\0";
pub const crypto_pwhash_argon2id_OPSLIMIT_MIN: c_ulonglong = 1;
pub const crypto_pwhash_argon2id_OPSLIMIT_MAX: c_ulonglong = 4294967295;
pub const crypto_pwhash_argon2id_MEMLIMIT_MIN: usize = 8192;
/// `((SIZE_MAX >= 4398046510080U) ? 4398046510080U : ...)`
pub const crypto_pwhash_argon2id_MEMLIMIT_MAX: usize = 4398046510080;
pub const crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE: c_ulonglong = 2;
pub const crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE: usize = 67108864;
pub const crypto_pwhash_argon2id_OPSLIMIT_MODERATE: c_ulonglong = 3;
pub const crypto_pwhash_argon2id_MEMLIMIT_MODERATE: usize = 268435456;
pub const crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE: c_ulonglong = 4;
pub const crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE: usize = 1073741824;

// ---------------------------------------------------------------------------
// pwhash_argon2id.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_alg_argon2id13() -> c_int {
    crypto_pwhash_argon2id_ALG_ARGON2ID13
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_bytes_min() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_BYTES_MIN >= ARGON2_MIN_OUTLEN); */
    const _: () = assert!(crypto_pwhash_argon2id_BYTES_MIN as u64 >= ARGON2_MIN_OUTLEN as u64);
    crypto_pwhash_argon2id_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_bytes_max() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_BYTES_MAX <= ARGON2_MAX_OUTLEN); */
    const _: () = assert!(crypto_pwhash_argon2id_BYTES_MAX <= ARGON2_MAX_OUTLEN as u64);
    crypto_pwhash_argon2id_BYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_passwd_min() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_PASSWD_MIN >= ARGON2_MIN_PWD_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2id_PASSWD_MIN as u64 >= ARGON2_MIN_PWD_LENGTH as u64);
    crypto_pwhash_argon2id_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_passwd_max() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_PASSWD_MAX <= ARGON2_MAX_PWD_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2id_PASSWD_MAX as u64 <= ARGON2_MAX_PWD_LENGTH as u64);
    crypto_pwhash_argon2id_PASSWD_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_saltbytes() -> usize {
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_SALTBYTES >= ARGON2_MIN_SALT_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2id_SALTBYTES as u64 >= ARGON2_MIN_SALT_LENGTH as u64);
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_SALTBYTES <= ARGON2_MAX_SALT_LENGTH); */
    const _: () = assert!(crypto_pwhash_argon2id_SALTBYTES as u64 <= ARGON2_MAX_SALT_LENGTH as u64);
    crypto_pwhash_argon2id_SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_strbytes() -> usize {
    crypto_pwhash_argon2id_STRBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_strprefix() -> *const c_char {
    crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_min() -> c_ulonglong {
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_OPSLIMIT_MIN >= ARGON2_MIN_TIME); */
    const _: () = assert!(crypto_pwhash_argon2id_OPSLIMIT_MIN >= ARGON2_MIN_TIME as c_ulonglong);
    crypto_pwhash_argon2id_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_max() -> c_ulonglong {
    /* COMPILER_ASSERT(crypto_pwhash_argon2id_OPSLIMIT_MAX <= ARGON2_MAX_TIME); */
    const _: () = assert!(crypto_pwhash_argon2id_OPSLIMIT_MAX <= ARGON2_MAX_TIME as c_ulonglong);
    crypto_pwhash_argon2id_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_min() -> usize {
    /* COMPILER_ASSERT((crypto_pwhash_argon2id_MEMLIMIT_MIN / 1024U) >= ARGON2_MIN_MEMORY); */
    const _: () =
        assert!((crypto_pwhash_argon2id_MEMLIMIT_MIN / 1024) as u64 >= ARGON2_MIN_MEMORY as u64);
    crypto_pwhash_argon2id_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_max() -> usize {
    /* COMPILER_ASSERT((crypto_pwhash_argon2id_MEMLIMIT_MAX / 1024U) <= ARGON2_MAX_MEMORY); */
    const _: () = assert!((crypto_pwhash_argon2id_MEMLIMIT_MAX / 1024) as u64 <= ARGON2_MAX_MEMORY);
    crypto_pwhash_argon2id_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_interactive() -> c_ulonglong {
    crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_interactive() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_moderate() -> c_ulonglong {
    crypto_pwhash_argon2id_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_moderate() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_sensitive() -> c_ulonglong {
    crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_sensitive() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id(
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
    if outlen > crypto_pwhash_argon2id_BYTES_MAX {
        set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if outlen < crypto_pwhash_argon2id_BYTES_MIN as c_ulonglong {
        set_errno(EINVAL);
        return -1;
    }
    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX as c_ulonglong
        || opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2id_PASSWD_MIN as c_ulonglong
        || opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN
    {
        set_errno(EINVAL);
        return -1;
    }
    if (out as *const c_void) == (passwd as *const c_void) {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    /* switch (alg) { case crypto_pwhash_argon2id_ALG_ARGON2ID13: ... default: ... } */
    if alg == crypto_pwhash_argon2id_ALG_ARGON2ID13 {
        if unsafe {
            _sodium_argon2id_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                crypto_pwhash_argon2id_SALTBYTES,
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
pub unsafe extern "C" fn crypto_pwhash_argon2id_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; crypto_pwhash_argon2id_SALTBYTES] = [0u8; crypto_pwhash_argon2id_SALTBYTES];

    unsafe { memset(out as *mut u8, 0, crypto_pwhash_argon2id_STRBYTES) };
    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX as c_ulonglong
        || opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2id_PASSWD_MIN as c_ulonglong
        || opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN
    {
        set_errno(EINVAL);
        return -1;
    }
    randombytes_buf(
        salt.as_mut_ptr() as *mut c_void,
        crypto_pwhash_argon2id_SALTBYTES,
    );
    if unsafe {
        _sodium_argon2id_hash_encoded(
            opslimit as u32,
            (memlimit / 1024) as u32,
            1u32,
            passwd as *const c_void,
            passwdlen as usize,
            salt.as_ptr() as *const c_void,
            crypto_pwhash_argon2id_SALTBYTES,
            STR_HASHBYTES,
            out,
            crypto_pwhash_argon2id_STRBYTES,
        )
    } != ARGON2_OK
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_verify(
    str: *const c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
) -> c_int {
    let verify_ret: c_int;

    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX as c_ulonglong {
        set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    /* LCOV_EXCL_START */
    if passwdlen < crypto_pwhash_argon2id_PASSWD_MIN as c_ulonglong {
        set_errno(EINVAL);
        return -1;
    }
    /* LCOV_EXCL_STOP */

    verify_ret =
        unsafe { _sodium_argon2id_verify(str, passwd as *const c_void, passwdlen as usize) };
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        set_errno(EINVAL);
    }
    -1
}
