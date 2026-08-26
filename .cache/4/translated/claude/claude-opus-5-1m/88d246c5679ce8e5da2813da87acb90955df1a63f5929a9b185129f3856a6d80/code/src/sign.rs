//! Translation of `crypto_sign/ed25519/ref10/sign.c`.
//!
//! `ED25519_NONDETERMINISTIC` is **not** defined in the reference build, so the
//! deterministic nonce path is the one translated here.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* HAVE_TI_MODE is *not* defined: typedef int32_t fe25519[10]; */
pub type fe25519 = [i32; 10];

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
const crypto_sign_ed25519_BYTES: usize = 64;

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: c_ulonglong) -> c_int;
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);

    /* private/ed25519_ref10.h -- names after private/quirks.h renaming */
    #[link_name = "_sodium_ge25519_scalarmult_base"]
    fn ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    #[link_name = "_sodium_ge25519_p3_tobytes"]
    fn ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    #[link_name = "_sodium_sc25519_reduce"]
    fn sc25519_reduce(s: *mut u8);
    #[link_name = "_sodium_sc25519_muladd"]
    fn sc25519_muladd(s: *mut u8, a: *const u8, b: *const u8, c: *const u8);
}

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
        crypto_hash_sha512_update(hs, DOM2PREFIX.as_ptr(), DOM2PREFIX.len() as c_ulonglong);
    }
}

#[inline]
unsafe fn _crypto_sign_ed25519_clamp(k: *mut u8) {
    *k.add(0) &= 248;
    *k.add(31) &= 127;
    *k.add(31) |= 64;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
    prehashed: c_int,
) -> c_int {
    let mut hs: crypto_hash_sha512_state = core::mem::zeroed();
    let mut az: [u8; 64] = [0; 64];
    let mut nonce: [u8; 64] = [0; 64];
    let mut hram: [u8; 64] = [0; 64];
    let mut R: ge25519_p3 = core::mem::zeroed();

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);

    crypto_hash_sha512(az.as_mut_ptr(), sk, 32);
    crypto_hash_sha512_update(&mut hs, az.as_ptr().add(32), 32);

    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, nonce.as_mut_ptr());

    memmove(sig.add(32), sk.add(32), 32);

    sc25519_reduce(nonce.as_mut_ptr());
    ge25519_scalarmult_base(&mut R, nonce.as_ptr());
    ge25519_p3_tobytes(sig, &R);

    _crypto_sign_ed25519_ref10_hinit(&mut hs, prehashed);
    crypto_hash_sha512_update(&mut hs, sig, 64);
    crypto_hash_sha512_update(&mut hs, m, mlen);
    crypto_hash_sha512_final(&mut hs, hram.as_mut_ptr());

    sc25519_reduce(hram.as_mut_ptr());
    _crypto_sign_ed25519_clamp(az.as_mut_ptr());
    sc25519_muladd(
        sig.add(32),
        hram.as_ptr(),
        az.as_ptr(),
        nonce.as_ptr(),
    );

    sodium_memzero(az.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&az));
    sodium_memzero(
        nonce.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&nonce),
    );

    if !siglen_p.is_null() {
        *siglen_p = 64u64;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_detached(
    sig: *mut u8,
    siglen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    _crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519(
    sm: *mut u8,
    smlen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    let mut siglen: c_ulonglong = 0;

    memmove(sm.add(crypto_sign_ed25519_BYTES), m, mlen as usize);
    /* LCOV_EXCL_START */
    if crypto_sign_ed25519_detached(
        sm,
        &mut siglen,
        sm.add(crypto_sign_ed25519_BYTES),
        mlen,
        sk,
    ) != 0
        || siglen != crypto_sign_ed25519_BYTES as c_ulonglong
    {
        if !smlen_p.is_null() {
            *smlen_p = 0;
        }
        memset(
            sm,
            0,
            (mlen.wrapping_add(crypto_sign_ed25519_BYTES as c_ulonglong)) as usize,
        );
        return -1;
    }
    /* LCOV_EXCL_STOP */

    if !smlen_p.is_null() {
        *smlen_p = mlen.wrapping_add(siglen);
    }
    0
}
