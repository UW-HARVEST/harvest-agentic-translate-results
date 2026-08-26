//! Translation of
//! `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c`.
//!
//! None of the functions exported by this file are renamed by
//! `private/quirks.h`; the `escrypt_*` helpers it *calls* are.

use crate::common::{memset, SODIUM_SIZE_MAX};
use core::ffi::{c_char, c_int, c_ulonglong, c_void};
use core::ptr;

/* crypto_scrypt.h */
#[repr(C)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

pub type escrypt_local_t = escrypt_region_t;

/* crypto_pwhash_scryptsalsa208sha256.h */
const crypto_pwhash_scryptsalsa208sha256_BYTES_MIN: usize = 16;
const crypto_pwhash_scryptsalsa208sha256_BYTES_MAX: u64 =
    if SODIUM_SIZE_MAX < 0x1fffffffe0u64 {
        SODIUM_SIZE_MAX
    } else {
        0x1fffffffe0u64
    };
const crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN: u64 = 0;
const crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX: u64 = SODIUM_SIZE_MAX;
const crypto_pwhash_scryptsalsa208sha256_SALTBYTES: usize = 32;
const crypto_pwhash_scryptsalsa208sha256_STRBYTES: usize = 102;
const crypto_pwhash_scryptsalsa208sha256_STRPREFIX: &[u8; 4] = b"$7$\0";
const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN: c_ulonglong = 32768;
const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX: c_ulonglong = 4294967295;
const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN: usize = 16777216;
const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX: usize =
    if (usize::MAX as u64) < 68719476736u64 {
        usize::MAX
    } else {
        68719476736u64 as usize
    };
const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE: c_ulonglong = 524288;
const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE: usize = 16777216;
const crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE: c_ulonglong = 33554432;
const crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE: usize = 1073741824;

/* crypto_scrypt.h */
const crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES: usize = 57;
const crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES: usize = 32;

/* <errno.h> */
const EINVAL: c_int = 22;
const EFBIG: c_int = 27;

extern "C" {
    fn _sodium_escrypt_init_local(local: *mut escrypt_local_t) -> c_int;
    fn _sodium_escrypt_free_local(local: *mut escrypt_local_t) -> c_int;
    fn _sodium_escrypt_gensalt_r(
        N_log2: u32,
        r: u32,
        p: u32,
        src: *const u8,
        srclen: usize,
        buf: *mut u8,
        buflen: usize,
    ) -> *mut u8;
    fn _sodium_escrypt_r(
        local: *mut escrypt_local_t,
        passwd: *const u8,
        passwdlen: usize,
        setting: *const u8,
        buf: *mut u8,
        buflen: usize,
    ) -> *mut u8;
    fn _sodium_escrypt_parse_setting(
        setting: *const u8,
        N_log2_p: *mut u32,
        r_p: *mut u32,
        p_p: *mut u32,
    ) -> *const u8;

    fn crypto_pwhash_scryptsalsa208sha256_ll(
        passwd: *const u8,
        passwdlen: usize,
        salt: *const u8,
        saltlen: usize,
        N: u64,
        r: u32,
        p: u32,
        buf: *mut u8,
        buflen: usize,
    ) -> c_int;

    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
    fn __errno_location() -> *mut c_int;
}

/* static int pickparams(unsigned long long opslimit, const size_t memlimit,
                         uint32_t *const N_log2, uint32_t *const p,
                         uint32_t *const r) */
unsafe fn pickparams(
    mut opslimit: c_ulonglong,
    memlimit: usize,
    N_log2: *mut u32,
    p: *mut u32,
    r: *mut u32,
) -> c_int {
    let mut maxN: c_ulonglong;
    let mut maxrp: c_ulonglong;

    if opslimit < 32768 {
        opslimit = 32768;
    }
    *r = 8;
    if opslimit < (memlimit / 32) as c_ulonglong {
        *p = 1;
        maxN = opslimit / ((*r).wrapping_mul(4) as c_ulonglong);
        *N_log2 = 1;
        while *N_log2 < 63 {
            if 1u64.wrapping_shl(*N_log2) > maxN / 2 {
                break;
            }
            *N_log2 = (*N_log2).wrapping_add(1);
        }
    } else {
        maxN = (memlimit / ((*r as usize).wrapping_mul(128))) as c_ulonglong;
        *N_log2 = 1;
        while *N_log2 < 63 {
            if 1u64.wrapping_shl(*N_log2) > maxN / 2 {
                break;
            }
            *N_log2 = (*N_log2).wrapping_add(1);
        }
        maxrp = (opslimit / 4) / 1u64.wrapping_shl(*N_log2);
        /* LCOV_EXCL_START */
        if maxrp > 0x3fffffff {
            maxrp = 0x3fffffff;
        }
        /* LCOV_EXCL_STOP */
        *p = (maxrp as u32) / *r;
    }
    0
}

/* static size_t sodium_strnlen(const char *str, size_t maxlen) */
unsafe fn sodium_strnlen(str_: *const c_char, maxlen: usize) -> usize {
    let mut i: usize = 0;

    /* ACQUIRE_FENCE expands to `(void) 0` in the reference build. */
    while i < maxlen && *str_.add(i) != 0 {
        i = i.wrapping_add(1);
    }
    i
}

/* size_t crypto_pwhash_scryptsalsa208sha256_bytes_min(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_BYTES_MIN
}

/* size_t crypto_pwhash_scryptsalsa208sha256_bytes_max(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_BYTES_MAX as usize
}

/* size_t crypto_pwhash_scryptsalsa208sha256_passwd_min(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN as usize
}

/* size_t crypto_pwhash_scryptsalsa208sha256_passwd_max(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX as usize
}

/* size_t crypto_pwhash_scryptsalsa208sha256_saltbytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_saltbytes() -> usize {
    crypto_pwhash_scryptsalsa208sha256_SALTBYTES
}

/* size_t crypto_pwhash_scryptsalsa208sha256_strbytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strbytes() -> usize {
    crypto_pwhash_scryptsalsa208sha256_STRBYTES
}

/* const char *crypto_pwhash_scryptsalsa208sha256_strprefix(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strprefix() -> *const c_char {
    crypto_pwhash_scryptsalsa208sha256_STRPREFIX.as_ptr() as *const c_char
}

/* unsigned long long crypto_pwhash_scryptsalsa208sha256_opslimit_min(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_min() -> c_ulonglong {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MIN
}

/* unsigned long long crypto_pwhash_scryptsalsa208sha256_opslimit_max(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_max() -> c_ulonglong {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_MAX
}

/* size_t crypto_pwhash_scryptsalsa208sha256_memlimit_min(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_min() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MIN
}

/* size_t crypto_pwhash_scryptsalsa208sha256_memlimit_max(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_max() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_MAX
}

/* unsigned long long crypto_pwhash_scryptsalsa208sha256_opslimit_interactive(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_interactive() -> c_ulonglong
{
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_INTERACTIVE
}

/* size_t crypto_pwhash_scryptsalsa208sha256_memlimit_interactive(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_interactive() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_INTERACTIVE
}

/* unsigned long long crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive() -> c_ulonglong {
    crypto_pwhash_scryptsalsa208sha256_OPSLIMIT_SENSITIVE
}

/* size_t crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive() -> usize {
    crypto_pwhash_scryptsalsa208sha256_MEMLIMIT_SENSITIVE
}

/* int crypto_pwhash_scryptsalsa208sha256(unsigned char *const out,
                                          unsigned long long outlen,
                                          const char *const passwd,
                                          unsigned long long passwdlen,
                                          const unsigned char *const salt,
                                          unsigned long long opslimit,
                                          size_t memlimit) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256(
    out: *mut u8,
    outlen: c_ulonglong,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    salt: *const u8,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    let mut N_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(out, 0, outlen as usize);
    if passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX
        || outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX
    {
        *__errno_location() = EFBIG; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (outlen as usize) < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        *__errno_location() = EINVAL; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (out as *const c_void) == (passwd as *const c_void) {
        *__errno_location() = EINVAL; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    crypto_pwhash_scryptsalsa208sha256_ll(
        passwd as *const u8,
        passwdlen as usize,
        salt,
        crypto_pwhash_scryptsalsa208sha256_SALTBYTES,
        1u64.wrapping_shl(N_log2),
        r,
        p,
        out,
        outlen as usize,
    )
}

/* int crypto_pwhash_scryptsalsa208sha256_str(
       char out[crypto_pwhash_scryptsalsa208sha256_STRBYTES],
       const char *const passwd, unsigned long long passwdlen,
       unsigned long long opslimit, size_t memlimit) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES] =
        [0; crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES];
    let mut setting: [c_char; crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES + 1] =
        [0; crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES + 1];
    let mut escrypt_local = escrypt_local_t {
        base: ptr::null_mut(),
        aligned: ptr::null_mut(),
        size: 0,
    };
    let mut N_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(
        out as *mut u8,
        0,
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    );
    if passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX {
        *__errno_location() = EFBIG; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        *__errno_location() = EINVAL; /* LCOV_EXCL_LINE */
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
        setting.as_mut_ptr() as *mut u8,
        crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES + 1,
    )
    .is_null()
    {
        *__errno_location() = EINVAL; /* LCOV_EXCL_LINE */
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
        *__errno_location() = EINVAL;
        return -1;
        /* LCOV_EXCL_STOP */
    }
    _sodium_escrypt_free_local(&mut escrypt_local);

    /* COMPILER_ASSERT(SETTING_SIZE(STRSALTBYTES) == STRSETTINGBYTES); */
    const _: () = assert!(3 + 1 + 5 + 5 + ((32 * 8) + 5) / 6 == 57);
    /* COMPILER_ASSERT(STRSETTINGBYTES + 1U + STRHASHBYTES_ENCODED + 1U == STRBYTES); */
    const _: () = assert!(57 + 1 + 43 + 1 == 102);

    0
}

/* int crypto_pwhash_scryptsalsa208sha256_str_verify(const char *str,
                                                     const char *const passwd,
                                                     unsigned long long passwdlen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: c_ulonglong,
) -> c_int {
    let mut wanted: [c_char; crypto_pwhash_scryptsalsa208sha256_STRBYTES] =
        [0; crypto_pwhash_scryptsalsa208sha256_STRBYTES];
    let mut escrypt_local = escrypt_local_t {
        base: ptr::null_mut(),
        aligned: ptr::null_mut(),
        size: 0,
    };
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
        wanted.as_mut_ptr() as *mut u8,
        0,
        crypto_pwhash_scryptsalsa208sha256_STRBYTES,
    );
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        str_ as *const u8,
        wanted.as_mut_ptr() as *mut u8,
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

/* int crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(
       const char *str, unsigned long long opslimit, size_t memlimit) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(
    str_: *const c_char,
    opslimit: c_ulonglong,
    memlimit: usize,
) -> c_int {
    let mut N_log2: u32 = 0;
    let mut N_log2_: u32 = 0;
    let mut p: u32 = 0;
    let mut p_: u32 = 0;
    let mut r: u32 = 0;
    let mut r_: u32 = 0;

    if pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0 {
        *__errno_location() = EINVAL; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if sodium_strnlen(str_, crypto_pwhash_scryptsalsa208sha256_STRBYTES)
        != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1
    {
        *__errno_location() = EINVAL;
        return -1;
    }
    if _sodium_escrypt_parse_setting(str_ as *const u8, &mut N_log2_, &mut r_, &mut p_).is_null()
    {
        *__errno_location() = EINVAL; /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if N_log2 != N_log2_ || r != r_ || p != p_ {
        return 1;
    }
    0
}
