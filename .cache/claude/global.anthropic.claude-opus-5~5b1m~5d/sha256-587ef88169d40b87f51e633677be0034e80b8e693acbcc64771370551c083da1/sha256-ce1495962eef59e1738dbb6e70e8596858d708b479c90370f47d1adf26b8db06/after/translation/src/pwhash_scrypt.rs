//! Translation of `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c`.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr::null_mut;

use crate::csys::{memset, set_errno, EINVAL};

const EFBIG: c_int = 27;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;

    fn crypto_pwhash_scryptsalsa208sha256_ll(
        passwd: *const u8,
        passwdlen: usize,
        salt: *const u8,
        saltlen: usize,
        n: u64,
        r: u32,
        p: u32,
        buf: *mut u8,
        buflen: usize,
    ) -> c_int;

    #[link_name = "_sodium_escrypt_init_local"]
    fn escrypt_init_local(local: *mut escrypt_local_t) -> c_int;
    #[link_name = "_sodium_escrypt_free_local"]
    fn escrypt_free_local(local: *mut escrypt_local_t) -> c_int;
    #[link_name = "_sodium_escrypt_r"]
    fn escrypt_r(
        local: *mut escrypt_local_t,
        passwd: *const u8,
        passwdlen: usize,
        setting: *const u8,
        buf: *mut u8,
        buflen: usize,
    ) -> *mut u8;
    #[link_name = "_sodium_escrypt_gensalt_r"]
    fn escrypt_gensalt_r(
        n_log2: u32,
        r: u32,
        p: u32,
        src: *const u8,
        srclen: usize,
        buf: *mut u8,
        buflen: usize,
    ) -> *mut u8;
    #[link_name = "_sodium_escrypt_parse_setting"]
    fn escrypt_parse_setting(
        setting: *const u8,
        n_log2_p: *mut u32,
        r_p: *mut u32,
        p_p: *mut u32,
    ) -> *const u8;
}

/// `escrypt_local_t` — mirrors `escrypt_region_t` from `crypto_scrypt.h`.
#[repr(C)]
struct escrypt_local_t {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

// ---------------------------------------------------------------------
// crypto_pwhash_scryptsalsa208sha256.h constants
// ---------------------------------------------------------------------
const BYTES_MIN: usize = 16;
const PASSWD_MIN: u64 = 0;
const SALTBYTES: usize = 32;
const STRBYTES: usize = 102;
const OPSLIMIT_MIN: u64 = 32768;
const OPSLIMIT_MAX: u64 = 4294967295;
const MEMLIMIT_MIN: usize = 16777216;
const OPSLIMIT_INTERACTIVE: u64 = 524288;
const MEMLIMIT_INTERACTIVE: usize = 16777216;
const OPSLIMIT_SENSITIVE: u64 = 33554432;
const MEMLIMIT_SENSITIVE: usize = 1073741824;

// crypto_scrypt.h
const STRSALTBYTES: usize = 32;
const STRSETTINGBYTES: usize = 57;

static STRPREFIX_CSTR: &[u8; 4] = b"$7$\0";

#[inline]
fn bytes2chars(bytes: usize) -> usize {
    (bytes * 8 + 5) / 6
}

// ---------------------------------------------------------------------

unsafe fn pickparams(
    mut opslimit: u64,
    memlimit: usize,
    n_log2: *mut u32,
    p: *mut u32,
    r: *mut u32,
) -> c_int {
    let maxn: u64;
    let mut maxrp: u64;

    if opslimit < 32768 {
        opslimit = 32768;
    }
    *r = 8;
    if opslimit < (memlimit as u64) / 32 {
        *p = 1;
        maxn = opslimit / ((*r as u64) * 4);
        *n_log2 = 1;
        while *n_log2 < 63 {
            if (1u64 << *n_log2) > maxn / 2 {
                break;
            }
            *n_log2 += 1;
        }
    } else {
        maxn = (memlimit as u64) / ((*r as u64) * 128);
        *n_log2 = 1;
        while *n_log2 < 63 {
            if (1u64 << *n_log2) > maxn / 2 {
                break;
            }
            *n_log2 += 1;
        }
        maxrp = (opslimit / 4) / (1u64 << *n_log2);
        if maxrp > 0x3fffffff {
            maxrp = 0x3fffffff;
        }
        *p = (maxrp as u32) / *r;
    }
    0
}

unsafe fn sodium_strnlen(str_: *const c_char, maxlen: usize) -> usize {
    let mut i: usize = 0;
    while i < maxlen && *str_.add(i) != 0 {
        i += 1;
    }
    i
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_min() -> usize {
    BYTES_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_max() -> usize {
    let a = crate::common::SODIUM_SIZE_MAX;
    let b: u64 = 0x1fffffffe0;
    (if a < b { a } else { b }) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_min() -> usize {
    PASSWD_MIN as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_max() -> usize {
    crate::common::SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_saltbytes() -> usize {
    SALTBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strbytes() -> usize {
    STRBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strprefix() -> *const c_char {
    STRPREFIX_CSTR.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_min() -> u64 {
    OPSLIMIT_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_max() -> u64 {
    OPSLIMIT_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_min() -> usize {
    MEMLIMIT_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_max() -> usize {
    let a: u64 = usize::MAX as u64;
    let b: u64 = 68719476736;
    (if a < b { a } else { b }) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_interactive() -> u64 {
    OPSLIMIT_INTERACTIVE
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_interactive() -> usize {
    MEMLIMIT_INTERACTIVE
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive() -> u64 {
    OPSLIMIT_SENSITIVE
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive() -> usize {
    MEMLIMIT_SENSITIVE
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut n_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(out as *mut c_void, 0, outlen as usize);

    let passwd_max = crypto_pwhash_scryptsalsa208sha256_passwd_max() as u64;
    let bytes_max = crypto_pwhash_scryptsalsa208sha256_bytes_max() as u64;
    if passwdlen > passwd_max || outlen > bytes_max {
        set_errno(EFBIG);
        return -1;
    }
    if outlen < BYTES_MIN as u64 || pickparams(opslimit, memlimit, &mut n_log2, &mut p, &mut r) != 0
    {
        set_errno(EINVAL);
        return -1;
    }
    if (out as usize) == (passwd as usize) {
        set_errno(EINVAL);
        return -1;
    }
    crypto_pwhash_scryptsalsa208sha256_ll(
        passwd as *const u8,
        passwdlen as usize,
        salt,
        SALTBYTES,
        1u64 << n_log2,
        r,
        p,
        out,
        outlen as usize,
    )
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt = [0u8; STRSALTBYTES];
    let mut setting = [0u8; STRSETTINGBYTES + 1];
    let mut local_ctx = escrypt_local_t {
        base: null_mut(),
        aligned: null_mut(),
        size: 0,
    };
    let mut n_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(out as *mut c_void, 0, STRBYTES);

    if passwdlen > crypto_pwhash_scryptsalsa208sha256_passwd_max() as u64 {
        set_errno(EFBIG);
        return -1;
    }
    if passwdlen < PASSWD_MIN || pickparams(opslimit, memlimit, &mut n_log2, &mut p, &mut r) != 0 {
        set_errno(EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if escrypt_gensalt_r(
        n_log2,
        r,
        p,
        salt.as_ptr(),
        salt.len(),
        setting.as_mut_ptr(),
        setting.len(),
    )
    .is_null()
    {
        set_errno(EINVAL);
        return -1;
    }
    if escrypt_init_local(&mut local_ctx) != 0 {
        return -1;
    }
    if escrypt_r(
        &mut local_ctx,
        passwd as *const u8,
        passwdlen as usize,
        setting.as_ptr(),
        out as *mut u8,
        STRBYTES,
    )
    .is_null()
    {
        escrypt_free_local(&mut local_ctx);
        set_errno(EINVAL);
        return -1;
    }
    escrypt_free_local(&mut local_ctx);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let mut wanted = [0u8; STRBYTES];
    let mut local_ctx = escrypt_local_t {
        base: null_mut(),
        aligned: null_mut(),
        size: 0,
    };

    if sodium_strnlen(str_, STRBYTES) != STRBYTES - 1 {
        return -1;
    }
    if escrypt_init_local(&mut local_ctx) != 0 {
        return -1;
    }
    memset(wanted.as_mut_ptr() as *mut c_void, 0, wanted.len());
    if escrypt_r(
        &mut local_ctx,
        passwd as *const u8,
        passwdlen as usize,
        str_ as *const u8,
        wanted.as_mut_ptr(),
        wanted.len(),
    )
    .is_null()
    {
        escrypt_free_local(&mut local_ctx);
        return -1;
    }
    escrypt_free_local(&mut local_ctx);
    let ret = sodium_memcmp(
        wanted.as_ptr() as *const c_void,
        str_ as *const c_void,
        wanted.len(),
    );
    sodium_memzero(wanted.as_mut_ptr() as *mut c_void, wanted.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut n_log2: u32 = 0;
    let mut n_log2_: u32 = 0;
    let mut p: u32 = 0;
    let mut p_: u32 = 0;
    let mut r: u32 = 0;
    let mut r_: u32 = 0;

    if pickparams(opslimit, memlimit, &mut n_log2, &mut p, &mut r) != 0 {
        set_errno(EINVAL);
        return -1;
    }
    if sodium_strnlen(str_, STRBYTES) != STRBYTES - 1 {
        set_errno(EINVAL);
        return -1;
    }
    if escrypt_parse_setting(str_ as *const u8, &mut n_log2_, &mut r_, &mut p_).is_null() {
        set_errno(EINVAL);
        return -1;
    }
    if n_log2 != n_log2_ || r != r_ || p != p_ {
        return 1;
    }
    0
}
