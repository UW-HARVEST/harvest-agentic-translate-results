//! crypto_core ed25519 (core_ed25519.c) plus the ge25519 from_hash/from_uniform
//! and sc25519_invert glue used here.
use crate::ed25519::ge25519;
use crate::ed25519::h2c::{core_h2c_string_to_hash, CORE_H2C_SHA512};
use crate::ed25519::sc25519;
use core::ffi::{c_int, c_void};

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
    fn sodium_add(a: *mut u8, b: *const u8, len: usize);
    fn sodium_sub(a: *mut u8, b: *const u8, len: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

const HASH_GE_L: usize = 48;
const ED25519_BYTES: usize = 32;
const ED25519_HASHBYTES: usize = 64;
const ED25519_UNIFORMBYTES: usize = 32;
const ED25519_SCALARBYTES: usize = 32;
const ED25519_NONREDUCEDSCALARBYTES: usize = 64;

/* 2^252+27742317777372353535851937790883648493 */
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_is_valid_point(p: *const u8) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    if ge25519::is_canonical(psl) == 0 {
        return 0;
    }
    let (pp, r) = ge25519::frombytes(psl);
    if r != 0
        || ge25519::is_on_curve(&pp) == 0
        || ge25519::has_small_order(&pp) != 0
        || ge25519::is_on_main_subgroup(&pp) == 0
    {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_add(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    let qsl = core::slice::from_raw_parts(q, 32);
    let (pp, pr) = ge25519::frombytes(psl);
    if pr != 0 || ge25519::is_on_curve(&pp) == 0 {
        return -1;
    }
    let (qq, qr) = ge25519::frombytes(qsl);
    if qr != 0 || ge25519::is_on_curve(&qq) == 0 {
        return -1;
    }
    let rp = ge25519::p3_add(&pp, &qq);
    let out = ge25519::p3_tobytes(&rp);
    core::ptr::copy_nonoverlapping(out.as_ptr(), r, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    let qsl = core::slice::from_raw_parts(q, 32);
    let (pp, pr) = ge25519::frombytes(psl);
    if pr != 0 || ge25519::is_on_curve(&pp) == 0 {
        return -1;
    }
    let (qq, qr) = ge25519::frombytes(qsl);
    if qr != 0 || ge25519::is_on_curve(&qq) == 0 {
        return -1;
    }
    let rp = ge25519::p3_sub(&pp, &qq);
    let out = ge25519::p3_tobytes(&rp);
    core::ptr::copy_nonoverlapping(out.as_ptr(), r, 32);
    0
}

unsafe fn string_to_points(
    px: *mut u8,
    n: usize,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h = [0u8; ED25519_HASHBYTES];
    let mut h_be = [0u8; 2 * HASH_GE_L];
    if n > 2 {
        panic!("abort");
    }
    if core_h2c_string_to_hash(
        h_be.as_mut_ptr(),
        n * HASH_GE_L,
        ctx,
        ctx_len,
        msg,
        msg_len,
        hash_alg,
    ) != 0
    {
        return -1;
    }
    for i in 0..n {
        for j in 0..HASH_GE_L {
            h[j] = h_be[i * HASH_GE_L + HASH_GE_L - 1 - j];
        }
        for k in HASH_GE_L..h.len() {
            h[k] = 0;
        }
        let out = ge25519::from_hash(&h);
        core::ptr::copy_nonoverlapping(out.as_ptr(), px.add(i * ED25519_BYTES), 32);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_from_string_nu(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    string_to_points(p, 1, ctx, ctx_len, msg, msg_len, hash_alg)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_from_string(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut px = [0u8; 2 * ED25519_BYTES];
    if string_to_points(px.as_mut_ptr(), 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0 {
        return -1;
    }
    crypto_core_ed25519_add(p, px.as_ptr(), px.as_ptr().add(ED25519_BYTES))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_random(p: *mut u8) {
    let mut h = [0u8; ED25519_UNIFORMBYTES];
    randombytes_buf(h.as_mut_ptr() as *mut c_void, h.len());
    let out = ge25519::from_uniform(&h);
    core::ptr::copy_nonoverlapping(out.as_ptr(), p, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_random(r: *mut u8) {
    loop {
        randombytes_buf(r as *mut c_void, ED25519_SCALARBYTES);
        *r.add(ED25519_SCALARBYTES - 1) &= 0x1f;
        let rsl = core::slice::from_raw_parts(r, 32);
        if sc25519::sc_is_canonical(rsl) != 0 && sodium_is_zero(r, ED25519_SCALARBYTES) == 0 {
            break;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_invert(
    recip: *mut u8,
    s: *const u8,
) -> c_int {
    let ssl = core::slice::from_raw_parts(s, 32);
    let out = sc25519::sc_invert(ssl);
    core::ptr::copy_nonoverlapping(out.as_ptr(), recip, 32);
    -sodium_is_zero(s, ED25519_SCALARBYTES)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_negate(neg: *mut u8, s: *const u8) {
    let mut t_ = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    let mut s_ = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    t_[ED25519_SCALARBYTES..ED25519_SCALARBYTES + 32].copy_from_slice(&L);
    core::ptr::copy_nonoverlapping(s, s_.as_mut_ptr(), ED25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), t_.len());
    sc25519::sc_reduce(&mut t_);
    core::ptr::copy_nonoverlapping(t_.as_ptr(), neg, ED25519_SCALARBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_complement(comp: *mut u8, s: *const u8) {
    let mut t_ = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    let mut s_ = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    t_[0] = t_[0].wrapping_add(1);
    t_[ED25519_SCALARBYTES..ED25519_SCALARBYTES + 32].copy_from_slice(&L);
    core::ptr::copy_nonoverlapping(s, s_.as_mut_ptr(), ED25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), t_.len());
    sc25519::sc_reduce(&mut t_);
    core::ptr::copy_nonoverlapping(t_.as_ptr(), comp, ED25519_SCALARBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_add(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let mut x_ = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    let mut y_ = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    core::ptr::copy_nonoverlapping(x, x_.as_mut_ptr(), ED25519_SCALARBYTES);
    core::ptr::copy_nonoverlapping(y, y_.as_mut_ptr(), ED25519_SCALARBYTES);
    sodium_add(x_.as_mut_ptr(), y_.as_ptr(), ED25519_SCALARBYTES);
    crypto_core_ed25519_scalar_reduce(z, x_.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let mut yn = [0u8; ED25519_SCALARBYTES];
    crypto_core_ed25519_scalar_negate(yn.as_mut_ptr(), y);
    crypto_core_ed25519_scalar_add(z, x, yn.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_mul(
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
pub unsafe extern "C" fn crypto_core_ed25519_scalar_reduce(r: *mut u8, s: *const u8) {
    let mut t = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    core::ptr::copy_nonoverlapping(s, t.as_mut_ptr(), ED25519_NONREDUCEDSCALARBYTES);
    sc25519::sc_reduce(&mut t);
    core::ptr::copy_nonoverlapping(t.as_ptr(), r, ED25519_SCALARBYTES);
    sodium_memzero(t.as_mut_ptr() as *mut c_void, t.len());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_is_canonical(s: *const u8) -> c_int {
    let ssl = core::slice::from_raw_parts(s, 32);
    sc25519::sc_is_canonical(ssl)
}

const HASH_SC_L: usize = 48;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_from_string(
    s: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h = [0u8; ED25519_NONREDUCEDSCALARBYTES];
    let mut h_be = [0u8; HASH_SC_L];
    if core_h2c_string_to_hash(
        h_be.as_mut_ptr(),
        HASH_SC_L,
        ctx,
        ctx_len,
        msg,
        msg_len,
        hash_alg,
    ) != 0
    {
        return -1;
    }
    for i in 0..HASH_SC_L {
        h[i] = h_be[HASH_SC_L - 1 - i];
    }
    for k in HASH_SC_L..h.len() {
        h[k] = 0;
    }
    crypto_core_ed25519_scalar_reduce(s, h.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_bytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_nonreducedscalarbytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_uniformbytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_hashbytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_scalarbytes() -> usize {
    32
}

/* keep constants referenced */
const _: c_int = CORE_H2C_SHA512;
