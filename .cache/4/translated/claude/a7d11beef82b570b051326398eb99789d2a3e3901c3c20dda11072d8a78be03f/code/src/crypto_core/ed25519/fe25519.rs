//! `fe25519` field arithmetic for `crypto_core/ed25519/ref10`
//! (the `fe_25_5` representation: `typedef int32_t fe25519[10]`).
//!
//! Translation of
//!   * `include/sodium/private/ed25519_ref10_fe_25_5.h`
//!   * `crypto_core/ed25519/ref10/fe_25_5/fe.h`
//!     (`fe25519_frombytes`, `fe25519_reduce`, `fe25519_tobytes`)
//!   * the `fe25519_*` helpers that live at the top of
//!     `crypto_core/ed25519/ref10/ed25519_ref10.c`
//!     (`load_3`, `load_4`, `fe25519_sqmul`, `fe25519_invert`,
//!     `fe25519_pow22523`, `fe25519_cneg`, `fe25519_abs`,
//!     `fe25519_unchecked_sqrt`, `fe25519_sqrt`, `fe25519_notsquare`).
//!
//! `HAVE_TI_MODE` is **undefined** in the reference build, so this is the code
//! path that is actually used; the `fe_51` variants are not translated.
//!
//! # Calling convention
//!
//! The C code freely aliases arguments (`fe25519_mul(h, f, h)`,
//! `fe25519_sq(t1, t1)`, ...).  To keep that legal in safe Rust every function
//! that has a separate output parameter takes its **inputs by value**
//! (`Fe25519` is `Copy` and only 40 bytes):
//!
//! ```ignore
//! pub fn fe25519_mul(h: &mut Fe25519, f: Fe25519, g: Fe25519);
//! ```
//!
//! so `fe25519_mul(&mut h, f, h)` is written `fe25519_mul(&mut h, f, h_copy)`
//! (the copy is implicit because the value is read before the call).  For
//! in-place chains the value-returning helpers [`fe_sq`] and [`fe_mul`] are
//! provided, which mirror the C source one line at a time.
//!
//! The two exceptions are `fe25519_cmov` (`f` is in/out, `g` is read-only) and
//! `fe25519_cswap` (two `&mut` operands).
//!
//! Only three symbols are exported with C linkage (`private/quirks.h` renames
//! them): `_sodium_fe25519_frombytes`, `_sodium_fe25519_tobytes` and
//! `_sodium_fe25519_invert`.
//!
//! Every intermediate integer type of the C code (`int32_t` / `int64_t` /
//! `uint32_t` / `uint64_t`) is reproduced exactly; all C truncations and
//! wrap-arounds are spelled out with `as`/`wrapping_*`.

use crate::crypto_core::ed25519::ref10_tables::FE25519_SQRTM1;
use crate::crypto_core::ed25519::types::Fe25519;

/* ------------------------------------------------------------------ */
/* load_3 / load_4 (ed25519_ref10.c)                                   */
/* ------------------------------------------------------------------ */

/// C:
/// ```c
/// static inline uint64_t load_3(const unsigned char *in);
/// ```
#[inline]
pub(crate) fn load_3(input: &[u8]) -> u64 {
    let mut result: u64;

    result = input[0] as u64;
    result |= (input[1] as u64) << 8;
    result |= (input[2] as u64) << 16;

    result
}

/// C:
/// ```c
/// static inline uint64_t load_4(const unsigned char *in);
/// ```
#[inline]
pub(crate) fn load_4(input: &[u8]) -> u64 {
    let mut result: u64;

    result = input[0] as u64;
    result |= (input[1] as u64) << 8;
    result |= (input[2] as u64) << 16;
    result |= (input[3] as u64) << 24;

    result
}

/* ------------------------------------------------------------------ */
/* ed25519_ref10_fe_25_5.h                                             */
/* ------------------------------------------------------------------ */

/// `h = 0`
#[inline]
pub fn fe25519_0(h: &mut Fe25519) {
    h.0 = [0i32; 10];
}

/// `h = 1`
#[inline]
pub fn fe25519_1(h: &mut Fe25519) {
    h[0] = 1;
    h[1] = 0;
    h.0[2..10].fill(0);
}

/// `h = f + g`
///
/// Preconditions: `|f|`, `|g|` bounded by 1.1*2^25, 1.1*2^24, ...
/// Postconditions: `|h|` bounded by 1.1*2^26, 1.1*2^25, ...
#[inline]
pub fn fe25519_add(h: &mut Fe25519, f: Fe25519, g: Fe25519) {
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

    h[0] = h0;
    h[1] = h1;
    h[2] = h2;
    h[3] = h3;
    h[4] = h4;
    h[5] = h5;
    h[6] = h6;
    h[7] = h7;
    h[8] = h8;
    h[9] = h9;
}

/// `h = f - g`
#[inline]
pub fn fe25519_sub(h: &mut Fe25519, f: Fe25519, g: Fe25519) {
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

    h[0] = h0;
    h[1] = h1;
    h[2] = h2;
    h[3] = h3;
    h[4] = h4;
    h[5] = h5;
    h[6] = h6;
    h[7] = h7;
    h[8] = h8;
    h[9] = h9;
}

/// `h = -f`
#[inline]
pub fn fe25519_neg(h: &mut Fe25519, f: Fe25519) {
    let h0 = 0i32.wrapping_sub(f[0]);
    let h1 = 0i32.wrapping_sub(f[1]);
    let h2 = 0i32.wrapping_sub(f[2]);
    let h3 = 0i32.wrapping_sub(f[3]);
    let h4 = 0i32.wrapping_sub(f[4]);
    let h5 = 0i32.wrapping_sub(f[5]);
    let h6 = 0i32.wrapping_sub(f[6]);
    let h7 = 0i32.wrapping_sub(f[7]);
    let h8 = 0i32.wrapping_sub(f[8]);
    let h9 = 0i32.wrapping_sub(f[9]);

    h[0] = h0;
    h[1] = h1;
    h[2] = h2;
    h[3] = h3;
    h[4] = h4;
    h[5] = h5;
    h[6] = h6;
    h[7] = h7;
    h[8] = h8;
    h[9] = h9;
}

/// Replace `(f,g)` with `(g,g)` if `b == 1`; leave them alone if `b == 0`.
///
/// Precondition: `b` in `{0,1}`.
///
/// C: `uint32_t mask = (uint32_t) (-(int32_t) b);`
pub fn fe25519_cmov(f: &mut Fe25519, g: Fe25519, b: u32) {
    let mask: u32 = (0i32.wrapping_sub(b as i32)) as u32;

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

    /* `int32_t &= uint32_t` in C: the AND happens on the identical bit
     * patterns, so masking with `mask as i32` is equivalent. */
    let m = mask as i32;
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

    f[0] = f0 ^ x0;
    f[1] = f1 ^ x1;
    f[2] = f2 ^ x2;
    f[3] = f3 ^ x3;
    f[4] = f4 ^ x4;
    f[5] = f5 ^ x5;
    f[6] = f6 ^ x6;
    f[7] = f7 ^ x7;
    f[8] = f8 ^ x8;
    f[9] = f9 ^ x9;
}

/// Conditional swap.
///
/// C: `uint32_t mask = (uint32_t) (-(int64_t) b);`
pub fn fe25519_cswap(f: &mut Fe25519, g: &mut Fe25519, b: u32) {
    let mask: u32 = (0i64.wrapping_sub(b as i64)) as u32;

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

    let m = mask as i32;
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

    f[0] = f0 ^ x0;
    f[1] = f1 ^ x1;
    f[2] = f2 ^ x2;
    f[3] = f3 ^ x3;
    f[4] = f4 ^ x4;
    f[5] = f5 ^ x5;
    f[6] = f6 ^ x6;
    f[7] = f7 ^ x7;
    f[8] = f8 ^ x8;
    f[9] = f9 ^ x9;

    g[0] = g0 ^ x0;
    g[1] = g1 ^ x1;
    g[2] = g2 ^ x2;
    g[3] = g3 ^ x3;
    g[4] = g4 ^ x4;
    g[5] = g5 ^ x5;
    g[6] = g6 ^ x6;
    g[7] = g7 ^ x7;
    g[8] = g8 ^ x8;
    g[9] = g9 ^ x9;
}

/// `h = f`
#[inline]
pub fn fe25519_copy(h: &mut Fe25519, f: Fe25519) {
    h.0 = f.0;
}

/// Return 1 if `f` is in `{1,3,5,...,q-2}`, 0 if `f` is in `{0,2,4,...,q-1}`.
#[inline]
pub fn fe25519_isnegative(f: Fe25519) -> i32 {
    let mut s = [0u8; 32];

    fe25519_tobytes(&mut s, &f);

    (s[0] & 1) as i32
}

/// Return 1 if `f == 0`, 0 otherwise.
#[inline]
pub fn fe25519_iszero(f: Fe25519) -> i32 {
    let mut s = [0u8; 32];

    fe25519_tobytes(&mut s, &f);

    sodium_is_zero_32(&s)
}

/// Constant-time `sodium_is_zero(s, 32)` (see `sodium/utils.c`).
#[inline]
fn sodium_is_zero_32(s: &[u8; 32]) -> i32 {
    let mut d: u8 = 0;
    for i in 0..32 {
        d |= s[i];
    }
    1 & ((d as i32).wrapping_sub(1) >> 8)
}

/// `h = f * g`
///
/// Preconditions: `|f|`, `|g|` bounded by 1.65*2^26, 1.65*2^25, ...
/// Postconditions: `|h|` bounded by 1.01*2^25, 1.01*2^24, ...
pub fn fe25519_mul(h: &mut Fe25519, f: Fe25519, g: Fe25519) {
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

    let g1_19 = 19i32.wrapping_mul(g1); /* 1.959375*2^29 */
    let g2_19 = 19i32.wrapping_mul(g2); /* 1.959375*2^30; still ok */
    let g3_19 = 19i32.wrapping_mul(g3);
    let g4_19 = 19i32.wrapping_mul(g4);
    let g5_19 = 19i32.wrapping_mul(g5);
    let g6_19 = 19i32.wrapping_mul(g6);
    let g7_19 = 19i32.wrapping_mul(g7);
    let g8_19 = 19i32.wrapping_mul(g8);
    let g9_19 = 19i32.wrapping_mul(g9);
    let f1_2 = 2i32.wrapping_mul(f1);
    let f3_2 = 2i32.wrapping_mul(f3);
    let f5_2 = 2i32.wrapping_mul(f5);
    let f7_2 = 2i32.wrapping_mul(f7);
    let f9_2 = 2i32.wrapping_mul(f9);

    let f0g0 = (f0 as i64).wrapping_mul(g0 as i64);
    let f0g1 = (f0 as i64).wrapping_mul(g1 as i64);
    let f0g2 = (f0 as i64).wrapping_mul(g2 as i64);
    let f0g3 = (f0 as i64).wrapping_mul(g3 as i64);
    let f0g4 = (f0 as i64).wrapping_mul(g4 as i64);
    let f0g5 = (f0 as i64).wrapping_mul(g5 as i64);
    let f0g6 = (f0 as i64).wrapping_mul(g6 as i64);
    let f0g7 = (f0 as i64).wrapping_mul(g7 as i64);
    let f0g8 = (f0 as i64).wrapping_mul(g8 as i64);
    let f0g9 = (f0 as i64).wrapping_mul(g9 as i64);
    let f1g0 = (f1 as i64).wrapping_mul(g0 as i64);
    let f1g1_2 = (f1_2 as i64).wrapping_mul(g1 as i64);
    let f1g2 = (f1 as i64).wrapping_mul(g2 as i64);
    let f1g3_2 = (f1_2 as i64).wrapping_mul(g3 as i64);
    let f1g4 = (f1 as i64).wrapping_mul(g4 as i64);
    let f1g5_2 = (f1_2 as i64).wrapping_mul(g5 as i64);
    let f1g6 = (f1 as i64).wrapping_mul(g6 as i64);
    let f1g7_2 = (f1_2 as i64).wrapping_mul(g7 as i64);
    let f1g8 = (f1 as i64).wrapping_mul(g8 as i64);
    let f1g9_38 = (f1_2 as i64).wrapping_mul(g9_19 as i64);
    let f2g0 = (f2 as i64).wrapping_mul(g0 as i64);
    let f2g1 = (f2 as i64).wrapping_mul(g1 as i64);
    let f2g2 = (f2 as i64).wrapping_mul(g2 as i64);
    let f2g3 = (f2 as i64).wrapping_mul(g3 as i64);
    let f2g4 = (f2 as i64).wrapping_mul(g4 as i64);
    let f2g5 = (f2 as i64).wrapping_mul(g5 as i64);
    let f2g6 = (f2 as i64).wrapping_mul(g6 as i64);
    let f2g7 = (f2 as i64).wrapping_mul(g7 as i64);
    let f2g8_19 = (f2 as i64).wrapping_mul(g8_19 as i64);
    let f2g9_19 = (f2 as i64).wrapping_mul(g9_19 as i64);
    let f3g0 = (f3 as i64).wrapping_mul(g0 as i64);
    let f3g1_2 = (f3_2 as i64).wrapping_mul(g1 as i64);
    let f3g2 = (f3 as i64).wrapping_mul(g2 as i64);
    let f3g3_2 = (f3_2 as i64).wrapping_mul(g3 as i64);
    let f3g4 = (f3 as i64).wrapping_mul(g4 as i64);
    let f3g5_2 = (f3_2 as i64).wrapping_mul(g5 as i64);
    let f3g6 = (f3 as i64).wrapping_mul(g6 as i64);
    let f3g7_38 = (f3_2 as i64).wrapping_mul(g7_19 as i64);
    let f3g8_19 = (f3 as i64).wrapping_mul(g8_19 as i64);
    let f3g9_38 = (f3_2 as i64).wrapping_mul(g9_19 as i64);
    let f4g0 = (f4 as i64).wrapping_mul(g0 as i64);
    let f4g1 = (f4 as i64).wrapping_mul(g1 as i64);
    let f4g2 = (f4 as i64).wrapping_mul(g2 as i64);
    let f4g3 = (f4 as i64).wrapping_mul(g3 as i64);
    let f4g4 = (f4 as i64).wrapping_mul(g4 as i64);
    let f4g5 = (f4 as i64).wrapping_mul(g5 as i64);
    let f4g6_19 = (f4 as i64).wrapping_mul(g6_19 as i64);
    let f4g7_19 = (f4 as i64).wrapping_mul(g7_19 as i64);
    let f4g8_19 = (f4 as i64).wrapping_mul(g8_19 as i64);
    let f4g9_19 = (f4 as i64).wrapping_mul(g9_19 as i64);
    let f5g0 = (f5 as i64).wrapping_mul(g0 as i64);
    let f5g1_2 = (f5_2 as i64).wrapping_mul(g1 as i64);
    let f5g2 = (f5 as i64).wrapping_mul(g2 as i64);
    let f5g3_2 = (f5_2 as i64).wrapping_mul(g3 as i64);
    let f5g4 = (f5 as i64).wrapping_mul(g4 as i64);
    let f5g5_38 = (f5_2 as i64).wrapping_mul(g5_19 as i64);
    let f5g6_19 = (f5 as i64).wrapping_mul(g6_19 as i64);
    let f5g7_38 = (f5_2 as i64).wrapping_mul(g7_19 as i64);
    let f5g8_19 = (f5 as i64).wrapping_mul(g8_19 as i64);
    let f5g9_38 = (f5_2 as i64).wrapping_mul(g9_19 as i64);
    let f6g0 = (f6 as i64).wrapping_mul(g0 as i64);
    let f6g1 = (f6 as i64).wrapping_mul(g1 as i64);
    let f6g2 = (f6 as i64).wrapping_mul(g2 as i64);
    let f6g3 = (f6 as i64).wrapping_mul(g3 as i64);
    let f6g4_19 = (f6 as i64).wrapping_mul(g4_19 as i64);
    let f6g5_19 = (f6 as i64).wrapping_mul(g5_19 as i64);
    let f6g6_19 = (f6 as i64).wrapping_mul(g6_19 as i64);
    let f6g7_19 = (f6 as i64).wrapping_mul(g7_19 as i64);
    let f6g8_19 = (f6 as i64).wrapping_mul(g8_19 as i64);
    let f6g9_19 = (f6 as i64).wrapping_mul(g9_19 as i64);
    let f7g0 = (f7 as i64).wrapping_mul(g0 as i64);
    let f7g1_2 = (f7_2 as i64).wrapping_mul(g1 as i64);
    let f7g2 = (f7 as i64).wrapping_mul(g2 as i64);
    let f7g3_38 = (f7_2 as i64).wrapping_mul(g3_19 as i64);
    let f7g4_19 = (f7 as i64).wrapping_mul(g4_19 as i64);
    let f7g5_38 = (f7_2 as i64).wrapping_mul(g5_19 as i64);
    let f7g6_19 = (f7 as i64).wrapping_mul(g6_19 as i64);
    let f7g7_38 = (f7_2 as i64).wrapping_mul(g7_19 as i64);
    let f7g8_19 = (f7 as i64).wrapping_mul(g8_19 as i64);
    let f7g9_38 = (f7_2 as i64).wrapping_mul(g9_19 as i64);
    let f8g0 = (f8 as i64).wrapping_mul(g0 as i64);
    let f8g1 = (f8 as i64).wrapping_mul(g1 as i64);
    let f8g2_19 = (f8 as i64).wrapping_mul(g2_19 as i64);
    let f8g3_19 = (f8 as i64).wrapping_mul(g3_19 as i64);
    let f8g4_19 = (f8 as i64).wrapping_mul(g4_19 as i64);
    let f8g5_19 = (f8 as i64).wrapping_mul(g5_19 as i64);
    let f8g6_19 = (f8 as i64).wrapping_mul(g6_19 as i64);
    let f8g7_19 = (f8 as i64).wrapping_mul(g7_19 as i64);
    let f8g8_19 = (f8 as i64).wrapping_mul(g8_19 as i64);
    let f8g9_19 = (f8 as i64).wrapping_mul(g9_19 as i64);
    let f9g0 = (f9 as i64).wrapping_mul(g0 as i64);
    let f9g1_38 = (f9_2 as i64).wrapping_mul(g1_19 as i64);
    let f9g2_19 = (f9 as i64).wrapping_mul(g2_19 as i64);
    let f9g3_38 = (f9_2 as i64).wrapping_mul(g3_19 as i64);
    let f9g4_19 = (f9 as i64).wrapping_mul(g4_19 as i64);
    let f9g5_38 = (f9_2 as i64).wrapping_mul(g5_19 as i64);
    let f9g6_19 = (f9 as i64).wrapping_mul(g6_19 as i64);
    let f9g7_38 = (f9_2 as i64).wrapping_mul(g7_19 as i64);
    let f9g8_19 = (f9 as i64).wrapping_mul(g8_19 as i64);
    let f9g9_38 = (f9_2 as i64).wrapping_mul(g9_19 as i64);

    /* |h0| <= 1.4*2^60, |h1| <= 1.7*2^59: no i64 overflow */
    let mut h0: i64 = f0g0 + f1g9_38 + f2g8_19 + f3g7_38 + f4g6_19 + f5g5_38 + f6g4_19 + f7g3_38
        + f8g2_19
        + f9g1_38;
    let mut h1: i64 = f0g1 + f1g0 + f2g9_19 + f3g8_19 + f4g7_19 + f5g6_19 + f6g5_19 + f7g4_19
        + f8g3_19
        + f9g2_19;
    let mut h2: i64 = f0g2 + f1g1_2 + f2g0 + f3g9_38 + f4g8_19 + f5g7_38 + f6g6_19 + f7g5_38
        + f8g4_19
        + f9g3_38;
    let mut h3: i64 = f0g3 + f1g2 + f2g1 + f3g0 + f4g9_19 + f5g8_19 + f6g7_19 + f7g6_19 + f8g5_19
        + f9g4_19;
    let mut h4: i64 = f0g4 + f1g3_2 + f2g2 + f3g1_2 + f4g0 + f5g9_38 + f6g8_19 + f7g7_38 + f8g6_19
        + f9g5_38;
    let mut h5: i64 = f0g5 + f1g4 + f2g3 + f3g2 + f4g1 + f5g0 + f6g9_19 + f7g8_19 + f8g7_19
        + f9g6_19;
    let mut h6: i64 = f0g6 + f1g5_2 + f2g4 + f3g3_2 + f4g2 + f5g1_2 + f6g0 + f7g9_38 + f8g8_19
        + f9g7_38;
    let mut h7: i64 =
        f0g7 + f1g6 + f2g5 + f3g4 + f4g3 + f5g2 + f6g1 + f7g0 + f8g9_19 + f9g8_19;
    let mut h8: i64 = f0g8 + f1g7_2 + f2g6 + f3g5_2 + f4g4 + f5g3_2 + f6g2 + f7g1_2 + f8g0
        + f9g9_38;
    let mut h9: i64 = f0g9 + f1g8 + f2g7 + f3g6 + f4g5 + f5g4 + f6g3 + f7g2 + f8g1 + f9g0;

    let mut carry0: i64;
    let carry1: i64;
    let carry2: i64;
    let carry3: i64;
    let mut carry4: i64;
    let carry5: i64;
    let carry6: i64;
    let carry7: i64;
    let carry8: i64;
    let carry9: i64;

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));

    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));

    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));

    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));

    h[0] = h0 as i32;
    h[1] = h1 as i32;
    h[2] = h2 as i32;
    h[3] = h3 as i32;
    h[4] = h4 as i32;
    h[5] = h5 as i32;
    h[6] = h6 as i32;
    h[7] = h7 as i32;
    h[8] = h8 as i32;
    h[9] = h9 as i32;
}

/// `h = f * f`
pub fn fe25519_sq(h: &mut Fe25519, f: Fe25519) {
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

    let f0_2 = 2i32.wrapping_mul(f0);
    let f1_2 = 2i32.wrapping_mul(f1);
    let f2_2 = 2i32.wrapping_mul(f2);
    let f3_2 = 2i32.wrapping_mul(f3);
    let f4_2 = 2i32.wrapping_mul(f4);
    let f5_2 = 2i32.wrapping_mul(f5);
    let f6_2 = 2i32.wrapping_mul(f6);
    let f7_2 = 2i32.wrapping_mul(f7);
    let f5_38 = 38i32.wrapping_mul(f5); /* 1.959375*2^30 */
    let f6_19 = 19i32.wrapping_mul(f6); /* 1.959375*2^30 */
    let f7_38 = 38i32.wrapping_mul(f7); /* 1.959375*2^30 */
    let f8_19 = 19i32.wrapping_mul(f8); /* 1.959375*2^30 */
    let f9_38 = 38i32.wrapping_mul(f9); /* 1.959375*2^30 */

    let f0f0 = (f0 as i64).wrapping_mul(f0 as i64);
    let f0f1_2 = (f0_2 as i64).wrapping_mul(f1 as i64);
    let f0f2_2 = (f0_2 as i64).wrapping_mul(f2 as i64);
    let f0f3_2 = (f0_2 as i64).wrapping_mul(f3 as i64);
    let f0f4_2 = (f0_2 as i64).wrapping_mul(f4 as i64);
    let f0f5_2 = (f0_2 as i64).wrapping_mul(f5 as i64);
    let f0f6_2 = (f0_2 as i64).wrapping_mul(f6 as i64);
    let f0f7_2 = (f0_2 as i64).wrapping_mul(f7 as i64);
    let f0f8_2 = (f0_2 as i64).wrapping_mul(f8 as i64);
    let f0f9_2 = (f0_2 as i64).wrapping_mul(f9 as i64);
    let f1f1_2 = (f1_2 as i64).wrapping_mul(f1 as i64);
    let f1f2_2 = (f1_2 as i64).wrapping_mul(f2 as i64);
    let f1f3_4 = (f1_2 as i64).wrapping_mul(f3_2 as i64);
    let f1f4_2 = (f1_2 as i64).wrapping_mul(f4 as i64);
    let f1f5_4 = (f1_2 as i64).wrapping_mul(f5_2 as i64);
    let f1f6_2 = (f1_2 as i64).wrapping_mul(f6 as i64);
    let f1f7_4 = (f1_2 as i64).wrapping_mul(f7_2 as i64);
    let f1f8_2 = (f1_2 as i64).wrapping_mul(f8 as i64);
    let f1f9_76 = (f1_2 as i64).wrapping_mul(f9_38 as i64);
    let f2f2 = (f2 as i64).wrapping_mul(f2 as i64);
    let f2f3_2 = (f2_2 as i64).wrapping_mul(f3 as i64);
    let f2f4_2 = (f2_2 as i64).wrapping_mul(f4 as i64);
    let f2f5_2 = (f2_2 as i64).wrapping_mul(f5 as i64);
    let f2f6_2 = (f2_2 as i64).wrapping_mul(f6 as i64);
    let f2f7_2 = (f2_2 as i64).wrapping_mul(f7 as i64);
    let f2f8_38 = (f2_2 as i64).wrapping_mul(f8_19 as i64);
    let f2f9_38 = (f2 as i64).wrapping_mul(f9_38 as i64);
    let f3f3_2 = (f3_2 as i64).wrapping_mul(f3 as i64);
    let f3f4_2 = (f3_2 as i64).wrapping_mul(f4 as i64);
    let f3f5_4 = (f3_2 as i64).wrapping_mul(f5_2 as i64);
    let f3f6_2 = (f3_2 as i64).wrapping_mul(f6 as i64);
    let f3f7_76 = (f3_2 as i64).wrapping_mul(f7_38 as i64);
    let f3f8_38 = (f3_2 as i64).wrapping_mul(f8_19 as i64);
    let f3f9_76 = (f3_2 as i64).wrapping_mul(f9_38 as i64);
    let f4f4 = (f4 as i64).wrapping_mul(f4 as i64);
    let f4f5_2 = (f4_2 as i64).wrapping_mul(f5 as i64);
    let f4f6_38 = (f4_2 as i64).wrapping_mul(f6_19 as i64);
    let f4f7_38 = (f4 as i64).wrapping_mul(f7_38 as i64);
    let f4f8_38 = (f4_2 as i64).wrapping_mul(f8_19 as i64);
    let f4f9_38 = (f4 as i64).wrapping_mul(f9_38 as i64);
    let f5f5_38 = (f5 as i64).wrapping_mul(f5_38 as i64);
    let f5f6_38 = (f5_2 as i64).wrapping_mul(f6_19 as i64);
    let f5f7_76 = (f5_2 as i64).wrapping_mul(f7_38 as i64);
    let f5f8_38 = (f5_2 as i64).wrapping_mul(f8_19 as i64);
    let f5f9_76 = (f5_2 as i64).wrapping_mul(f9_38 as i64);
    let f6f6_19 = (f6 as i64).wrapping_mul(f6_19 as i64);
    let f6f7_38 = (f6 as i64).wrapping_mul(f7_38 as i64);
    let f6f8_38 = (f6_2 as i64).wrapping_mul(f8_19 as i64);
    let f6f9_38 = (f6 as i64).wrapping_mul(f9_38 as i64);
    let f7f7_38 = (f7 as i64).wrapping_mul(f7_38 as i64);
    let f7f8_38 = (f7_2 as i64).wrapping_mul(f8_19 as i64);
    let f7f9_76 = (f7_2 as i64).wrapping_mul(f9_38 as i64);
    let f8f8_19 = (f8 as i64).wrapping_mul(f8_19 as i64);
    let f8f9_38 = (f8 as i64).wrapping_mul(f9_38 as i64);
    let f9f9_38 = (f9 as i64).wrapping_mul(f9_38 as i64);

    let mut h0: i64 = f0f0 + f1f9_76 + f2f8_38 + f3f7_76 + f4f6_38 + f5f5_38;
    let mut h1: i64 = f0f1_2 + f2f9_38 + f3f8_38 + f4f7_38 + f5f6_38;
    let mut h2: i64 = f0f2_2 + f1f1_2 + f3f9_76 + f4f8_38 + f5f7_76 + f6f6_19;
    let mut h3: i64 = f0f3_2 + f1f2_2 + f4f9_38 + f5f8_38 + f6f7_38;
    let mut h4: i64 = f0f4_2 + f1f3_4 + f2f2 + f5f9_76 + f6f8_38 + f7f7_38;
    let mut h5: i64 = f0f5_2 + f1f4_2 + f2f3_2 + f6f9_38 + f7f8_38;
    let mut h6: i64 = f0f6_2 + f1f5_4 + f2f4_2 + f3f3_2 + f7f9_76 + f8f8_19;
    let mut h7: i64 = f0f7_2 + f1f6_2 + f2f5_2 + f3f4_2 + f8f9_38;
    let mut h8: i64 = f0f8_2 + f1f7_4 + f2f6_2 + f3f5_4 + f4f4 + f9f9_38;
    let mut h9: i64 = f0f9_2 + f1f8_2 + f2f7_2 + f3f6_2 + f4f5_2;

    let mut carry0: i64;
    let carry1: i64;
    let carry2: i64;
    let carry3: i64;
    let mut carry4: i64;
    let carry5: i64;
    let carry6: i64;
    let carry7: i64;
    let carry8: i64;
    let carry9: i64;

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));

    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));

    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));

    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));

    h[0] = h0 as i32;
    h[1] = h1 as i32;
    h[2] = h2 as i32;
    h[3] = h3 as i32;
    h[4] = h4 as i32;
    h[5] = h5 as i32;
    h[6] = h6 as i32;
    h[7] = h7 as i32;
    h[8] = h8 as i32;
    h[9] = h9 as i32;
}

/// `h = 2 * f * f`
pub fn fe25519_sq2(h: &mut Fe25519, f: Fe25519) {
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

    let f0_2 = 2i32.wrapping_mul(f0);
    let f1_2 = 2i32.wrapping_mul(f1);
    let f2_2 = 2i32.wrapping_mul(f2);
    let f3_2 = 2i32.wrapping_mul(f3);
    let f4_2 = 2i32.wrapping_mul(f4);
    let f5_2 = 2i32.wrapping_mul(f5);
    let f6_2 = 2i32.wrapping_mul(f6);
    let f7_2 = 2i32.wrapping_mul(f7);
    let f5_38 = 38i32.wrapping_mul(f5); /* 1.959375*2^30 */
    let f6_19 = 19i32.wrapping_mul(f6); /* 1.959375*2^30 */
    let f7_38 = 38i32.wrapping_mul(f7); /* 1.959375*2^30 */
    let f8_19 = 19i32.wrapping_mul(f8); /* 1.959375*2^30 */
    let f9_38 = 38i32.wrapping_mul(f9); /* 1.959375*2^30 */

    let f0f0 = (f0 as i64).wrapping_mul(f0 as i64);
    let f0f1_2 = (f0_2 as i64).wrapping_mul(f1 as i64);
    let f0f2_2 = (f0_2 as i64).wrapping_mul(f2 as i64);
    let f0f3_2 = (f0_2 as i64).wrapping_mul(f3 as i64);
    let f0f4_2 = (f0_2 as i64).wrapping_mul(f4 as i64);
    let f0f5_2 = (f0_2 as i64).wrapping_mul(f5 as i64);
    let f0f6_2 = (f0_2 as i64).wrapping_mul(f6 as i64);
    let f0f7_2 = (f0_2 as i64).wrapping_mul(f7 as i64);
    let f0f8_2 = (f0_2 as i64).wrapping_mul(f8 as i64);
    let f0f9_2 = (f0_2 as i64).wrapping_mul(f9 as i64);
    let f1f1_2 = (f1_2 as i64).wrapping_mul(f1 as i64);
    let f1f2_2 = (f1_2 as i64).wrapping_mul(f2 as i64);
    let f1f3_4 = (f1_2 as i64).wrapping_mul(f3_2 as i64);
    let f1f4_2 = (f1_2 as i64).wrapping_mul(f4 as i64);
    let f1f5_4 = (f1_2 as i64).wrapping_mul(f5_2 as i64);
    let f1f6_2 = (f1_2 as i64).wrapping_mul(f6 as i64);
    let f1f7_4 = (f1_2 as i64).wrapping_mul(f7_2 as i64);
    let f1f8_2 = (f1_2 as i64).wrapping_mul(f8 as i64);
    let f1f9_76 = (f1_2 as i64).wrapping_mul(f9_38 as i64);
    let f2f2 = (f2 as i64).wrapping_mul(f2 as i64);
    let f2f3_2 = (f2_2 as i64).wrapping_mul(f3 as i64);
    let f2f4_2 = (f2_2 as i64).wrapping_mul(f4 as i64);
    let f2f5_2 = (f2_2 as i64).wrapping_mul(f5 as i64);
    let f2f6_2 = (f2_2 as i64).wrapping_mul(f6 as i64);
    let f2f7_2 = (f2_2 as i64).wrapping_mul(f7 as i64);
    let f2f8_38 = (f2_2 as i64).wrapping_mul(f8_19 as i64);
    let f2f9_38 = (f2 as i64).wrapping_mul(f9_38 as i64);
    let f3f3_2 = (f3_2 as i64).wrapping_mul(f3 as i64);
    let f3f4_2 = (f3_2 as i64).wrapping_mul(f4 as i64);
    let f3f5_4 = (f3_2 as i64).wrapping_mul(f5_2 as i64);
    let f3f6_2 = (f3_2 as i64).wrapping_mul(f6 as i64);
    let f3f7_76 = (f3_2 as i64).wrapping_mul(f7_38 as i64);
    let f3f8_38 = (f3_2 as i64).wrapping_mul(f8_19 as i64);
    let f3f9_76 = (f3_2 as i64).wrapping_mul(f9_38 as i64);
    let f4f4 = (f4 as i64).wrapping_mul(f4 as i64);
    let f4f5_2 = (f4_2 as i64).wrapping_mul(f5 as i64);
    let f4f6_38 = (f4_2 as i64).wrapping_mul(f6_19 as i64);
    let f4f7_38 = (f4 as i64).wrapping_mul(f7_38 as i64);
    let f4f8_38 = (f4_2 as i64).wrapping_mul(f8_19 as i64);
    let f4f9_38 = (f4 as i64).wrapping_mul(f9_38 as i64);
    let f5f5_38 = (f5 as i64).wrapping_mul(f5_38 as i64);
    let f5f6_38 = (f5_2 as i64).wrapping_mul(f6_19 as i64);
    let f5f7_76 = (f5_2 as i64).wrapping_mul(f7_38 as i64);
    let f5f8_38 = (f5_2 as i64).wrapping_mul(f8_19 as i64);
    let f5f9_76 = (f5_2 as i64).wrapping_mul(f9_38 as i64);
    let f6f6_19 = (f6 as i64).wrapping_mul(f6_19 as i64);
    let f6f7_38 = (f6 as i64).wrapping_mul(f7_38 as i64);
    let f6f8_38 = (f6_2 as i64).wrapping_mul(f8_19 as i64);
    let f6f9_38 = (f6 as i64).wrapping_mul(f9_38 as i64);
    let f7f7_38 = (f7 as i64).wrapping_mul(f7_38 as i64);
    let f7f8_38 = (f7_2 as i64).wrapping_mul(f8_19 as i64);
    let f7f9_76 = (f7_2 as i64).wrapping_mul(f9_38 as i64);
    let f8f8_19 = (f8 as i64).wrapping_mul(f8_19 as i64);
    let f8f9_38 = (f8 as i64).wrapping_mul(f9_38 as i64);
    let f9f9_38 = (f9 as i64).wrapping_mul(f9_38 as i64);

    let mut h0: i64 = f0f0 + f1f9_76 + f2f8_38 + f3f7_76 + f4f6_38 + f5f5_38;
    let mut h1: i64 = f0f1_2 + f2f9_38 + f3f8_38 + f4f7_38 + f5f6_38;
    let mut h2: i64 = f0f2_2 + f1f1_2 + f3f9_76 + f4f8_38 + f5f7_76 + f6f6_19;
    let mut h3: i64 = f0f3_2 + f1f2_2 + f4f9_38 + f5f8_38 + f6f7_38;
    let mut h4: i64 = f0f4_2 + f1f3_4 + f2f2 + f5f9_76 + f6f8_38 + f7f7_38;
    let mut h5: i64 = f0f5_2 + f1f4_2 + f2f3_2 + f6f9_38 + f7f8_38;
    let mut h6: i64 = f0f6_2 + f1f5_4 + f2f4_2 + f3f3_2 + f7f9_76 + f8f8_19;
    let mut h7: i64 = f0f7_2 + f1f6_2 + f2f5_2 + f3f4_2 + f8f9_38;
    let mut h8: i64 = f0f8_2 + f1f7_4 + f2f6_2 + f3f5_4 + f4f4 + f9f9_38;
    let mut h9: i64 = f0f9_2 + f1f8_2 + f2f7_2 + f3f6_2 + f4f5_2;

    let mut carry0: i64;
    let carry1: i64;
    let carry2: i64;
    let carry3: i64;
    let mut carry4: i64;
    let carry5: i64;
    let carry6: i64;
    let carry7: i64;
    let carry8: i64;
    let carry9: i64;

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

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));

    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));

    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));

    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));

    h[0] = h0 as i32;
    h[1] = h1 as i32;
    h[2] = h2 as i32;
    h[3] = h3 as i32;
    h[4] = h4 as i32;
    h[5] = h5 as i32;
    h[6] = h6 as i32;
    h[7] = h7 as i32;
    h[8] = h8 as i32;
    h[9] = h9 as i32;
}

/// `h = f * n` for a small unsigned scalar `n`.
pub fn fe25519_mul32(h: &mut Fe25519, f: Fe25519, n: u32) {
    let sn = n as i64;
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
    let carry0: i64;
    let carry1: i64;
    let carry2: i64;
    let carry3: i64;
    let carry4: i64;
    let carry5: i64;
    let carry6: i64;
    let carry7: i64;
    let carry8: i64;
    let carry9: i64;

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));
    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    h[0] = h0 as i32;
    h[1] = h1 as i32;
    h[2] = h2 as i32;
    h[3] = h3 as i32;
    h[4] = h4 as i32;
    h[5] = h5 as i32;
    h[6] = h6 as i32;
    h[7] = h7 as i32;
    h[8] = h8 as i32;
    h[9] = h9 as i32;
}

/* ------------------------------------------------------------------ */
/* value-returning helpers (used to mirror the aliasing C call sites)  */
/* ------------------------------------------------------------------ */

/// `f * f`, returned by value (helper for in-place C chains such as
/// `fe25519_sq(t1, t1)`).
#[inline]
pub(crate) fn fe_sq(f: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_sq(&mut h, f);
    h
}

/// `f * g`, returned by value.
#[inline]
pub(crate) fn fe_mul(f: Fe25519, g: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_mul(&mut h, f, g);
    h
}

/* ------------------------------------------------------------------ */
/* fe_25_5/fe.h                                                        */
/* ------------------------------------------------------------------ */

/// `h = s`, ignoring the top bit of `s` (which must be at least 32 bytes).
pub fn fe25519_frombytes(h: &mut Fe25519, s: &[u8]) {
    let mut h0: i64 = load_4(&s[0..]) as i64;
    let mut h1: i64 = (load_3(&s[4..]) << 6) as i64;
    let mut h2: i64 = (load_3(&s[7..]) << 5) as i64;
    let mut h3: i64 = (load_3(&s[10..]) << 3) as i64;
    let mut h4: i64 = (load_3(&s[13..]) << 2) as i64;
    let mut h5: i64 = load_4(&s[16..]) as i64;
    let mut h6: i64 = (load_3(&s[20..]) << 7) as i64;
    let mut h7: i64 = (load_3(&s[23..]) << 5) as i64;
    let mut h8: i64 = (load_3(&s[26..]) << 4) as i64;
    let mut h9: i64 = ((load_3(&s[29..]) & 8388607) << 2) as i64;

    let carry0: i64;
    let carry1: i64;
    let carry2: i64;
    let carry3: i64;
    let carry4: i64;
    let carry5: i64;
    let carry6: i64;
    let carry7: i64;
    let carry8: i64;
    let carry9: i64;

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));
    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    h[0] = h0 as i32;
    h[1] = h1 as i32;
    h[2] = h2 as i32;
    h[3] = h3 as i32;
    h[4] = h4 as i32;
    h[5] = h5 as i32;
    h[6] = h6 as i32;
    h[7] = h7 as i32;
    h[8] = h8 as i32;
    h[9] = h9 as i32;
}

/// Fully reduce `f` mod 2^255-19 into the canonical limb representation.
///
/// C: `static void fe25519_reduce(fe25519 h, const fe25519 f)`.
/// Everything here is 32-bit arithmetic; note the `uint32_t` casts, which make
/// the first shift of `q` a *logical* shift.
pub(crate) fn fe25519_reduce(h: &mut Fe25519, f: Fe25519) {
    let mut h0 = f[0];
    let mut h1 = f[1];
    let mut h2 = f[2];
    let mut h3 = f[3];
    let mut h4 = f[4];
    let mut h5 = f[5];
    let mut h6 = f[6];
    let mut h7 = f[7];
    let mut h8 = f[8];
    let mut h9 = f[9];

    let mut q: i32;
    let carry0: i32;
    let carry1: i32;
    let carry2: i32;
    let carry3: i32;
    let carry4: i32;
    let carry5: i32;
    let carry6: i32;
    let carry7: i32;
    let carry8: i32;
    let carry9: i32;

    /* `(19 * h9 + ((uint32_t) 1L << 24)) >> 25`: the sum is `uint32_t`,
     * so the shift is logical, and the result is converted back to int32_t. */
    q = (((19i32.wrapping_mul(h9) as u32).wrapping_add(1u32 << 24)) >> 25) as i32;
    q = (h0.wrapping_add(q)) >> 26;
    q = (h1.wrapping_add(q)) >> 25;
    q = (h2.wrapping_add(q)) >> 26;
    q = (h3.wrapping_add(q)) >> 25;
    q = (h4.wrapping_add(q)) >> 26;
    q = (h5.wrapping_add(q)) >> 25;
    q = (h6.wrapping_add(q)) >> 26;
    q = (h7.wrapping_add(q)) >> 25;
    q = (h8.wrapping_add(q)) >> 26;
    q = (h9.wrapping_add(q)) >> 25;

    /* Goal: Output h-(2^255-19)q, which is between 0 and 2^255-20. */
    h0 = h0.wrapping_add(19i32.wrapping_mul(q));
    /* Goal: Output h-2^255 q, which is between 0 and 2^255-20. */

    carry0 = h0 >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i32 << 26));
    carry1 = h1 >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i32 << 25));
    carry2 = h2 >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i32 << 26));
    carry3 = h3 >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i32 << 25));
    carry4 = h4 >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i32 << 26));
    carry5 = h5 >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i32 << 25));
    carry6 = h6 >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i32 << 26));
    carry7 = h7 >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i32 << 25));
    carry8 = h8 >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i32 << 26));
    carry9 = h9 >> 25;
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i32 << 25));

    h[0] = h0;
    h[1] = h1;
    h[2] = h2;
    h[3] = h3;
    h[4] = h4;
    h[5] = h5;
    h[6] = h6;
    h[7] = h7;
    h[8] = h8;
    h[9] = h9;
}

/// Serialise `h` (32 bytes, little-endian, fully reduced).
pub fn fe25519_tobytes(s: &mut [u8], h: &Fe25519) {
    let mut t = Fe25519::ZERO;

    fe25519_reduce(&mut t, *h);
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
}

/* ------------------------------------------------------------------ */
/* ed25519_ref10.c: inversion, exponentiation, square roots            */
/* ------------------------------------------------------------------ */

/// C: `static inline void fe25519_sqmul(fe25519 s, const int n, const fe25519 a)`
///
/// `s = s^(2^n) * a`
pub(crate) fn fe25519_sqmul(s: &mut Fe25519, n: i32, a: Fe25519) {
    let mut i: i32 = 0;

    while i < n {
        *s = fe_sq(*s);
        i += 1;
    }
    *s = fe_mul(*s, a);
}

/// Inversion - sets `out` to 0 if `z == 0`.
pub fn fe25519_invert(out: &mut Fe25519, z: &Fe25519) {
    let z = *z;
    let mut t0: Fe25519;
    let mut t1: Fe25519;
    let mut t2: Fe25519;
    let mut t3: Fe25519;
    let mut i: i32;

    t0 = fe_sq(z);
    t1 = fe_sq(t0);
    t1 = fe_sq(t1);
    t1 = fe_mul(z, t1);
    t0 = fe_mul(t0, t1);
    t2 = fe_sq(t0);
    t1 = fe_mul(t1, t2);
    t2 = fe_sq(t1);
    i = 1;
    while i < 5 {
        t2 = fe_sq(t2);
        i += 1;
    }
    t1 = fe_mul(t2, t1);
    t2 = fe_sq(t1);
    i = 1;
    while i < 10 {
        t2 = fe_sq(t2);
        i += 1;
    }
    t2 = fe_mul(t2, t1);
    t3 = fe_sq(t2);
    i = 1;
    while i < 20 {
        t3 = fe_sq(t3);
        i += 1;
    }
    t2 = fe_mul(t3, t2);
    i = 1;
    while i < 11 {
        t2 = fe_sq(t2);
        i += 1;
    }
    t1 = fe_mul(t2, t1);
    t2 = fe_sq(t1);
    i = 1;
    while i < 50 {
        t2 = fe_sq(t2);
        i += 1;
    }
    t2 = fe_mul(t2, t1);
    t3 = fe_sq(t2);
    i = 1;
    while i < 100 {
        t3 = fe_sq(t3);
        i += 1;
    }
    t2 = fe_mul(t3, t2);
    i = 1;
    while i < 51 {
        t2 = fe_sq(t2);
        i += 1;
    }
    t1 = fe_mul(t2, t1);
    i = 1;
    while i < 6 {
        t1 = fe_sq(t1);
        i += 1;
    }
    fe25519_mul(out, t1, t0);
}

/// In-place convenience wrapper for [`fe25519_invert`] (the C code does
/// `fe25519_invert(x, x)` in a few places).
#[inline]
pub fn fe25519_invert_in_place(z: &mut Fe25519) {
    let t = *z;
    fe25519_invert(z, &t);
}

/// Returns `z^((p-5)/8) = z^(2^252-3)`.
///
/// Used to compute square roots since we have `p = 5 (mod 8)`;
/// see Cohen and Frey.
pub fn fe25519_pow22523(out: &mut Fe25519, z: Fe25519) {
    let mut t0: Fe25519;
    let mut t1: Fe25519;
    let mut t2: Fe25519;
    let mut i: i32;

    t0 = fe_sq(z);
    t1 = fe_sq(t0);
    t1 = fe_sq(t1);
    t1 = fe_mul(z, t1);
    t0 = fe_mul(t0, t1);
    t0 = fe_sq(t0);
    t0 = fe_mul(t1, t0);
    t1 = fe_sq(t0);
    i = 1;
    while i < 5 {
        t1 = fe_sq(t1);
        i += 1;
    }
    t0 = fe_mul(t1, t0);
    t1 = fe_sq(t0);
    i = 1;
    while i < 10 {
        t1 = fe_sq(t1);
        i += 1;
    }
    t1 = fe_mul(t1, t0);
    t2 = fe_sq(t1);
    i = 1;
    while i < 20 {
        t2 = fe_sq(t2);
        i += 1;
    }
    t1 = fe_mul(t2, t1);
    i = 1;
    while i < 11 {
        t1 = fe_sq(t1);
        i += 1;
    }
    t0 = fe_mul(t1, t0);
    t1 = fe_sq(t0);
    i = 1;
    while i < 50 {
        t1 = fe_sq(t1);
        i += 1;
    }
    t1 = fe_mul(t1, t0);
    t2 = fe_sq(t1);
    i = 1;
    while i < 100 {
        t2 = fe_sq(t2);
        i += 1;
    }
    t1 = fe_mul(t2, t1);
    i = 1;
    while i < 51 {
        t1 = fe_sq(t1);
        i += 1;
    }
    t0 = fe_mul(t1, t0);
    t0 = fe_sq(t0);
    t0 = fe_sq(t0);
    fe25519_mul(out, t0, z);
}

/// `h = -h` if `b != 0`, `h` unchanged otherwise (`b` in `{0,1}`).
#[inline]
pub fn fe25519_cneg(h: &mut Fe25519, b: u32) {
    let mut negf = Fe25519::ZERO;

    fe25519_neg(&mut negf, *h);
    fe25519_cmov(h, negf, b);
}

/// `h = |h|`
#[inline]
pub fn fe25519_abs(h: &mut Fe25519) {
    let b = fe25519_isnegative(*h) as u32;
    fe25519_cneg(h, b);
}

/// Square root without checking that `x2` actually is a square.
pub fn fe25519_unchecked_sqrt(x: &mut Fe25519, x2: Fe25519) {
    let mut p_root = Fe25519::ZERO;
    let mut m_root = Fe25519::ZERO;
    let mut m_root2 = Fe25519::ZERO;
    let mut e = Fe25519::ZERO;

    fe25519_pow22523(&mut e, x2);
    fe25519_mul(&mut p_root, e, x2);
    fe25519_mul(&mut m_root, p_root, FE25519_SQRTM1);
    fe25519_sq(&mut m_root2, m_root);
    fe25519_sub(&mut e, x2, m_root2);
    fe25519_copy(x, p_root);
    let b = fe25519_iszero(e) as u32;
    fe25519_cmov(x, m_root, b);
}

/// `x = sqrt(x2)`; returns 0 on success, -1 if `x2` is not a square.
pub fn fe25519_sqrt(x: &mut Fe25519, x2: Fe25519) -> i32 {
    let mut check = Fe25519::ZERO;
    let mut x2_copy = Fe25519::ZERO;

    fe25519_copy(&mut x2_copy, x2);
    fe25519_unchecked_sqrt(x, x2);
    fe25519_sq(&mut check, *x);
    let c = check;
    fe25519_sub(&mut check, c, x2_copy);

    fe25519_iszero(check) - 1
}

/// Returns 1 if `x` is *not* a square (Jacobi symbol `x^((p-1)/2)`), else 0.
pub fn fe25519_notsquare(x: Fe25519) -> i32 {
    let mut _10 = Fe25519::ZERO;
    let mut _11 = Fe25519::ZERO;
    let mut _1100 = Fe25519::ZERO;
    let mut _1111 = Fe25519::ZERO;
    let mut _11110000 = Fe25519::ZERO;
    let mut _11111111 = Fe25519::ZERO;
    let mut t = Fe25519::ZERO;
    let mut u = Fe25519::ZERO;
    let mut v = Fe25519::ZERO;
    let mut s = [0u8; 32];

    /* Jacobi symbol - x^((p-1)/2) */
    fe25519_mul(&mut _10, x, x);
    fe25519_mul(&mut _11, x, _10);
    fe25519_sq(&mut _1100, _11);
    _1100 = fe_sq(_1100);
    fe25519_mul(&mut _1111, _11, _1100);
    fe25519_sq(&mut _11110000, _1111);
    _11110000 = fe_sq(_11110000);
    _11110000 = fe_sq(_11110000);
    _11110000 = fe_sq(_11110000);
    fe25519_mul(&mut _11111111, _1111, _11110000);
    fe25519_copy(&mut t, _11111111);
    fe25519_sqmul(&mut t, 2, _11);
    fe25519_copy(&mut u, t);
    fe25519_sqmul(&mut t, 10, u);
    fe25519_sqmul(&mut t, 10, u);
    fe25519_copy(&mut v, t);
    fe25519_sqmul(&mut t, 30, v);
    fe25519_copy(&mut v, t);
    fe25519_sqmul(&mut t, 60, v);
    fe25519_copy(&mut v, t);
    fe25519_sqmul(&mut t, 120, v);
    fe25519_sqmul(&mut t, 10, u);
    fe25519_sqmul(&mut t, 3, _11);
    t = fe_sq(t);

    fe25519_tobytes(&mut s, &t);

    (s[1] & 1) as i32
}

/* ------------------------------------------------------------------ */
/* C ABI (private/quirks.h renames)                                    */
/* ------------------------------------------------------------------ */

/// C: `void fe25519_frombytes(fe25519 h, const unsigned char *s)`
///
/// # Safety
/// `h` must point to a writable `fe25519` (10 `int32_t`), `s` to 32 readable
/// bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_frombytes(h: *mut Fe25519, s: *const u8) {
    let sl = unsafe { core::slice::from_raw_parts(s, 32) };
    let mut tmp = Fe25519::ZERO;

    fe25519_frombytes(&mut tmp, sl);
    unsafe { *h = tmp };
}

/// C: `void fe25519_tobytes(unsigned char *s, const fe25519 h)`
///
/// # Safety
/// `s` must point to 32 writable bytes, `h` to a readable `fe25519`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_tobytes(s: *mut u8, h: *const Fe25519) {
    let hv = unsafe { *h };
    let mut buf = [0u8; 32];

    fe25519_tobytes(&mut buf, &hv);
    unsafe { core::ptr::copy_nonoverlapping(buf.as_ptr(), s, 32) };
}

/// C: `void fe25519_invert(fe25519 out, const fe25519 z)`
///
/// # Safety
/// Both pointers must reference readable/writable `fe25519` values; `out` and
/// `z` may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_invert(out: *mut Fe25519, z: *const Fe25519) {
    let zv = unsafe { *z };
    let mut o = Fe25519::ZERO;

    fe25519_invert(&mut o, &zv);
    unsafe { *out = o };
}

/* ------------------------------------------------------------------ */
/* tests                                                               */
/* ------------------------------------------------------------------ */

#[cfg(test)]
mod tests {
    use super::*;

    fn fe_from(bytes: &[u8; 32]) -> Fe25519 {
        let mut h = Fe25519::ZERO;
        fe25519_frombytes(&mut h, bytes);
        h
    }

    fn fe_bytes(f: &Fe25519) -> [u8; 32] {
        let mut s = [0u8; 32];
        fe25519_tobytes(&mut s, f);
        s
    }

    fn one() -> Fe25519 {
        let mut o = Fe25519::ZERO;
        fe25519_1(&mut o);
        o
    }

    #[test]
    fn roundtrip() {
        let mut b = [0u8; 32];
        for i in 0..32 {
            b[i] = (i as u8).wrapping_mul(7).wrapping_add(3);
        }
        b[31] &= 127;
        let f = fe_from(&b);
        assert_eq!(fe_bytes(&f), b);
    }

    #[test]
    fn mul_matches_sq() {
        let mut b = [0u8; 32];
        for i in 0..32 {
            b[i] = (i as u8).wrapping_mul(11).wrapping_add(1);
        }
        b[31] &= 127;
        let f = fe_from(&b);
        let mut m = Fe25519::ZERO;
        let mut s = Fe25519::ZERO;
        fe25519_mul(&mut m, f, f);
        fe25519_sq(&mut s, f);
        assert_eq!(fe_bytes(&m), fe_bytes(&s));

        let mut s2 = Fe25519::ZERO;
        fe25519_sq2(&mut s2, f);
        let mut dbl = Fe25519::ZERO;
        fe25519_add(&mut dbl, s, s);
        assert_eq!(fe_bytes(&s2), fe_bytes(&dbl));

        let mut m32 = Fe25519::ZERO;
        fe25519_mul32(&mut m32, f, 121666);
        let mut acc = Fe25519::ZERO;
        let mut c121666 = Fe25519::ZERO;
        fe25519_frombytes(
            &mut c121666,
            &[
                0x42, 0xdb, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0,
            ],
        );
        fe25519_mul(&mut acc, f, c121666);
        assert_eq!(fe_bytes(&m32), fe_bytes(&acc));
    }

    #[test]
    fn invert_gives_one() {
        let mut b = [0u8; 32];
        for i in 0..32 {
            b[i] = (i as u8).wrapping_mul(13).wrapping_add(5);
        }
        b[31] &= 127;
        let f = fe_from(&b);
        let mut inv = Fe25519::ZERO;
        fe25519_invert(&mut inv, &f);
        let prod = fe_mul(f, inv);
        assert_eq!(fe_bytes(&prod), fe_bytes(&one()));
        assert_eq!(fe25519_iszero(prod), 0);
        assert_eq!(fe25519_iszero(Fe25519::ZERO), 1);
    }

    #[test]
    fn sqrt_of_square() {
        let mut b = [0u8; 32];
        for i in 0..32 {
            b[i] = (i as u8).wrapping_mul(3).wrapping_add(9);
        }
        b[31] &= 127;
        let f = fe_from(&b);
        let sq = fe_sq(f);
        let mut r = Fe25519::ZERO;
        assert_eq!(fe25519_sqrt(&mut r, sq), 0);
        let mut rr = Fe25519::ZERO;
        fe25519_sq(&mut rr, r);
        assert_eq!(fe_bytes(&rr), fe_bytes(&sq));
        assert_eq!(fe25519_notsquare(sq), 0);

        /* 2 is not a square mod 2^255-19 */
        let mut two = [0u8; 32];
        two[0] = 2;
        let t = fe_from(&two);
        assert_eq!(fe25519_notsquare(t), 1);
        let mut bad = Fe25519::ZERO;
        assert_eq!(fe25519_sqrt(&mut bad, t), -1);
    }

    #[test]
    fn cmov_cswap_neg() {
        let a = fe_from(&[1u8; 32]);
        let mut b = fe_from(&[2u8; 32]);
        let mut x = a;
        fe25519_cmov(&mut x, b, 0);
        assert_eq!(fe_bytes(&x), fe_bytes(&a));
        fe25519_cmov(&mut x, b, 1);
        assert_eq!(fe_bytes(&x), fe_bytes(&b));

        let mut p = a;
        let mut q = b;
        fe25519_cswap(&mut p, &mut q, 1);
        assert_eq!(fe_bytes(&p), fe_bytes(&b));
        assert_eq!(fe_bytes(&q), fe_bytes(&a));

        let mut n = Fe25519::ZERO;
        fe25519_neg(&mut n, a);
        let mut sum = Fe25519::ZERO;
        fe25519_add(&mut sum, a, n);
        assert_eq!(fe25519_iszero(sum), 1);

        b = a;
        fe25519_abs(&mut b);
        assert_eq!(fe25519_isnegative(b), 0);
    }
}
