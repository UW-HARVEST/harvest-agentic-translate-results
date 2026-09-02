//! Translation of `crypto_core/ed25519/core_ed25519.c` and
//! `include/sodium/crypto_core_ed25519.h`.

use core::ffi::c_int;

use crate::common::memset;
use crate::crypto_core::core_h2c::_sodium_core_h2c_string_to_hash;
use crate::crypto_core::ed25519_ref10::ge::{
    _sodium_ge25519_frombytes, _sodium_ge25519_from_hash, _sodium_ge25519_from_uniform,
    _sodium_ge25519_has_small_order, _sodium_ge25519_is_canonical,
    _sodium_ge25519_is_on_curve, _sodium_ge25519_is_on_main_subgroup,
    _sodium_ge25519_p3_add, _sodium_ge25519_p3_sub, _sodium_ge25519_p3_tobytes,
};
use crate::crypto_core::ed25519_ref10::ge25519_p3;
use crate::crypto_core::ed25519_ref10::sc::{
    sc25519_invert, sc25519_is_canonical, sc25519_mul, sc25519_reduce,
};
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::{sodium_add, sodium_is_zero, sodium_memzero, sodium_sub};

pub const crypto_core_ed25519_BYTES: usize = 32;
pub const crypto_core_ed25519_UNIFORMBYTES: usize = 32;
pub const crypto_core_ed25519_HASHBYTES: usize = 64;
pub const crypto_core_ed25519_SCALARBYTES: usize = 32;
pub const crypto_core_ed25519_NONREDUCEDSCALARBYTES: usize = 64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_is_valid_point(p: *const u8) -> c_int {
    let mut p_p3: ge25519_p3 = core::mem::zeroed();

    if _sodium_ge25519_is_canonical(p) == 0
        || _sodium_ge25519_frombytes(&mut p_p3, p) != 0
        || _sodium_ge25519_is_on_curve(&p_p3) == 0
        || _sodium_ge25519_has_small_order(&p_p3) != 0
        || _sodium_ge25519_is_on_main_subgroup(&p_p3) == 0
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
    let mut p_p3: ge25519_p3 = core::mem::zeroed();
    let mut q_p3: ge25519_p3 = core::mem::zeroed();
    let mut r_p3: ge25519_p3 = core::mem::zeroed();

    if _sodium_ge25519_frombytes(&mut p_p3, p) != 0
        || _sodium_ge25519_is_on_curve(&p_p3) == 0
        || _sodium_ge25519_frombytes(&mut q_p3, q) != 0
        || _sodium_ge25519_is_on_curve(&q_p3) == 0
    {
        return -1;
    }
    _sodium_ge25519_p3_add(&mut r_p3, &p_p3, &q_p3);
    _sodium_ge25519_p3_tobytes(r, &r_p3);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3: ge25519_p3 = core::mem::zeroed();
    let mut q_p3: ge25519_p3 = core::mem::zeroed();
    let mut r_p3: ge25519_p3 = core::mem::zeroed();

    if _sodium_ge25519_frombytes(&mut p_p3, p) != 0
        || _sodium_ge25519_is_on_curve(&p_p3) == 0
        || _sodium_ge25519_frombytes(&mut q_p3, q) != 0
        || _sodium_ge25519_is_on_curve(&q_p3) == 0
    {
        return -1;
    }
    _sodium_ge25519_p3_sub(&mut r_p3, &p_p3, &q_p3);
    _sodium_ge25519_p3_tobytes(r, &r_p3);

    0
}

const HASH_GE_L: usize = 48;

unsafe fn _string_to_points(
    px: *mut u8,
    n: usize,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    let mut h: [u8; crypto_core_ed25519_HASHBYTES] = [0; crypto_core_ed25519_HASHBYTES];
    let mut h_be: [u8; 2 * HASH_GE_L] = [0; 2 * HASH_GE_L];
    let mut i: usize;
    let mut j: usize;

    if n > 2 {
        crate::abort(); /* LCOV_EXCL_LINE */
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
        return -1; /* LCOV_EXCL_LINE */
    }
    i = 0;
    while i < n {
        j = 0;
        while j < HASH_GE_L {
            h[j] = h_be[i * HASH_GE_L + HASH_GE_L - 1 - j];
            j += 1;
        }
        memset(h.as_mut_ptr().add(j), 0, core::mem::size_of_val(&h) - j);
        _sodium_ge25519_from_hash(
            px.add(i * crypto_core_ed25519_BYTES),
            h.as_ptr(),
        );
        i += 1;
    }
    0
}

/* LCOV_EXCL_START */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_from_string_nu(
    p: *mut u8,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    _string_to_points(p, 1, ctx, ctx_len, msg, msg_len, hash_alg)
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
    let mut px: [u8; 2 * crypto_core_ed25519_BYTES] = [0; 2 * crypto_core_ed25519_BYTES];

    if _string_to_points(px.as_mut_ptr(), 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0 {
        return -1;
    }
    crypto_core_ed25519_add(
        p,
        px.as_ptr(),
        px.as_ptr().add(crypto_core_ed25519_BYTES),
    )
}
/* LCOV_EXCL_STOP */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_random(p: *mut u8) {
    let mut h: [u8; crypto_core_ed25519_UNIFORMBYTES] =
        [0; crypto_core_ed25519_UNIFORMBYTES];

    randombytes_buf(
        h.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of_val(&h),
    );
    _sodium_ge25519_from_uniform(p, h.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_random(r: *mut u8) {
    loop {
        randombytes_buf(
            r as *mut core::ffi::c_void,
            crypto_core_ed25519_SCALARBYTES,
        );
        *r.add(crypto_core_ed25519_SCALARBYTES - 1) &= 0x1f;
        if !(sc25519_is_canonical(r) == 0
            || sodium_is_zero(r, crypto_core_ed25519_SCALARBYTES) != 0)
        {
            break;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_invert(
    recip: *mut u8,
    s: *const u8,
) -> c_int {
    sc25519_invert(recip, s);

    -sodium_is_zero(s, crypto_core_ed25519_SCALARBYTES)
}

/* 2^252+27742317777372353535851937790883648493 */
static L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
    0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x10,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_negate(
    neg: *mut u8,
    s: *const u8,
) {
    let mut t_: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut s_: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    memset(t_.as_mut_ptr(), 0, core::mem::size_of_val(&t_));
    memset(s_.as_mut_ptr(), 0, core::mem::size_of_val(&s_));
    crate::common::memcpy(
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        L.as_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    crate::common::memcpy(s_.as_mut_ptr(), s, crypto_core_ed25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), core::mem::size_of_val(&t_));
    sc25519_reduce(t_.as_mut_ptr());
    crate::common::memcpy(neg, t_.as_ptr(), crypto_core_ed25519_SCALARBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_complement(
    comp: *mut u8,
    s: *const u8,
) {
    let mut t_: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut s_: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    memset(t_.as_mut_ptr(), 0, core::mem::size_of_val(&t_));
    memset(s_.as_mut_ptr(), 0, core::mem::size_of_val(&s_));
    t_[0] = t_[0].wrapping_add(1);
    crate::common::memcpy(
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        L.as_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    crate::common::memcpy(s_.as_mut_ptr(), s, crypto_core_ed25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), core::mem::size_of_val(&t_));
    sc25519_reduce(t_.as_mut_ptr());
    crate::common::memcpy(comp, t_.as_ptr(), crypto_core_ed25519_SCALARBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_add(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let mut x_: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut y_: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    memset(x_.as_mut_ptr(), 0, core::mem::size_of_val(&x_));
    memset(y_.as_mut_ptr(), 0, core::mem::size_of_val(&y_));
    crate::common::memcpy(x_.as_mut_ptr(), x, crypto_core_ed25519_SCALARBYTES);
    crate::common::memcpy(y_.as_mut_ptr(), y, crypto_core_ed25519_SCALARBYTES);
    sodium_add(x_.as_mut_ptr(), y_.as_ptr(), crypto_core_ed25519_SCALARBYTES);
    crypto_core_ed25519_scalar_reduce(z, x_.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let mut yn: [u8; crypto_core_ed25519_SCALARBYTES] =
        [0; crypto_core_ed25519_SCALARBYTES];

    crypto_core_ed25519_scalar_negate(yn.as_mut_ptr(), y);
    crypto_core_ed25519_scalar_add(z, x, yn.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_mul(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    sc25519_mul(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_reduce(
    r: *mut u8,
    s: *const u8,
) {
    let mut t: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    crate::common::memcpy(t.as_mut_ptr(), s, core::mem::size_of_val(&t));
    sc25519_reduce(t.as_mut_ptr());
    crate::common::memcpy(r, t.as_ptr(), crypto_core_ed25519_SCALARBYTES);
    sodium_memzero(
        t.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of_val(&t),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_is_canonical(s: *const u8) -> c_int {
    sc25519_is_canonical(s)
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
    let mut h: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut h_be: [u8; HASH_SC_L] = [0; HASH_SC_L];
    let mut i: usize;

    if _sodium_core_h2c_string_to_hash(
        h_be.as_mut_ptr(),
        core::mem::size_of_val(&h_be),
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
    memset(h.as_mut_ptr().add(i), 0, core::mem::size_of_val(&h) - i);
    crypto_core_ed25519_scalar_reduce(s, h.as_ptr());

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_bytes() -> usize {
    crypto_core_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_nonreducedscalarbytes() -> usize {
    crypto_core_ed25519_NONREDUCEDSCALARBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_uniformbytes() -> usize {
    crypto_core_ed25519_UNIFORMBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_hashbytes() -> usize {
    crypto_core_ed25519_HASHBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_ed25519_scalarbytes() -> usize {
    crypto_core_ed25519_SCALARBYTES
}
