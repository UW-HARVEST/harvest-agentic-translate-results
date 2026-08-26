//! Translation of `crypto_core/ed25519/core_ed25519.c`.
//!
//! The `ge25519_*` / `sc25519_*` helpers live in
//! `crypto_core/ed25519/ref10/ed25519_ref10.c` and `core_h2c_string_to_hash`
//! in `crypto_core/ed25519/core_h2c.c`; all of them are reached through the
//! linker under their `private/quirks.h` names.

use core::ffi::{c_int, c_void};

/* crypto_core_ed25519.h */
const crypto_core_ed25519_BYTES: usize = 32;
const crypto_core_ed25519_UNIFORMBYTES: usize = 32;
const crypto_core_ed25519_HASHBYTES: usize = 64;
const crypto_core_ed25519_SCALARBYTES: usize = 32;
const crypto_core_ed25519_NONREDUCEDSCALARBYTES: usize = 64;

/* `typedef int32_t fe25519[10];` (HAVE_TI_MODE is unset) */
type Fe = [i32; 10];

/* `ge25519_p3` from `private/ed25519_ref10.h` */
#[repr(C)]
#[derive(Copy, Clone)]
struct ge25519_p3 {
    X: Fe,
    Y: Fe,
    Z: Fe,
    T: Fe,
}

impl ge25519_p3 {
    const fn new() -> Self {
        ge25519_p3 { X: [0; 10], Y: [0; 10], Z: [0; 10], T: [0; 10] }
    }
}

extern "C" {
    /* <stdlib.h> */
    fn abort() -> !;

    /* crypto_core/ed25519/ref10/ed25519_ref10.c */
    fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ge25519_is_on_curve(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_p3_add(
        r: *mut ge25519_p3,
        p: *const ge25519_p3,
        q: *const ge25519_p3,
    );
    fn _sodium_ge25519_p3_sub(
        r: *mut ge25519_p3,
        p: *const ge25519_p3,
        q: *const ge25519_p3,
    );
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8);
    fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8);
    fn _sodium_sc25519_invert(recip: *mut u8, s: *const u8);
    fn _sodium_sc25519_reduce(s: *mut u8);
    fn _sodium_sc25519_mul(s: *mut u8, a: *const u8, b: *const u8);
    fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int;

    /* crypto_core/ed25519/core_h2c.c */
    fn _sodium_core_h2c_string_to_hash(
        h: *mut u8,
        h_len: usize,
        ctx: *const u8,
        ctx_len: usize,
        msg: *const u8,
        msg_len: usize,
        hash_alg: c_int,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
    fn sodium_add(a: *mut u8, b: *const u8, len: usize);
    fn sodium_sub(a: *mut u8, b: *const u8, len: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_is_valid_point(
    p: *const u8,
) -> c_int {
    let mut p_p3 = ge25519_p3::new();
    let p_p3p: *mut ge25519_p3 = &mut p_p3;

    if _sodium_ge25519_is_canonical(p) == 0
        || _sodium_ge25519_frombytes(p_p3p, p) != 0
        || _sodium_ge25519_is_on_curve(p_p3p) == 0
        || _sodium_ge25519_has_small_order(p_p3p) != 0
        || _sodium_ge25519_is_on_main_subgroup(p_p3p) == 0
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
    let mut p_p3 = ge25519_p3::new();
    let mut q_p3 = ge25519_p3::new();
    let mut r_p3 = ge25519_p3::new();
    let p_p3p: *mut ge25519_p3 = &mut p_p3;
    let q_p3p: *mut ge25519_p3 = &mut q_p3;
    let r_p3p: *mut ge25519_p3 = &mut r_p3;

    if _sodium_ge25519_frombytes(p_p3p, p) != 0
        || _sodium_ge25519_is_on_curve(p_p3p) == 0
        || _sodium_ge25519_frombytes(q_p3p, q) != 0
        || _sodium_ge25519_is_on_curve(q_p3p) == 0
    {
        return -1;
    }
    _sodium_ge25519_p3_add(r_p3p, p_p3p, q_p3p);
    _sodium_ge25519_p3_tobytes(r, r_p3p);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3 = ge25519_p3::new();
    let mut q_p3 = ge25519_p3::new();
    let mut r_p3 = ge25519_p3::new();
    let p_p3p: *mut ge25519_p3 = &mut p_p3;
    let q_p3p: *mut ge25519_p3 = &mut q_p3;
    let r_p3p: *mut ge25519_p3 = &mut r_p3;

    if _sodium_ge25519_frombytes(p_p3p, p) != 0
        || _sodium_ge25519_is_on_curve(p_p3p) == 0
        || _sodium_ge25519_frombytes(q_p3p, q) != 0
        || _sodium_ge25519_is_on_curve(q_p3p) == 0
    {
        return -1;
    }
    _sodium_ge25519_p3_sub(r_p3p, p_p3p, q_p3p);
    _sodium_ge25519_p3_tobytes(r, r_p3p);

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
    let mut h = [0u8; crypto_core_ed25519_HASHBYTES];
    let mut h_be = [0u8; 2 * HASH_GE_L];
    let mut i: usize;
    let mut j: usize;

    if n > 2 {
        abort(); /* LCOV_EXCL_LINE */
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
        /* memset(&h[j], 0, (sizeof h) - j); */
        let mut k = j;
        while k < crypto_core_ed25519_HASHBYTES {
            h[k] = 0;
            k += 1;
        }
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
    let mut px = [0u8; 2 * crypto_core_ed25519_BYTES];

    if _string_to_points(
        px.as_mut_ptr(),
        2,
        ctx,
        ctx_len,
        msg,
        msg_len,
        hash_alg,
    ) != 0
    {
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
    let mut h = [0u8; crypto_core_ed25519_UNIFORMBYTES];

    randombytes_buf(
        h.as_mut_ptr() as *mut c_void,
        crypto_core_ed25519_UNIFORMBYTES,
    );
    _sodium_ge25519_from_uniform(p, h.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_random(r: *mut u8) {
    loop {
        randombytes_buf(r as *mut c_void, crypto_core_ed25519_SCALARBYTES);
        *r.add(crypto_core_ed25519_SCALARBYTES - 1) &= 0x1f;
        if !(_sodium_sc25519_is_canonical(r) == 0
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
    _sodium_sc25519_invert(recip, s);

    -sodium_is_zero(s, crypto_core_ed25519_SCALARBYTES)
}

/* 2^252+27742317777372353535851937790883648493 */
static L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2,
    0xde, 0xf9, 0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_negate(
    neg: *mut u8,
    s: *const u8,
) {
    let mut t_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut s_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    /* memset(t_, 0, sizeof t_); memset(s_, 0, sizeof s_); */
    core::ptr::copy_nonoverlapping(
        L.as_ptr(),
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        crypto_core_ed25519_SCALARBYTES,
    );
    core::ptr::copy_nonoverlapping(
        s,
        s_.as_mut_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    sodium_sub(
        t_.as_mut_ptr(),
        s_.as_ptr(),
        crypto_core_ed25519_NONREDUCEDSCALARBYTES,
    );
    _sodium_sc25519_reduce(t_.as_mut_ptr());
    core::ptr::copy_nonoverlapping(
        t_.as_ptr(),
        neg,
        crypto_core_ed25519_SCALARBYTES,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_complement(
    comp: *mut u8,
    s: *const u8,
) {
    let mut t_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut s_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    /* memset(t_, 0, sizeof t_); memset(s_, 0, sizeof s_); */
    t_[0] = t_[0].wrapping_add(1);
    core::ptr::copy_nonoverlapping(
        L.as_ptr(),
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        crypto_core_ed25519_SCALARBYTES,
    );
    core::ptr::copy_nonoverlapping(
        s,
        s_.as_mut_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    sodium_sub(
        t_.as_mut_ptr(),
        s_.as_ptr(),
        crypto_core_ed25519_NONREDUCEDSCALARBYTES,
    );
    _sodium_sc25519_reduce(t_.as_mut_ptr());
    core::ptr::copy_nonoverlapping(
        t_.as_ptr(),
        comp,
        crypto_core_ed25519_SCALARBYTES,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_add(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let mut x_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut y_ = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    /* memset(x_, 0, sizeof x_); memset(y_, 0, sizeof y_); */
    core::ptr::copy_nonoverlapping(
        x,
        x_.as_mut_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    core::ptr::copy_nonoverlapping(
        y,
        y_.as_mut_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    sodium_add(
        x_.as_mut_ptr(),
        y_.as_ptr(),
        crypto_core_ed25519_SCALARBYTES,
    );
    crypto_core_ed25519_scalar_reduce(z, x_.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let mut yn = [0u8; crypto_core_ed25519_SCALARBYTES];

    crypto_core_ed25519_scalar_negate(yn.as_mut_ptr(), y);
    crypto_core_ed25519_scalar_add(z, x, yn.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_mul(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    _sodium_sc25519_mul(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_reduce(
    r: *mut u8,
    s: *const u8,
) {
    let mut t = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    core::ptr::copy_nonoverlapping(
        s,
        t.as_mut_ptr(),
        crypto_core_ed25519_NONREDUCEDSCALARBYTES,
    );
    _sodium_sc25519_reduce(t.as_mut_ptr());
    core::ptr::copy_nonoverlapping(
        t.as_ptr(),
        r,
        crypto_core_ed25519_SCALARBYTES,
    );
    sodium_memzero(
        t.as_mut_ptr() as *mut c_void,
        crypto_core_ed25519_NONREDUCEDSCALARBYTES,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_is_canonical(
    s: *const u8,
) -> c_int {
    _sodium_sc25519_is_canonical(s)
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
    let mut h = [0u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut h_be = [0u8; HASH_SC_L];
    let mut i: usize;

    if _sodium_core_h2c_string_to_hash(
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
    i = 0;
    while i < HASH_SC_L {
        h[i] = h_be[HASH_SC_L - 1 - i];
        i += 1;
    }
    /* memset(&h[i], 0, (sizeof h) - i); */
    let mut k = i;
    while k < crypto_core_ed25519_NONREDUCEDSCALARBYTES {
        h[k] = 0;
        k += 1;
    }
    crypto_core_ed25519_scalar_reduce(s, h.as_ptr());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_bytes() -> usize {
    crypto_core_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_nonreducedscalarbytes() -> usize {
    crypto_core_ed25519_NONREDUCEDSCALARBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_uniformbytes() -> usize {
    crypto_core_ed25519_UNIFORMBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_hashbytes() -> usize {
    crypto_core_ed25519_HASHBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalarbytes() -> usize {
    crypto_core_ed25519_SCALARBYTES
}
