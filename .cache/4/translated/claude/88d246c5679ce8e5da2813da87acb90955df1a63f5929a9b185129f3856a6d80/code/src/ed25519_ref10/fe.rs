//! `fe25519` — arithmetic in GF(2^255-19) using 10 signed limbs of
//! alternating 26 / 25 bits (`int32_t fe25519[10]`).
//!
//! Sources translated here (all of them, verbatim, `HAVE_TI_MODE` **not**
//! defined so the `#else` / 10x25.5-limb branch is taken):
//!
//! * `include/sodium/private/ed25519_ref10_fe_25_5.h`
//!   (`fe25519_0`, `fe25519_1`, `fe25519_add`, `fe25519_sub`, `fe25519_neg`,
//!    `fe25519_cmov`, `fe25519_cswap`, `fe25519_copy`, `fe25519_isnegative`,
//!    `fe25519_iszero`, `fe25519_mul`, `fe25519_sq`, `fe25519_sq2`,
//!    `fe25519_mul32`)
//! * `crypto_core/ed25519/ref10/fe_25_5/fe.h`
//!   (`fe25519_frombytes`, `fe25519_reduce`, `fe25519_tobytes`)
//! * `crypto_core/ed25519/ref10/ed25519_ref10.c` lines 52..256
//!   (`fe25519_sqmul`, `fe25519_invert`, `fe25519_pow22523`, `fe25519_cneg`,
//!    `fe25519_abs`, `fe25519_unchecked_sqrt`, `fe25519_sqrt`,
//!    `fe25519_notsquare`)
//!
//! # API provided to the sibling modules (`ge.rs`, `sc.rs`, `h2c.rs`, `ristretto.rs`)
//!
//! Three flavours of every operation are exported, so callers can always pick a
//! form that satisfies Rust's aliasing rules:
//!
//! 1. **Value form** (the primitive; everything else is built on it):
//!    ```ignore
//!    fe_add(f,&g)->Fe25519, fe_sub, fe_mul, fe_sq, fe_sq2, fe_neg, fe_mul32,
//!    fe_reduce, fe_invert, fe_pow22523, fe_cneg, fe_abs, fe_unchecked_sqrt,
//!    fe_cmov, fe_tobytes, fe_frombytes, fe_0, fe_1
//!    ```
//!    These *cannot* alias by construction — use them for `x = x * y` patterns.
//!
//! 2. **C-mirroring out-param form** (out param first, exactly like the C):
//!    ```ignore
//!    fe25519_0(h), fe25519_1(h), fe25519_add(h,f,g), fe25519_sub(h,f,g),
//!    fe25519_neg(h,f), fe25519_cmov(f,g,b), fe25519_cswap(f,g,b),
//!    fe25519_copy(h,f), fe25519_isnegative(f)->i32, fe25519_iszero(f)->i32,
//!    fe25519_mul(h,f,g), fe25519_sq(h,f), fe25519_sq2(h,f),
//!    fe25519_mul32(h,f,n), fe25519_frombytes(h,&[u8;32]),
//!    fe25519_reduce(h,f), fe25519_tobytes(&mut [u8;32],h),
//!    fe25519_sqmul(s,n,a), fe25519_invert(out,z), fe25519_pow22523(out,z),
//!    fe25519_cneg(h,b), fe25519_abs(h), fe25519_unchecked_sqrt(x,x2),
//!    fe25519_sqrt(x,x2)->i32, fe25519_notsquare(x)->i32
//!    ```
//!    Use when the destination is distinct from the sources.
//!
//! 3. **Raw-pointer form** (`*_p` suffix), for the (very common) aliasing calls
//!    such as `fe25519_mul(h, h, g)` and for writing into struct fields without
//!    fighting the borrow checker:
//!    ```ignore
//!    unsafe { fe25519_mul_p(h, h, g) }        // *mut i32, *const i32, *const i32
//!    ```
//!    Every function above has a `_p` twin.  The `_p` twins load *all* inputs
//!    into locals before storing anything, exactly like the C, so overlap is
//!    always safe.
//!
//! Exported (`no_mangle`) symbols, after the renaming in `private/quirks.h`:
//! `_sodium_fe25519_frombytes`, `_sodium_fe25519_tobytes`,
//! `_sodium_fe25519_invert`.

use core::ffi::c_int;

use super::tables::FE25519_SQRTM1;
use super::{load_3, load_4, Fe25519};

extern "C" {
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
}

/* ------------------------------------------------------------------------- */
/*  h = 0                                                                     */
/* ------------------------------------------------------------------------- */

/// `static inline void fe25519_0(fe25519 h)` — `memset(&h[0], 0, 10 * sizeof h[0]);`
#[inline]
pub fn fe_0() -> Fe25519 {
    [0; 10]
}

#[inline]
pub fn fe25519_0(h: &mut Fe25519) {
    *h = fe_0();
}

#[inline]
pub unsafe fn fe25519_0_p(h: *mut i32) {
    let v = fe_0();
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  h = 1                                                                     */
/* ------------------------------------------------------------------------- */

/// `static inline void fe25519_1(fe25519 h)`
#[inline]
pub fn fe_1() -> Fe25519 {
    let mut h: Fe25519 = [0; 10];
    h[0] = 1;
    h[1] = 0;
    /* memset(&h[2], 0, 8 * sizeof h[0]) */
    h
}

#[inline]
pub fn fe25519_1(h: &mut Fe25519) {
    *h = fe_1();
}

#[inline]
pub unsafe fn fe25519_1_p(h: *mut i32) {
    let v = fe_1();
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  h = f + g                                                                 */
/* ------------------------------------------------------------------------- */

/// `static inline void fe25519_add(fe25519 h, const fe25519 f, const fe25519 g)`
#[inline]
pub fn fe_add(f: &Fe25519, g: &Fe25519) -> Fe25519 {
    let h0 = f[0].wrapping_add(g[0]);
    let h1 = f[1].wrapping_add(g[1]);
    let h2 = f[2].wrapping_add(g[2]);
    let h3 = f[3].wrapping_add(g[3]);
    let h4 = f[4].wrapping_add(g[4]);
    let h5 = f[5].wrapping_add(g[5]);
    let h6 = f[6].wrapping_add(g[6]);
    let h7 = f[7].wrapping_add(g[7]);
    let h8 = f[8].wrapping_add(g[8]);
    let h9 = f[9].wrapping_add(g[9]);

    [h0, h1, h2, h3, h4, h5, h6, h7, h8, h9]
}

#[inline]
pub fn fe25519_add(h: &mut Fe25519, f: &Fe25519, g: &Fe25519) {
    *h = fe_add(f, g);
}

#[inline]
pub unsafe fn fe25519_add_p(h: *mut i32, f: *const i32, g: *const i32) {
    let v = fe_add(&load(f), &load(g));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  h = f - g                                                                 */
/* ------------------------------------------------------------------------- */

/// `static void fe25519_sub(fe25519 h, const fe25519 f, const fe25519 g)`
#[inline]
pub fn fe_sub(f: &Fe25519, g: &Fe25519) -> Fe25519 {
    let h0 = f[0].wrapping_sub(g[0]);
    let h1 = f[1].wrapping_sub(g[1]);
    let h2 = f[2].wrapping_sub(g[2]);
    let h3 = f[3].wrapping_sub(g[3]);
    let h4 = f[4].wrapping_sub(g[4]);
    let h5 = f[5].wrapping_sub(g[5]);
    let h6 = f[6].wrapping_sub(g[6]);
    let h7 = f[7].wrapping_sub(g[7]);
    let h8 = f[8].wrapping_sub(g[8]);
    let h9 = f[9].wrapping_sub(g[9]);

    [h0, h1, h2, h3, h4, h5, h6, h7, h8, h9]
}

#[inline]
pub fn fe25519_sub(h: &mut Fe25519, f: &Fe25519, g: &Fe25519) {
    *h = fe_sub(f, g);
}

#[inline]
pub unsafe fn fe25519_sub_p(h: *mut i32, f: *const i32, g: *const i32) {
    let v = fe_sub(&load(f), &load(g));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  h = -f                                                                    */
/* ------------------------------------------------------------------------- */

/// `static inline void fe25519_neg(fe25519 h, const fe25519 f)`
#[inline]
pub fn fe_neg(f: &Fe25519) -> Fe25519 {
    let h0 = f[0].wrapping_neg();
    let h1 = f[1].wrapping_neg();
    let h2 = f[2].wrapping_neg();
    let h3 = f[3].wrapping_neg();
    let h4 = f[4].wrapping_neg();
    let h5 = f[5].wrapping_neg();
    let h6 = f[6].wrapping_neg();
    let h7 = f[7].wrapping_neg();
    let h8 = f[8].wrapping_neg();
    let h9 = f[9].wrapping_neg();

    [h0, h1, h2, h3, h4, h5, h6, h7, h8, h9]
}

#[inline]
pub fn fe25519_neg(h: &mut Fe25519, f: &Fe25519) {
    *h = fe_neg(f);
}

#[inline]
pub unsafe fn fe25519_neg_p(h: *mut i32, f: *const i32) {
    let v = fe_neg(&load(f));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  conditional move / swap  (constant time, no branches)                     */
/* ------------------------------------------------------------------------- */

/// `static void fe25519_cmov(fe25519 f, const fe25519 g, unsigned int b)`
///
/// Returns the (possibly) updated `f`.
#[inline]
pub fn fe_cmov(f: &Fe25519, g: &Fe25519, b: u32) -> Fe25519 {
    /* uint32_t mask = (uint32_t) (-(int32_t) b); */
    let mask: u32 = (b as i32).wrapping_neg() as u32;
    let m = mask as i32;

    let f0 = f[0];
    let f1 = f[1];
    let f2 = f[2];
    let f3 = f[3];
    let f4 = f[4];
    let f5 = f[5];
    let f6 = f[6];
    let f7 = f[7];
    let f8 = f[8];
    let f9 = f[9];

    let mut x0 = f0 ^ g[0];
    let mut x1 = f1 ^ g[1];
    let mut x2 = f2 ^ g[2];
    let mut x3 = f3 ^ g[3];
    let mut x4 = f4 ^ g[4];
    let mut x5 = f5 ^ g[5];
    let mut x6 = f6 ^ g[6];
    let mut x7 = f7 ^ g[7];
    let mut x8 = f8 ^ g[8];
    let mut x9 = f9 ^ g[9];

    x0 &= m;
    x1 &= m;
    x2 &= m;
    x3 &= m;
    x4 &= m;
    x5 &= m;
    x6 &= m;
    x7 &= m;
    x8 &= m;
    x9 &= m;

    [
        f0 ^ x0,
        f1 ^ x1,
        f2 ^ x2,
        f3 ^ x3,
        f4 ^ x4,
        f5 ^ x5,
        f6 ^ x6,
        f7 ^ x7,
        f8 ^ x8,
        f9 ^ x9,
    ]
}

#[inline]
pub fn fe25519_cmov(f: &mut Fe25519, g: &Fe25519, b: u32) {
    let r = fe_cmov(f, g, b);
    *f = r;
}

#[inline]
pub unsafe fn fe25519_cmov_p(f: *mut i32, g: *const i32, b: u32) {
    let v = fe_cmov(&load(f), &load(g), b);
    core::ptr::copy_nonoverlapping(v.as_ptr(), f, 10);
}

/// `static void fe25519_cswap(fe25519 f, fe25519 g, unsigned int b)`
///
/// Returns the new `(f, g)`.
#[inline]
pub fn fe_cswap(f: &Fe25519, g: &Fe25519, b: u32) -> (Fe25519, Fe25519) {
    /* uint32_t mask = (uint32_t) (-(int64_t) b); */
    let mask: u32 = (b as i64).wrapping_neg() as u32;
    let m = mask as i32;

    let f0 = f[0];
    let f1 = f[1];
    let f2 = f[2];
    let f3 = f[3];
    let f4 = f[4];
    let f5 = f[5];
    let f6 = f[6];
    let f7 = f[7];
    let f8 = f[8];
    let f9 = f[9];

    let g0 = g[0];
    let g1 = g[1];
    let g2 = g[2];
    let g3 = g[3];
    let g4 = g[4];
    let g5 = g[5];
    let g6 = g[6];
    let g7 = g[7];
    let g8 = g[8];
    let g9 = g[9];

    let mut x0 = f0 ^ g0;
    let mut x1 = f1 ^ g1;
    let mut x2 = f2 ^ g2;
    let mut x3 = f3 ^ g3;
    let mut x4 = f4 ^ g4;
    let mut x5 = f5 ^ g5;
    let mut x6 = f6 ^ g6;
    let mut x7 = f7 ^ g7;
    let mut x8 = f8 ^ g8;
    let mut x9 = f9 ^ g9;

    x0 &= m;
    x1 &= m;
    x2 &= m;
    x3 &= m;
    x4 &= m;
    x5 &= m;
    x6 &= m;
    x7 &= m;
    x8 &= m;
    x9 &= m;

    (
        [
            f0 ^ x0,
            f1 ^ x1,
            f2 ^ x2,
            f3 ^ x3,
            f4 ^ x4,
            f5 ^ x5,
            f6 ^ x6,
            f7 ^ x7,
            f8 ^ x8,
            f9 ^ x9,
        ],
        [
            g0 ^ x0,
            g1 ^ x1,
            g2 ^ x2,
            g3 ^ x3,
            g4 ^ x4,
            g5 ^ x5,
            g6 ^ x6,
            g7 ^ x7,
            g8 ^ x8,
            g9 ^ x9,
        ],
    )
}

#[inline]
pub fn fe25519_cswap(f: &mut Fe25519, g: &mut Fe25519, b: u32) {
    let (nf, ng) = fe_cswap(f, g, b);
    *f = nf;
    *g = ng;
}

#[inline]
pub unsafe fn fe25519_cswap_p(f: *mut i32, g: *mut i32, b: u32) {
    let (nf, ng) = fe_cswap(&load(f), &load(g), b);
    core::ptr::copy_nonoverlapping(nf.as_ptr(), f, 10);
    core::ptr::copy_nonoverlapping(ng.as_ptr(), g, 10);
}

/* ------------------------------------------------------------------------- */
/*  h = f                                                                     */
/* ------------------------------------------------------------------------- */

/// `static inline void fe25519_copy(fe25519 h, const fe25519 f)`
#[inline]
pub fn fe25519_copy(h: &mut Fe25519, f: &Fe25519) {
    *h = *f;
}

#[inline]
pub unsafe fn fe25519_copy_p(h: *mut i32, f: *const i32) {
    core::ptr::copy(f, h, 10);
}

/* ------------------------------------------------------------------------- */
/*  sign / zero tests                                                         */
/* ------------------------------------------------------------------------- */

/// `static inline int fe25519_isnegative(const fe25519 f)`
#[inline]
pub fn fe25519_isnegative(f: &Fe25519) -> i32 {
    let mut s = [0u8; 32];

    fe25519_tobytes(&mut s, f);

    (s[0] & 1) as i32
}

#[inline]
pub unsafe fn fe25519_isnegative_p(f: *const i32) -> i32 {
    fe25519_isnegative(&load(f))
}

/// `static inline int fe25519_iszero(const fe25519 f)`
#[inline]
pub fn fe25519_iszero(f: &Fe25519) -> i32 {
    let mut s = [0u8; 32];

    fe25519_tobytes(&mut s, f);

    unsafe { sodium_is_zero(s.as_ptr(), 32) as i32 }
}

#[inline]
pub unsafe fn fe25519_iszero_p(f: *const i32) -> i32 {
    fe25519_iszero(&load(f))
}

/* ------------------------------------------------------------------------- */
/*  h = f * g                                                                 */
/* ------------------------------------------------------------------------- */

/// `static void fe25519_mul(fe25519 h, const fe25519 f, const fe25519 g)`
pub fn fe_mul(f: &Fe25519, g: &Fe25519) -> Fe25519 {
    let f0: i32 = f[0];
    let f1: i32 = f[1];
    let f2: i32 = f[2];
    let f3: i32 = f[3];
    let f4: i32 = f[4];
    let f5: i32 = f[5];
    let f6: i32 = f[6];
    let f7: i32 = f[7];
    let f8: i32 = f[8];
    let f9: i32 = f[9];

    let g0: i32 = g[0];
    let g1: i32 = g[1];
    let g2: i32 = g[2];
    let g3: i32 = g[3];
    let g4: i32 = g[4];
    let g5: i32 = g[5];
    let g6: i32 = g[6];
    let g7: i32 = g[7];
    let g8: i32 = g[8];
    let g9: i32 = g[9];

    let g1_19: i32 = 19i32.wrapping_mul(g1); /* 1.959375*2^29 */
    let g2_19: i32 = 19i32.wrapping_mul(g2); /* 1.959375*2^30; still ok */
    let g3_19: i32 = 19i32.wrapping_mul(g3);
    let g4_19: i32 = 19i32.wrapping_mul(g4);
    let g5_19: i32 = 19i32.wrapping_mul(g5);
    let g6_19: i32 = 19i32.wrapping_mul(g6);
    let g7_19: i32 = 19i32.wrapping_mul(g7);
    let g8_19: i32 = 19i32.wrapping_mul(g8);
    let g9_19: i32 = 19i32.wrapping_mul(g9);
    let f1_2: i32 = 2i32.wrapping_mul(f1);
    let f3_2: i32 = 2i32.wrapping_mul(f3);
    let f5_2: i32 = 2i32.wrapping_mul(f5);
    let f7_2: i32 = 2i32.wrapping_mul(f7);
    let f9_2: i32 = 2i32.wrapping_mul(f9);

    let f0g0: i64 = (f0 as i64).wrapping_mul(g0 as i64);
    let f0g1: i64 = (f0 as i64).wrapping_mul(g1 as i64);
    let f0g2: i64 = (f0 as i64).wrapping_mul(g2 as i64);
    let f0g3: i64 = (f0 as i64).wrapping_mul(g3 as i64);
    let f0g4: i64 = (f0 as i64).wrapping_mul(g4 as i64);
    let f0g5: i64 = (f0 as i64).wrapping_mul(g5 as i64);
    let f0g6: i64 = (f0 as i64).wrapping_mul(g6 as i64);
    let f0g7: i64 = (f0 as i64).wrapping_mul(g7 as i64);
    let f0g8: i64 = (f0 as i64).wrapping_mul(g8 as i64);
    let f0g9: i64 = (f0 as i64).wrapping_mul(g9 as i64);
    let f1g0: i64 = (f1 as i64).wrapping_mul(g0 as i64);
    let f1g1_2: i64 = (f1_2 as i64).wrapping_mul(g1 as i64);
    let f1g2: i64 = (f1 as i64).wrapping_mul(g2 as i64);
    let f1g3_2: i64 = (f1_2 as i64).wrapping_mul(g3 as i64);
    let f1g4: i64 = (f1 as i64).wrapping_mul(g4 as i64);
    let f1g5_2: i64 = (f1_2 as i64).wrapping_mul(g5 as i64);
    let f1g6: i64 = (f1 as i64).wrapping_mul(g6 as i64);
    let f1g7_2: i64 = (f1_2 as i64).wrapping_mul(g7 as i64);
    let f1g8: i64 = (f1 as i64).wrapping_mul(g8 as i64);
    let f1g9_38: i64 = (f1_2 as i64).wrapping_mul(g9_19 as i64);
    let f2g0: i64 = (f2 as i64).wrapping_mul(g0 as i64);
    let f2g1: i64 = (f2 as i64).wrapping_mul(g1 as i64);
    let f2g2: i64 = (f2 as i64).wrapping_mul(g2 as i64);
    let f2g3: i64 = (f2 as i64).wrapping_mul(g3 as i64);
    let f2g4: i64 = (f2 as i64).wrapping_mul(g4 as i64);
    let f2g5: i64 = (f2 as i64).wrapping_mul(g5 as i64);
    let f2g6: i64 = (f2 as i64).wrapping_mul(g6 as i64);
    let f2g7: i64 = (f2 as i64).wrapping_mul(g7 as i64);
    let f2g8_19: i64 = (f2 as i64).wrapping_mul(g8_19 as i64);
    let f2g9_19: i64 = (f2 as i64).wrapping_mul(g9_19 as i64);
    let f3g0: i64 = (f3 as i64).wrapping_mul(g0 as i64);
    let f3g1_2: i64 = (f3_2 as i64).wrapping_mul(g1 as i64);
    let f3g2: i64 = (f3 as i64).wrapping_mul(g2 as i64);
    let f3g3_2: i64 = (f3_2 as i64).wrapping_mul(g3 as i64);
    let f3g4: i64 = (f3 as i64).wrapping_mul(g4 as i64);
    let f3g5_2: i64 = (f3_2 as i64).wrapping_mul(g5 as i64);
    let f3g6: i64 = (f3 as i64).wrapping_mul(g6 as i64);
    let f3g7_38: i64 = (f3_2 as i64).wrapping_mul(g7_19 as i64);
    let f3g8_19: i64 = (f3 as i64).wrapping_mul(g8_19 as i64);
    let f3g9_38: i64 = (f3_2 as i64).wrapping_mul(g9_19 as i64);
    let f4g0: i64 = (f4 as i64).wrapping_mul(g0 as i64);
    let f4g1: i64 = (f4 as i64).wrapping_mul(g1 as i64);
    let f4g2: i64 = (f4 as i64).wrapping_mul(g2 as i64);
    let f4g3: i64 = (f4 as i64).wrapping_mul(g3 as i64);
    let f4g4: i64 = (f4 as i64).wrapping_mul(g4 as i64);
    let f4g5: i64 = (f4 as i64).wrapping_mul(g5 as i64);
    let f4g6_19: i64 = (f4 as i64).wrapping_mul(g6_19 as i64);
    let f4g7_19: i64 = (f4 as i64).wrapping_mul(g7_19 as i64);
    let f4g8_19: i64 = (f4 as i64).wrapping_mul(g8_19 as i64);
    let f4g9_19: i64 = (f4 as i64).wrapping_mul(g9_19 as i64);
    let f5g0: i64 = (f5 as i64).wrapping_mul(g0 as i64);
    let f5g1_2: i64 = (f5_2 as i64).wrapping_mul(g1 as i64);
    let f5g2: i64 = (f5 as i64).wrapping_mul(g2 as i64);
    let f5g3_2: i64 = (f5_2 as i64).wrapping_mul(g3 as i64);
    let f5g4: i64 = (f5 as i64).wrapping_mul(g4 as i64);
    let f5g5_38: i64 = (f5_2 as i64).wrapping_mul(g5_19 as i64);
    let f5g6_19: i64 = (f5 as i64).wrapping_mul(g6_19 as i64);
    let f5g7_38: i64 = (f5_2 as i64).wrapping_mul(g7_19 as i64);
    let f5g8_19: i64 = (f5 as i64).wrapping_mul(g8_19 as i64);
    let f5g9_38: i64 = (f5_2 as i64).wrapping_mul(g9_19 as i64);
    let f6g0: i64 = (f6 as i64).wrapping_mul(g0 as i64);
    let f6g1: i64 = (f6 as i64).wrapping_mul(g1 as i64);
    let f6g2: i64 = (f6 as i64).wrapping_mul(g2 as i64);
    let f6g3: i64 = (f6 as i64).wrapping_mul(g3 as i64);
    let f6g4_19: i64 = (f6 as i64).wrapping_mul(g4_19 as i64);
    let f6g5_19: i64 = (f6 as i64).wrapping_mul(g5_19 as i64);
    let f6g6_19: i64 = (f6 as i64).wrapping_mul(g6_19 as i64);
    let f6g7_19: i64 = (f6 as i64).wrapping_mul(g7_19 as i64);
    let f6g8_19: i64 = (f6 as i64).wrapping_mul(g8_19 as i64);
    let f6g9_19: i64 = (f6 as i64).wrapping_mul(g9_19 as i64);
    let f7g0: i64 = (f7 as i64).wrapping_mul(g0 as i64);
    let f7g1_2: i64 = (f7_2 as i64).wrapping_mul(g1 as i64);
    let f7g2: i64 = (f7 as i64).wrapping_mul(g2 as i64);
    let f7g3_38: i64 = (f7_2 as i64).wrapping_mul(g3_19 as i64);
    let f7g4_19: i64 = (f7 as i64).wrapping_mul(g4_19 as i64);
    let f7g5_38: i64 = (f7_2 as i64).wrapping_mul(g5_19 as i64);
    let f7g6_19: i64 = (f7 as i64).wrapping_mul(g6_19 as i64);
    let f7g7_38: i64 = (f7_2 as i64).wrapping_mul(g7_19 as i64);
    let f7g8_19: i64 = (f7 as i64).wrapping_mul(g8_19 as i64);
    let f7g9_38: i64 = (f7_2 as i64).wrapping_mul(g9_19 as i64);
    let f8g0: i64 = (f8 as i64).wrapping_mul(g0 as i64);
    let f8g1: i64 = (f8 as i64).wrapping_mul(g1 as i64);
    let f8g2_19: i64 = (f8 as i64).wrapping_mul(g2_19 as i64);
    let f8g3_19: i64 = (f8 as i64).wrapping_mul(g3_19 as i64);
    let f8g4_19: i64 = (f8 as i64).wrapping_mul(g4_19 as i64);
    let f8g5_19: i64 = (f8 as i64).wrapping_mul(g5_19 as i64);
    let f8g6_19: i64 = (f8 as i64).wrapping_mul(g6_19 as i64);
    let f8g7_19: i64 = (f8 as i64).wrapping_mul(g7_19 as i64);
    let f8g8_19: i64 = (f8 as i64).wrapping_mul(g8_19 as i64);
    let f8g9_19: i64 = (f8 as i64).wrapping_mul(g9_19 as i64);
    let f9g0: i64 = (f9 as i64).wrapping_mul(g0 as i64);
    let f9g1_38: i64 = (f9_2 as i64).wrapping_mul(g1_19 as i64);
    let f9g2_19: i64 = (f9 as i64).wrapping_mul(g2_19 as i64);
    let f9g3_38: i64 = (f9_2 as i64).wrapping_mul(g3_19 as i64);
    let f9g4_19: i64 = (f9 as i64).wrapping_mul(g4_19 as i64);
    let f9g5_38: i64 = (f9_2 as i64).wrapping_mul(g5_19 as i64);
    let f9g6_19: i64 = (f9 as i64).wrapping_mul(g6_19 as i64);
    let f9g7_38: i64 = (f9_2 as i64).wrapping_mul(g7_19 as i64);
    let f9g8_19: i64 = (f9 as i64).wrapping_mul(g8_19 as i64);
    let f9g9_38: i64 = (f9_2 as i64).wrapping_mul(g9_19 as i64);

    let mut h0: i64 = f0g0
        .wrapping_add(f1g9_38)
        .wrapping_add(f2g8_19)
        .wrapping_add(f3g7_38)
        .wrapping_add(f4g6_19)
        .wrapping_add(f5g5_38)
        .wrapping_add(f6g4_19)
        .wrapping_add(f7g3_38)
        .wrapping_add(f8g2_19)
        .wrapping_add(f9g1_38);
    let mut h1: i64 = f0g1
        .wrapping_add(f1g0)
        .wrapping_add(f2g9_19)
        .wrapping_add(f3g8_19)
        .wrapping_add(f4g7_19)
        .wrapping_add(f5g6_19)
        .wrapping_add(f6g5_19)
        .wrapping_add(f7g4_19)
        .wrapping_add(f8g3_19)
        .wrapping_add(f9g2_19);
    let mut h2: i64 = f0g2
        .wrapping_add(f1g1_2)
        .wrapping_add(f2g0)
        .wrapping_add(f3g9_38)
        .wrapping_add(f4g8_19)
        .wrapping_add(f5g7_38)
        .wrapping_add(f6g6_19)
        .wrapping_add(f7g5_38)
        .wrapping_add(f8g4_19)
        .wrapping_add(f9g3_38);
    let mut h3: i64 = f0g3
        .wrapping_add(f1g2)
        .wrapping_add(f2g1)
        .wrapping_add(f3g0)
        .wrapping_add(f4g9_19)
        .wrapping_add(f5g8_19)
        .wrapping_add(f6g7_19)
        .wrapping_add(f7g6_19)
        .wrapping_add(f8g5_19)
        .wrapping_add(f9g4_19);
    let mut h4: i64 = f0g4
        .wrapping_add(f1g3_2)
        .wrapping_add(f2g2)
        .wrapping_add(f3g1_2)
        .wrapping_add(f4g0)
        .wrapping_add(f5g9_38)
        .wrapping_add(f6g8_19)
        .wrapping_add(f7g7_38)
        .wrapping_add(f8g6_19)
        .wrapping_add(f9g5_38);
    let mut h5: i64 = f0g5
        .wrapping_add(f1g4)
        .wrapping_add(f2g3)
        .wrapping_add(f3g2)
        .wrapping_add(f4g1)
        .wrapping_add(f5g0)
        .wrapping_add(f6g9_19)
        .wrapping_add(f7g8_19)
        .wrapping_add(f8g7_19)
        .wrapping_add(f9g6_19);
    let mut h6: i64 = f0g6
        .wrapping_add(f1g5_2)
        .wrapping_add(f2g4)
        .wrapping_add(f3g3_2)
        .wrapping_add(f4g2)
        .wrapping_add(f5g1_2)
        .wrapping_add(f6g0)
        .wrapping_add(f7g9_38)
        .wrapping_add(f8g8_19)
        .wrapping_add(f9g7_38);
    let mut h7: i64 = f0g7
        .wrapping_add(f1g6)
        .wrapping_add(f2g5)
        .wrapping_add(f3g4)
        .wrapping_add(f4g3)
        .wrapping_add(f5g2)
        .wrapping_add(f6g1)
        .wrapping_add(f7g0)
        .wrapping_add(f8g9_19)
        .wrapping_add(f9g8_19);
    let mut h8: i64 = f0g8
        .wrapping_add(f1g7_2)
        .wrapping_add(f2g6)
        .wrapping_add(f3g5_2)
        .wrapping_add(f4g4)
        .wrapping_add(f5g3_2)
        .wrapping_add(f6g2)
        .wrapping_add(f7g1_2)
        .wrapping_add(f8g0)
        .wrapping_add(f9g9_38);
    let mut h9: i64 = f0g9
        .wrapping_add(f1g8)
        .wrapping_add(f2g7)
        .wrapping_add(f3g6)
        .wrapping_add(f4g5)
        .wrapping_add(f5g4)
        .wrapping_add(f6g3)
        .wrapping_add(f7g2)
        .wrapping_add(f8g1)
        .wrapping_add(f9g0);

    /*
     |h0| <= (1.65*1.65*2^52*(1+19+19+19+19)+1.65*1.65*2^50*(38+38+38+38+38))
     i.e. |h0| <= 1.4*2^60; narrower ranges for h2, h4, h6, h8
     |h1| <= (1.65*1.65*2^51*(1+1+19+19+19+19+19+19+19+19))
     i.e. |h1| <= 1.7*2^59; narrower ranges for h3, h5, h7, h9
     */

    let mut carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    let mut carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    /* |h0| <= 2^25 ; |h4| <= 2^25 ; |h1| <= 1.71*2^59 ; |h5| <= 1.71*2^59 */

    let carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    let carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));

    let carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    let carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));

    let carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    let carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    let carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    let carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

#[inline]
pub fn fe25519_mul(h: &mut Fe25519, f: &Fe25519, g: &Fe25519) {
    *h = fe_mul(f, g);
}

#[inline]
pub unsafe fn fe25519_mul_p(h: *mut i32, f: *const i32, g: *const i32) {
    let v = fe_mul(&load(f), &load(g));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  h = f * f   and   h = 2 * f * f                                           */
/* ------------------------------------------------------------------------- */

/// Shared body of `fe25519_sq` (`double_it == false`) and `fe25519_sq2`
/// (`double_it == true`).  The two C functions are character-for-character
/// identical except for the `h0 += h0; ...` block in `fe25519_sq2`.
#[inline(always)]
fn fe_sq_impl(f: &Fe25519, double_it: bool) -> Fe25519 {
    let f0: i32 = f[0];
    let f1: i32 = f[1];
    let f2: i32 = f[2];
    let f3: i32 = f[3];
    let f4: i32 = f[4];
    let f5: i32 = f[5];
    let f6: i32 = f[6];
    let f7: i32 = f[7];
    let f8: i32 = f[8];
    let f9: i32 = f[9];

    let f0_2: i32 = 2i32.wrapping_mul(f0);
    let f1_2: i32 = 2i32.wrapping_mul(f1);
    let f2_2: i32 = 2i32.wrapping_mul(f2);
    let f3_2: i32 = 2i32.wrapping_mul(f3);
    let f4_2: i32 = 2i32.wrapping_mul(f4);
    let f5_2: i32 = 2i32.wrapping_mul(f5);
    let f6_2: i32 = 2i32.wrapping_mul(f6);
    let f7_2: i32 = 2i32.wrapping_mul(f7);
    let f5_38: i32 = 38i32.wrapping_mul(f5); /* 1.959375*2^30 */
    let f6_19: i32 = 19i32.wrapping_mul(f6); /* 1.959375*2^30 */
    let f7_38: i32 = 38i32.wrapping_mul(f7); /* 1.959375*2^30 */
    let f8_19: i32 = 19i32.wrapping_mul(f8); /* 1.959375*2^30 */
    let f9_38: i32 = 38i32.wrapping_mul(f9); /* 1.959375*2^30 */

    let f0f0: i64 = (f0 as i64).wrapping_mul(f0 as i64);
    let f0f1_2: i64 = (f0_2 as i64).wrapping_mul(f1 as i64);
    let f0f2_2: i64 = (f0_2 as i64).wrapping_mul(f2 as i64);
    let f0f3_2: i64 = (f0_2 as i64).wrapping_mul(f3 as i64);
    let f0f4_2: i64 = (f0_2 as i64).wrapping_mul(f4 as i64);
    let f0f5_2: i64 = (f0_2 as i64).wrapping_mul(f5 as i64);
    let f0f6_2: i64 = (f0_2 as i64).wrapping_mul(f6 as i64);
    let f0f7_2: i64 = (f0_2 as i64).wrapping_mul(f7 as i64);
    let f0f8_2: i64 = (f0_2 as i64).wrapping_mul(f8 as i64);
    let f0f9_2: i64 = (f0_2 as i64).wrapping_mul(f9 as i64);
    let f1f1_2: i64 = (f1_2 as i64).wrapping_mul(f1 as i64);
    let f1f2_2: i64 = (f1_2 as i64).wrapping_mul(f2 as i64);
    let f1f3_4: i64 = (f1_2 as i64).wrapping_mul(f3_2 as i64);
    let f1f4_2: i64 = (f1_2 as i64).wrapping_mul(f4 as i64);
    let f1f5_4: i64 = (f1_2 as i64).wrapping_mul(f5_2 as i64);
    let f1f6_2: i64 = (f1_2 as i64).wrapping_mul(f6 as i64);
    let f1f7_4: i64 = (f1_2 as i64).wrapping_mul(f7_2 as i64);
    let f1f8_2: i64 = (f1_2 as i64).wrapping_mul(f8 as i64);
    let f1f9_76: i64 = (f1_2 as i64).wrapping_mul(f9_38 as i64);
    let f2f2: i64 = (f2 as i64).wrapping_mul(f2 as i64);
    let f2f3_2: i64 = (f2_2 as i64).wrapping_mul(f3 as i64);
    let f2f4_2: i64 = (f2_2 as i64).wrapping_mul(f4 as i64);
    let f2f5_2: i64 = (f2_2 as i64).wrapping_mul(f5 as i64);
    let f2f6_2: i64 = (f2_2 as i64).wrapping_mul(f6 as i64);
    let f2f7_2: i64 = (f2_2 as i64).wrapping_mul(f7 as i64);
    let f2f8_38: i64 = (f2_2 as i64).wrapping_mul(f8_19 as i64);
    let f2f9_38: i64 = (f2 as i64).wrapping_mul(f9_38 as i64);
    let f3f3_2: i64 = (f3_2 as i64).wrapping_mul(f3 as i64);
    let f3f4_2: i64 = (f3_2 as i64).wrapping_mul(f4 as i64);
    let f3f5_4: i64 = (f3_2 as i64).wrapping_mul(f5_2 as i64);
    let f3f6_2: i64 = (f3_2 as i64).wrapping_mul(f6 as i64);
    let f3f7_76: i64 = (f3_2 as i64).wrapping_mul(f7_38 as i64);
    let f3f8_38: i64 = (f3_2 as i64).wrapping_mul(f8_19 as i64);
    let f3f9_76: i64 = (f3_2 as i64).wrapping_mul(f9_38 as i64);
    let f4f4: i64 = (f4 as i64).wrapping_mul(f4 as i64);
    let f4f5_2: i64 = (f4_2 as i64).wrapping_mul(f5 as i64);
    let f4f6_38: i64 = (f4_2 as i64).wrapping_mul(f6_19 as i64);
    let f4f7_38: i64 = (f4 as i64).wrapping_mul(f7_38 as i64);
    let f4f8_38: i64 = (f4_2 as i64).wrapping_mul(f8_19 as i64);
    let f4f9_38: i64 = (f4 as i64).wrapping_mul(f9_38 as i64);
    let f5f5_38: i64 = (f5 as i64).wrapping_mul(f5_38 as i64);
    let f5f6_38: i64 = (f5_2 as i64).wrapping_mul(f6_19 as i64);
    let f5f7_76: i64 = (f5_2 as i64).wrapping_mul(f7_38 as i64);
    let f5f8_38: i64 = (f5_2 as i64).wrapping_mul(f8_19 as i64);
    let f5f9_76: i64 = (f5_2 as i64).wrapping_mul(f9_38 as i64);
    let f6f6_19: i64 = (f6 as i64).wrapping_mul(f6_19 as i64);
    let f6f7_38: i64 = (f6 as i64).wrapping_mul(f7_38 as i64);
    let f6f8_38: i64 = (f6_2 as i64).wrapping_mul(f8_19 as i64);
    let f6f9_38: i64 = (f6 as i64).wrapping_mul(f9_38 as i64);
    let f7f7_38: i64 = (f7 as i64).wrapping_mul(f7_38 as i64);
    let f7f8_38: i64 = (f7_2 as i64).wrapping_mul(f8_19 as i64);
    let f7f9_76: i64 = (f7_2 as i64).wrapping_mul(f9_38 as i64);
    let f8f8_19: i64 = (f8 as i64).wrapping_mul(f8_19 as i64);
    let f8f9_38: i64 = (f8 as i64).wrapping_mul(f9_38 as i64);
    let f9f9_38: i64 = (f9 as i64).wrapping_mul(f9_38 as i64);

    let mut h0: i64 = f0f0
        .wrapping_add(f1f9_76)
        .wrapping_add(f2f8_38)
        .wrapping_add(f3f7_76)
        .wrapping_add(f4f6_38)
        .wrapping_add(f5f5_38);
    let mut h1: i64 = f0f1_2
        .wrapping_add(f2f9_38)
        .wrapping_add(f3f8_38)
        .wrapping_add(f4f7_38)
        .wrapping_add(f5f6_38);
    let mut h2: i64 = f0f2_2
        .wrapping_add(f1f1_2)
        .wrapping_add(f3f9_76)
        .wrapping_add(f4f8_38)
        .wrapping_add(f5f7_76)
        .wrapping_add(f6f6_19);
    let mut h3: i64 = f0f3_2
        .wrapping_add(f1f2_2)
        .wrapping_add(f4f9_38)
        .wrapping_add(f5f8_38)
        .wrapping_add(f6f7_38);
    let mut h4: i64 = f0f4_2
        .wrapping_add(f1f3_4)
        .wrapping_add(f2f2)
        .wrapping_add(f5f9_76)
        .wrapping_add(f6f8_38)
        .wrapping_add(f7f7_38);
    let mut h5: i64 = f0f5_2
        .wrapping_add(f1f4_2)
        .wrapping_add(f2f3_2)
        .wrapping_add(f6f9_38)
        .wrapping_add(f7f8_38);
    let mut h6: i64 = f0f6_2
        .wrapping_add(f1f5_4)
        .wrapping_add(f2f4_2)
        .wrapping_add(f3f3_2)
        .wrapping_add(f7f9_76)
        .wrapping_add(f8f8_19);
    let mut h7: i64 = f0f7_2
        .wrapping_add(f1f6_2)
        .wrapping_add(f2f5_2)
        .wrapping_add(f3f4_2)
        .wrapping_add(f8f9_38);
    let mut h8: i64 = f0f8_2
        .wrapping_add(f1f7_4)
        .wrapping_add(f2f6_2)
        .wrapping_add(f3f5_4)
        .wrapping_add(f4f4)
        .wrapping_add(f9f9_38);
    let mut h9: i64 = f0f9_2
        .wrapping_add(f1f8_2)
        .wrapping_add(f2f7_2)
        .wrapping_add(f3f6_2)
        .wrapping_add(f4f5_2);

    if double_it {
        /* fe25519_sq2 only */
        h0 = h0.wrapping_add(h0);
        h1 = h1.wrapping_add(h1);
        h2 = h2.wrapping_add(h2);
        h3 = h3.wrapping_add(h3);
        h4 = h4.wrapping_add(h4);
        h5 = h5.wrapping_add(h5);
        h6 = h6.wrapping_add(h6);
        h7 = h7.wrapping_add(h7);
        h8 = h8.wrapping_add(h8);
        h9 = h9.wrapping_add(h9);
    }

    let mut carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    let mut carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));

    let carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    let carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));

    let carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    let carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));

    let carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    let carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    let carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    let carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

/// `static void fe25519_sq(fe25519 h, const fe25519 f)`
#[inline]
pub fn fe_sq(f: &Fe25519) -> Fe25519 {
    fe_sq_impl(f, false)
}

#[inline]
pub fn fe25519_sq(h: &mut Fe25519, f: &Fe25519) {
    *h = fe_sq(f);
}

#[inline]
pub unsafe fn fe25519_sq_p(h: *mut i32, f: *const i32) {
    let v = fe_sq(&load(f));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/// `static void fe25519_sq2(fe25519 h, const fe25519 f)`  (h = 2*f*f)
#[inline]
pub fn fe_sq2(f: &Fe25519) -> Fe25519 {
    fe_sq_impl(f, true)
}

#[inline]
pub fn fe25519_sq2(h: &mut Fe25519, f: &Fe25519) {
    *h = fe_sq2(f);
}

#[inline]
pub unsafe fn fe25519_sq2_p(h: *mut i32, f: *const i32) {
    let v = fe_sq2(&load(f));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  h = f * n   (n a small unsigned scalar)                                   */
/* ------------------------------------------------------------------------- */

/// `static inline void fe25519_mul32(fe25519 h, const fe25519 f, uint32_t n)`
pub fn fe_mul32(f: &Fe25519, n: u32) -> Fe25519 {
    let sn: i64 = n as i64;
    let f0: i32 = f[0];
    let f1: i32 = f[1];
    let f2: i32 = f[2];
    let f3: i32 = f[3];
    let f4: i32 = f[4];
    let f5: i32 = f[5];
    let f6: i32 = f[6];
    let f7: i32 = f[7];
    let f8: i32 = f[8];
    let f9: i32 = f[9];
    let mut h0: i64 = (f0 as i64).wrapping_mul(sn);
    let mut h1: i64 = (f1 as i64).wrapping_mul(sn);
    let mut h2: i64 = (f2 as i64).wrapping_mul(sn);
    let mut h3: i64 = (f3 as i64).wrapping_mul(sn);
    let mut h4: i64 = (f4 as i64).wrapping_mul(sn);
    let mut h5: i64 = (f5 as i64).wrapping_mul(sn);
    let mut h6: i64 = (f6 as i64).wrapping_mul(sn);
    let mut h7: i64 = (f7 as i64).wrapping_mul(sn);
    let mut h8: i64 = (f8 as i64).wrapping_mul(sn);
    let mut h9: i64 = (f9 as i64).wrapping_mul(sn);

    let carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));
    let carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    let carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    let carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));
    let carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    let carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    let carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    let carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    let carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));
    let carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

#[inline]
pub fn fe25519_mul32(h: &mut Fe25519, f: &Fe25519, n: u32) {
    *h = fe_mul32(f, n);
}

#[inline]
pub unsafe fn fe25519_mul32_p(h: *mut i32, f: *const i32, n: u32) {
    let v = fe_mul32(&load(f), n);
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/* ------------------------------------------------------------------------- */
/*  serialisation                                                             */
/* ------------------------------------------------------------------------- */

/// `void fe25519_frombytes(fe25519 h, const unsigned char *s)` — ignores the
/// top bit of `s`.
pub unsafe fn fe_frombytes_ptr(s: *const u8) -> Fe25519 {
    let mut h0: i64 = load_4(s) as i64;
    let mut h1: i64 = ((load_3(s.add(4))) << 6) as i64;
    let mut h2: i64 = ((load_3(s.add(7))) << 5) as i64;
    let mut h3: i64 = ((load_3(s.add(10))) << 3) as i64;
    let mut h4: i64 = ((load_3(s.add(13))) << 2) as i64;
    let mut h5: i64 = load_4(s.add(16)) as i64;
    let mut h6: i64 = ((load_3(s.add(20))) << 7) as i64;
    let mut h7: i64 = ((load_3(s.add(23))) << 5) as i64;
    let mut h8: i64 = ((load_3(s.add(26))) << 4) as i64;
    let mut h9: i64 = ((load_3(s.add(29)) & 8388607) << 2) as i64;

    let carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));
    let carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    let carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    let carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));
    let carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    let carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    let carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    let carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    let carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));
    let carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    [
        h0 as i32, h1 as i32, h2 as i32, h3 as i32, h4 as i32, h5 as i32, h6 as i32, h7 as i32,
        h8 as i32, h9 as i32,
    ]
}

#[inline]
pub fn fe_frombytes(s: &[u8; 32]) -> Fe25519 {
    unsafe { fe_frombytes_ptr(s.as_ptr()) }
}

#[inline]
pub fn fe25519_frombytes(h: &mut Fe25519, s: &[u8; 32]) {
    *h = fe_frombytes(s);
}

#[inline]
pub unsafe fn fe25519_frombytes_p(h: *mut i32, s: *const u8) {
    let v = fe_frombytes_ptr(s);
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/// `void fe25519_frombytes(fe25519 h, const unsigned char *s)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_frombytes(h: *mut i32, s: *const u8) {
    fe25519_frombytes_p(h, s);
}

/// `static void fe25519_reduce(fe25519 h, const fe25519 f)`
///
/// Note the exact C typing: `q = (19 * h9 + ((uint32_t) 1L << 24)) >> 25;` mixes
/// `int32_t` with `uint32_t`, so the addition and the shift are performed on
/// **unsigned int** (logical shift).  All later `q = (hN + q) >> k;` steps are
/// plain signed `int` (arithmetic shift).  `h0 -= carryN * ((uint32_t) 1L << k)`
/// is likewise an unsigned multiply/subtract, which is bit-identical to
/// `i32::wrapping_sub(i32::wrapping_mul(...))`.
pub fn fe_reduce(f: &Fe25519) -> Fe25519 {
    let mut h0: i32 = f[0];
    let mut h1: i32 = f[1];
    let mut h2: i32 = f[2];
    let mut h3: i32 = f[3];
    let mut h4: i32 = f[4];
    let mut h5: i32 = f[5];
    let mut h6: i32 = f[6];
    let mut h7: i32 = f[7];
    let mut h8: i32 = f[8];
    let mut h9: i32 = f[9];

    let mut q: i32;

    /* unsigned arithmetic + logical shift, exactly as in the C */
    q = (((19i32.wrapping_mul(h9)) as u32).wrapping_add(1u32 << 24) >> 25) as i32;
    q = h0.wrapping_add(q) >> 26;
    q = h1.wrapping_add(q) >> 25;
    q = h2.wrapping_add(q) >> 26;
    q = h3.wrapping_add(q) >> 25;
    q = h4.wrapping_add(q) >> 26;
    q = h5.wrapping_add(q) >> 25;
    q = h6.wrapping_add(q) >> 26;
    q = h7.wrapping_add(q) >> 25;
    q = h8.wrapping_add(q) >> 26;
    q = h9.wrapping_add(q) >> 25;

    /* Goal: Output h-(2^255-19)q, which is between 0 and 2^255-20. */
    h0 = h0.wrapping_add(19i32.wrapping_mul(q));
    /* Goal: Output h-2^255 q, which is between 0 and 2^255-20. */

    let carry0 = h0 >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i32 << 26));
    let carry1 = h1 >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i32 << 25));
    let carry2 = h2 >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i32 << 26));
    let carry3 = h3 >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i32 << 25));
    let carry4 = h4 >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i32 << 26));
    let carry5 = h5 >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i32 << 25));
    let carry6 = h6 >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i32 << 26));
    let carry7 = h7 >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i32 << 25));
    let carry8 = h8 >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i32 << 26));
    let carry9 = h9 >> 25;
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i32 << 25));

    [h0, h1, h2, h3, h4, h5, h6, h7, h8, h9]
}

#[inline]
pub fn fe25519_reduce(h: &mut Fe25519, f: &Fe25519) {
    *h = fe_reduce(f);
}

#[inline]
pub unsafe fn fe25519_reduce_p(h: *mut i32, f: *const i32) {
    let v = fe_reduce(&load(f));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/// `void fe25519_tobytes(unsigned char *s, const fe25519 h)`
pub fn fe_tobytes(h: &Fe25519) -> [u8; 32] {
    let t = fe_reduce(h);
    let mut s = [0u8; 32];

    s[0] = (t[0] >> 0) as u8;
    s[1] = (t[0] >> 8) as u8;
    s[2] = (t[0] >> 16) as u8;
    s[3] = (((t[0] >> 24) as u32) | (t[1] as u32).wrapping_mul(1u32 << 2)) as u8;
    s[4] = (t[1] >> 6) as u8;
    s[5] = (t[1] >> 14) as u8;
    s[6] = (((t[1] >> 22) as u32) | (t[2] as u32).wrapping_mul(1u32 << 3)) as u8;
    s[7] = (t[2] >> 5) as u8;
    s[8] = (t[2] >> 13) as u8;
    s[9] = (((t[2] >> 21) as u32) | (t[3] as u32).wrapping_mul(1u32 << 5)) as u8;
    s[10] = (t[3] >> 3) as u8;
    s[11] = (t[3] >> 11) as u8;
    s[12] = (((t[3] >> 19) as u32) | (t[4] as u32).wrapping_mul(1u32 << 6)) as u8;
    s[13] = (t[4] >> 2) as u8;
    s[14] = (t[4] >> 10) as u8;
    s[15] = (t[4] >> 18) as u8;
    s[16] = (t[5] >> 0) as u8;
    s[17] = (t[5] >> 8) as u8;
    s[18] = (t[5] >> 16) as u8;
    s[19] = (((t[5] >> 24) as u32) | (t[6] as u32).wrapping_mul(1u32 << 1)) as u8;
    s[20] = (t[6] >> 7) as u8;
    s[21] = (t[6] >> 15) as u8;
    s[22] = (((t[6] >> 23) as u32) | (t[7] as u32).wrapping_mul(1u32 << 3)) as u8;
    s[23] = (t[7] >> 5) as u8;
    s[24] = (t[7] >> 13) as u8;
    s[25] = (((t[7] >> 21) as u32) | (t[8] as u32).wrapping_mul(1u32 << 4)) as u8;
    s[26] = (t[8] >> 4) as u8;
    s[27] = (t[8] >> 12) as u8;
    s[28] = (((t[8] >> 20) as u32) | (t[9] as u32).wrapping_mul(1u32 << 6)) as u8;
    s[29] = (t[9] >> 2) as u8;
    s[30] = (t[9] >> 10) as u8;
    s[31] = (t[9] >> 18) as u8;

    s
}

#[inline]
pub fn fe25519_tobytes(s: &mut [u8; 32], h: &Fe25519) {
    *s = fe_tobytes(h);
}

#[inline]
pub unsafe fn fe25519_tobytes_p(s: *mut u8, h: *const i32) {
    let v = fe_tobytes(&load(h));
    core::ptr::copy_nonoverlapping(v.as_ptr(), s, 32);
}

/// `void fe25519_tobytes(unsigned char *s, const fe25519 h)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32) {
    fe25519_tobytes_p(s, h);
}

/* ------------------------------------------------------------------------- */
/*  ed25519_ref10.c lines 52..256                                             */
/* ------------------------------------------------------------------------- */

/// `static inline void fe25519_sqmul(fe25519 s, const int n, const fe25519 a)`
#[inline]
pub fn fe_sqmul(s: &Fe25519, n: i32, a: &Fe25519) -> Fe25519 {
    let mut s = *s;
    let mut i: i32 = 0;

    while i < n {
        s = fe_sq(&s);
        i += 1;
    }
    fe_mul(&s, a)
}

#[inline]
pub fn fe25519_sqmul(s: &mut Fe25519, n: i32, a: &Fe25519) {
    let v = fe_sqmul(s, n, a);
    *s = v;
}

#[inline]
pub unsafe fn fe25519_sqmul_p(s: *mut i32, n: i32, a: *const i32) {
    let v = fe_sqmul(&load(s), n, &load(a));
    core::ptr::copy_nonoverlapping(v.as_ptr(), s, 10);
}

/// `void fe25519_invert(fe25519 out, const fe25519 z)` — inversion; sets `out`
/// to 0 if `z == 0`.
pub fn fe_invert(z: &Fe25519) -> Fe25519 {
    let mut t0: Fe25519;
    let mut t1: Fe25519;
    let mut t2: Fe25519;
    let mut t3: Fe25519;
    let mut i: i32;

    t0 = fe_sq(z);
    t1 = fe_sq(&t0);
    t1 = fe_sq(&t1);
    t1 = fe_mul(z, &t1);
    t0 = fe_mul(&t0, &t1);
    t2 = fe_sq(&t0);
    t1 = fe_mul(&t1, &t2);
    t2 = fe_sq(&t1);
    i = 1;
    while i < 5 {
        t2 = fe_sq(&t2);
        i += 1;
    }
    t1 = fe_mul(&t2, &t1);
    t2 = fe_sq(&t1);
    i = 1;
    while i < 10 {
        t2 = fe_sq(&t2);
        i += 1;
    }
    t2 = fe_mul(&t2, &t1);
    t3 = fe_sq(&t2);
    i = 1;
    while i < 20 {
        t3 = fe_sq(&t3);
        i += 1;
    }
    t2 = fe_mul(&t3, &t2);
    i = 1;
    while i < 11 {
        t2 = fe_sq(&t2);
        i += 1;
    }
    t1 = fe_mul(&t2, &t1);
    t2 = fe_sq(&t1);
    i = 1;
    while i < 50 {
        t2 = fe_sq(&t2);
        i += 1;
    }
    t2 = fe_mul(&t2, &t1);
    t3 = fe_sq(&t2);
    i = 1;
    while i < 100 {
        t3 = fe_sq(&t3);
        i += 1;
    }
    t2 = fe_mul(&t3, &t2);
    i = 1;
    while i < 51 {
        t2 = fe_sq(&t2);
        i += 1;
    }
    t1 = fe_mul(&t2, &t1);
    i = 1;
    while i < 6 {
        t1 = fe_sq(&t1);
        i += 1;
    }
    fe_mul(&t1, &t0)
}

#[inline]
pub fn fe25519_invert(out: &mut Fe25519, z: &Fe25519) {
    *out = fe_invert(z);
}

#[inline]
pub unsafe fn fe25519_invert_p(out: *mut i32, z: *const i32) {
    let v = fe_invert(&load(z));
    core::ptr::copy_nonoverlapping(v.as_ptr(), out, 10);
}

/// `void fe25519_invert(fe25519 out, const fe25519 z)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_invert(out: *mut i32, z: *const i32) {
    fe25519_invert_p(out, z);
}

/// `static void fe25519_pow22523(fe25519 out, const fe25519 z)`
///
/// returns `z^((p-5)/8) = z^(2^252-3)`
pub fn fe_pow22523(z: &Fe25519) -> Fe25519 {
    let mut t0: Fe25519;
    let mut t1: Fe25519;
    let mut t2: Fe25519;
    let mut i: i32;

    t0 = fe_sq(z);
    t1 = fe_sq(&t0);
    t1 = fe_sq(&t1);
    t1 = fe_mul(z, &t1);
    t0 = fe_mul(&t0, &t1);
    t0 = fe_sq(&t0);
    t0 = fe_mul(&t1, &t0);
    t1 = fe_sq(&t0);
    i = 1;
    while i < 5 {
        t1 = fe_sq(&t1);
        i += 1;
    }
    t0 = fe_mul(&t1, &t0);
    t1 = fe_sq(&t0);
    i = 1;
    while i < 10 {
        t1 = fe_sq(&t1);
        i += 1;
    }
    t1 = fe_mul(&t1, &t0);
    t2 = fe_sq(&t1);
    i = 1;
    while i < 20 {
        t2 = fe_sq(&t2);
        i += 1;
    }
    t1 = fe_mul(&t2, &t1);
    i = 1;
    while i < 11 {
        t1 = fe_sq(&t1);
        i += 1;
    }
    t0 = fe_mul(&t1, &t0);
    t1 = fe_sq(&t0);
    i = 1;
    while i < 50 {
        t1 = fe_sq(&t1);
        i += 1;
    }
    t1 = fe_mul(&t1, &t0);
    t2 = fe_sq(&t1);
    i = 1;
    while i < 100 {
        t2 = fe_sq(&t2);
        i += 1;
    }
    t1 = fe_mul(&t2, &t1);
    i = 1;
    while i < 51 {
        t1 = fe_sq(&t1);
        i += 1;
    }
    t0 = fe_mul(&t1, &t0);
    t0 = fe_sq(&t0);
    t0 = fe_sq(&t0);
    fe_mul(&t0, z)
}

#[inline]
pub fn fe25519_pow22523(out: &mut Fe25519, z: &Fe25519) {
    *out = fe_pow22523(z);
}

#[inline]
pub unsafe fn fe25519_pow22523_p(out: *mut i32, z: *const i32) {
    let v = fe_pow22523(&load(z));
    core::ptr::copy_nonoverlapping(v.as_ptr(), out, 10);
}

/// `static inline void fe25519_cneg(fe25519 h, unsigned int b)`
#[inline]
pub fn fe_cneg(h: &Fe25519, b: u32) -> Fe25519 {
    let negf = fe_neg(h);

    fe_cmov(h, &negf, b)
}

#[inline]
pub fn fe25519_cneg(h: &mut Fe25519, b: u32) {
    let v = fe_cneg(h, b);
    *h = v;
}

#[inline]
pub unsafe fn fe25519_cneg_p(h: *mut i32, b: u32) {
    let v = fe_cneg(&load(h), b);
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/// `static inline void fe25519_abs(fe25519 h)`
#[inline]
pub fn fe_abs(h: &Fe25519) -> Fe25519 {
    fe_cneg(h, fe25519_isnegative(h) as u32)
}

#[inline]
pub fn fe25519_abs(h: &mut Fe25519) {
    let v = fe_abs(h);
    *h = v;
}

#[inline]
pub unsafe fn fe25519_abs_p(h: *mut i32) {
    let v = fe_abs(&load(h));
    core::ptr::copy_nonoverlapping(v.as_ptr(), h, 10);
}

/// `static void fe25519_unchecked_sqrt(fe25519 x, const fe25519 x2)`
pub fn fe_unchecked_sqrt(x2: &Fe25519) -> Fe25519 {
    let p_root: Fe25519;
    let m_root: Fe25519;
    let m_root2: Fe25519;
    let e: Fe25519;
    let mut x: Fe25519;

    let e0 = fe_pow22523(x2);
    p_root = fe_mul(&e0, x2);
    m_root = fe_mul(&p_root, &FE25519_SQRTM1);
    m_root2 = fe_sq(&m_root);
    e = fe_sub(x2, &m_root2);
    x = p_root;
    x = fe_cmov(&x, &m_root, fe25519_iszero(&e) as u32);

    x
}

#[inline]
pub fn fe25519_unchecked_sqrt(x: &mut Fe25519, x2: &Fe25519) {
    *x = fe_unchecked_sqrt(x2);
}

#[inline]
pub unsafe fn fe25519_unchecked_sqrt_p(x: *mut i32, x2: *const i32) {
    let v = fe_unchecked_sqrt(&load(x2));
    core::ptr::copy_nonoverlapping(v.as_ptr(), x, 10);
}

/// `static int fe25519_sqrt(fe25519 x, const fe25519 x2)`
///
/// Returns 0 on success, -1 if `x2` is not a square.
pub fn fe_sqrt(x2: &Fe25519) -> (Fe25519, i32) {
    let x2_copy: Fe25519 = *x2;
    let x = fe_unchecked_sqrt(x2);
    let mut check = fe_sq(&x);
    check = fe_sub(&check, &x2_copy);

    (x, fe25519_iszero(&check) - 1)
}

#[inline]
pub fn fe25519_sqrt(x: &mut Fe25519, x2: &Fe25519) -> i32 {
    let (v, r) = fe_sqrt(x2);
    *x = v;
    r
}

#[inline]
pub unsafe fn fe25519_sqrt_p(x: *mut i32, x2: *const i32) -> i32 {
    let (v, r) = fe_sqrt(&load(x2));
    core::ptr::copy_nonoverlapping(v.as_ptr(), x, 10);
    r
}

/// `static int fe25519_notsquare(const fe25519 x)`
pub fn fe25519_notsquare(x: &Fe25519) -> i32 {
    let _10: Fe25519;
    let _11: Fe25519;
    let mut _1100: Fe25519;
    let _1111: Fe25519;
    let mut _11110000: Fe25519;
    let _11111111: Fe25519;
    let mut t: Fe25519;
    let u: Fe25519;
    let mut v: Fe25519;

    /* Jacobi symbol - x^((p-1)/2) */
    _10 = fe_mul(x, x);
    _11 = fe_mul(x, &_10);
    _1100 = fe_sq(&_11);
    _1100 = fe_sq(&_1100);
    _1111 = fe_mul(&_11, &_1100);
    _11110000 = fe_sq(&_1111);
    _11110000 = fe_sq(&_11110000);
    _11110000 = fe_sq(&_11110000);
    _11110000 = fe_sq(&_11110000);
    _11111111 = fe_mul(&_1111, &_11110000);
    t = _11111111;
    t = fe_sqmul(&t, 2, &_11);
    u = t;
    t = fe_sqmul(&t, 10, &u);
    t = fe_sqmul(&t, 10, &u);
    v = t;
    t = fe_sqmul(&t, 30, &v);
    v = t;
    t = fe_sqmul(&t, 60, &v);
    v = t;
    t = fe_sqmul(&t, 120, &v);
    t = fe_sqmul(&t, 10, &u);
    t = fe_sqmul(&t, 3, &_11);
    t = fe_sq(&t);

    let s = fe_tobytes(&t);

    (s[1] & 1) as i32
}

#[inline]
pub unsafe fn fe25519_notsquare_p(x: *const i32) -> i32 {
    fe25519_notsquare(&load(x))
}

/* ------------------------------------------------------------------------- */
/*  internal helper                                                           */
/* ------------------------------------------------------------------------- */

/// Read 10 limbs out of a raw pointer into a value.  Used by the `_p` variants
/// so that input/output overlap is always safe (the C loads every limb into a
/// local before it stores anything, too).
#[inline(always)]
unsafe fn load(p: *const i32) -> Fe25519 {
    [
        *p.add(0),
        *p.add(1),
        *p.add(2),
        *p.add(3),
        *p.add(4),
        *p.add(5),
        *p.add(6),
        *p.add(7),
        *p.add(8),
        *p.add(9),
    ]
}
