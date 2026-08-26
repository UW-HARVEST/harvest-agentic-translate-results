pub mod soft;

// Translation of `crypto_aead/aegis128l/aead_aegis128l.c`.
//
// Neither `HAVE_ARMCRYPTO`/`NATIVE_LITTLE_ENDIAN` nor
// `HAVE_AVXINTRIN_H`/`HAVE_WMMINTRIN_H` are defined in the reference build, so
// the armcrypto and aesni backends do not exist: the soft backend is the only
// implementation and `_crypto_aead_aegis128l_pick_best_implementation()`
// merely re-selects it and returns 0.

use core::ffi::{c_int, c_void};

use crate::randombytes::randombytes_buf;
use crate::sodium::core::sodium_misuse;

// ---------------------------------------------------------------------------
// constants (include/sodium/crypto_aead_aegis128l.h)
// ---------------------------------------------------------------------------

pub const crypto_aead_aegis128l_KEYBYTES: usize = 16;
pub const crypto_aead_aegis128l_NSECBYTES: usize = 0;
pub const crypto_aead_aegis128l_NPUBBYTES: usize = 16;
pub const crypto_aead_aegis128l_ABYTES: usize = 32;

/// `SODIUM_MIN(SODIUM_SIZE_MAX - crypto_aead_aegis128l_ABYTES, (1ULL << 61) - 1)`
pub const crypto_aead_aegis128l_MESSAGEBYTES_MAX: u64 = {
    let a = crate::common::SODIUM_SIZE_MAX - crypto_aead_aegis128l_ABYTES as u64;
    let b = (1u64 << 61) - 1;
    if a < b { a } else { b }
};

// ---------------------------------------------------------------------------
// crypto_aead/aegis128l/implementations.h
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct aegis128l_implementation {
///     int (*encrypt_detached)(uint8_t *c, uint8_t *mac, size_t maclen, const uint8_t *m,
///                             size_t mlen, const uint8_t *ad, size_t adlen,
///                             const uint8_t *npub, const uint8_t *k);
///     int (*decrypt_detached)(uint8_t *m, const uint8_t *c, size_t clen, const uint8_t *mac,
///                             size_t maclen, const uint8_t *ad, size_t adlen,
///                             const uint8_t *npub, const uint8_t *k);
/// } aegis128l_implementation;
/// ```
#[repr(C)]
pub struct aegis128l_implementation {
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

/// `static const aegis128l_implementation *implementation = &aegis128l_soft_implementation;`
static mut implementation: *const aegis128l_implementation =
    &raw const soft::aegis128l_soft_implementation;

// ---------------------------------------------------------------------------
// aead_aegis128l.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_keybytes() -> usize {
    crypto_aead_aegis128l_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_nsecbytes() -> usize {
    crypto_aead_aegis128l_NSECBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_npubbytes() -> usize {
    crypto_aead_aegis128l_NPUBBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_abytes() -> usize {
    crypto_aead_aegis128l_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_messagebytes_max() -> usize {
    crypto_aead_aegis128l_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_aead_aegis128l_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_encrypt(
    c: *mut u8,
    clen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        let mut clen: u64 = 0;
        let ret: c_int;

        if mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX {
            sodium_misuse();
        }
        ret = crypto_aead_aegis128l_encrypt_detached(
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
                clen = mlen.wrapping_add(crypto_aead_aegis128l_ABYTES as u64);
            }
            *clen_p = clen;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_decrypt(
    m: *mut u8,
    mlen_p: *mut u64,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        let mut mlen: u64 = 0;
        let mut ret: c_int = -1;

        if clen >= crypto_aead_aegis128l_ABYTES as u64 {
            ret = crypto_aead_aegis128l_decrypt_detached(
                m,
                nsec,
                c,
                clen.wrapping_sub(crypto_aead_aegis128l_ABYTES as u64),
                c.add(clen as usize)
                    .offset(-(crypto_aead_aegis128l_ABYTES as isize)),
                ad,
                adlen,
                npub,
                k,
            );
        }
        if !mlen_p.is_null() {
            if ret == 0 {
                mlen = clen.wrapping_sub(crypto_aead_aegis128l_ABYTES as u64);
            }
            *mlen_p = mlen;
        }
        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        let maclen: usize = crypto_aead_aegis128l_ABYTES;

        let _ = nsec;
        if !maclen_p.is_null() {
            *maclen_p = maclen as u64;
        }
        if mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX
            || adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX
        {
            sodium_misuse();
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis128l_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    mac: *const u8,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        let maclen: usize = crypto_aead_aegis128l_ABYTES;

        let _ = nsec;
        if clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX
            || adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX
        {
            return -1;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_aead_aegis128l_pick_best_implementation() -> c_int {
    unsafe {
        implementation = &raw const soft::aegis128l_soft_implementation;

        0
    }
}
