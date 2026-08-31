//! Translation of c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c

use core::ffi::c_int;

use crate::fe25519::ge25519_p3;

// crypto_scalarmult_ristretto255_BYTES / SCALARBYTES (32U each).
const crypto_scalarmult_ristretto255_BYTES: usize = 32;
const crypto_scalarmult_ristretto255_SCALARBYTES: usize = 32;

extern "C" {
    fn _sodium_ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    fn _sodium_ge25519_scalarmult(h: *mut ge25519_p3, a: *const u8, p: *const ge25519_p3);
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let t: *mut u8 = q;
    let mut q_ge = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut p_ge = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut i: u32;

    if _sodium_ristretto255_frombytes(p_ge.as_mut_ptr(), p) != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    *t.add(31) &= 127;
    _sodium_ge25519_scalarmult(q_ge.as_mut_ptr(), t, p_ge.as_ptr());
    _sodium_ristretto255_p3_tobytes(q, q_ge.as_ptr());
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    let t: *mut u8 = q;
    let mut q_ge = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut i: u32;

    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    *t.add(31) &= 127;
    _sodium_ge25519_scalarmult_base(q_ge.as_mut_ptr(), t);
    _sodium_ristretto255_p3_tobytes(q, q_ge.as_ptr());
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_bytes() -> usize {
    crypto_scalarmult_ristretto255_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_scalarbytes() -> usize {
    crypto_scalarmult_ristretto255_SCALARBYTES
}
