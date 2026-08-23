//! Translation of `crypto_core/ed25519/core_ristretto255.c`.
//!
//! The `ge25519_*` / `ristretto255_*` / `sc25519_*` helpers live in
//! `crypto_core/ed25519/ref10/ed25519_ref10.c`, `core_h2c_string_to_hash` in
//! `crypto_core/ed25519/core_h2c.c` and the `crypto_core_ed25519_*` functions
//! in `crypto_core/ed25519/core_ed25519.c`; all of them are reached through
//! the linker.

use core::ffi::{c_int, c_void};

/* crypto_core_ristretto255.h */
const crypto_core_ristretto255_BYTES: usize = 32;
const crypto_core_ristretto255_HASHBYTES: usize = 64;
const crypto_core_ristretto255_SCALARBYTES: usize = 32;
const crypto_core_ristretto255_NONREDUCEDSCALARBYTES: usize = 64;

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
    /* crypto_core/ed25519/ref10/ed25519_ref10.c */
    fn _sodium_ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    fn _sodium_ristretto255_from_hash(s: *mut u8, h: *const u8);
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

    /* crypto_core/ed25519/core_ed25519.c */
    fn crypto_core_ed25519_scalar_random(r: *mut u8);
    fn crypto_core_ed25519_scalar_invert(
        recip: *mut u8,
        s: *const u8,
    ) -> c_int;
    fn crypto_core_ed25519_scalar_negate(neg: *mut u8, s: *const u8);
    fn crypto_core_ed25519_scalar_complement(comp: *mut u8, s: *const u8);
    fn crypto_core_ed25519_scalar_add(z: *mut u8, x: *const u8, y: *const u8);
    fn crypto_core_ed25519_scalar_sub(z: *mut u8, x: *const u8, y: *const u8);
    fn crypto_core_ed25519_scalar_reduce(r: *mut u8, s: *const u8);
    fn crypto_core_ed25519_scalar_from_string(
        s: *mut u8,
        ctx: *const u8,
        ctx_len: usize,
        msg: *const u8,
        msg_len: usize,
        hash_alg: c_int,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_is_valid_point(
    p: *const u8,
) -> c_int {
    let mut p_p3 = ge25519_p3::new();
    let p_p3p: *mut ge25519_p3 = &mut p_p3;

    if _sodium_ristretto255_frombytes(p_p3p, p) != 0 {
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
    let mut p_p3 = ge25519_p3::new();
    let mut q_p3 = ge25519_p3::new();
    let mut r_p3 = ge25519_p3::new();
    let p_p3p: *mut ge25519_p3 = &mut p_p3;
    let q_p3p: *mut ge25519_p3 = &mut q_p3;
    let r_p3p: *mut ge25519_p3 = &mut r_p3;

    if _sodium_ristretto255_frombytes(p_p3p, p) != 0
        || _sodium_ristretto255_frombytes(q_p3p, q) != 0
    {
        return -1;
    }
    _sodium_ge25519_p3_add(r_p3p, p_p3p, q_p3p);
    _sodium_ristretto255_p3_tobytes(r, r_p3p);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_sub(
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

    if _sodium_ristretto255_frombytes(p_p3p, p) != 0
        || _sodium_ristretto255_frombytes(q_p3p, q) != 0
    {
        return -1;
    }
    _sodium_ge25519_p3_sub(r_p3p, p_p3p, q_p3p);
    _sodium_ristretto255_p3_tobytes(r, r_p3p);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_from_hash(
    p: *mut u8,
    r: *const u8,
) -> c_int {
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
    let mut h = [0u8; crypto_core_ristretto255_HASHBYTES];

    if _sodium_core_h2c_string_to_hash(
        h.as_mut_ptr(),
        crypto_core_ristretto255_HASHBYTES,
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
    let mut h = [0u8; crypto_core_ristretto255_HASHBYTES];

    randombytes_buf(
        h.as_mut_ptr() as *mut c_void,
        crypto_core_ristretto255_HASHBYTES,
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
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_negate(
    neg: *mut u8,
    s: *const u8,
) {
    crypto_core_ed25519_scalar_negate(neg, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_complement(
    comp: *mut u8,
    s: *const u8,
) {
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
    _sodium_sc25519_mul(z, x, y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_reduce(
    r: *mut u8,
    s: *const u8,
) {
    crypto_core_ed25519_scalar_reduce(r, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalar_is_canonical(
    s: *const u8,
) -> c_int {
    _sodium_sc25519_is_canonical(s)
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
pub unsafe extern "C" fn crypto_core_ristretto255_bytes() -> usize {
    crypto_core_ristretto255_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_nonreducedscalarbytes() -> usize
{
    crypto_core_ristretto255_NONREDUCEDSCALARBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_hashbytes() -> usize {
    crypto_core_ristretto255_HASHBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_ristretto255_scalarbytes() -> usize {
    crypto_core_ristretto255_SCALARBYTES
}
