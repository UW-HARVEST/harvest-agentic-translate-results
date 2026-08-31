//! Translation of c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c

use core::ffi::{c_char, c_int, c_void};

use crate::plat::{set_errno, EINVAL};

// EFBIG is not provided by crate::plat; on x86_64 Linux it is 27.
const EFBIG: c_int = 27;

// Constants from include/sodium/crypto_pwhash_scryptsalsa208sha256.h.
const BYTES_MIN: usize = 16;
// SODIUM_MIN(SODIUM_SIZE_MAX, 0x1fffffffe0ULL) == 0x1fffffffe0 on 64-bit.
const BYTES_MAX: usize = 0x1fffffffe0;
const PASSWD_MIN: usize = 0;
// SODIUM_SIZE_MAX == usize::MAX on 64-bit.
const PASSWD_MAX: usize = usize::MAX;
const SALTBYTES: usize = 32;
const STRBYTES: usize = 102;
const STRPREFIX: &[u8] = b"$7$\0";
const OPSLIMIT_MIN: u64 = 32768;
const OPSLIMIT_MAX: u64 = 4294967295;
const MEMLIMIT_MIN: usize = 16777216;
// SODIUM_MIN(SIZE_MAX, 68719476736ULL) == 68719476736 on 64-bit.
const MEMLIMIT_MAX: usize = 68719476736;
const OPSLIMIT_INTERACTIVE: u64 = 524288;
const MEMLIMIT_INTERACTIVE: usize = 16777216;
const OPSLIMIT_SENSITIVE: u64 = 33554432;
const MEMLIMIT_SENSITIVE: usize = 1073741824;

// From crypto_scrypt.h.
const STRSALTBYTES: usize = 32;
const STRSETTINGBYTES: usize = 57;

// escrypt_local_t mirror (crypto_scrypt.h).
#[repr(C)]
struct escrypt_region_t {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}
type escrypt_local_t = escrypt_region_t;

extern "C" {
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;

    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;

    // Defined in the translated scrypt sources (linker names after quirks.h).
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

    // crypto_pwhash_scryptsalsa208sha256_ll is defined in crypto_scrypt_common.rs.
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
}

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
        maxN = opslimit / (*r as u64 * 4);
        *N_log2 = 1;
        while *N_log2 < 63 {
            if (1u64 << *N_log2) > maxN / 2 {
                break;
            }
            *N_log2 += 1;
        }
    } else {
        maxN = (memlimit / ((*r as usize) * 128)) as u64;
        *N_log2 = 1;
        while *N_log2 < 63 {
            if (1u64 << *N_log2) > maxN / 2 {
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
    0
}

unsafe fn sodium_strnlen(str_: *const c_char, maxlen: usize) -> usize {
    let mut i: usize = 0;

    // ACQUIRE_FENCE expands to nothing.
    while i < maxlen && *str_.add(i) != 0 {
        i += 1;
    }
    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_min() -> usize {
    BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_bytes_max() -> usize {
    BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_min() -> usize {
    PASSWD_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_passwd_max() -> usize {
    PASSWD_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_saltbytes() -> usize {
    SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strbytes() -> usize {
    STRBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_strprefix() -> *const c_char {
    STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_min() -> u64 {
    OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_max() -> u64 {
    OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_min() -> usize {
    MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_max() -> usize {
    MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_interactive() -> u64 {
    OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_interactive() -> usize {
    MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive() -> u64 {
    OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive() -> usize {
    MEMLIMIT_SENSITIVE
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

    memset(out as *mut c_void, 0, outlen as usize);
    if passwdlen as usize > PASSWD_MAX || outlen as usize > BYTES_MAX {
        set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (outlen as usize) < BYTES_MIN
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if out as *const c_void == passwd as *const c_void {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    crypto_pwhash_scryptsalsa208sha256_ll(
        passwd as *const u8,
        passwdlen as usize,
        salt,
        SALTBYTES,
        (1u64) << N_log2,
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
    let mut escrypt_local: escrypt_local_t = escrypt_region_t {
        base: core::ptr::null_mut(),
        aligned: core::ptr::null_mut(),
        size: 0,
    };
    let mut N_log2: u32 = 0;
    let mut p: u32 = 0;
    let mut r: u32 = 0;

    memset(out as *mut c_void, 0, STRBYTES);
    if passwdlen as usize > PASSWD_MAX {
        set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (passwdlen as usize) < PASSWD_MIN
        || pickparams(opslimit, memlimit, &mut N_log2, &mut p, &mut r) != 0
    {
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; STRSALTBYTES]>());
    if _sodium_escrypt_gensalt_r(
        N_log2,
        r,
        p,
        salt.as_ptr(),
        core::mem::size_of::<[u8; STRSALTBYTES]>(),
        setting.as_mut_ptr() as *mut u8,
        core::mem::size_of::<[c_char; STRSETTINGBYTES + 1]>(),
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
        setting.as_ptr() as *const u8,
        out as *mut u8,
        STRBYTES,
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

    // COMPILER_ASSERT lines expand to nothing.

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let mut wanted: [c_char; STRBYTES] = [0; STRBYTES];
    let mut escrypt_local: escrypt_local_t = escrypt_region_t {
        base: core::ptr::null_mut(),
        aligned: core::ptr::null_mut(),
        size: 0,
    };
    let ret: c_int;

    if sodium_strnlen(str_, STRBYTES) != STRBYTES - 1 {
        return -1;
    }
    if _sodium_escrypt_init_local(&mut escrypt_local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    memset(
        wanted.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of::<[c_char; STRBYTES]>(),
    );
    if _sodium_escrypt_r(
        &mut escrypt_local,
        passwd as *const u8,
        passwdlen as usize,
        str_ as *const u8,
        wanted.as_mut_ptr() as *mut u8,
        core::mem::size_of::<[c_char; STRBYTES]>(),
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
        core::mem::size_of::<[c_char; STRBYTES]>(),
    );
    sodium_memzero(
        wanted.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[c_char; STRBYTES]>(),
    );

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
        set_errno(EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if sodium_strnlen(str_, STRBYTES) != STRBYTES - 1 {
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
