//! Translation of `crypto_aead/aegis256/aead_aegis256.c`.
//!
//! Neither `HAVE_ARMCRYPTO`/`NATIVE_LITTLE_ENDIAN` nor
//! `HAVE_AVXINTRIN_H`/`HAVE_WMMINTRIN_H` are defined in the reference build, so
//! only the portable (`aegis256_soft`) backend exists and the two `#if`-guarded
//! blocks of `_crypto_aead_aegis256_pick_best_implementation()` are removed by
//! the preprocessor.

use core::ffi::{c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------------- */
/* `implementations.h`                                                       */
/* ------------------------------------------------------------------------- */

#[repr(C)]
pub struct aegis256_implementation {
    pub encrypt_detached: unsafe extern "C" fn(
        c: *mut u8,
        mac: *mut u8,
        maclen: usize,
        m: *const u8,
        mlen: usize,
        ad: *const u8,
        adlen: usize,
        npub: *const u8,
        k: *const u8,
    ) -> c_int,
    pub decrypt_detached: unsafe extern "C" fn(
        m: *mut u8,
        c: *const u8,
        clen: usize,
        mac: *const u8,
        maclen: usize,
        ad: *const u8,
        adlen: usize,
        npub: *const u8,
        k: *const u8,
    ) -> c_int,
}

extern "C" {
    /* `aegis256_soft.h` */
    static aegis256_soft_implementation: aegis256_implementation;

    /* `core.h` */
    fn sodium_misuse() -> !;

    /* `randombytes.h` */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* ------------------------------------------------------------------------- */
/* `crypto_aead_aegis256.h`                                                  */
/* ------------------------------------------------------------------------- */

/* #define crypto_aead_aegis256_KEYBYTES 32U */
const crypto_aead_aegis256_KEYBYTES: usize = 32;
/* #define crypto_aead_aegis256_NSECBYTES 0U */
const crypto_aead_aegis256_NSECBYTES: usize = 0;
/* #define crypto_aead_aegis256_NPUBBYTES 32U */
const crypto_aead_aegis256_NPUBBYTES: usize = 32;
/* #define crypto_aead_aegis256_ABYTES 32U */
const crypto_aead_aegis256_ABYTES: usize = 32;
/*
 * #define crypto_aead_aegis256_MESSAGEBYTES_MAX \
 *     SODIUM_MIN(SODIUM_SIZE_MAX - crypto_aead_aegis256_ABYTES, (1ULL << 61) - 1)
 *
 * SODIUM_SIZE_MAX == SODIUM_MIN(UINT64_MAX, SIZE_MAX) == 2^64 - 1 here, so the
 * minimum is (1ULL << 61) - 1.
 */
const crypto_aead_aegis256_MESSAGEBYTES_MAX: c_ulonglong = {
    let a: c_ulonglong = (u64::MAX as c_ulonglong) - (crypto_aead_aegis256_ABYTES as c_ulonglong);
    let b: c_ulonglong = (1u64 << 61) - 1;
    if a < b {
        a
    } else {
        b
    }
};

/* ------------------------------------------------------------------------- */

static mut implementation: *const aegis256_implementation =
    core::ptr::addr_of!(aegis256_soft_implementation);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_keybytes() -> usize {
    crypto_aead_aegis256_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_nsecbytes() -> usize {
    crypto_aead_aegis256_NSECBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_npubbytes() -> usize {
    crypto_aead_aegis256_NPUBBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_abytes() -> usize {
    crypto_aead_aegis256_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_messagebytes_max() -> usize {
    crypto_aead_aegis256_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_aead_aegis256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt(
    c: *mut u8,
    clen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut clen: c_ulonglong = 0;
    let ret: c_int;

    if mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ret = crypto_aead_aegis256_encrypt_detached(
        c,
        c.add(mlen as usize),
        core::ptr::null_mut(),
        m,
        mlen,
        ad,
        adlen,
        nsec,
        npub,
        k,
    );
    if !clen_p.is_null() {
        if ret == 0 {
            clen = mlen + crypto_aead_aegis256_ABYTES as c_ulonglong;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt(
    m: *mut u8,
    mlen_p: *mut c_ulonglong,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut mlen: c_ulonglong = 0;
    let mut ret: c_int = -1;

    if clen >= crypto_aead_aegis256_ABYTES as c_ulonglong {
        ret = crypto_aead_aegis256_decrypt_detached(
            m,
            nsec,
            c,
            clen - crypto_aead_aegis256_ABYTES as c_ulonglong,
            c.add((clen - crypto_aead_aegis256_ABYTES as c_ulonglong) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - crypto_aead_aegis256_ABYTES as c_ulonglong;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let maclen: usize = crypto_aead_aegis256_ABYTES;

    let _ = nsec; /* (void) nsec; */
    if !maclen_p.is_null() {
        *maclen_p = maclen as c_ulonglong;
    }
    if mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX || adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX
    {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*implementation).encrypt_detached)(
        c,
        mac,
        maclen,
        m,
        mlen as usize,
        ad,
        adlen as usize,
        npub,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let maclen: usize = crypto_aead_aegis256_ABYTES;

    let _ = nsec; /* (void) nsec; */
    if clen > crypto_aead_aegis256_MESSAGEBYTES_MAX || adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    ((*implementation).decrypt_detached)(
        m,
        c,
        clen as usize,
        mac,
        maclen,
        ad,
        adlen as usize,
        npub,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_aead_aegis256_pick_best_implementation() -> c_int {
    implementation = core::ptr::addr_of!(aegis256_soft_implementation);

    /*
     * #if defined(HAVE_ARMCRYPTO) && defined(NATIVE_LITTLE_ENDIAN)
     *     if (sodium_runtime_has_armcrypto()) {
     *         implementation = &aegis256_armcrypto_implementation;
     *         return 0;
     *     }
     * #endif
     *
     * #if defined(HAVE_AVXINTRIN_H) && defined(HAVE_WMMINTRIN_H)
     *     if (sodium_runtime_has_aesni() & sodium_runtime_has_avx()) {
     *         implementation = &aegis256_aesni_implementation;
     *         return 0;
     *     }
     * #endif
     *
     * Neither macro pair is defined in this build, so both blocks are removed
     * by the preprocessor.
     */
    0 /* LCOV_EXCL_LINE */
}
