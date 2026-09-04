//! Translation of `crypto_core/ed25519/core_ristretto255.c` and
//! `include/sodium/crypto_core_ristretto255.h`.

use core::ffi::c_int;

use crate::crypto_core::core_ed25519::{
    crypto_core_ed25519_scalar_add, crypto_core_ed25519_scalar_complement,
    crypto_core_ed25519_scalar_from_string, crypto_core_ed25519_scalar_invert,
    crypto_core_ed25519_scalar_negate, crypto_core_ed25519_scalar_random,
    crypto_core_ed25519_scalar_reduce, crypto_core_ed25519_scalar_sub,
};
use crate::crypto_core::ed25519_ref10::ge::{
    _sodium_ge25519_p3_add, _sodium_ge25519_p3_sub, _sodium_ristretto255_from_hash,
    _sodium_ristretto255_frombytes, _sodium_ristretto255_p3_tobytes,
};
use crate::crypto_core::ed25519_ref10::ge25519_p3;
use crate::crypto_core::ed25519_ref10::sc::{sc25519_is_canonical, sc25519_mul};
use crate::randombytes::randombytes_buf;

pub const crypto_core_ristretto255_BYTES: usize = 32;
pub const crypto_core_ristretto255_HASHBYTES: usize = 64;
pub const crypto_core_ristretto255_SCALARBYTES: usize = 32;
pub const crypto_core_ristretto255_NONREDUCEDSCALARBYTES: usize = 64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_is_valid_point(p: *const u8) -> c_int {
    let mut p_p3: ge25519_p3 = core::mem::zeroed();

    if _sodium_ristretto255_frombytes(&mut p_p3, p) != 0 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_add(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3: ge25519_p3 = core::mem::zeroed();
    let mut q_p3: ge25519_p3 = core::mem::zeroed();
    let mut r_p3: ge25519_p3 = core::mem::zeroed();

    if _sodium_ristretto255_frombytes(&mut p_p3, p) != 0
        || _sodium_ristretto255_frombytes(&mut q_p3, q) != 0
    {
        return -1;
    }
    _sodium_ge25519_p3_add(&mut r_p3, &p_p3, &q_p3);
    _sodium_ristretto255_p3_tobytes(r, &r_p3);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3: ge25519_p3 = core::mem::zeroed();
    let mut q_p3: ge25519_p3 = core::mem::zeroed();
    let mut r_p3: ge25519_p3 = core::mem::zeroed();

    if _sodium_ristretto255_frombytes(&mut p_p3, p) != 0
        || _sodium_ristretto255_frombytes(&mut q_p3, q) != 0
    {
        return -1;
    }
    _sodium_ge25519_p3_sub(&mut r_p3, &p_p3, &q_p3);
    _sodium_ristretto255_p3_tobytes(r, &r_p3);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_from_hash(p: *mut u8, r: *const u8) -> c_int {
    _sodium_ristretto255_from_hash(p, r);

    0
}

unsafe fn _string_to_element(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h: [u8; crypto_core_ristretto255_HASHBYTES] = [0; crypto_core_ristretto255_HASHBYTES];

    if crate::crypto_core::core_h2c::_sodium_core_h2c_string_to_hash(
        h.as_mut_ptr(),
        core::mem::size_of_val(&h),
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_random(p: *mut u8) {
    let mut h: [u8; crypto_core_ristretto255_HASHBYTES] = [0; crypto_core_ristretto255_HASHBYTES];

    randombytes_buf(
        h.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of_val(&h),
    );
    let _ = crypto_core_ristretto255_from_hash(p, h.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_random(r: *mut u8) {
    crypto_core_ed25519_scalar_random(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_invert(
    recip: *mut u8,
    s: *const u8,
) -> c_int {
    crypto_core_ed25519_scalar_invert(recip, s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_negate(neg: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_negate(neg, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_complement(comp: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_complement(comp, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_add(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    crypto_core_ed25519_scalar_add(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    crypto_core_ed25519_scalar_sub(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_mul(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    sc25519_mul(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_reduce(r: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_reduce(r, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_is_canonical(s: *const u8) -> c_int {
    sc25519_is_canonical(s)
}

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

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_bytes() -> usize {
    crypto_core_ristretto255_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_nonreducedscalarbytes() -> usize {
    crypto_core_ristretto255_NONREDUCEDSCALARBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_hashbytes() -> usize {
    crypto_core_ristretto255_HASHBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_scalarbytes() -> usize {
    crypto_core_ristretto255_SCALARBYTES
}
