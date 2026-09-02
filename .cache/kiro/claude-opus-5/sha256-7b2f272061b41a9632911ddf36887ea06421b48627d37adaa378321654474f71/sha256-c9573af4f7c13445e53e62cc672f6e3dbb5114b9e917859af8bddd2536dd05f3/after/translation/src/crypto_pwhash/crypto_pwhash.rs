//! Translation of c_src/libsodium/crypto_pwhash/crypto_pwhash.c

use core::ffi::{c_char, c_int};

// Constants from crypto_pwhash.h / crypto_pwhash_argon2i.h / crypto_pwhash_argon2id.h
const crypto_pwhash_ALG_ARGON2I13: c_int = 1; // crypto_pwhash_argon2i_ALG_ARGON2I13
const crypto_pwhash_ALG_ARGON2ID13: c_int = 2; // crypto_pwhash_argon2id_ALG_ARGON2ID13
const crypto_pwhash_ALG_DEFAULT: c_int = crypto_pwhash_ALG_ARGON2ID13;

const crypto_pwhash_BYTES_MIN: usize = 16; // crypto_pwhash_argon2id_BYTES_MIN
// crypto_pwhash_argon2id_BYTES_MAX == SODIUM_MIN(SODIUM_SIZE_MAX, 4294967295U)
const crypto_pwhash_BYTES_MAX: usize = crate::common::SODIUM_SIZE_MAX & 4294967295;
const crypto_pwhash_PASSWD_MIN: usize = 0;
const crypto_pwhash_PASSWD_MAX: usize = 4294967295;
const crypto_pwhash_SALTBYTES: usize = 16;
const crypto_pwhash_STRBYTES: usize = 128;
const crypto_pwhash_STRPREFIX: &[u8] = b"$argon2id$\0"; // crypto_pwhash_argon2id_STRPREFIX
const crypto_pwhash_OPSLIMIT_MIN: u64 = 1; // argon2id OPSLIMIT_MIN
const crypto_pwhash_OPSLIMIT_MAX: u64 = 4294967295;
const crypto_pwhash_MEMLIMIT_MIN: usize = 8192;
// crypto_pwhash_argon2id_MEMLIMIT_MAX:
// ((SIZE_MAX >= 4398046510080U) ? 4398046510080U : ...) -> 64-bit => 4398046510080
const crypto_pwhash_MEMLIMIT_MAX: usize = 4398046510080;
const crypto_pwhash_OPSLIMIT_INTERACTIVE: u64 = 2;
const crypto_pwhash_MEMLIMIT_INTERACTIVE: usize = 67108864;
const crypto_pwhash_OPSLIMIT_MODERATE: u64 = 3;
const crypto_pwhash_MEMLIMIT_MODERATE: usize = 268435456;
const crypto_pwhash_OPSLIMIT_SENSITIVE: u64 = 4;
const crypto_pwhash_MEMLIMIT_SENSITIVE: usize = 1073741824;
const crypto_pwhash_PRIMITIVE: &[u8] = b"argon2id,argon2i\0";

// crypto_pwhash_argon2i_STRPREFIX == "$argon2i$"
const crypto_pwhash_argon2i_STRPREFIX: &[u8] = b"$argon2i$\0";
// crypto_pwhash_argon2id_STRPREFIX == "$argon2id$"
const crypto_pwhash_argon2id_STRPREFIX: &[u8] = b"$argon2id$\0";

extern "C" {
    fn crypto_pwhash_argon2i(
        out: *mut u8,
        outlen: u64,
        passwd: *const c_char,
        passwdlen: u64,
        salt: *const u8,
        opslimit: u64,
        memlimit: usize,
        alg: c_int,
    ) -> c_int;
    fn crypto_pwhash_argon2id(
        out: *mut u8,
        outlen: u64,
        passwd: *const c_char,
        passwdlen: u64,
        salt: *const u8,
        opslimit: u64,
        memlimit: usize,
        alg: c_int,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str(
        out: *mut c_char,
        passwd: *const c_char,
        passwdlen: u64,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str(
        out: *mut c_char,
        passwd: *const c_char,
        passwdlen: u64,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str_verify(
        str_: *const c_char,
        passwd: *const c_char,
        passwdlen: u64,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str_verify(
        str_: *const c_char,
        passwd: *const c_char,
        passwdlen: u64,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str_needs_rehash(
        str_: *const c_char,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str_needs_rehash(
        str_: *const c_char,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
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
pub unsafe extern "C" fn crypto_pwhash_opslimit_min() -> u64 {
    crypto_pwhash_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_max() -> u64 {
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
pub unsafe extern "C" fn crypto_pwhash_opslimit_interactive() -> u64 {
    crypto_pwhash_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_interactive() -> usize {
    crypto_pwhash_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_moderate() -> u64 {
    crypto_pwhash_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_moderate() -> usize {
    crypto_pwhash_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_opslimit_sensitive() -> u64 {
    crypto_pwhash_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_memlimit_sensitive() -> usize {
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
        x if x == crypto_pwhash_ALG_ARGON2I13 => crypto_pwhash_argon2i(
            out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg,
        ),
        x if x == crypto_pwhash_ALG_ARGON2ID13 => crypto_pwhash_argon2id(
            out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg,
        ),
        _ => {
            crate::plat::set_errno(crate::plat::EINVAL);
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
        x if x == crypto_pwhash_ALG_ARGON2I13 => {
            return crypto_pwhash_argon2i_str(out, passwd, passwdlen, opslimit, memlimit);
        }
        x if x == crypto_pwhash_ALG_ARGON2ID13 => {
            return crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit);
        }
        _ => {}
    }
    crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    /* NOTREACHED */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    // sizeof crypto_pwhash_argon2id_STRPREFIX - 1 == strlen("$argon2id$") == 10
    if strncmp(
        str_,
        crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2id_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2id_str_verify(str_, passwd, passwdlen);
    }
    // sizeof crypto_pwhash_argon2i_STRPREFIX - 1 == strlen("$argon2i$") == 9
    if strncmp(
        str_,
        crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char,
        crypto_pwhash_argon2i_STRPREFIX.len() - 1,
    ) == 0
    {
        return crypto_pwhash_argon2i_str_verify(str_, passwd, passwdlen);
    }
    crate::plat::set_errno(crate::plat::EINVAL);

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
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
    crate::plat::set_errno(crate::plat::EINVAL);

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_primitive() -> *const c_char {
    crypto_pwhash_PRIMITIVE.as_ptr() as *const c_char
}
