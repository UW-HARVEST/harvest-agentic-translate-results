//! Translation of c_src/libsodium/crypto_core/ed25519/core_ed25519.c

use core::ffi::{c_int, c_void};

use crate::fe25519::ge25519_p3;

// Constants from include/sodium/crypto_core_ed25519.h
const crypto_core_ed25519_BYTES: usize = 32;
const crypto_core_ed25519_UNIFORMBYTES: usize = 32;
const crypto_core_ed25519_HASHBYTES: usize = 64;
const crypto_core_ed25519_SCALARBYTES: usize = 32;
const crypto_core_ed25519_NONREDUCEDSCALARBYTES: usize = 64;

extern "C" {
    // ref10 exported symbols (quirks.h renaming applied).
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
    fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8);
    fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8);
    fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_sc25519_invert(recip: *mut u8, s: *const u8);
    fn _sodium_sc25519_reduce(s: *mut u8);
    fn _sodium_sc25519_mul(s: *mut u8, a: *const u8, b: *const u8);

    // core_h2c (same C file group, but different .c file -> extern).
    fn _sodium_core_h2c_string_to_hash(
        h: *mut u8,
        h_len: usize,
        ctx: *const u8,
        ctx_len: usize,
        msg: *const u8,
        msg_len: usize,
        hash_alg: c_int,
    ) -> c_int;

    // Exported helpers.
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
    fn sodium_add(a: *mut u8, b: *const u8, len: usize);
    fn sodium_sub(a: *mut u8, b: *const u8, len: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);

    // libc.
    fn abort() -> !;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_is_valid_point(p: *const u8) -> c_int {
    let mut p_p3 = core::mem::MaybeUninit::<ge25519_p3>::uninit();

    if _sodium_ge25519_is_canonical(p) == 0
        || _sodium_ge25519_frombytes(p_p3.as_mut_ptr(), p) != 0
        || _sodium_ge25519_is_on_curve(p_p3.as_ptr()) == 0
        || _sodium_ge25519_has_small_order(p_p3.as_ptr()) != 0
        || _sodium_ge25519_is_on_main_subgroup(p_p3.as_ptr()) == 0
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
    let mut p_p3 = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut q_p3 = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut r_p3 = core::mem::MaybeUninit::<ge25519_p3>::uninit();

    if _sodium_ge25519_frombytes(p_p3.as_mut_ptr(), p) != 0
        || _sodium_ge25519_is_on_curve(p_p3.as_ptr()) == 0
        || _sodium_ge25519_frombytes(q_p3.as_mut_ptr(), q) != 0
        || _sodium_ge25519_is_on_curve(q_p3.as_ptr()) == 0
    {
        return -1;
    }
    _sodium_ge25519_p3_add(r_p3.as_mut_ptr(), p_p3.as_ptr(), q_p3.as_ptr());
    _sodium_ge25519_p3_tobytes(r, r_p3.as_ptr());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_sub(
    r: *mut u8,
    p: *const u8,
    q: *const u8,
) -> c_int {
    let mut p_p3 = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut q_p3 = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut r_p3 = core::mem::MaybeUninit::<ge25519_p3>::uninit();

    if _sodium_ge25519_frombytes(p_p3.as_mut_ptr(), p) != 0
        || _sodium_ge25519_is_on_curve(p_p3.as_ptr()) == 0
        || _sodium_ge25519_frombytes(q_p3.as_mut_ptr(), q) != 0
        || _sodium_ge25519_is_on_curve(q_p3.as_ptr()) == 0
    {
        return -1;
    }
    _sodium_ge25519_p3_sub(r_p3.as_mut_ptr(), p_p3.as_ptr(), q_p3.as_ptr());
    _sodium_ge25519_p3_tobytes(r, r_p3.as_ptr());

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
        abort(); // LCOV_EXCL_LINE
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
        return -1; // LCOV_EXCL_LINE
    }
    // COMPILER_ASSERT(sizeof h >= HASH_GE_L);
    i = 0;
    while i < n {
        j = 0;
        while j < HASH_GE_L {
            h[j] = h_be[i * HASH_GE_L + HASH_GE_L - 1 - j];
            j += 1;
        }
        core::ptr::write_bytes(h.as_mut_ptr().add(j), 0, (core::mem::size_of::<[u8; crypto_core_ed25519_HASHBYTES]>()) - j);
        _sodium_ge25519_from_hash(px.add(i * crypto_core_ed25519_BYTES), h.as_ptr());
        i += 1;
    }
    0
}

// LCOV_EXCL_START
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
    crypto_core_ed25519_add(p, px.as_ptr(), px.as_ptr().add(crypto_core_ed25519_BYTES))
}
// LCOV_EXCL_STOP

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_random(p: *mut u8) {
    let mut h: [u8; crypto_core_ed25519_UNIFORMBYTES] = [0; crypto_core_ed25519_UNIFORMBYTES];

    randombytes_buf(h.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; crypto_core_ed25519_UNIFORMBYTES]>());
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

// 2^252+27742317777372353535851937790883648493
static L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7,
    0xa2, 0xde, 0xf9, 0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
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

    core::ptr::copy_nonoverlapping(
        L.as_ptr(),
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        crypto_core_ed25519_SCALARBYTES,
    );
    core::ptr::copy_nonoverlapping(s, s_.as_mut_ptr(), crypto_core_ed25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), core::mem::size_of::<[u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES]>());
    _sodium_sc25519_reduce(t_.as_mut_ptr());
    core::ptr::copy_nonoverlapping(t_.as_ptr(), neg, crypto_core_ed25519_SCALARBYTES);
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

    t_[0] = t_[0].wrapping_add(1);
    core::ptr::copy_nonoverlapping(
        L.as_ptr(),
        t_.as_mut_ptr().add(crypto_core_ed25519_SCALARBYTES),
        crypto_core_ed25519_SCALARBYTES,
    );
    core::ptr::copy_nonoverlapping(s, s_.as_mut_ptr(), crypto_core_ed25519_SCALARBYTES);
    sodium_sub(t_.as_mut_ptr(), s_.as_ptr(), core::mem::size_of::<[u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES]>());
    _sodium_sc25519_reduce(t_.as_mut_ptr());
    core::ptr::copy_nonoverlapping(t_.as_ptr(), comp, crypto_core_ed25519_SCALARBYTES);
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

    core::ptr::copy_nonoverlapping(x, x_.as_mut_ptr(), crypto_core_ed25519_SCALARBYTES);
    core::ptr::copy_nonoverlapping(y, y_.as_mut_ptr(), crypto_core_ed25519_SCALARBYTES);
    sodium_add(x_.as_mut_ptr(), y_.as_ptr(), crypto_core_ed25519_SCALARBYTES);
    crypto_core_ed25519_scalar_reduce(z, x_.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_sub(
    z: *mut u8,
    x: *const u8,
    y: *const u8,
) {
    let mut yn: [u8; crypto_core_ed25519_SCALARBYTES] = [0; crypto_core_ed25519_SCALARBYTES];

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
    let mut t: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];

    core::ptr::copy_nonoverlapping(s, t.as_mut_ptr(), core::mem::size_of::<[u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES]>());
    _sodium_sc25519_reduce(t.as_mut_ptr());
    core::ptr::copy_nonoverlapping(t.as_ptr(), r, crypto_core_ed25519_SCALARBYTES);
    sodium_memzero(t.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES]>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ed25519_scalar_is_canonical(s: *const u8) -> c_int {
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
    let mut h: [u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES] =
        [0; crypto_core_ed25519_NONREDUCEDSCALARBYTES];
    let mut h_be: [u8; HASH_SC_L] = [0; HASH_SC_L];
    let mut i: usize;

    if _sodium_core_h2c_string_to_hash(
        h_be.as_mut_ptr(),
        core::mem::size_of::<[u8; HASH_SC_L]>(),
        ctx,
        ctx_len,
        msg,
        msg_len,
        hash_alg,
    ) != 0
    {
        return -1;
    }
    // COMPILER_ASSERT(sizeof h >= sizeof h_be);
    i = 0;
    while i < HASH_SC_L {
        h[i] = h_be[HASH_SC_L - 1 - i];
        i += 1;
    }
    core::ptr::write_bytes(h.as_mut_ptr().add(i), 0, (core::mem::size_of::<[u8; crypto_core_ed25519_NONREDUCEDSCALARBYTES]>()) - i);
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
