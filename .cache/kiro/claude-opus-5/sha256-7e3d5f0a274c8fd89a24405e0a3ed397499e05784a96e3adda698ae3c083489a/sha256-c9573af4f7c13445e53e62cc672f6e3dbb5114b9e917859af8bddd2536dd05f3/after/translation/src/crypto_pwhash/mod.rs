pub mod argon2;
pub mod scrypt;

// Translation of `crypto_pwhash/crypto_pwhash.c`
// plus the constants from `include/sodium/crypto_pwhash.h`.

use core::ffi::{c_char, c_int};

use crate::sodium_core::sodium_misuse;

use self::argon2::argon2_core::{Argon2_i, Argon2_id};
use self::argon2::pwhash_argon2i::*;
use self::argon2::pwhash_argon2id::*;

const EINVAL: c_int = libc::EINVAL;

/* ---- crypto_pwhash.h constants (aliases of argon2id values) ---- */
pub const crypto_pwhash_ALG_ARGON2I13: c_int = crypto_pwhash_argon2i_ALG_ARGON2I13;
pub const crypto_pwhash_ALG_ARGON2ID13: c_int = crypto_pwhash_argon2id_ALG_ARGON2ID13;
pub const crypto_pwhash_ALG_DEFAULT: c_int = crypto_pwhash_ALG_ARGON2ID13;

pub const crypto_pwhash_BYTES_MIN: usize = crypto_pwhash_argon2id_BYTES_MIN;
pub const crypto_pwhash_BYTES_MAX: usize = crypto_pwhash_argon2id_BYTES_MAX;
pub const crypto_pwhash_PASSWD_MIN: usize = crypto_pwhash_argon2id_PASSWD_MIN;
pub const crypto_pwhash_PASSWD_MAX: u64 = crypto_pwhash_argon2id_PASSWD_MAX;
pub const crypto_pwhash_SALTBYTES: usize = crypto_pwhash_argon2id_SALTBYTES;
pub const crypto_pwhash_STRBYTES: usize = crypto_pwhash_argon2id_STRBYTES;
pub const crypto_pwhash_STRPREFIX: &[u8] = crypto_pwhash_argon2id_STRPREFIX;
pub const crypto_pwhash_OPSLIMIT_MIN: u64 = crypto_pwhash_argon2id_OPSLIMIT_MIN;
pub const crypto_pwhash_OPSLIMIT_MAX: u64 = crypto_pwhash_argon2id_OPSLIMIT_MAX;
pub const crypto_pwhash_MEMLIMIT_MIN: usize = crypto_pwhash_argon2id_MEMLIMIT_MIN;
pub const crypto_pwhash_MEMLIMIT_MAX: usize = crypto_pwhash_argon2id_MEMLIMIT_MAX;
pub const crypto_pwhash_OPSLIMIT_INTERACTIVE: u64 = crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE;
pub const crypto_pwhash_MEMLIMIT_INTERACTIVE: usize = crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE;
pub const crypto_pwhash_OPSLIMIT_MODERATE: u64 = crypto_pwhash_argon2id_OPSLIMIT_MODERATE;
pub const crypto_pwhash_MEMLIMIT_MODERATE: usize = crypto_pwhash_argon2id_MEMLIMIT_MODERATE;
pub const crypto_pwhash_OPSLIMIT_SENSITIVE: u64 = crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE;
pub const crypto_pwhash_MEMLIMIT_SENSITIVE: usize = crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE;

pub const crypto_pwhash_PRIMITIVE: &[u8] = b"argon2id,argon2i\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_alg_argon2i13() -> c_int {
    crypto_pwhash_ALG_ARGON2I13
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_alg_argon2id13() -> c_int {
    crypto_pwhash_ALG_ARGON2ID13
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_alg_default() -> c_int {
    crypto_pwhash_ALG_DEFAULT
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_bytes_min() -> usize {
    crypto_pwhash_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_bytes_max() -> usize {
    crypto_pwhash_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_passwd_min() -> usize {
    crypto_pwhash_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_passwd_max() -> usize {
    crypto_pwhash_PASSWD_MAX as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_saltbytes() -> usize {
    crypto_pwhash_SALTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_strbytes() -> usize {
    crypto_pwhash_STRBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_strprefix() -> *const c_char {
    crypto_pwhash_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_min() -> u64 {
    crypto_pwhash_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_max() -> u64 {
    crypto_pwhash_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_min() -> usize {
    crypto_pwhash_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_max() -> usize {
    crypto_pwhash_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_interactive() -> u64 {
    crypto_pwhash_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_interactive() -> usize {
    crypto_pwhash_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_moderate() -> u64 {
    crypto_pwhash_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_moderate() -> usize {
    crypto_pwhash_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_sensitive() -> u64 {
    crypto_pwhash_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_sensitive() -> usize {
    crypto_pwhash_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
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
            crate::set_errno(EINVAL);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_alg(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
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
    sodium_misuse();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_verify(
    str: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    if strncmp(
        str,
        crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2id_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2id_str_verify(str, passwd, passwdlen);
    }
    if strncmp(
        str,
        crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2i_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2i_str_verify(str, passwd, passwdlen);
    }
    crate::set_errno(EINVAL);

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_needs_rehash(
    str: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    if strncmp(
        str,
        crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2id_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2id_str_needs_rehash(str, opslimit, memlimit);
    }
    if strncmp(
        str,
        crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2i_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2i_str_needs_rehash(str, opslimit, memlimit);
    }
    crate::set_errno(EINVAL);

    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_primitive() -> *const c_char {
    crypto_pwhash_PRIMITIVE.as_ptr() as *const c_char
}

/* local strncmp (signed char target). */
unsafe fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int {
    let mut i: usize = 0;
    while i < n {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 != c2 {
            return (c1 as u8 as c_int) - (c2 as u8 as c_int);
        }
        if c1 == 0 {
            return 0;
        }
        i += 1;
    }
    0
}
