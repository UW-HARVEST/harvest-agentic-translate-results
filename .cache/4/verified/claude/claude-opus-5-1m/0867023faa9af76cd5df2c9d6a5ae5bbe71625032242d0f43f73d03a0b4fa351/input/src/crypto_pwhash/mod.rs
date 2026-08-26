//! Translation of `crypto_pwhash/crypto_pwhash.c` plus the constants of
//! `include/sodium/crypto_pwhash.h` (they all alias the argon2id ones).

pub mod argon2;
pub mod scrypt;

use core::ffi::{c_char, c_int, c_ulonglong};

use crate::common::{set_errno, EINVAL};
use crate::crypto_pwhash::argon2::pwhash_argon2i::*;
use crate::crypto_pwhash::argon2::pwhash_argon2id::*;
use crate::sodium::core::sodium_misuse;

// ---------------------------------------------------------------------------
// include/sodium/crypto_pwhash.h -- every constant aliases an argon2 one
// ---------------------------------------------------------------------------

/// `#define crypto_pwhash_ALG_ARGON2I13 crypto_pwhash_argon2i_ALG_ARGON2I13`
pub const crypto_pwhash_ALG_ARGON2I13: c_int = crypto_pwhash_argon2i_ALG_ARGON2I13;
/// `#define crypto_pwhash_ALG_ARGON2ID13 crypto_pwhash_argon2id_ALG_ARGON2ID13`
pub const crypto_pwhash_ALG_ARGON2ID13: c_int = crypto_pwhash_argon2id_ALG_ARGON2ID13;
/// `#define crypto_pwhash_ALG_DEFAULT crypto_pwhash_ALG_ARGON2ID13`
pub const crypto_pwhash_ALG_DEFAULT: c_int = crypto_pwhash_ALG_ARGON2ID13;

pub const crypto_pwhash_BYTES_MIN: usize = crypto_pwhash_argon2id_BYTES_MIN;
pub const crypto_pwhash_BYTES_MAX: u64 = crypto_pwhash_argon2id_BYTES_MAX;
pub const crypto_pwhash_PASSWD_MIN: usize = crypto_pwhash_argon2id_PASSWD_MIN;
pub const crypto_pwhash_PASSWD_MAX: usize = crypto_pwhash_argon2id_PASSWD_MAX;
pub const crypto_pwhash_SALTBYTES: usize = crypto_pwhash_argon2id_SALTBYTES;
pub const crypto_pwhash_STRBYTES: usize = crypto_pwhash_argon2id_STRBYTES;
pub const crypto_pwhash_STRPREFIX: &[u8; 11] = crypto_pwhash_argon2id_STRPREFIX;
pub const crypto_pwhash_OPSLIMIT_MIN: c_ulonglong = crypto_pwhash_argon2id_OPSLIMIT_MIN;
pub const crypto_pwhash_OPSLIMIT_MAX: c_ulonglong = crypto_pwhash_argon2id_OPSLIMIT_MAX;
pub const crypto_pwhash_MEMLIMIT_MIN: usize = crypto_pwhash_argon2id_MEMLIMIT_MIN;
pub const crypto_pwhash_MEMLIMIT_MAX: usize = crypto_pwhash_argon2id_MEMLIMIT_MAX;
pub const crypto_pwhash_OPSLIMIT_INTERACTIVE: c_ulonglong =
    crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE;
pub const crypto_pwhash_MEMLIMIT_INTERACTIVE: usize = crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE;
pub const crypto_pwhash_OPSLIMIT_MODERATE: c_ulonglong = crypto_pwhash_argon2id_OPSLIMIT_MODERATE;
pub const crypto_pwhash_MEMLIMIT_MODERATE: usize = crypto_pwhash_argon2id_MEMLIMIT_MODERATE;
pub const crypto_pwhash_OPSLIMIT_SENSITIVE: c_ulonglong = crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE;
pub const crypto_pwhash_MEMLIMIT_SENSITIVE: usize = crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE;
/// `#define crypto_pwhash_PRIMITIVE "argon2id,argon2i"`
pub const crypto_pwhash_PRIMITIVE: &[u8; 17] = b"argon2id,argon2i\0";

// ---------------------------------------------------------------------------
// crypto_pwhash/crypto_pwhash.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_alg_argon2i13() -> c_int {
    crypto_pwhash_ALG_ARGON2I13
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_alg_argon2id13() -> c_int {
    crypto_pwhash_ALG_ARGON2ID13
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_alg_default() -> c_int {
    crypto_pwhash_ALG_DEFAULT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_bytes_min() -> usize {
    crypto_pwhash_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_bytes_max() -> usize {
    crypto_pwhash_BYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_passwd_min() -> usize {
    crypto_pwhash_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_passwd_max() -> usize {
    crypto_pwhash_PASSWD_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_saltbytes() -> usize {
    crypto_pwhash_SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_strbytes() -> usize {
    crypto_pwhash_STRBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_strprefix() -> *const c_char {
    crypto_pwhash_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_min() -> c_ulonglong {
    crypto_pwhash_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_max() -> c_ulonglong {
    crypto_pwhash_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_min() -> usize {
    crypto_pwhash_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_max() -> usize {
    crypto_pwhash_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_interactive() -> c_ulonglong {
    crypto_pwhash_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_interactive() -> usize {
    crypto_pwhash_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_moderate() -> c_ulonglong {
    crypto_pwhash_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_moderate() -> usize {
    crypto_pwhash_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_sensitive() -> c_ulonglong {
    crypto_pwhash_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_sensitive() -> usize {
    crypto_pwhash_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash(
    out: *mut u8,
    outlen: c_ulonglong,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    salt: *const u8,
    opslimit: c_ulonglong,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    /* switch (alg) { ... } */
    if alg == crypto_pwhash_ALG_ARGON2I13 {
        unsafe {
            crypto_pwhash_argon2i(out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)
        }
    } else if alg == crypto_pwhash_ALG_ARGON2ID13 {
        unsafe {
            crypto_pwhash_argon2id(out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)
        }
    } else {
        set_errno(EINVAL);
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    unsafe { crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_alg(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    opslimit: c_ulonglong,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    /* switch (alg) { ... } (no default: falls through to sodium_misuse()) */
    if alg == crypto_pwhash_ALG_ARGON2I13 {
        return unsafe { crypto_pwhash_argon2i_str(out, passwd, passwdlen, opslimit, memlimit) };
    }
    if alg == crypto_pwhash_ALG_ARGON2ID13 {
        return unsafe { crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit) };
    }
    sodium_misuse(); /* LCOV_EXCL_LINE */
    /* NOTREACHED */
    /* return -1; -- `sodium_misuse()` never returns */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_verify(
    str: *const c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
) -> c_int {
    if unsafe { strncmp_prefix_eq(str, crypto_pwhash_argon2id_STRPREFIX) } {
        return unsafe { crypto_pwhash_argon2id_str_verify(str, passwd, passwdlen) };
    }
    if unsafe { strncmp_prefix_eq(str, crypto_pwhash_argon2i_STRPREFIX) } {
        return unsafe { crypto_pwhash_argon2i_str_verify(str, passwd, passwdlen) };
    }
    set_errno(EINVAL);

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_needs_rehash(
    str: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    if unsafe { strncmp_prefix_eq(str, crypto_pwhash_argon2id_STRPREFIX) } {
        return unsafe { crypto_pwhash_argon2id_str_needs_rehash(str, opslimit, memlimit) };
    }
    if unsafe { strncmp_prefix_eq(str, crypto_pwhash_argon2i_STRPREFIX) } {
        return unsafe { crypto_pwhash_argon2i_str_needs_rehash(str, opslimit, memlimit) };
    }
    set_errno(EINVAL);

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_primitive() -> *const c_char {
    crypto_pwhash_PRIMITIVE.as_ptr() as *const c_char
}

/// `strncmp(str, prefix, sizeof prefix - 1) == 0`, where `prefix` is a
/// NUL-terminated string literal (so its byte array includes the final NUL,
/// which is *not* compared).
#[inline]
unsafe fn strncmp_prefix_eq<const N: usize>(str: *const c_char, prefix: &[u8; N]) -> bool {
    let mut i: usize = 0;
    while i < N - 1 {
        if (unsafe { *str.add(i) } as u8) != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}
