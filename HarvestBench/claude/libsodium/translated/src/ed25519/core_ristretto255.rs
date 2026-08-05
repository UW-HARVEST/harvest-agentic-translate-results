//! crypto_core ristretto255 (core_ristretto255.c).
use crate::ed25519::core_ed25519;
use crate::ed25519::ge25519;
use crate::ed25519::h2c::core_h2c_string_to_hash;
use crate::ed25519::ristretto255;
use crate::ed25519::sc25519;
use core::ffi::{c_int, c_void};

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const RISTRETTO_HASHBYTES: usize = 64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_is_valid_point(p: *const u8) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    let (_pp, r) = ristretto255::frombytes(psl);
    if r != 0 {
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
    let psl = core::slice::from_raw_parts(p, 32);
    let qsl = core::slice::from_raw_parts(q, 32);
    let (pp, pr) = ristretto255::frombytes(psl);
    let (qq, qr) = ristretto255::frombytes(qsl);
    if pr != 0 || qr != 0 {
        return -1;
    }
    let rp = ge25519::p3_add(&pp, &qq);
    let out = ristretto255::p3_tobytes(&rp);
    core::ptr::copy_nonoverlapping(out.as_ptr(), r, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    let qsl = core::slice::from_raw_parts(q, 32);
    let (pp, pr) = ristretto255::frombytes(psl);
    let (qq, qr) = ristretto255::frombytes(qsl);
    if pr != 0 || qr != 0 {
        return -1;
    }
    let rp = ge25519::p3_sub(&pp, &qq);
    let out = ristretto255::p3_tobytes(&rp);
    core::ptr::copy_nonoverlapping(out.as_ptr(), r, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_from_hash(p: *mut u8, r: *const u8) -> c_int {
    let rsl = core::slice::from_raw_parts(r, 64);
    let out = ristretto255::from_hash(rsl);
    core::ptr::copy_nonoverlapping(out.as_ptr(), p, 32);
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
    let mut h = [0u8; RISTRETTO_HASHBYTES];
    if core_h2c_string_to_hash(
        h.as_mut_ptr(),
        h.len(),
        ctx,
        ctx_len,
        msg,
        msg_len,
        hash_alg,
    ) != 0
    {
        return -1;
    }
    let out = ristretto255::from_hash(&h);
    core::ptr::copy_nonoverlapping(out.as_ptr(), p, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_random(p: *mut u8) {
    let mut h = [0u8; RISTRETTO_HASHBYTES];
    randombytes_buf(h.as_mut_ptr() as *mut c_void, h.len());
    crypto_core_ristretto255_from_hash(p, h.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_random(r: *mut u8) {
    core_ed25519::crypto_core_ed25519_scalar_random(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_invert(
    recip: *mut u8,
    s: *const u8,
) -> c_int {
    core_ed25519::crypto_core_ed25519_scalar_invert(recip, s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_negate(neg: *mut u8, s: *const u8) {
    core_ed25519::crypto_core_ed25519_scalar_negate(neg, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_complement(
    comp: *mut u8,
    s: *const u8,
) {
    core_ed25519::crypto_core_ed25519_scalar_complement(comp, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_add(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    core_ed25519::crypto_core_ed25519_scalar_add(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    core_ed25519::crypto_core_ed25519_scalar_sub(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_mul(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let xsl = core::slice::from_raw_parts(x, 32);
    let ysl = core::slice::from_raw_parts(y, 32);
    let out = sc25519::sc_mul(xsl, ysl);
    core::ptr::copy_nonoverlapping(out.as_ptr(), z, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_reduce(r: *mut u8, s: *const u8) {
    core_ed25519::crypto_core_ed25519_scalar_reduce(r, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_is_canonical(s: *const u8) -> c_int {
    let ssl = core::slice::from_raw_parts(s, 32);
    sc25519::sc_is_canonical(ssl)
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
    core_ed25519::crypto_core_ed25519_scalar_from_string(s, ctx, ctx_len, msg, msg_len, hash_alg)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_bytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_nonreducedscalarbytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_hashbytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ristretto255_scalarbytes() -> usize {
    32
}
