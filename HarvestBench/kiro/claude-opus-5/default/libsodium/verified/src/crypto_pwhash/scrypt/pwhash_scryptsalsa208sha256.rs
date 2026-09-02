//! Translation of crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c
//! (+ include/sodium/crypto_pwhash_scryptsalsa208sha256.h)

use core::ffi::{c_char, c_int, c_void};

use crate::common::memset;
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::{sodium_memcmp, sodium_memzero};
use crate::EINVAL;

use super::crypto_scrypt_common::{
    _sodium_escrypt_gensalt_r, _sodium_escrypt_parse_setting, _sodium_escrypt_r,
    crypto_pwhash_scryptsalsa208sha256_ll,
};
use super::scrypt_platform::*;

/* ---- constants from crypto_pwhash_scryptsalsa208sha256.h ---- */

pub const crypto_pwhash_scryptsalsa208sha256_BYTES_MIN: usize = 16;
pub const crypto_pwhash_scryptsalsa208sha256_BYTES_MAX: usize = 0x1fffffffe0;
pub const crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN: usize = 0;
pub const crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX: usize = usize::MAX;
pub const crypto_pwhash_scryptsalsa208sha256_SALTBYTES: usize = 32;
pub const crypto_pwhash_scryptsalsa208sha256_STRBYTES: usize = 102;
pub const crypto_pwhash_scryptsalsa208sha256_STRPREFIX: &[u8] = b"$7$\0";
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN: u64 = 32768;
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX: u64 = 4294967295;
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN: usize = 16777216;
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX: usize = 68719476736;
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE: u64 = 524288;
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE: usize = 16777216;
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE: u64 = 33554432;
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE: usize = 1073741824;

/* STR* constants from crypto_scrypt.h */
const STRSALTBYTES: usize = 32;
const STRSETTINGBYTES: usize = 57;
const STRHASHBYTES_ENCODED: usize = 43;

/* BYTES2CHARS(bytes) ((((bytes) *8) + 5) / 6) */
/* SETTING_SIZE only used in a COMPILER_ASSERT (compile-time), omitted. */

fn pickparams(
    mut opslimit: u64,
    memlimit: usize,
    N_log2: *mut u32,
    p: *mut u32,
    r: *mut u32,
) -> c_int {
    let maxN: u64;
    let mut maxrp: u64;

    unsafe {
        if opslimit < 32768 {
            opslimit = 32768;
        }
        *r = 8;
        if opslimit < (memlimit / 32) as u64 {
            *p = 1;
            maxN = opslimit / ((*r * 4) as u64);
            *N_log2 = 1;
            while *N_log2 < 63 {
                if (1u64 << *N_log2) > maxN / 2 {
                    break;
                }
                *N_log2 += 1;
            }
        } else {
            let maxN2 = (memlimit / ((*r as usize) * 128)) as u64;
            *N_log2 = 1;
            while *N_log2 < 63 {
                if (1u64 << *N_log2) > maxN2 / 2 {
                    break;
                }
                *N_log2 += 1;
            }
            maxrp = (opslimit / 4) / (1u64 << *N_log2);
            /* LCOV_EXCL_START */
            if maxrp > 0x3fffffff {
                maxrp = 0x3fffffff;
            }
            /* LCOV_EXCL_STOP */
            *p = (maxrp as u32) / *r;
        }
    }
    0
}

unsafe fn sodium_strnlen(str_: *const c_char, maxlen: usize) -> usize {
    let mut i: usize = 0;

    /* ACQUIRE_FENCE is a no-op in the reference build. */
    while i < maxlen && *str_.add(i) != 0 {
        i += 1;
    }
    i
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_saltbytes() -> usize {
    crypto_pwhash_scryptsalsa208sha256_SALTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_strbytes() -> usize {
    crypto_pwhash_scryptsalsa208sha256_STRBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_strprefix() -> *const c_char {
    crypto_pwhash_scryptsalsa208sha256_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_min() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_max() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_interactive() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_interactive() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut N_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(out, 0, outlen as usize);
    if passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX as u64
        || outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX as u64
    {
        crate::set_errno(libc::EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (outlen as usize) < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        crate::set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (out as *const c_void) == (passwd as *const c_void) {
        crate::set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    crypto_pwhash_scryptsalsa208sha256_ll(
        passwd as *const u8,
        passwdlen as usize,
        salt,
        crypto_pwhash_scryptsalsa208sha256_SALTBYTES,
        1u64 << N_log2,
        r,
        p,
        out,
        outlen as usize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; STRSALTBYTES] = [0; STRSALTBYTES];
    let mut setting: [c_char; STRSETTINGBYTES + 1] = [0; STRSETTINGBYTES + 1];
    let mut escrypt_local: escrypt_local_t = core::mem::zeroed();
    let mut N_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(out as *mut u8, 0, crypto_pwhash_scryptsalsa208sha256_STRBYTES);
    if passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX as u64 {
        crate::set_errno(libc::EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN as u64
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        crate::set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&salt));
    if _sodium_escrypt_gensalt_r(
        N_log2,
        r,
        p,
        salt.as_ptr(),
        core::mem::size_of_val(&salt),
        setting.as_mut_ptr() as *mut u8,
        core::mem::size_of_val(&setting),
    )
    .is_null()
    {
        crate::set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if _sodium_escrypt_init_local(&mut escrypt_local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        setting.as_ptr() as *const u8,
        out as *mut u8,
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    )
    .is_null()
    {
        /* LCOV_EXCL_START */
        _sodium_escrypt_free_local(&mut escrypt_local);
        crate::set_errno(EINVAL);
        return -1;
        /* LCOV_EXCL_STOP */
    }
    _sodium_escrypt_free_local(&mut escrypt_local);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let mut wanted: [c_char; crypto_pwhash_scryptsalsa208sha256_STRBYTES] =
        [0; crypto_pwhash_scryptsalsa208sha256_STRBYTES];
    let mut escrypt_local: escrypt_local_t = core::mem::zeroed();
    let ret: c_int;

    if sodium_strnlen(str_, crypto_pwhash_scryptsalsa208sha256_STRBYTES)
        != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1
    {
        return -1;
    }
    if _sodium_escrypt_init_local(&mut escrypt_local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    memset(wanted.as_mut_ptr() as *mut u8, 0, core::mem::size_of_val(&wanted));
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        str_ as *const u8,
        wanted.as_mut_ptr() as *mut u8,
        core::mem::size_of_val(&wanted),
    )
    .is_null()
    {
        _sodium_escrypt_free_local(&mut escrypt_local);
        return -1;
    }
    _sodium_escrypt_free_local(&mut escrypt_local);
    ret = sodium_memcmp(
        wanted.as_ptr() as *const c_void,
        str_ as *const c_void,
        core::mem::size_of_val(&wanted),
    );
    sodium_memzero(wanted.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&wanted));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut N_log2: u32 = 0;
    let mut N_log2_: u32 = 0;
    let mut p: u32 = 0;
    let mut p_: u32 = 0;
    let mut r: u32 = 0;
    let mut r_: u32 = 0;

    if pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0 {
        crate::set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if sodium_strnlen(str_, crypto_pwhash_scryptsalsa208sha256_STRBYTES)
        != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1
    {
        crate::set_errno(EINVAL);
        return -1;
    }
    if _sodium_escrypt_parse_setting(str_ as *const u8, &mut N_log2_, &mut r_, &mut p_).is_null() {
        crate::set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if N_log2 != N_log2_ || r != r_ || p != p_ {
        return 1;
    }
    0
}
