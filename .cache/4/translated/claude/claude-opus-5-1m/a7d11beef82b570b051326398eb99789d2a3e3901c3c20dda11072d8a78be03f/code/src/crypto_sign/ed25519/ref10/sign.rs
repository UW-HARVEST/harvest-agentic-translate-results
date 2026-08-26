//! Translation of `crypto_sign/ed25519/ref10/sign.c`.
//!
//! `ED25519_NONDETERMINISTIC` is **not** defined in the reference build, so the
//! deterministic nonce derivation (`crypto_hash_sha512_update(&hs, az + 32, 32)`)
//! is the one that is compiled in.
#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};

use crate::crypto_core::ed25519::types::Ge25519P3;
use crate::crypto_hash::sha512::crypto_hash_sha512_state;
use crate::sodium::utils::sodium_memzero;

use super::super::crypto_sign_ed25519_BYTES;

unsafe extern "C" {
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;
    fn _sodium_ge25519_scalarmult_base(h: *mut Ge25519P3, a: *const u8);
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const Ge25519P3);
    fn _sodium_sc25519_reduce(s: *mut u8);
    fn _sodium_sc25519_muladd(s: *mut u8, a: *const u8, b: *const u8, c: *const u8);
}

/// ```c
/// void
/// _crypto_sign_ed25519_ref10_hinit(crypto_hash_sha512_state *hs, int prehashed)
/// {
///     static const unsigned char DOM2PREFIX[32 + 2] = {
///         'S', 'i', 'g', 'E', 'd', '2', '5', '5', '1', '9', ' ',
///         'n', 'o', ' ',
///         'E', 'd', '2', '5', '5', '1', '9', ' ',
///         'c', 'o', 'l', 'l', 'i', 's', 'i', 'o', 'n', 's', 1, 0
///     };
///
///     crypto_hash_sha512_init(hs);
///     if (prehashed) {
///         crypto_hash_sha512_update(hs, DOM2PREFIX, sizeof DOM2PREFIX);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_ref10_hinit(
    hs: *mut crypto_hash_sha512_state,
    prehashed: c_int,
) {
    static DOM2PREFIX: [u8; 32 + 2] = [
        b'S', b'i', b'g', b'E', b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'n', b'o', b' ', b'E',
        b'd', b'2', b'5', b'5', b'1', b'9', b' ', b'c', b'o', b'l', b'l', b'i', b's', b'i', b'o',
        b'n', b's', 1, 0,
    ];

    crypto_hash_sha512_init(hs);
    if prehashed != 0 {
        crypto_hash_sha512_update(hs, DOM2PREFIX.as_ptr(), DOM2PREFIX.len() as u64);
    }
}

/// ```c
/// static inline void
/// _crypto_sign_ed25519_clamp(unsigned char k[32])
/// {
///     k[0] &= 248;
///     k[31] &= 127;
///     k[31] |= 64;
/// }
/// ```
#[inline]
unsafe fn _crypto_sign_ed25519_clamp(k: *mut u8) {
    *k.add(0) &= 248;
    *k.add(31) &= 127;
    *k.add(31) |= 64;
}

/// ```c
/// int
/// _crypto_sign_ed25519_detached(unsigned char *sig, unsigned long long *siglen_p,
///                               const unsigned char *m, unsigned long long mlen,
///                               const unsigned char *sk, int prehashed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs = crypto_hash_sha512_state {
        state: [0u64; 8],
        count: [0u64; 2],
        buf: [0u8; 128],
    };
    let mut az = [0u8; 64];
    let mut nonce = [0u8; 64];
    let mut hram = [0u8; 64];
    let mut R: Ge25519P3 = Ge25519P3::default();

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);

    crypto_hash_sha512(az.as_mut_ptr(), sk, 32);
    /* !ED25519_NONDETERMINISTIC */
    crypto_hash_sha512_update(&mut hs, az.as_ptr().add(32), 32);

    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, nonce.as_mut_ptr());

    core::ptr::copy(sk.add(32), sig.add(32), 32);

    _sodium_sc25519_reduce(nonce.as_mut_ptr());
    _sodium_ge25519_scalarmult_base(&mut R, nonce.as_ptr());
    _sodium_ge25519_p3_tobytes(sig, &R);

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig as *const u8, 64);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, hram.as_mut_ptr());

    _sodium_sc25519_reduce(hram.as_mut_ptr());
    _crypto_sign_ed25519_clamp(az.as_mut_ptr());
    _sodium_sc25519_muladd(
        sig.add(32),
        hram.as_ptr(),
        az.as_ptr(),
        nonce.as_ptr(),
    );

    sodium_memzero(az.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(nonce.as_mut_ptr() as *mut c_void, 64);

    if !siglen_p.is_null() {
        *siglen_p = 64u64;
    }
    0
}

/// ```c
/// int
/// crypto_sign_ed25519_detached(unsigned char *sig, unsigned long long *siglen_p,
///                              const unsigned char *m, unsigned long long mlen,
///                              const unsigned char *sk)
/// {
///     return _crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk, 0);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk, 0)
}

/// ```c
/// int
/// crypto_sign_ed25519(unsigned char *sm, unsigned long long *smlen_p,
///                     const unsigned char *m, unsigned long long mlen,
///                     const unsigned char *sk)
/// {
///     unsigned long long siglen;
///
///     memmove(sm + crypto_sign_ed25519_BYTES, m, mlen);
///     /* LCOV_EXCL_START */
///     if (crypto_sign_ed25519_detached(
///             sm, &siglen, sm + crypto_sign_ed25519_BYTES, mlen, sk) != 0 ||
///         siglen != crypto_sign_ed25519_BYTES) {
///         if (smlen_p != NULL) {
///             *smlen_p = 0;
///         }
///         memset(sm, 0, mlen + crypto_sign_ed25519_BYTES);
///         return -1;
///     }
///     /* LCOV_EXCL_STOP */
///
///     if (smlen_p != NULL) {
///         *smlen_p = mlen + siglen;
///     }
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519(
    sm: *mut u8,
    smlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    let mut siglen: u64 = 0;

    core::ptr::copy(m, sm.add(crypto_sign_ed25519_BYTES), mlen as usize);
    /* LCOV_EXCL_START */
    if crypto_sign_ed25519_detached(
        sm,
        &mut siglen,
        sm.add(crypto_sign_ed25519_BYTES) as *const u8,
        mlen,
        sk,
    ) != 0
        || siglen != crypto_sign_ed25519_BYTES as u64
    {
        if !smlen_p.is_null() {
            *smlen_p = 0;
        }
        crate::common::memset(
            sm,
            0,
            mlen.wrapping_add(crypto_sign_ed25519_BYTES as u64) as usize,
        );
        return -1;
    }
    /* LCOV_EXCL_STOP */

    if !smlen_p.is_null() {
        *smlen_p = mlen.wrapping_add(siglen);
    }
    0
}
