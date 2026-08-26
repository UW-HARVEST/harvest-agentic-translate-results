//! Translation of
//! `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c`.
//!
//! No `HAVE_GCC_MEMORY_FENCES` / `HAVE_C11_MEMORY_FENCES` in the reference
//! build, so `ACQUIRE_FENCE` expands to `(void) 0`.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_char, c_int, c_void};

use crate::common::{EINVAL, SODIUM_SIZE_MAX, memset, set_errno};
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::{sodium_memcmp, sodium_memzero};

use crate::crypto_pwhash::scrypt::common::{
    _sodium_escrypt_gensalt_r, _sodium_escrypt_parse_setting, _sodium_escrypt_r,
    crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES,
    crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES,
    crypto_pwhash_scryptsalsa208sha256_ll, escrypt_local_t, escrypt_region_t,
};
use crate::crypto_pwhash::scrypt::platform::{
    _sodium_escrypt_free_local, _sodium_escrypt_init_local,
};

/// `<errno.h>`: `EFBIG` on Linux.
const EFBIG: c_int = 27;

// ---------------------------------------------------------------------------
// crypto_pwhash_scryptsalsa208sha256.h
// ---------------------------------------------------------------------------

/// `#define crypto_pwhash_scryptsalsa208sha256_BYTES_MIN 16U`
pub const crypto_pwhash_scryptsalsa208sha256_BYTES_MIN: usize = 16;
/// `SODIUM_MIN(SODIUM_SIZE_MAX, 0x1fffffffe0ULL)`
pub const crypto_pwhash_scryptsalsa208sha256_BYTES_MAX: u64 = if SODIUM_SIZE_MAX < 0x1fffffffe0u64 {
    SODIUM_SIZE_MAX
} else {
    0x1fffffffe0u64
};
/// `#define crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN 0U`
pub const crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN: usize = 0;
/// `#define crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX SODIUM_SIZE_MAX`
pub const crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX: u64 = SODIUM_SIZE_MAX;
/// `#define crypto_pwhash_scryptsalsa208sha256_SALTBYTES 32U`
pub const crypto_pwhash_scryptsalsa208sha256_SALTBYTES: usize = 32;
/// `#define crypto_pwhash_scryptsalsa208sha256_STRBYTES 102U`
pub const crypto_pwhash_scryptsalsa208sha256_STRBYTES: usize = 102;
/// `#define crypto_pwhash_scryptsalsa208sha256_STRPREFIX "$7$"`
static crypto_pwhash_scryptsalsa208sha256_STRPREFIX: [u8; 4] = *b"$7$\0";
/// `#define crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN 32768U`
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN: u64 = 32768;
/// `#define crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX 4294967295U`
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX: u64 = 4294967295;
/// `#define crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN 16777216U`
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN: usize = 16777216;
/// `SODIUM_MIN(SIZE_MAX, 68719476736ULL)`
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX: usize =
    if (usize::MAX as u64) < 68719476736u64 {
        usize::MAX
    } else {
        68719476736u64 as usize
    };
/// `#define crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE 524288U`
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE: u64 = 524288;
/// `#define crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE 16777216U`
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE: usize = 16777216;
/// `#define crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE 33554432U`
pub const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE: u64 = 33554432;
/// `#define crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE 1073741824U`
pub const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE: usize = 1073741824;

// ---------------------------------------------------------------------------

/// ```c
/// static int
/// pickparams(unsigned long long opslimit, const size_t memlimit,
///            uint32_t *const N_log2, uint32_t *const p, uint32_t *const r)
/// ```
unsafe fn pickparams(
    mut opslimit: u64,
    memlimit: usize,
    N_log2: *mut u32,
    p: *mut u32,
    r: *mut u32,
) -> c_int {
    let maxN: u64;
    let mut maxrp: u64;

    if opslimit < 32768 {
        opslimit = 32768;
    }
    *r = 8;
    if opslimit < (memlimit / 32) as u64 {
        *p = 1;
        maxN = opslimit / ((*r).wrapping_mul(4) as u64);
        *N_log2 = 1;
        while *N_log2 < 63 {
            if (1u64 << *N_log2) > maxN / 2 {
                break;
            }
            *N_log2 = (*N_log2).wrapping_add(1);
        }
    } else {
        maxN = (memlimit / (*r as usize).wrapping_mul(128)) as u64;
        *N_log2 = 1;
        while *N_log2 < 63 {
            if (1u64 << *N_log2) > maxN / 2 {
                break;
            }
            *N_log2 = (*N_log2).wrapping_add(1);
        }
        maxrp = (opslimit / 4) / (1u64 << *N_log2);
        /* LCOV_EXCL_START */
        if maxrp > 0x3fffffff {
            maxrp = 0x3fffffff;
        }
        /* LCOV_EXCL_STOP */
        *p = (maxrp as u32) / *r;
    }

    0
}

/// ```c
/// static size_t
/// sodium_strnlen(const char *str, size_t maxlen)
/// ```
unsafe fn sodium_strnlen(str_: *const c_char, maxlen: usize) -> usize {
    let mut i: usize = 0;

    // ACQUIRE_FENCE -> (void) 0
    while i < maxlen && *str_.add(i) != 0 {
        i += 1;
    }

    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_BYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_saltbytes() -> usize {
    crypto_pwhash_scryptsalsa208sha256_SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strbytes() -> usize {
    crypto_pwhash_scryptsalsa208sha256_STRBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strprefix() -> *const c_char {
    crypto_pwhash_scryptsalsa208sha256_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_min() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_max() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_interactive() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_interactive() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive() -> u64 {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE
}

/// ```c
/// int
/// crypto_pwhash_scryptsalsa208sha256(unsigned char *const       out,
///                                    unsigned long long         outlen,
///                                    const char *const          passwd,
///                                    unsigned long long         passwdlen,
///                                    const unsigned char *const salt,
///                                    unsigned long long opslimit, size_t memlimit)
/// ```
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
    if passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX
        || outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX
    {
        set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN as u64
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (out as *const c_void) == (passwd as *const c_void) {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
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

/// ```c
/// int
/// crypto_pwhash_scryptsalsa208sha256_str(
///     char              out[crypto_pwhash_scryptsalsa208sha256_STRBYTES],
///     const char *const passwd, unsigned long long passwdlen,
///     unsigned long long opslimit, size_t memlimit)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt = [0u8; crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES];
    let mut setting = [0u8; crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES + 1];
    let mut escrypt_local: escrypt_local_t = escrypt_region_t::zeroed();
    let mut N_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(
        out as *mut u8,
        0,
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    );
    if passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX {
        set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN as u64
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    randombytes_buf(
        salt.as_mut_ptr() as *mut c_void,
        crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES,
    );
    if _sodium_escrypt_gensalt_r(
        N_log2,
        r,
        p,
        salt.as_ptr(),
        crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES,
        setting.as_mut_ptr(),
        crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES + 1,
    )
    .is_null()
    {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if _sodium_escrypt_init_local(&mut escrypt_local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        setting.as_ptr(),
        out as *mut u8,
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    )
    .is_null()
    {
        /* LCOV_EXCL_START */
        _sodium_escrypt_free_local(&mut escrypt_local);
        set_errno(EINVAL);
        return -1;
        /* LCOV_EXCL_STOP */
    }
    _sodium_escrypt_free_local(&mut escrypt_local);

    0
}

/// ```c
/// int
/// crypto_pwhash_scryptsalsa208sha256_str_verify(
///     const char        *str,
///     const char *const passwd, unsigned long long passwdlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let mut wanted = [0u8; crypto_pwhash_scryptsalsa208sha256_STRBYTES];
    let mut escrypt_local: escrypt_local_t = escrypt_region_t::zeroed();
    let ret: c_int;

    if sodium_strnlen(str_, crypto_pwhash_scryptsalsa208sha256_STRBYTES)
        != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1
    {
        return -1;
    }
    if _sodium_escrypt_init_local(&mut escrypt_local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    memset(
        wanted.as_mut_ptr(),
        0,
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    );
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        str_ as *const u8,
        wanted.as_mut_ptr(),
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
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
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    );
    sodium_memzero(
        wanted.as_mut_ptr() as *mut c_void,
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    );

    ret
}

/// ```c
/// int
/// crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(
///     const char * str,
///     unsigned long long opslimit, size_t memlimit)
/// ```
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
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if sodium_strnlen(str_, crypto_pwhash_scryptsalsa208sha256_STRBYTES)
        != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1
    {
        set_errno(EINVAL);
        return -1;
    }
    if _sodium_escrypt_parse_setting(str_ as *const u8, &mut N_log2_, &mut r_, &mut p_).is_null() {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if N_log2 != N_log2_ || r != r_ || p != p_ {
        return 1;
    }

    0
}
