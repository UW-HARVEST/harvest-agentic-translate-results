//! Translation of `crypto_pwhash/argon2/pwhash_argon2i.c`
//! plus the constants from `include/sodium/crypto_pwhash_argon2i.h`.

use core::ffi::{c_char, c_int, c_void};

use crate::randombytes::randombytes_buf;

use super::argon2::{
    _sodium_argon2i_hash_encoded, _sodium_argon2i_hash_raw, _sodium_argon2i_verify,
};
use super::argon2_core::*;
use super::argon2_encoding::_sodium_argon2_decode_string;

/* errno codes not exported from lib.rs */
const EFBIG: c_int = libc::EFBIG;
const EINVAL: c_int = libc::EINVAL;

/* ---- crypto_pwhash_argon2i.h constants ---- */
pub const crypto_pwhash_argon2i_ALG_ARGON2I13: c_int = 1;
pub const crypto_pwhash_argon2i_BYTES_MIN: usize = 16;
pub const crypto_pwhash_argon2i_BYTES_MAX: usize = 4294967295; /* min(SIZE_MAX, 4294967295) */
pub const crypto_pwhash_argon2i_PASSWD_MIN: usize = 0;
pub const crypto_pwhash_argon2i_PASSWD_MAX: u64 = 4294967295;
pub const crypto_pwhash_argon2i_SALTBYTES: usize = 16;
pub const crypto_pwhash_argon2i_STRBYTES: usize = 128;
pub const crypto_pwhash_argon2i_STRPREFIX: &[u8] = b"$argon2i$\0";
pub const crypto_pwhash_argon2i_OPSLIMIT_MIN: u64 = 3;
pub const crypto_pwhash_argon2i_OPSLIMIT_MAX: u64 = 4294967295;
pub const crypto_pwhash_argon2i_MEMLIMIT_MIN: usize = 8192;
/* SIZE_MAX >= 4398046510080 on LP64 -> 4398046510080 */
pub const crypto_pwhash_argon2i_MEMLIMIT_MAX: usize = 4398046510080;
pub const crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE: u64 = 4;
pub const crypto_pwhash_argon2i_MEMLIMIT_INTERACTIVE: usize = 33554432;
pub const crypto_pwhash_argon2i_OPSLIMIT_MODERATE: u64 = 6;
pub const crypto_pwhash_argon2i_MEMLIMIT_MODERATE: usize = 134217728;
pub const crypto_pwhash_argon2i_OPSLIMIT_SENSITIVE: u64 = 8;
pub const crypto_pwhash_argon2i_MEMLIMIT_SENSITIVE: usize = 536870912;

/* crypto_pwhash_STRBYTES (used by _needs_rehash). */
const crypto_pwhash_STRBYTES: usize = 128;

const STR_HASHBYTES: usize = 32;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_alg_argon2i13() -> c_int {
    crypto_pwhash_argon2i_ALG_ARGON2I13
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_bytes_min() -> usize {
    crypto_pwhash_argon2i_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_bytes_max() -> usize {
    crypto_pwhash_argon2i_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_passwd_min() -> usize {
    crypto_pwhash_argon2i_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_passwd_max() -> usize {
    crypto_pwhash_argon2i_PASSWD_MAX as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_saltbytes() -> usize {
    crypto_pwhash_argon2i_SALTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_strbytes() -> usize {
    crypto_pwhash_argon2i_STRBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_strprefix() -> *const c_char {
    crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_min() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_max() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_min() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_max() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_interactive() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_interactive() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_moderate() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_moderate() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_opslimit_sensitive() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_argon2i_memlimit_sensitive() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i(
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
    if outlen > crypto_pwhash_argon2i_BYTES_MAX as u64 {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (outlen as usize) < crypto_pwhash_argon2i_BYTES_MIN {
        crate::set_errno(EINVAL);
        return -1;
    }
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (passwdlen as usize) < crypto_pwhash_argon2i_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        crate::set_errno(EINVAL);
        return -1;
    }
    if (out as *const c_void) == (passwd as *const c_void) {
        crate::set_errno(EINVAL);
        return -1;
    }
    match alg {
        crypto_pwhash_argon2i_ALG_ARGON2I13 => {
            if _sodium_argon2i_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                crypto_pwhash_argon2i_SALTBYTES,
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
pub unsafe extern "C" fn crypto_pwhash_argon2i_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; crypto_pwhash_argon2i_SALTBYTES] = [0; crypto_pwhash_argon2i_SALTBYTES];

    crate::common::memset(out as *mut u8, 0, crypto_pwhash_argon2i_STRBYTES);
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (passwdlen as usize) < crypto_pwhash_argon2i_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        crate::set_errno(EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if _sodium_argon2i_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
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
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_verify(
    str: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let verify_ret: c_int;

    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX {
        crate::set_errno(EFBIG);
        return -1;
    }
    if (passwdlen as usize) < crypto_pwhash_argon2i_PASSWD_MIN {
        crate::set_errno(EINVAL);
        return -1;
    }

    verify_ret = _sodium_argon2i_verify(str, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        crate::set_errno(EINVAL);
    }
    -1
}

unsafe fn strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

unsafe fn _needs_rehash(
    str: *const c_char,
    opslimit: u64,
    mut memlimit: usize,
    type_: argon2_type,
) -> c_int {
    let fodder: *mut u8;
    let mut ctx: argon2_context = core::mem::zeroed();
    let fodder_len: usize;
    let mut ret: c_int;

    fodder_len = strlen(str);
    memlimit /= 1024;
    if opslimit > u32::MAX as u64
        || memlimit > u32::MAX as usize
        || fodder_len >= crypto_pwhash_STRBYTES
    {
        crate::set_errno(EINVAL);
        return -1;
    }
    /* memset(&ctx, 0, sizeof ctx): already zeroed. */
    fodder = libc::calloc(fodder_len, 1) as *mut u8;
    if fodder.is_null() {
        return -1;
    }
    ctx.out = fodder;
    ctx.pwd = fodder;
    ctx.salt = fodder;
    ctx.outlen = fodder_len as u32;
    ctx.pwdlen = fodder_len as u32;
    ctx.saltlen = fodder_len as u32;
    ctx.ad = core::ptr::null_mut();
    ctx.secret = core::ptr::null_mut();
    ctx.adlen = 0;
    ctx.secretlen = 0;
    if _sodium_argon2_decode_string(&mut ctx, str, type_) != 0 {
        crate::set_errno(EINVAL);
        ret = -1;
    } else if ctx.t_cost != opslimit as u32 || ctx.m_cost != memlimit as u32 {
        ret = 1;
    } else {
        ret = 0;
    }
    libc::free(fodder as *mut c_void);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_needs_rehash(
    str: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    _needs_rehash(str, opslimit, memlimit, Argon2_i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_needs_rehash(
    str: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    _needs_rehash(str, opslimit, memlimit, Argon2_id)
}
