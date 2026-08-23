//! Translation of `crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c`
//! together with the shared declarations of
//! `crypto_pwhash/scryptsalsa208sha256/crypto_scrypt.h`.
//!
//! `HAVE_EMMINTRIN_H` is **not** defined in the reference build, so
//! `escrypt_kdf` is unconditionally `escrypt_kdf_nosse`
//! (linker name `_sodium_escrypt_kdf_nosse`, see `private/quirks.h`).

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::common::{memcpy, strchr, strlen};
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::sodium_memzero;

use crate::crypto_pwhash::scrypt::nosse::_sodium_escrypt_kdf_nosse;
use crate::crypto_pwhash::scrypt::platform::{
    _sodium_escrypt_free_local, _sodium_escrypt_init_local,
};

unsafe extern "C" {
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
}

// ---------------------------------------------------------------------------
// crypto_scrypt.h
// ---------------------------------------------------------------------------

/// `#define crypto_pwhash_scryptsalsa208sha256_STRPREFIXBYTES 14`
pub const crypto_pwhash_scryptsalsa208sha256_STRPREFIXBYTES: usize = 14;
/// `#define crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES 57`
pub const crypto_pwhash_scryptsalsa208sha256_STRSETTINGBYTES: usize = 57;
/// `#define crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES 32`
pub const crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES: usize = 32;
/// `#define crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES_ENCODED 43`
pub const crypto_pwhash_scryptsalsa208sha256_STRSALTBYTES_ENCODED: usize = 43;
/// `#define crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES 32`
pub const crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES: usize = 32;
/// `#define crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES_ENCODED 43`
pub const crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES_ENCODED: usize = 43;

/// `#define BYTES2CHARS(bytes) ((((bytes) *8) + 5) / 6)`
#[inline(always)]
pub const fn BYTES2CHARS(bytes: usize) -> usize {
    (bytes.wrapping_mul(8).wrapping_add(5)) / 6
}

/// ```c
/// typedef struct {
///     void * base, *aligned;
///     size_t size;
/// } escrypt_region_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

impl escrypt_region_t {
    /// Helper used in place of C's "uninitialized automatic": every caller in
    /// the C sources runs `escrypt_init_local()` (which zeroes all fields)
    /// before the value is ever read.
    #[inline(always)]
    pub const fn zeroed() -> Self {
        escrypt_region_t {
            base: ptr::null_mut(),
            aligned: ptr::null_mut(),
            size: 0,
        }
    }
}

/// `typedef escrypt_region_t escrypt_local_t;`
pub type escrypt_local_t = escrypt_region_t;

/// ```c
/// typedef int (*escrypt_kdf_t)(escrypt_local_t *__local, const uint8_t *__passwd,
///                              size_t __passwdlen, const uint8_t *__salt,
///                              size_t __saltlen, uint64_t __N, uint32_t __r,
///                              uint32_t __p, uint8_t *__buf, size_t __buflen);
/// ```
pub type escrypt_kdf_t = unsafe extern "C" fn(
    *mut escrypt_local_t,
    *const u8,
    usize,
    *const u8,
    usize,
    u64,
    u32,
    u32,
    *mut u8,
    usize,
) -> c_int;

// ---------------------------------------------------------------------------
// crypto_scrypt-common.c
// ---------------------------------------------------------------------------

/// ```c
/// static const char *const itoa64 =
///     "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
/// ```
///
/// Kept NUL terminated because `decode64_one()` hands it to `strchr()`, which
/// (for `src == 0`) legitimately matches the terminator and yields index 64.
static itoa64: [u8; 65] =
    *b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\0";

/// ```c
/// static uint8_t *
/// encode64_uint32(uint8_t *dst, size_t dstlen, uint32_t src, uint32_t srcbits)
/// ```
unsafe fn encode64_uint32(
    mut dst: *mut u8,
    mut dstlen: usize,
    mut src: u32,
    srcbits: u32,
) -> *mut u8 {
    let mut bit: u32;

    bit = 0;
    while bit < srcbits {
        if dstlen < 1 {
            return ptr::null_mut(); /* LCOV_EXCL_LINE */
        }
        *dst = itoa64[(src & 0x3f) as usize];
        dst = dst.add(1);
        dstlen -= 1;
        src >>= 6;

        bit = bit.wrapping_add(6);
    }

    dst
}

/// ```c
/// static uint8_t *
/// encode64(uint8_t *dst, size_t dstlen, const uint8_t *src, size_t srclen)
/// ```
unsafe fn encode64(mut dst: *mut u8, mut dstlen: usize, src: *const u8, srclen: usize) -> *mut u8 {
    let mut i: usize;

    i = 0;
    while i < srclen {
        let dnext: *mut u8;
        let mut value: u32 = 0;
        let mut bits: u32 = 0;

        loop {
            value |= (*src.add(i) as u32) << bits;
            i += 1;
            bits = bits.wrapping_add(8);
            if !(bits < 24 && i < srclen) {
                break;
            }
        }

        dnext = encode64_uint32(dst, dstlen, value, bits);
        if dnext.is_null() {
            return ptr::null_mut(); /* LCOV_EXCL_LINE */
        }
        dstlen -= (dnext as usize).wrapping_sub(dst as usize);
        dst = dnext;
    }

    dst
}

/// ```c
/// static int
/// decode64_one(uint32_t *dst, uint8_t src)
/// ```
unsafe fn decode64_one(dst: *mut u32, src: u8) -> c_int {
    let p = strchr(itoa64.as_ptr() as *const c_char, src as c_int);

    if !p.is_null() {
        *dst = ((p as usize).wrapping_sub(itoa64.as_ptr() as usize)) as u32;
        return 0;
    }
    *dst = 0;

    -1
}

/// ```c
/// static const uint8_t *
/// decode64_uint32(uint32_t *dst, uint32_t dstbits, const uint8_t *src)
/// ```
unsafe fn decode64_uint32(dst: *mut u32, dstbits: u32, mut src: *const u8) -> *const u8 {
    let mut bit: u32;
    let value: u32;

    let mut acc: u32 = 0;
    bit = 0;
    while bit < dstbits {
        let mut one: u32 = 0;
        if decode64_one(&mut one, *src) != 0 {
            *dst = 0;
            return ptr::null();
        }
        src = src.add(1);
        acc |= one << bit;

        bit = bit.wrapping_add(6);
    }
    value = acc;
    *dst = value;

    src
}

/// ```c
/// const uint8_t *
/// escrypt_parse_setting(const uint8_t *setting,
///                       uint32_t *N_log2_p, uint32_t *r_p, uint32_t *p_p)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_parse_setting(
    setting: *const u8,
    N_log2_p: *mut u32,
    r_p: *mut u32,
    p_p: *mut u32,
) -> *const u8 {
    let mut src: *const u8;

    if *setting.add(0) != b'$' || *setting.add(1) != b'7' || *setting.add(2) != b'$' {
        return ptr::null();
    }
    src = setting.add(3);

    if decode64_one(N_log2_p, *src) != 0 {
        return ptr::null();
    }
    src = src.add(1);

    src = decode64_uint32(r_p, 30, src);
    if src.is_null() {
        return ptr::null();
    }

    src = decode64_uint32(p_p, 30, src);
    if src.is_null() {
        return ptr::null();
    }

    src
}

/// ```c
/// uint8_t *
/// escrypt_r(escrypt_local_t *local, const uint8_t *passwd, size_t passwdlen,
///           const uint8_t *setting, uint8_t *buf, size_t buflen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_r(
    local: *mut escrypt_local_t,
    passwd: *const u8,
    passwdlen: usize,
    setting: *const u8,
    buf: *mut u8,
    buflen: usize,
) -> *mut u8 {
    let mut hash = [0u8; crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES];
    let escrypt_kdf: escrypt_kdf_t;
    let mut src: *const u8;
    let salt: *const u8;
    let mut dst: *mut u8;
    let prefixlen: usize;
    let saltlen: usize;
    let need: usize;
    let N: u64;
    let mut N_log2: u32 = 0;
    let mut r: u32 = 0;
    let mut p: u32 = 0;

    if !buf.is_null() {
        randombytes_buf(buf as *mut c_void, buflen);
    }

    src = _sodium_escrypt_parse_setting(setting, &mut N_log2, &mut r, &mut p);
    if src.is_null() {
        return ptr::null_mut();
    }
    // `(uint64_t) 1 << N_log2`: `N_log2` may legitimately be 64 here (a NUL
    // right after "$7$" makes `strchr()` match the terminator), which is UB in
    // C.  `wrapping_shl` reproduces what x86-64 `shl` does for that input.
    N = 1u64.wrapping_shl(N_log2);
    prefixlen = (src as usize).wrapping_sub(setting as usize);

    salt = src;
    src = strrchr(salt as *const c_char, '$' as c_int) as *const u8;
    if !src.is_null() {
        saltlen = (src as usize).wrapping_sub(salt as usize);
    } else {
        saltlen = strlen(salt as *const c_char);
    }
    need = prefixlen
        .wrapping_add(saltlen)
        .wrapping_add(1)
        .wrapping_add(crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES_ENCODED)
        .wrapping_add(1);
    if buf.is_null() || need > buflen || need < saltlen {
        return ptr::null_mut();
    }
    // HAVE_EMMINTRIN_H undefined
    escrypt_kdf = _sodium_escrypt_kdf_nosse;
    if escrypt_kdf(
        local,
        passwd,
        passwdlen,
        salt,
        saltlen,
        N,
        r,
        p,
        hash.as_mut_ptr(),
        crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES,
    ) != 0
    {
        return ptr::null_mut();
    }
    dst = buf;
    memcpy(dst, setting, prefixlen.wrapping_add(saltlen));
    dst = dst.add(prefixlen.wrapping_add(saltlen));
    *dst = b'$';
    dst = dst.add(1);

    dst = encode64(
        dst,
        buflen.wrapping_sub((dst as usize).wrapping_sub(buf as usize)),
        hash.as_ptr(),
        crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES,
    );
    sodium_memzero(
        hash.as_mut_ptr() as *mut c_void,
        crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES,
    );
    if dst.is_null() || (dst as usize) >= (buf as usize).wrapping_add(buflen) {
        return ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    *dst = 0; /* NUL termination */

    buf
}

/// ```c
/// uint8_t *
/// escrypt_gensalt_r(uint32_t N_log2, uint32_t r, uint32_t p, const uint8_t *src,
///                   size_t srclen, uint8_t *buf, size_t buflen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_gensalt_r(
    N_log2: u32,
    r: u32,
    p: u32,
    src: *const u8,
    srclen: usize,
    buf: *mut u8,
    buflen: usize,
) -> *mut u8 {
    let mut dst: *mut u8;
    // (sizeof "$7$" - 1U) + (1U /* N_log2 */) + (5U /* r */) + (5U /* p */)
    let prefixlen: usize = 3usize + 1usize + 5usize + 5usize;
    let saltlen: usize = BYTES2CHARS(srclen);
    let need: usize;

    need = prefixlen.wrapping_add(saltlen).wrapping_add(1);
    if need > buflen || need < saltlen || saltlen < srclen {
        return ptr::null_mut(); /* LCOV_EXCL_LINE */
    }
    if N_log2 > 63 || ((r as u64).wrapping_mul(p as u64) >= (1u32 << 30) as u64) {
        return ptr::null_mut(); /* LCOV_EXCL_LINE */
    }
    dst = buf;
    *dst = b'$';
    dst = dst.add(1);
    *dst = b'7';
    dst = dst.add(1);
    *dst = b'$';
    dst = dst.add(1);

    *dst = itoa64[N_log2 as usize];
    dst = dst.add(1);

    dst = encode64_uint32(
        dst,
        buflen.wrapping_sub((dst as usize).wrapping_sub(buf as usize)),
        r,
        30,
    );
    if dst.is_null() {
        return ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    dst = encode64_uint32(
        dst,
        buflen.wrapping_sub((dst as usize).wrapping_sub(buf as usize)),
        p,
        30,
    );
    if dst.is_null() {
        return ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    dst = encode64(
        dst,
        buflen.wrapping_sub((dst as usize).wrapping_sub(buf as usize)),
        src,
        srclen,
    );
    if dst.is_null() || (dst as usize) >= (buf as usize).wrapping_add(buflen) {
        return ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    *dst = 0; /* NUL termination */

    buf
}

/// ```c
/// int
/// crypto_pwhash_scryptsalsa208sha256_ll(const uint8_t *passwd, size_t passwdlen,
///                                       const uint8_t *salt, size_t saltlen,
///                                       uint64_t N, uint32_t r, uint32_t p,
///                                       uint8_t *buf, size_t buflen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_ll(
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    N: u64,
    r: u32,
    p: u32,
    buf: *mut u8,
    buflen: usize,
) -> c_int {
    let escrypt_kdf: escrypt_kdf_t;
    let mut local: escrypt_local_t = escrypt_region_t::zeroed();
    let retval: c_int;

    if _sodium_escrypt_init_local(&mut local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    // HAVE_EMMINTRIN_H undefined
    escrypt_kdf = _sodium_escrypt_kdf_nosse;
    retval = escrypt_kdf(
        &mut local, passwd, passwdlen, salt, saltlen, N, r, p, buf, buflen,
    );
    if _sodium_escrypt_free_local(&mut local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }

    retval
}
