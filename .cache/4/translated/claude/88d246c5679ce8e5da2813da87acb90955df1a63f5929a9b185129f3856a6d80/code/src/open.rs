//! Translation of `crypto_sign/ed25519/ref10/open.c`.
//!
//! `ED25519_COMPAT` is **not** defined in the reference build, so the
//! canonicality-checking branch is the one translated here.  `ACQUIRE_FENCE`
//! expands to `(void) 0` because neither `HAVE_GCC_MEMORY_FENCES` nor
//! `HAVE_C11_MEMORY_FENCES` is defined.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong};

/* HAVE_TI_MODE is *not* defined: typedef int32_t fe25519[10]; */
pub type fe25519 = [i32; 10];

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p2 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p3 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

/* Layout of `crypto_hash_sha512_state` from
 * include/sodium/crypto_hash_sha512.h (sizeof == 208 on x86-64). */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

/* #define crypto_sign_ed25519_BYTES 64U */
const crypto_sign_ed25519_BYTES: u64 = 64;
/* #define crypto_sign_ed25519_MESSAGEBYTES_MAX
 *     (SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES) */
const crypto_sign_ed25519_MESSAGEBYTES_MAX: u64 = SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES;

extern "C" {
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;

    /* crypto_sign/ed25519/ref10/sign.c */
    fn _crypto_sign_ed25519_ref10_hinit(hs: *mut crypto_hash_sha512_state, prehashed: c_int);

    /* private/ed25519_ref10.h -- names after private/quirks.h renaming */
    #[link_name = "_sodium_ge25519_frombytes"]
    fn ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_frombytes_negate_vartime"]
    fn ge25519_frombytes_negate_vartime(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_has_small_order"]
    fn ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ge25519_is_canonical"]
    fn ge25519_is_canonical(s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_p2_to_p3"]
    fn ge25519_p2_to_p3(r: *mut ge25519_p3, p: *const ge25519_p2);
    #[link_name = "_sodium_ge25519_p3_sub"]
    fn ge25519_p3_sub(r: *mut ge25519_p3, p: *const ge25519_p3, q: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_double_scalarmult_vartime"]
    fn ge25519_double_scalarmult_vartime(
        r: *mut ge25519_p2,
        a: *const u8,
        A: *const ge25519_p3,
        b: *const u8,
    );
    #[link_name = "_sodium_sc25519_reduce"]
    fn sc25519_reduce(s: *mut u8);
    #[link_name = "_sodium_sc25519_is_canonical"]
    fn sc25519_is_canonical(s: *const u8) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    pk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs: crypto_hash_sha512_state = core::mem::zeroed();
    let mut h: [u8; 64] = [0; 64];
    let mut check: ge25519_p3 = core::mem::zeroed();
    let mut expected_r: ge25519_p3 = core::mem::zeroed();
    let mut A: ge25519_p3 = core::mem::zeroed();
    let mut sb_ah: ge25519_p3 = core::mem::zeroed();
    let mut sb_ah_p2: ge25519_p2 = core::mem::zeroed();

    /* ACQUIRE_FENCE == (void) 0 */

    if (*sig.add(63) & 240) != 0 && sc25519_is_canonical(sig.add(32)) == 0 {
        return -1;
    }
    if ge25519_is_canonical(pk) == 0 {
        return -1;
    }
    if ge25519_frombytes_negate_vartime(&mut A, pk) != 0 || ge25519_has_small_order(&A) != 0 {
        return -1;
    }
    if ge25519_frombytes(&mut expected_r, sig) != 0
        || ge25519_has_small_order(&expected_r) != 0
    {
        return -1;
    }
    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 32);
    crypto_hash_sha512_update(&mut hs, pk, 32);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, h.as_mut_ptr());
    sc25519_reduce(h.as_mut_ptr());

    ge25519_double_scalarmult_vartime(&mut sb_ah_p2, h.as_ptr(), &A, sig.add(32));
    ge25519_p2_to_p3(&mut sb_ah, &sb_ah_p2);
    ge25519_p3_sub(&mut check, &expected_r, &sb_ah);

    ge25519_has_small_order(&check).wrapping_sub(1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_verify_detached(sig, m, mlen, pk, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_open(
    m: *mut u8,
    mlen_p: *mut c_ulonglong,
    sm: *const u8,
    smlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    let mlen: c_ulonglong;

    'badsig: loop {
        if smlen < 64 || smlen.wrapping_sub(64) > crypto_sign_ed25519_MESSAGEBYTES_MAX {
            break 'badsig;
        }
        mlen = smlen - 64;
        if crypto_sign_ed25519_verify_detached(sm, sm.add(64), mlen, pk) != 0 {
            if !m.is_null() {
                memset(m, 0, mlen as usize);
            }
            break 'badsig;
        }
        if !mlen_p.is_null() {
            *mlen_p = mlen;
        }
        if !m.is_null() {
            memmove(m, sm.add(64), mlen as usize);
        }
        return 0;
    }

    /* badsig: */
    if !mlen_p.is_null() {
        *mlen_p = 0;
    }
    -1
}
