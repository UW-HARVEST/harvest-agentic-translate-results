//! Translation of `crypto_pwhash/argon2/pwhash_argon2id.c`
//! plus the constants from `include/sodium/crypto_pwhash_argon2id.h`.

use core::ffi::{c_char, c_int, c_void};

use crate::randombytes::randombytes_buf;

use super::argon2::{
    _sodium_argon2id_hash_encoded, _sodium_argon2id_hash_raw, _sodium_argon2id_verify,
};
use super::argon2_core::*;

const EFBIG: c_int = libc::EFBIG;
const EINVAL: c_int = libc::EINVAL;

/* ---- crypto_pwhash_argon2id.h constants ---- */
pub const crypto_pwhash_argon2id_ALG_ARGON2ID13: c_int = 2;
pub const crypto_pwhash_argon2id_BYTES_MIN: usize = 16;
pub const crypto_pwhash_argon2id_BYTES_MAX: usize = 4294967295;
pub const crypto_pwhash_argon2id_PASSWD_MIN: usize = 0;
pub const crypto_pwhash_argon2id_PASSWD_MAX: u64 = 4294967295;
pub const crypto_pwhash_argon2id_SALTBYTES: usize = 16;
pub const crypto_pwhash_argon2id_STRBYTES: usize = 128;
pub const crypto_pwhash_argon2id_STRPREFIX: &[u8] = b"$argon2id$\0";
pub const crypto_pwhash_argon2id_OPSLIMIT_MIN: u64 = 1;
pub const crypto_pwhash_argon2id_OPSLIMIT_MAX: u64 = 4294967295;
pub const crypto_pwhash_argon2id_MEMLIMIT_MIN: usize = 8192;
pub const crypto_pwhash_argon2id_MEMLIMIT_MAX: usize = 4398046510080;
pub const crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE: u64 = 2;
pub const crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE: usize = 67108864;
pub const crypto_pwhash_argon2id_OPSLIMIT_MODERATE: u64 = 3;
pub const crypto_pwhash_argon2id_MEMLIMIT_MODERATE: usize = 268435456;
pub const crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE: u64 = 4;
pub const crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE: usize = 1073741824;

const STR_HASHBYTES: usize = 32;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_alg_argon2id13() -> c_int {
    crypto_pwhash_argon2id_ALG_ARGON2ID13
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_bytes_min() -> usize {
    crypto_pwhash_argon2id_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_bytes_max() -> usize {
    crypto_pwhash_argon2id_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_passwd_min() -> usize {
    crypto_pwhash_argon2id_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_passwd_max() -> usize {
    crypto_pwhash_argon2id_PASSWD_MAX as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_saltbytes() -> usize {
    crypto_pwhash_argon2id_SALTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_strbytes() -> usize {
    crypto_pwhash_argon2id_STRBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_strprefix() -> *const c_char {
    crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_min() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_max() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_min() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_max() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_interactive() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_interactive() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_moderate() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_moderate() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_opslimit_sensitive() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2id_memlimit_sensitive() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    crate::common::memset(out, 0, outlen as usize);
    if outlen > crypto_pwhash_argon2id_BYTES_MAX as u64 {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (outlen as usize) < crypto_pwhash_argon2id_BYTES_MIN {
        crate::set_errno(EINVAL);
        return -1;
    }
    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX
    {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (passwdlen as usize) < crypto_pwhash_argon2id_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN
    {
        crate::set_errno(EINVAL);
        return -1;
    }
    if (out as *const c_void) == (passwd as *const c_void) {
        crate::set_errno(EINVAL);
        return -1;
    }
    match alg {
        crypto_pwhash_argon2id_ALG_ARGON2ID13 => {
            if _sodium_argon2id_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                crypto_pwhash_argon2id_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            ) != ARGON2_OK
            {
                return -1;
            }
            0
        }
        _ => {
            crate::set_errno(EINVAL);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; crypto_pwhash_argon2id_SALTBYTES] = [0; crypto_pwhash_argon2id_SALTBYTES];

    crate::common::memset(out as *mut u8, 0, crypto_pwhash_argon2id_STRBYTES);
    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX
    {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (passwdlen as usize) < crypto_pwhash_argon2id_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN
    {
        crate::set_errno(EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if _sodium_argon2id_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
        1u32,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        salt.len(),
        STR_HASHBYTES,
        out,
        crypto_pwhash_argon2id_STRBYTES,
    ) != ARGON2_OK
    {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_verify(
    str: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let verify_ret: c_int;

    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (passwdlen as usize) < crypto_pwhash_argon2id_PASSWD_MIN {
        crate::set_errno(EINVAL);
        return -1;
    }

    verify_ret = _sodium_argon2id_verify(str, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        crate::set_errno(EINVAL);
    }
    -1
}
