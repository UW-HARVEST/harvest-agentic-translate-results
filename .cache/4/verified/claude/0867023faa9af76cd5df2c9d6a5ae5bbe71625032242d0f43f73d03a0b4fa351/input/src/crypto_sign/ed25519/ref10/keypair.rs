//! Translation of `crypto_sign/ed25519/ref10/keypair.c`.
#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};

use crate::crypto_core::ed25519::fe25519::{
    fe25519_1, fe25519_add, fe25519_invert_in_place, fe25519_mul, fe25519_sub, fe25519_tobytes,
};
use crate::crypto_core::ed25519::types::{Fe25519, Ge25519P3};
use crate::crypto_hash::sha512::crypto_hash_sha512_BYTES;
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::sodium_memzero;

/// `#define crypto_scalarmult_curve25519_BYTES 32U`
const crypto_scalarmult_curve25519_BYTES: usize = 32;

unsafe extern "C" {
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
    fn _sodium_ge25519_scalarmult_base(h: *mut Ge25519P3, a: *const u8);
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const Ge25519P3);
    fn _sodium_ge25519_frombytes_negate_vartime(h: *mut Ge25519P3, s: *const u8) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const Ge25519P3) -> c_int;
    fn _sodium_ge25519_is_on_main_subgroup(p: *const Ge25519P3) -> c_int;
}

/// ```c
/// int
/// crypto_sign_ed25519_seed_keypair(unsigned char *pk, unsigned char *sk,
///                                  const unsigned char *seed)
/// {
///     ge25519_p3 A;
///
///     crypto_hash_sha512(sk, seed, 32);
///     sk[0] &= 248;
///     sk[31] &= 127;
///     sk[31] |= 64;
///
///     ge25519_scalarmult_base(&A, sk);
///     ge25519_p3_tobytes(pk, &A);
///
///     memmove(sk, seed, 32);
///     memmove(sk + 32, pk, 32);
///
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut A: Ge25519P3 = Ge25519P3::default();

    crypto_hash_sha512(sk, seed, 32);
    *sk.add(0) &= 248;
    *sk.add(31) &= 127;
    *sk.add(31) |= 64;

    _sodium_ge25519_scalarmult_base(&mut A, sk);
    _sodium_ge25519_p3_tobytes(pk, &A);

    core::ptr::copy(seed, sk, 32);
    core::ptr::copy(pk as *const u8, sk.add(32), 32);

    0
}

/// ```c
/// int
/// crypto_sign_ed25519_keypair(unsigned char *pk, unsigned char *sk)
/// {
///     unsigned char seed[32];
///     int           ret;
///
///     randombytes_buf(seed, sizeof seed);
///     ret = crypto_sign_ed25519_seed_keypair(pk, sk, seed);
///     sodium_memzero(seed, sizeof seed);
///
///     return ret;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; 32];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 32);
    ret = crypto_sign_ed25519_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, 32);

    ret
}

/// ```c
/// int
/// crypto_sign_ed25519_pk_to_curve25519(unsigned char *curve25519_pk,
///                                      const unsigned char *ed25519_pk)
/// {
///     ge25519_p3 A;
///     fe25519    x;
///     fe25519    one_minus_y;
///
///     if (ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0 ||
///         ge25519_has_small_order(&A) != 0 ||
///         ge25519_is_on_main_subgroup(&A) == 0) {
///         return -1;
///     }
///     fe25519_1(one_minus_y);
///     /* assumes A.Z=1 */
///     fe25519_sub(one_minus_y, one_minus_y, A.Y);
///     fe25519_1(x);
///     fe25519_add(x, x, A.Y);
///     fe25519_invert(one_minus_y, one_minus_y);
///     fe25519_mul(x, x, one_minus_y);
///     fe25519_tobytes(curve25519_pk, x);
///
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_pk_to_curve25519(
    curve25519_pk: *mut u8,
    ed25519_pk: *const u8,
) -> c_int {
    let mut A: Ge25519P3 = Ge25519P3::default();
    let mut x: Fe25519 = Fe25519::ZERO;
    let mut one_minus_y: Fe25519 = Fe25519::ZERO;

    if _sodium_ge25519_frombytes_negate_vartime(&mut A, ed25519_pk) != 0
        || _sodium_ge25519_has_small_order(&A) != 0
        || _sodium_ge25519_is_on_main_subgroup(&A) == 0
    {
        return -1;
    }
    fe25519_1(&mut one_minus_y);
    /* assumes A.Z=1 */
    let t = one_minus_y;
    fe25519_sub(&mut one_minus_y, t, A.Y);
    fe25519_1(&mut x);
    let t = x;
    fe25519_add(&mut x, t, A.Y);
    fe25519_invert_in_place(&mut one_minus_y);
    let t = x;
    fe25519_mul(&mut x, t, one_minus_y);
    let mut out = [0u8; 32];
    fe25519_tobytes(&mut out, &x);
    core::ptr::copy_nonoverlapping(out.as_ptr(), curve25519_pk, 32);

    0
}

/// ```c
/// int
/// crypto_sign_ed25519_sk_to_curve25519(unsigned char *curve25519_sk,
///                                      const unsigned char *ed25519_sk)
/// {
///     unsigned char h[crypto_hash_sha512_BYTES];
///
///     crypto_hash_sha512(h, ed25519_sk, 32);
///     h[0] &= 248;
///     h[31] &= 127;
///     h[31] |= 64;
///     memcpy(curve25519_sk, h, crypto_scalarmult_curve25519_BYTES);
///     sodium_memzero(h, sizeof h);
///
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_curve25519(
    curve25519_sk: *mut u8,
    ed25519_sk: *const u8,
) -> c_int {
    let mut h = [0u8; crypto_hash_sha512_BYTES];

    crypto_hash_sha512(h.as_mut_ptr(), ed25519_sk, 32);
    h[0] &= 248;
    h[31] &= 127;
    h[31] |= 64;
    core::ptr::copy_nonoverlapping(
        h.as_ptr(),
        curve25519_sk,
        crypto_scalarmult_curve25519_BYTES,
    );
    sodium_memzero(h.as_mut_ptr() as *mut c_void, crypto_hash_sha512_BYTES);

    0
}
