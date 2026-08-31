//! Translation of:
//! - `crypto_core/ed25519/core_ed25519.c`
//! - `crypto_core/ed25519/core_ristretto255.c`
//! - `crypto_core/ed25519/core_h2c.c`

use core::ffi::{c_int, c_void};

use crate::types::{crypto_hash_sha256_state, crypto_hash_sha512_state, ge25519_p3};

// ===================== constants =====================

pub const crypto_core_ed25519_BYTES: usize = 32;
pub const crypto_core_ed25519_UNIFORMBYTES: usize = 32;
pub const crypto_core_ed25519_HASHBYTES: usize = 64;
pub const crypto_core_ed25519_SCALARBYTES: usize = 32;
pub const crypto_core_ed25519_NONREDUCEDSCALARBYTES: usize = 64;
pub const crypto_core_ed25519_H2CSHA256: c_int = 1;
pub const crypto_core_ed25519_H2CSHA512: c_int = 2;

pub const crypto_core_ristretto255_BYTES: usize = 32;
pub const crypto_core_ristretto255_HASHBYTES: usize = 64;
pub const crypto_core_ristretto255_SCALARBYTES: usize = 32;
pub const crypto_core_ristretto255_NONREDUCEDSCALARBYTES: usize = 64;
pub const crypto_core_ristretto255_H2CSHA256: c_int = 1;
pub const crypto_core_ristretto255_H2CSHA512: c_int = 2;

const CORE_H2C_SHA256: c_int = 1;
const CORE_H2C_SHA512: c_int = 2;

/// 2^252+27742317777372353535851937790883648493
static L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

// ===================== external dependencies =====================

extern "C" {
    #[link_name = "_sodium_ge25519_frombytes"]
    fn ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_p3_tobytes"]
    fn ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_p3_add"]
    fn ge25519_p3_add(r: *mut ge25519_p3, p: *const ge25519_p3, q: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_p3_sub"]
    fn ge25519_p3_sub(r: *mut ge25519_p3, p: *const ge25519_p3, q: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_scalarmult_base"]
    fn ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    #[link_name = "_sodium_ge25519_clear_cofactor"]
    fn ge25519_clear_cofactor(p3: *mut ge25519_p3);
    #[link_name = "_sodium_ge25519_is_canonical"]
    fn ge25519_is_canonical(s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_is_on_curve"]
    fn ge25519_is_on_curve(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ge25519_is_on_main_subgroup"]
    fn ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ge25519_has_small_order"]
    fn ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ge25519_from_uniform"]
    fn ge25519_from_uniform(s: *mut u8, r: *const u8);
    #[link_name = "_sodium_ge25519_from_hash"]
    fn ge25519_from_hash(s: *mut u8, h: *const u8);

    #[link_name = "_sodium_sc25519_invert"]
    fn sc25519_invert(recip: *mut u8, s: *const u8);
    #[link_name = "_sodium_sc25519_reduce"]
    fn sc25519_reduce(s: *mut u8);
    #[link_name = "_sodium_sc25519_mul"]
    fn sc25519_mul(s: *mut u8, a: *const u8, b: *const u8);
    #[link_name = "_sodium_sc25519_muladd"]
    fn sc25519_muladd(s: *mut u8, a: *const u8, b: *const u8, c: *const u8);
    #[link_name = "_sodium_sc25519_is_canonical"]
    fn sc25519_is_canonical(s: *const u8) -> c_int;

    #[link_name = "_sodium_ristretto255_frombytes"]
    fn ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ristretto255_p3_tobytes"]
    fn ristretto255_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    #[link_name = "_sodium_ristretto255_from_hash"]
    fn ristretto255_from_hash(s: *mut u8, h: *const u8);

    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha256_final(state: *mut crypto_hash_sha256_state, out: *mut u8) -> c_int;

    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
    fn sodium_add(a: *mut u8, b: *const u8, len: usize);
    fn sodium_sub(a: *mut u8, b: *const u8, len: usize);

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[inline(always)]
unsafe fn memcpy(dst: *mut u8, src: *const u8, n: usize) {
    crate::csys::memcpy(dst as *mut c_void, src as *const c_void, n);
}

#[inline(always)]
unsafe fn memset(dst: *mut u8, val: i32, n: usize) {
    crate::csys::memset(dst as *mut c_void, val, n);
}

// =====================================================================
// core_h2c.c
// =====================================================================

const H2C_SHA256_HASH_BYTES: usize = 32; // crypto_hash_sha256_BYTES
const H2C_SHA256_BLOCKBYTES: usize = 64;

unsafe fn core_h2c_string_to_hash_sha256(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let mut st: crypto_hash_sha256_state = core::mem::zeroed();
    let empty_block: [u8; H2C_SHA256_BLOCKBYTES] = [0u8; H2C_SHA256_BLOCKBYTES];
    let mut u0 = [0u8; H2C_SHA256_HASH_BYTES];
    let mut ux = [0u8; H2C_SHA256_HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    // C: assert(h_len <= 0xff);  the reference build does not define NDEBUG.
    if h_len > 0xff {
        crate::csys::abort();
    }
    if ctx_len > 0xff {
        crypto_hash_sha256_init(&mut st);
        crypto_hash_sha256_update(
            &mut st,
            b"H2C-OVERSIZE-DST-".as_ptr(),
            (b"H2C-OVERSIZE-DST-".len()) as u64,
        );
        crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha256_final(&mut st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = H2C_SHA256_HASH_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha256_init(&mut st);
    crypto_hash_sha256_update(&mut st, empty_block.as_ptr(), empty_block.len() as u64);
    crypto_hash_sha256_update(&mut st, msg, msg_len as u64);
    crypto_hash_sha256_update(&mut st, t.as_ptr(), 3u64);
    crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
    crypto_hash_sha256_update(&mut st, &ctx_len_u8, 1u64);
    crypto_hash_sha256_final(&mut st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < H2C_SHA256_HASH_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha256_init(&mut st);
        crypto_hash_sha256_update(&mut st, ux.as_ptr(), H2C_SHA256_HASH_BYTES as u64);
        crypto_hash_sha256_update(&mut st, &t[2], 1u64);
        crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha256_update(&mut st, &ctx_len_u8, 1u64);
        crypto_hash_sha256_final(&mut st, ux.as_mut_ptr());
        let n = if h_len - i >= ux.len() { ux.len() } else { h_len - i };
        memcpy(h.add(i), ux.as_ptr(), n);
        i += H2C_SHA256_HASH_BYTES;
    }
    0
}

const H2C_SHA512_HASH_BYTES: usize = 64; // crypto_hash_sha512_BYTES
const H2C_SHA512_BLOCKBYTES: usize = 128;

unsafe fn core_h2c_string_to_hash_sha512(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let mut st: crypto_hash_sha512_state = core::mem::zeroed();
    let empty_block: [u8; H2C_SHA512_BLOCKBYTES] = [0u8; H2C_SHA512_BLOCKBYTES];
    let mut u0 = [0u8; H2C_SHA512_HASH_BYTES];
    let mut ux = [0u8; H2C_SHA512_HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    // C: assert(h_len <= 0xff);  the reference build does not define NDEBUG.
    if h_len > 0xff {
        crate::csys::abort();
    }
    if ctx_len > 0xff {
        crypto_hash_sha512_init(&mut st);
        crypto_hash_sha512_update(
            &mut st,
            b"H2C-OVERSIZE-DST-".as_ptr(),
            (b"H2C-OVERSIZE-DST-".len()) as u64,
        );
        crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha512_final(&mut st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = H2C_SHA512_HASH_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha512_init(&mut st);
    crypto_hash_sha512_update(&mut st, empty_block.as_ptr(), empty_block.len() as u64);
    crypto_hash_sha512_update(&mut st, msg, msg_len as u64);
    crypto_hash_sha512_update(&mut st, t.as_ptr(), 3u64);
    crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
    crypto_hash_sha512_update(&mut st, &ctx_len_u8, 1u64);
    crypto_hash_sha512_final(&mut st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < H2C_SHA512_HASH_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha512_init(&mut st);
        crypto_hash_sha512_update(&mut st, ux.as_ptr(), H2C_SHA512_HASH_BYTES as u64);
        crypto_hash_sha512_update(&mut st, &t[2], 1u64);
        crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha512_update(&mut st, &ctx_len_u8, 1u64);
        crypto_hash_sha512_final(&mut st, ux.as_mut_ptr());
        let n = if h_len - i >= ux.len() { ux.len() } else { h_len - i };
        memcpy(h.add(i), ux.as_ptr(), n);
        i += H2C_SHA512_HASH_BYTES;
    }
    0
}

/// `core_h2c_string_to_hash` — renamed by `private/quirks.h` to
/// `_sodium_core_h2c_string_to_hash`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_core_h2c_string_to_hash(
    h: *mut u8,
    h_len: usize,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    match hash_alg {
        CORE_H2C_SHA256 => core_h2c_string_to_hash_sha256(h, h_len, ctx, ctx_len, msg, msg_len),
        CORE_H2C_SHA512 => core_h2c_string_to_hash_sha512(h, h_len, ctx, ctx_len, msg, msg_len),
        _ => {
            crate::csys::set_errno(crate::csys::EINVAL);
            -1
        }
    }
}

// =====================================================================
// core_ed25519.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_is_valid_point(p: *const u8) -> c_int {
    let mut p_p3 = ge25519_p3::zero();

    if ge25519_is_canonical(p) == 0
        || ge25519_frombytes(&mut p_p3, p) != 0
        || ge25519_is_on_curve(&p_p3) == 0
        || ge25519_has_small_order(&p_p3) != 0
        || ge25519_is_on_main_subgroup(&p_p3) == 0
    {
        return 0;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_add(r: *mut u8, p: *const u8, q: *const u8) -> c_int {
    let mut p_p3 = ge25519_p3::zero();
    let mut q_p3 = ge25519_p3::zero();
    let mut r_p3 = ge25519_p3::zero();

    if ge25519_frombytes(&mut p_p3, p) != 0
        || ge25519_is_on_curve(&p_p3) == 0
        || ge25519_frombytes(&mut q_p3, q) != 0
        || ge25519_is_on_curve(&q_p3) == 0
    {
        return -1;
    }
    ge25519_p3_add(&mut r_p3, &p_p3, &q_p3);
    ge25519_p3_tobytes(r, &r_p3);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_sub(r: *mut u8, p: *const u8, q: *const u8) -> c_int {
    let mut p_p3 = ge25519_p3::zero();
    let mut q_p3 = ge25519_p3::zero();
    let mut r_p3 = ge25519_p3::zero();

    if ge25519_frombytes(&mut p_p3, p) != 0
        || ge25519_is_on_curve(&p_p3) == 0
        || ge25519_frombytes(&mut q_p3, q) != 0
        || ge25519_is_on_curve(&q_p3) == 0
    {
        return -1;
    }
    ge25519_p3_sub(&mut r_p3, &p_p3, &q_p3);
    ge25519_p3_tobytes(r, &r_p3);

    0
}

const HASH_GE_L: usize = 48;

/// `_string_to_points` — static helper.
unsafe fn string_to_points(
    px: *mut u8,
    n: usize,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h = [0u8; crypto_core_ed25519_HASHBYTES];
    let mut h_be = [0u8; 2 * HASH_GE_L];
    let mut i: usize;
    let mut j: usize;

    if n > 2 {
        crate::csys::abort();
    }
    if _sodium_core_h2c_string_to_hash(
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
    i = 0;
    while i < n {
        j = 0;
        while j < HASH_GE_L {
            h[j] = h_be[i * HASH_GE_L + HASH_GE_L - 1 - j];
            j += 1;
        }
        memset(h.as_mut_ptr().add(j), 0, h.len() - j);
        ge25519_from_hash(px.add(i * crypto_core_ed25519_BYTES), h.as_ptr());
        i += 1;
    }
    0
}

/* LCOV_EXCL_START */
#[no_mangle]
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

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_from_string(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut px = [0u8; 2 * crypto_core_ed25519_BYTES];

    if string_to_points(px.as_mut_ptr(), 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0 {
        return -1;
    }
    crypto_core_ed25519_add(
        p,
        px.as_ptr(),
        px.as_ptr().add(crypto_core_ed25519_BYTES),
    )
}
/* LCOV_EXCL_STOP */

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_random(p: *mut u8) {
    let mut h = [0u8; crypto_core_ed25519_UNIFORMBYTES];

    randombytes_buf(h.as_mut_ptr() as *mut c_void, h.len());
    ge25519_from_uniform(p, h.as_ptr());
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_random(r: *mut u8) {
    loop {
        randombytes_buf(r as *mut c_void, crypto_core_ed25519_SCALARBYTES);
        *r.add(crypto_core_ed25519_SCALARBYTES - 1) &= 0x1f;
        if sc25519_is_canonical(r) != 0 && sodium_is_zero(r, crypto_core_ed25519_SCALARBYTES) == 0
        {
            break;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_invert(recip: *mut u8, s: *const u8) -> c_int {
    sc25519_invert(recip, s);

    -sodium_is_zero(s, crypto_core_ed25519_SCALARBYTES)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_negate(neg: *mut u8, s: *const u8) {
    let mut t_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut s_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    memcpy(
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        L.as_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    memcpy(s_.as_mut_ptr(), s, crypto_core_ed25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), t_.len());
    sc25519_reduce(t_.as_mut_ptr());
    memcpy(neg, t_.as_ptr(), crypto_core_ed25519_SCALARBYTES);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_complement(comp: *mut u8, s: *const u8) {
    let mut t_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut s_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    t_[0] = t_[0].wrapping_add(1);
    memcpy(
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        L.as_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    memcpy(s_.as_mut_ptr(), s, crypto_core_ed25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), t_.len());
    sc25519_reduce(t_.as_mut_ptr());
    memcpy(comp, t_.as_ptr(), crypto_core_ed25519_SCALARBYTES);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_add(z: *mut u8, x: *const u8, y: *const u8) {
    let mut x_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut y_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    memcpy(x_.as_mut_ptr(), x, crypto_core_ed25519_SCALARBYTES);
    memcpy(y_.as_mut_ptr(), y, crypto_core_ed25519_SCALARBYTES);
    sodium_add(x_.as_mut_ptr(), y_.as_ptr(), crypto_core_ed25519_SCALARBYTES);
    crypto_core_ed25519_scalar_reduce(z, x_.as_ptr());
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_sub(z: *mut u8, x: *const u8, y: *const u8) {
    let mut yn = [0u8; crypto_core_ed25519_SCALARBYTES];

    crypto_core_ed25519_scalar_negate(yn.as_mut_ptr(), y);
    crypto_core_ed25519_scalar_add(z, x, yn.as_ptr());
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_mul(z: *mut u8, x: *const u8, y: *const u8) {
    sc25519_mul(z, x, y);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_reduce(r: *mut u8, s: *const u8) {
    let mut t = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    memcpy(t.as_mut_ptr(), s, t.len());
    sc25519_reduce(t.as_mut_ptr());
    memcpy(r, t.as_ptr(), crypto_core_ed25519_SCALARBYTES);
    sodium_memzero(t.as_mut_ptr() as *mut c_void, t.len());
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_is_canonical(s: *const u8) -> c_int {
    sc25519_is_canonical(s)
}

const HASH_SC_L: usize = 48;

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_from_string(
    s: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut h_be = [0u8; HASH_SC_L];
    let mut i: usize;

    if _sodium_core_h2c_string_to_hash(
        h_be.as_mut_ptr(),
        h_be.len(),
        ctx,
        ctx_len,
        msg,
        msg_len,
        hash_alg,
    ) != 0
    {
        return -1;
    }
    i = 0;
    while i < HASH_SC_L {
        h[i] = h_be[HASH_SC_L - 1 - i];
        i += 1;
    }
    memset(h.as_mut_ptr().add(i), 0, h.len() - i);
    crypto_core_ed25519_scalar_reduce(s, h.as_ptr());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_bytes() -> usize {
    crypto_core_ed25519_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_nonreducedscalarbytes() -> usize {
    crypto_core_ed25519_NONREDUCEDSCALARBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_uniformbytes() -> usize {
    crypto_core_ed25519_UNIFORMBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_hashbytes() -> usize {
    crypto_core_ed25519_HASHBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ed25519_scalarbytes() -> usize {
    crypto_core_ed25519_SCALARBYTES
}

// =====================================================================
// core_ristretto255.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_is_valid_point(p: *const u8) -> c_int {
    let mut p_p3 = ge25519_p3::zero();

    if ristretto255_frombytes(&mut p_p3, p) != 0 {
        return 0;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_add(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3 = ge25519_p3::zero();
    let mut q_p3 = ge25519_p3::zero();
    let mut r_p3 = ge25519_p3::zero();

    if ristretto255_frombytes(&mut p_p3, p) != 0 || ristretto255_frombytes(&mut q_p3, q) != 0 {
        return -1;
    }
    ge25519_p3_add(&mut r_p3, &p_p3, &q_p3);
    ristretto255_p3_tobytes(r, &r_p3);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3 = ge25519_p3::zero();
    let mut q_p3 = ge25519_p3::zero();
    let mut r_p3 = ge25519_p3::zero();

    if ristretto255_frombytes(&mut p_p3, p) != 0 || ristretto255_frombytes(&mut q_p3, q) != 0 {
        return -1;
    }
    ge25519_p3_sub(&mut r_p3, &p_p3, &q_p3);
    ristretto255_p3_tobytes(r, &r_p3);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_from_hash(p: *mut u8, r: *const u8) -> c_int {
    ristretto255_from_hash(p, r);

    0
}

unsafe fn string_to_element(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h = [0u8; crypto_core_ristretto255_HASHBYTES];

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
        return -1;
    }
    ristretto255_from_hash(p, h.as_ptr());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_from_string(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    string_to_element(p, ctx, ctx_len, msg, msg_len, hash_alg)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_random(p: *mut u8) {
    let mut h = [0u8; crypto_core_ristretto255_HASHBYTES];

    randombytes_buf(h.as_mut_ptr() as *mut c_void, h.len());
    let _ = crypto_core_ristretto255_from_hash(p, h.as_ptr());
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_random(r: *mut u8) {
    crypto_core_ed25519_scalar_random(r);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_invert(
    recip: *mut u8,
    s: *const u8,
) -> c_int {
    crypto_core_ed25519_scalar_invert(recip, s)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_negate(neg: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_negate(neg, s);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_complement(comp: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_complement(comp, s);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_add(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    crypto_core_ed25519_scalar_add(z, x, y);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    crypto_core_ed25519_scalar_sub(z, x, y);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_mul(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    sc25519_mul(z, x, y);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_reduce(r: *mut u8, s: *const u8) {
    crypto_core_ed25519_scalar_reduce(r, s);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_is_canonical(s: *const u8) -> c_int {
    sc25519_is_canonical(s)
}

#[no_mangle]
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

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_bytes() -> usize {
    crypto_core_ristretto255_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_nonreducedscalarbytes() -> usize {
    crypto_core_ristretto255_NONREDUCEDSCALARBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_hashbytes() -> usize {
    crypto_core_ristretto255_HASHBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_core_ristretto255_scalarbytes() -> usize {
    crypto_core_ristretto255_SCALARBYTES
}
