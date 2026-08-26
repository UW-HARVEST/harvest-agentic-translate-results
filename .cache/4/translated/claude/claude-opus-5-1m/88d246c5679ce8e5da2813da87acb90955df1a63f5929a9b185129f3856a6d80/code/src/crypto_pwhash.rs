//! Translation of `crypto_pwhash/crypto_pwhash.c`

use core::ffi::{c_char, c_int, c_ulonglong};

/* ---------------------------------------------------------------- */
/* errno                                                             */
/* ---------------------------------------------------------------- */
const EINVAL: c_int = 22;

/* ---------------------------------------------------------------- */
/* crypto_pwhash.h constants (all aliases of the argon2id ones,      */
/* except ALG_ARGON2I13 which aliases the argon2i one)               */
/* ---------------------------------------------------------------- */
const crypto_pwhash_ALG_ARGON2I13: c_int = 1; /* crypto_pwhash_argon2i_ALG_ARGON2I13 */
const crypto_pwhash_ALG_ARGON2ID13: c_int = 2; /* crypto_pwhash_argon2id_ALG_ARGON2ID13 */
const crypto_pwhash_ALG_DEFAULT: c_int = crypto_pwhash_ALG_ARGON2ID13;

const crypto_pwhash_BYTES_MIN: usize = 16;
/* SODIUM_MIN(SODIUM_SIZE_MAX, 4294967295U) == 4294967295 on x86-64 */
const crypto_pwhash_BYTES_MAX: usize = 4294967295;
const crypto_pwhash_PASSWD_MIN: usize = 0;
const crypto_pwhash_PASSWD_MAX: usize = 4294967295;
const crypto_pwhash_SALTBYTES: usize = 16;
const crypto_pwhash_STRBYTES: usize = 128;
const crypto_pwhash_STRPREFIX: &[u8; 11] = b"$argon2id$\0";
const crypto_pwhash_OPSLIMIT_MIN: c_ulonglong = 1;
const crypto_pwhash_OPSLIMIT_MAX: c_ulonglong = 4294967295;
const crypto_pwhash_MEMLIMIT_MIN: usize = 8192;
/* SIZE_MAX >= 4398046510080U on x86-64 */
const crypto_pwhash_MEMLIMIT_MAX: usize = 4398046510080;
const crypto_pwhash_OPSLIMIT_INTERACTIVE: c_ulonglong = 2;
const crypto_pwhash_MEMLIMIT_INTERACTIVE: usize = 67108864;
const crypto_pwhash_OPSLIMIT_MODERATE: c_ulonglong = 3;
const crypto_pwhash_MEMLIMIT_MODERATE: usize = 268435456;
const crypto_pwhash_OPSLIMIT_SENSITIVE: c_ulonglong = 4;
const crypto_pwhash_MEMLIMIT_SENSITIVE: usize = 1073741824;

const crypto_pwhash_PRIMITIVE: &[u8; 17] = b"argon2id,argon2i\0";

/* Prefix strings used by the strncmp() dispatch below */
const crypto_pwhash_argon2id_STRPREFIX: &[u8; 11] = b"$argon2id$\0";
const crypto_pwhash_argon2i_STRPREFIX: &[u8; 10] = b"$argon2i$\0";

extern "C" {
    /* pwhash_argon2i.c */
    fn crypto_pwhash_argon2i(
        out: *mut u8,
        outlen: c_ulonglong,
        passwd: *const c_char,
        passwdlen: c_ulonglong,
        salt: *const u8,
        opslimit: c_ulonglong,
        memlimit: usize,
        alg: c_int,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str(
        out: *mut c_char,
        passwd: *const c_char,
        passwdlen: c_ulonglong,
        opslimit: c_ulonglong,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str_verify(
        str_: *const c_char,
        passwd: *const c_char,
        passwdlen: c_ulonglong,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str_needs_rehash(
        str_: *const c_char,
        opslimit: c_ulonglong,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str_needs_rehash(
        str_: *const c_char,
        opslimit: c_ulonglong,
        memlimit: usize,
    ) -> c_int;

    /* pwhash_argon2id.c */
    fn crypto_pwhash_argon2id(
        out: *mut u8,
        outlen: c_ulonglong,
        passwd: *const c_char,
        passwdlen: c_ulonglong,
        salt: *const u8,
        opslimit: c_ulonglong,
        memlimit: usize,
        alg: c_int,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str(
        out: *mut c_char,
        passwd: *const c_char,
        passwdlen: c_ulonglong,
        opslimit: c_ulonglong,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str_verify(
        str_: *const c_char,
        passwd: *const c_char,
        passwdlen: c_ulonglong,
    ) -> c_int;

    /* sodium/core.c */
    fn sodium_misuse() -> !;

    /* libc */
    fn __errno_location() -> *mut c_int;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
}

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
    crypto_pwhash_BYTES_MAX
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
    match alg {
        crypto_pwhash_ALG_ARGON2I13 => crypto_pwhash_argon2i(
            out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg,
        ),
        crypto_pwhash_ALG_ARGON2ID13 => crypto_pwhash_argon2id(
            out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg,
        ),
        _ => {
            *__errno_location() = EINVAL;
            -1
        }
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
    crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit)
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
    match alg {
        crypto_pwhash_ALG_ARGON2I13 => {
            return crypto_pwhash_argon2i_str(out, passwd, passwdlen, opslimit, memlimit);
        }
        crypto_pwhash_ALG_ARGON2ID13 => {
            return crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit);
        }
        _ => {}
    }
    sodium_misuse(); /* LCOV_EXCL_LINE */
    /* NOTREACHED */
    /* return -1; */ /* LCOV_EXCL_LINE */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
) -> c_int {
    if strncmp(
        str_,
        crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2id_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2id_str_verify(str_, passwd, passwdlen);
    }
    if strncmp(
        str_,
        crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2i_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2i_str_verify(str_, passwd, passwdlen);
    }
    *__errno_location() = EINVAL;

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_needs_rehash(
    str_: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    if strncmp(
        str_,
        crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2id_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2id_str_needs_rehash(str_, opslimit, memlimit);
    }
    if strncmp(
        str_,
        crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2i_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2i_str_needs_rehash(str_, opslimit, memlimit);
    }
    *__errno_location() = EINVAL;

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_primitive() -> *const c_char {
    crypto_pwhash_PRIMITIVE.as_ptr() as *const c_char
}
