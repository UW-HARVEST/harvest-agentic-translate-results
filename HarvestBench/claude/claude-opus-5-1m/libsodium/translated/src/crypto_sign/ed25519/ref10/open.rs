//! Translation of `crypto_sign/ed25519/ref10/open.c`.
//!
//! `ED25519_COMPAT` is **not** defined in the reference build, so the
//! canonicality checks (rather than the `sig[63] & 224` test) are compiled in.
//! `ACQUIRE_FENCE` expands to `(void) 0` because neither
//! `HAVE_GCC_MEMORY_FENCES` nor `HAVE_C11_MEMORY_FENCES` is defined.
#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::c_int;

use crate::crypto_core::ed25519::types::{Ge25519P2, Ge25519P3};
use crate::crypto_hash::sha512::crypto_hash_sha512_state;

use super::super::crypto_sign_ed25519_MESSAGEBYTES_MAX;
use super::sign::_crypto_sign_ed25519_ref10_hinit;

unsafe extern "C" {
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;
    fn _sodium_ge25519_frombytes(h: *mut Ge25519P3, s: *const u8) -> c_int;
    fn _sodium_ge25519_frombytes_negate_vartime(h: *mut Ge25519P3, s: *const u8) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const Ge25519P3) -> c_int;
    fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_ge25519_double_scalarmult_vartime(
        r: *mut Ge25519P2,
        a: *const u8,
        A: *const Ge25519P3,
        b: *const u8,
    );
    fn _sodium_ge25519_p2_to_p3(r: *mut Ge25519P3, p: *const Ge25519P2);
    fn _sodium_ge25519_p3_sub(r: *mut Ge25519P3, p: *const Ge25519P3, q: *const Ge25519P3);
    fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_sc25519_reduce(s: *mut u8);
}

/// ```c
/// int
/// _crypto_sign_ed25519_verify_detached(const unsigned char *sig,
///                                      const unsigned char *m,
///                                      unsigned long long   mlen,
///                                      const unsigned char *pk,
///                                      int prehashed)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs = crypto_hash_sha512_state {
        state: [0u64; 8],
        count: [0u64; 2],
        buf: [0u8; 128],
    };
    let mut h = [0u8; 64];
    let mut check: Ge25519P3 = Ge25519P3::default();
    let mut expected_r: Ge25519P3 = Ge25519P3::default();
    let mut A: Ge25519P3 = Ge25519P3::default();
    let mut sb_ah: Ge25519P3 = Ge25519P3::default();
    let mut sb_ah_p2: Ge25519P2 = Ge25519P2::default();

    /* ACQUIRE_FENCE is `(void) 0` in this configuration */
    /* !ED25519_COMPAT */
    if (*sig.add(63) & 240) != 0 && _sodium_sc25519_is_canonical(sig.add(32)) == 0 {
        return -1;
    }
    if _sodium_ge25519_is_canonical(pk) == 0 {
        return -1;
    }
    if _sodium_ge25519_frombytes_negate_vartime(&mut A, pk) != 0
        || _sodium_ge25519_has_small_order(&A) != 0
    {
        return -1;
    }
    if _sodium_ge25519_frombytes(&mut expected_r, sig) != 0
        || _sodium_ge25519_has_small_order(&expected_r) != 0
    {
        return -1;
    }
    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 32);
    crypto_hash_sha512_update(&mut hs, pk, 32);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, h.as_mut_ptr());
    _sodium_sc25519_reduce(h.as_mut_ptr());

    _sodium_ge25519_double_scalarmult_vartime(&mut sb_ah_p2, h.as_ptr(), &A, sig.add(32));
    _sodium_ge25519_p2_to_p3(&mut sb_ah, &sb_ah_p2);
    _sodium_ge25519_p3_sub(&mut check, &expected_r, &sb_ah);

    _sodium_ge25519_has_small_order(&check) - 1
}

/// ```c
/// int
/// crypto_sign_ed25519_verify_detached(const unsigned char *sig,
///                                     const unsigned char *m,
///                                     unsigned long long   mlen,
///                                     const unsigned char *pk)
/// {
///     return _crypto_sign_ed25519_verify_detached(sig, m, mlen, pk, 0);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_verify_detached(sig, m, mlen, pk, 0)
}

/// ```c
/// int
/// crypto_sign_ed25519_open(unsigned char *m, unsigned long long *mlen_p,
///                          const unsigned char *sm, unsigned long long smlen,
///                          const unsigned char *pk)
/// {
///     unsigned long long mlen;
///
///     if (smlen < 64 || smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX) {
///         goto badsig;
///     }
///     mlen = smlen - 64;
///     if (crypto_sign_ed25519_verify_detached(sm, sm + 64, mlen, pk) != 0) {
///         if (m != NULL) {
///             memset(m, 0, mlen);
///         }
///         goto badsig;
///     }
///     if (mlen_p != NULL) {
///         *mlen_p = mlen;
///     }
///     if (m != NULL) {
///         memmove(m, sm + 64, mlen);
///     }
///     return 0;
///
/// badsig:
///     if (mlen_p != NULL) {
///         *mlen_p = 0;
///     }
///     return -1;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_open(
    m: *mut u8,
    mlen_p: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    let mlen: u64;

    'badsig: {
        if smlen < 64 || smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX as u64 {
            break 'badsig;
        }
        mlen = smlen - 64;
        if crypto_sign_ed25519_verify_detached(sm, sm.add(64), mlen, pk) != 0 {
            if !m.is_null() {
                crate::common::memset(m, 0, mlen as usize);
            }
            break 'badsig;
        }
        if !mlen_p.is_null() {
            *mlen_p = mlen;
        }
        if !m.is_null() {
            core::ptr::copy(sm.add(64), m, mlen as usize);
        }
        return 0;
    }

    /* badsig: */
    if !mlen_p.is_null() {
        *mlen_p = 0;
    }
    -1
}
