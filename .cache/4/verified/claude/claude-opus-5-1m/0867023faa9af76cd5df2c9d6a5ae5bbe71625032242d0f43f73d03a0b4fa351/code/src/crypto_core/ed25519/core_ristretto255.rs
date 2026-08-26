//! Translation of `crypto_core/ed25519/core_ristretto255.c`.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};

use crate::crypto_core::ed25519::core_ed25519::{
    crypto_core_ed25519_scalar_add, crypto_core_ed25519_scalar_complement,
    crypto_core_ed25519_scalar_from_string, crypto_core_ed25519_scalar_invert,
    crypto_core_ed25519_scalar_negate, crypto_core_ed25519_scalar_random,
    crypto_core_ed25519_scalar_reduce, crypto_core_ed25519_scalar_sub,
};
use crate::crypto_core::ed25519::core_h2c::_sodium_core_h2c_string_to_hash;
use crate::crypto_core::ed25519::types::Ge25519P3;
use crate::randombytes::randombytes_buf;

/// `crypto_core_ristretto255_BYTES`
pub const crypto_core_ristretto255_BYTES: usize = 32;
/// `crypto_core_ristretto255_HASHBYTES`
pub const crypto_core_ristretto255_HASHBYTES: usize = 64;
/// `crypto_core_ristretto255_SCALARBYTES`
pub const crypto_core_ristretto255_SCALARBYTES: usize = 32;
/// `crypto_core_ristretto255_NONREDUCEDSCALARBYTES`
pub const crypto_core_ristretto255_NONREDUCEDSCALARBYTES: usize = 64;

unsafe extern "C" {
    fn _sodium_ristretto255_frombytes(h: *mut Ge25519P3, s: *const u8) -> c_int;
    fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const Ge25519P3);
    fn _sodium_ristretto255_from_hash(s: *mut u8, h: *const u8);
    fn _sodium_ge25519_p3_add(r: *mut Ge25519P3, p: *const Ge25519P3, q: *const Ge25519P3);
    fn _sodium_ge25519_p3_sub(r: *mut Ge25519P3, p: *const Ge25519P3, q: *const Ge25519P3);
    fn _sodium_sc25519_mul(s: *mut u8, a: *const u8, b: *const u8);
    fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int;
}

/// ```c
/// int
/// crypto_core_ristretto255_is_valid_point(const unsigned char *p)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_is_valid_point(p: *const u8) -> c_int {
    let mut p_p3 = Ge25519P3::default();

    if _sodium_ristretto255_frombytes(&mut p_p3, p) != 0 {
        return 0;
    }
    1
}

/// ```c
/// int
/// crypto_core_ristretto255_add(unsigned char *r,
///                              const unsigned char *p, const unsigned char *q)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_add(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3 = Ge25519P3::default();
    let mut q_p3 = Ge25519P3::default();
    let mut r_p3 = Ge25519P3::default();

    if _sodium_ristretto255_frombytes(&mut p_p3, p) != 0
        || _sodium_ristretto255_frombytes(&mut q_p3, q) != 0
    {
        return -1;
    }
    _sodium_ge25519_p3_add(&mut r_p3, &p_p3, &q_p3);
    _sodium_ristretto255_p3_tobytes(r, &r_p3);

    0
}

/// ```c
/// int
/// crypto_core_ristretto255_sub(unsigned char *r,
///                              const unsigned char *p, const unsigned char *q)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3 = Ge25519P3::default();
    let mut q_p3 = Ge25519P3::default();
    let mut r_p3 = Ge25519P3::default();

    if _sodium_ristretto255_frombytes(&mut p_p3, p) != 0
        || _sodium_ristretto255_frombytes(&mut q_p3, q) != 0
    {
        return -1;
    }
    _sodium_ge25519_p3_sub(&mut r_p3, &p_p3, &q_p3);
    _sodium_ristretto255_p3_tobytes(r, &r_p3);

    0
}

/// ```c
/// int
/// crypto_core_ristretto255_from_hash(unsigned char *p, const unsigned char *r)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_from_hash(
    p: *mut u8,
    r: *const u8,
) -> c_int {
    _sodium_ristretto255_from_hash(p, r);

    0
}

/// ```c
/// static int
/// _string_to_element(unsigned char *p,
///                    const unsigned char *ctx, size_t ctx_len,
///                    const unsigned char *msg, size_t msg_len,
///                    int hash_alg)
/// ```
unsafe fn _string_to_element(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h: [u8; crypto_core_ristretto255_HASHBYTES] = [0; crypto_core_ristretto255_HASHBYTES];

    if _sodium_core_h2c_string_to_hash(
        h.as_mut_ptr(),
        h.len(),
        ctx,
        ctx_len,
        msg,
        msg_len,
        hash_alg,
    ) != 0
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    _sodium_ristretto255_from_hash(p, h.as_ptr());

    0
}

/// ```c
/// int
/// crypto_core_ristretto255_from_string(unsigned char p[crypto_core_ristretto255_BYTES],
///                                         const unsigned char *ctx, size_t ctx_len,
///                                         const unsigned char *msg, size_t msg_len,
///                                         int hash_alg)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_from_string(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    _string_to_element(p, ctx, ctx_len, msg, msg_len, hash_alg)
}

/// ```c
/// void
/// crypto_core_ristretto255_random(unsigned char *p)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_random(p: *mut u8) {
    let mut h: [u8; crypto_core_ristretto255_HASHBYTES] = [0; crypto_core_ristretto255_HASHBYTES];

    randombytes_buf(h.as_mut_ptr() as *mut c_void, h.len());
    let _ = crypto_core_ristretto255_from_hash(p, h.as_ptr());
}

/// ```c
/// void
/// crypto_core_ristretto255_scalar_random(unsigned char *r)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_random(r: *mut u8) {
    crypto_core_ed25519_scalar_random(r);
}

/// ```c
/// int
/// crypto_core_ristretto255_scalar_invert(unsigned char *recip,
///                                        const unsigned char *s)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_invert(
    recip: *mut u8,
    s: *const u8,
) -> c_int {
    crypto_core_ed25519_scalar_invert(recip, s)
}

/// ```c
/// void
/// crypto_core_ristretto255_scalar_negate(unsigned char *neg,
///                                        const unsigned char *s)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_negate(neg: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_negate(neg, s);
}

/// ```c
/// void
/// crypto_core_ristretto255_scalar_complement(unsigned char *comp,
///                                            const unsigned char *s)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_complement(
    comp: *mut u8,
    s: *const u8,
) {
    crypto_core_ed25519_scalar_complement(comp, s);
}

/// ```c
/// void
/// crypto_core_ristretto255_scalar_add(unsigned char *z, const unsigned char *x,
///                                     const unsigned char *y)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_add(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    crypto_core_ed25519_scalar_add(z, x, y);
}

/// ```c
/// void
/// crypto_core_ristretto255_scalar_sub(unsigned char *z, const unsigned char *x,
///                                     const unsigned char *y)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    crypto_core_ed25519_scalar_sub(z, x, y);
}

/// ```c
/// void
/// crypto_core_ristretto255_scalar_mul(unsigned char *z, const unsigned char *x,
///                                     const unsigned char *y)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_mul(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    _sodium_sc25519_mul(z, x, y);
}

/// ```c
/// void
/// crypto_core_ristretto255_scalar_reduce(unsigned char *r,
///                                        const unsigned char *s)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_reduce(r: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_reduce(r, s);
}

/// ```c
/// int
/// crypto_core_ristretto255_scalar_is_canonical(const unsigned char *s)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_is_canonical(s: *const u8) -> c_int {
    _sodium_sc25519_is_canonical(s)
}

/// ```c
/// int
/// crypto_core_ristretto255_scalar_from_string(unsigned char *s,
///                                             const unsigned char *ctx, size_t ctx_len,
///                                             const unsigned char *msg, size_t msg_len,
///                                             int hash_alg)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_from_string(
    s: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    crypto_core_ed25519_scalar_from_string(s, ctx, ctx_len, msg, msg_len, hash_alg)
}

/// ```c
/// size_t
/// crypto_core_ristretto255_bytes(void)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_bytes() -> usize {
    crypto_core_ristretto255_BYTES
}

/// ```c
/// size_t
/// crypto_core_ristretto255_nonreducedscalarbytes(void)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_nonreducedscalarbytes() -> usize {
    crypto_core_ristretto255_NONREDUCEDSCALARBYTES
}

/// ```c
/// size_t
/// crypto_core_ristretto255_hashbytes(void)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_hashbytes() -> usize {
    crypto_core_ristretto255_HASHBYTES
}

/// ```c
/// size_t
/// crypto_core_ristretto255_scalarbytes(void)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalarbytes() -> usize {
    crypto_core_ristretto255_SCALARBYTES
}
