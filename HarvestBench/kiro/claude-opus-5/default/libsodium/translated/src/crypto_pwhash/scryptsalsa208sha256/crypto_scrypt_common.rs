//! Translation of c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c

use core::ffi::{c_char, c_int, c_void};

// escrypt_region_t / escrypt_local_t from crypto_scrypt.h.
#[repr(C)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

pub type escrypt_local_t = escrypt_region_t;

// escrypt_kdf_t function pointer type from crypto_scrypt.h.
type escrypt_kdf_t = unsafe extern "C" fn(
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

// crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES from crypto_scrypt.h.
const STRHASHBYTES: usize = 32;
// crypto_pwhash_scryptsalsa208sha256_STRHASHBYTES_ENCODED from crypto_scrypt.h.
const STRHASHBYTES_ENCODED: usize = 43;

// BYTES2CHARS(bytes) ((((bytes) * 8) + 5) / 6) from crypto_scrypt.h.
#[inline]
fn bytes2chars(bytes: usize) -> usize {
    ((bytes * 8) + 5) / 6
}

extern "C" {
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    // Exported symbols reached via extern "C" (rule 3 / rule 7).
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);

    // Defined in scrypt_platform.c / nosse (linker names after quirks.h).
    fn _sodium_escrypt_kdf_nosse(
        local: *mut escrypt_local_t,
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
    fn _sodium_escrypt_init_local(local: *mut escrypt_local_t) -> c_int;
    fn _sodium_escrypt_free_local(local: *mut escrypt_local_t) -> c_int;
}

// static const char *const itoa64
static ITOA64: &[u8; 64] =
    b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Mirror of C's `strchr(itoa64, src)`: returns the index within the 64-char
/// table, or 64 when `src == 0` (strchr matches the terminating NUL), else -1.
#[inline]
unsafe fn itoa64_strchr(src: u8) -> isize {
    let mut i: isize = 0;
    while i < 64 {
        if ITOA64[i as usize] == src {
            return i;
        }
        i += 1;
    }
    // C's strchr also matches the terminating '\0'.
    if src == 0 {
        return 64;
    }
    -1
}

unsafe fn encode64_uint32(
    mut dst: *mut u8,
    mut dstlen: usize,
    mut src: u32,
    srcbits: u32,
) -> *mut u8 {
    let mut bit: u32 = 0;

    while bit < srcbits {
        if dstlen < 1 {
            return core::ptr::null_mut(); /* LCOV_EXCL_LINE */
        }
        *dst = ITOA64[(src & 0x3f) as usize];
        dst = dst.add(1);
        dstlen -= 1;
        src >>= 6;
        bit += 6;
    }
    dst
}

unsafe fn encode64(
    mut dst: *mut u8,
    mut dstlen: usize,
    src: *const u8,
    srclen: usize,
) -> *mut u8 {
    let mut i: usize = 0;

    while i < srclen {
        let dnext: *mut u8;
        let mut value: u32 = 0;
        let mut bits: u32 = 0;

        loop {
            value |= (*src.add(i) as u32) << bits;
            i += 1;
            bits += 8;
            if !(bits < 24 && i < srclen) {
                break;
            }
        }

        dnext = encode64_uint32(dst, dstlen, value, bits);
        if dnext.is_null() {
            return core::ptr::null_mut(); /* LCOV_EXCL_LINE */
        }
        dstlen -= dnext.offset_from(dst) as usize;
        dst = dnext;
    }
    dst
}

unsafe fn decode64_one(dst: *mut u32, src: u8) -> c_int {
    let idx = itoa64_strchr(src);

    if idx >= 0 {
        *dst = idx as u32;
        return 0;
    }
    *dst = 0;

    -1
}

unsafe fn decode64_uint32(dst: *mut u32, dstbits: u32, mut src: *const u8) -> *const u8 {
    let mut bit: u32;
    let mut value: u32;

    value = 0;
    bit = 0;
    while bit < dstbits {
        let mut one: u32 = 0;
        if decode64_one(&mut one, *src) != 0 {
            *dst = 0;
            return core::ptr::null();
        }
        src = src.add(1);
        value |= one << bit;
        bit += 6;
    }
    *dst = value;

    src
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_parse_setting(
    setting: *const u8,
    N_log2_p: *mut u32,
    r_p: *mut u32,
    p_p: *mut u32,
) -> *const u8 {
    let mut src: *const u8;

    if *setting.add(0) != b'$' || *setting.add(1) != b'7' || *setting.add(2) != b'$' {
        return core::ptr::null();
    }
    src = setting.add(3);

    if decode64_one(N_log2_p, *src) != 0 {
        return core::ptr::null();
    }
    src = src.add(1);

    src = decode64_uint32(r_p, 30, src);
    if src.is_null() {
        return core::ptr::null();
    }

    src = decode64_uint32(p_p, 30, src);
    if src.is_null() {
        return core::ptr::null();
    }
    src
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_r(
    local: *mut escrypt_local_t,
    passwd: *const u8,
    passwdlen: usize,
    setting: *const u8,
    buf: *mut u8,
    buflen: usize,
) -> *mut u8 {
    let mut hash: [u8; STRHASHBYTES] = [0u8; STRHASHBYTES];
    let escrypt_kdf: escrypt_kdf_t;
    let mut src: *const u8;
    let salt: *const u8;
    let mut dst: *mut u8;
    let prefixlen: usize;
    let saltlen: usize;
    let need: usize;
    let n: u64;
    let mut N_log2: u32 = 0;
    let mut r: u32 = 0;
    let mut p: u32 = 0;

    if !buf.is_null() {
        randombytes_buf(buf as *mut c_void, buflen);
    }

    src = _sodium_escrypt_parse_setting(setting, &mut N_log2, &mut r, &mut p);
    if src.is_null() {
        return core::ptr::null_mut();
    }
    n = (1u64) << N_log2;
    prefixlen = src.offset_from(setting) as usize;

    salt = src;
    src = strrchr(salt as *const c_char, b'$' as c_int) as *const u8;
    if !src.is_null() {
        saltlen = src.offset_from(salt) as usize;
    } else {
        saltlen = strlen(salt as *const c_char);
    }
    need = prefixlen + saltlen + 1 + STRHASHBYTES_ENCODED + 1;
    if buf.is_null() || need > buflen || need < saltlen {
        return core::ptr::null_mut();
    }
    // HAVE_EMMINTRIN_H undefined: only escrypt_kdf_nosse is available.
    escrypt_kdf = _sodium_escrypt_kdf_nosse;

    if escrypt_kdf(
        local,
        passwd,
        passwdlen,
        salt,
        saltlen,
        n,
        r,
        p,
        hash.as_mut_ptr(),
        core::mem::size_of::<[u8; STRHASHBYTES]>(),
    ) != 0
    {
        return core::ptr::null_mut();
    }
    dst = buf;
    memcpy(
        dst as *mut c_void,
        setting as *const c_void,
        prefixlen + saltlen,
    );
    dst = dst.add(prefixlen + saltlen);
    *dst = b'$';
    dst = dst.add(1);

    dst = encode64(
        dst,
        buflen - (dst.offset_from(buf) as usize),
        hash.as_ptr(),
        core::mem::size_of::<[u8; STRHASHBYTES]>(),
    );
    sodium_memzero(
        hash.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[u8; STRHASHBYTES]>(),
    );
    if dst.is_null() || dst >= buf.add(buflen) {
        return core::ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    *dst = 0; /* NUL termination */

    buf
}

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
    let prefixlen: usize = (b"$7$".len()) + (1) + (5) + (5);
    let saltlen: usize = bytes2chars(srclen);
    let need: usize;

    need = prefixlen + saltlen + 1;
    if need > buflen || need < saltlen || saltlen < srclen {
        return core::ptr::null_mut(); /* LCOV_EXCL_LINE */
    }
    if N_log2 > 63 || ((r as u64) * (p as u64) >= (1u32 << 30) as u64) {
        return core::ptr::null_mut(); /* LCOV_EXCL_LINE */
    }
    dst = buf;
    *dst = b'$';
    dst = dst.add(1);
    *dst = b'7';
    dst = dst.add(1);
    *dst = b'$';
    dst = dst.add(1);

    *dst = ITOA64[N_log2 as usize];
    dst = dst.add(1);

    dst = encode64_uint32(dst, buflen - (dst.offset_from(buf) as usize), r, 30);
    if dst.is_null() {
        return core::ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    dst = encode64_uint32(dst, buflen - (dst.offset_from(buf) as usize), p, 30);
    if dst.is_null() {
        return core::ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    dst = encode64(dst, buflen - (dst.offset_from(buf) as usize), src, srclen);
    if dst.is_null() || dst >= buf.add(buflen) {
        return core::ptr::null_mut(); /* Can't happen LCOV_EXCL_LINE */
    }
    *dst = 0; /* NUL termination */

    buf
}

// crypto_pwhash_scryptsalsa208sha256_SALTBYTES is not used here.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_scryptsalsa208sha256_ll(
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    n: u64,
    r: u32,
    p: u32,
    buf: *mut u8,
    buflen: usize,
) -> c_int {
    let escrypt_kdf: escrypt_kdf_t;
    let mut local: escrypt_local_t = escrypt_region_t {
        base: core::ptr::null_mut(),
        aligned: core::ptr::null_mut(),
        size: 0,
    };
    let retval: c_int;

    if _sodium_escrypt_init_local(&mut local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    // HAVE_EMMINTRIN_H undefined: only escrypt_kdf_nosse is available.
    escrypt_kdf = _sodium_escrypt_kdf_nosse;

    retval = escrypt_kdf(&mut local, passwd, passwdlen, salt, saltlen, n, r, p, buf, buflen);
    if _sodium_escrypt_free_local(&mut local) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    retval
}
