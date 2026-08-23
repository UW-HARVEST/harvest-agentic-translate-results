//! Translation of `crypto_scalarmult/curve25519/ref10/x25519_ref10.c`.
//!
//! `HAVE_TI_MODE` is NOT defined in the reference build, so
//!   * `fe25519` is `int32_t[10]` (10 x 25.5-bit limbs), and
//!   * `fe25519_sub_lazy` is `#define`d to `fe25519_sub`.
//!
//! The C file `#include`s `private/ed25519_ref10.h`, which pulls in the
//! `static inline` field arithmetic from `private/ed25519_ref10_fe_25_5.h`.
//! That means this translation unit gets its *own private* copies of
//! `fe25519_0`, `fe25519_1`, `fe25519_add`, `fe25519_sub`, `fe25519_copy`,
//! `fe25519_cswap`, `fe25519_mul`, `fe25519_sq` and `fe25519_mul32`; they are
//! reproduced verbatim below as private helpers.
//!
//! `fe25519_frombytes`, `fe25519_tobytes`, `fe25519_invert` and
//! `ge25519_scalarmult_base` are *external* (non-`static`) functions living in
//! `crypto_core/ed25519/ref10/ed25519_ref10.c`, so they are called through the
//! linker under their `private/quirks.h` names.

use core::ffi::{c_int, c_uint, c_void};

/* `typedef int32_t fe25519[10];` */
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

extern "C" {
    /* crypto_core/ed25519/ref10/ed25519_ref10.c */
    fn _sodium_fe25519_frombytes(h: *mut i32, s: *const u8);
    fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32);
    fn _sodium_fe25519_invert(out: *mut i32, z: *const i32);
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

/* -------------------------------------------------------------------------
 * `private/ed25519_ref10_fe_25_5.h` — private copies (static inline in C)
 * ------------------------------------------------------------------------- */

/* h = 0 */
#[inline]
unsafe fn fe25519_0(h: *mut i32) {
    let mut i = 0usize;
    while i < 10 {
        *h.add(i) = 0;
        i += 1;
    }
}

/* h = 1 */
#[inline]
unsafe fn fe25519_1(h: *mut i32) {
    *h.add(0) = 1;
    *h.add(1) = 0;
    let mut i = 2usize;
    while i < 10 {
        *h.add(i) = 0;
        i += 1;
    }
}

/* h = f + g ; can overlap h with f or g */
#[inline]
unsafe fn fe25519_add(h: *mut i32, f: *const i32, g: *const i32) {
    let h0 = (*f.add(0)).wrapping_add(*g.add(0));
    let h1 = (*f.add(1)).wrapping_add(*g.add(1));
    let h2 = (*f.add(2)).wrapping_add(*g.add(2));
    let h3 = (*f.add(3)).wrapping_add(*g.add(3));
    let h4 = (*f.add(4)).wrapping_add(*g.add(4));
    let h5 = (*f.add(5)).wrapping_add(*g.add(5));
    let h6 = (*f.add(6)).wrapping_add(*g.add(6));
    let h7 = (*f.add(7)).wrapping_add(*g.add(7));
    let h8 = (*f.add(8)).wrapping_add(*g.add(8));
    let h9 = (*f.add(9)).wrapping_add(*g.add(9));

    *h.add(0) = h0;
    *h.add(1) = h1;
    *h.add(2) = h2;
    *h.add(3) = h3;
    *h.add(4) = h4;
    *h.add(5) = h5;
    *h.add(6) = h6;
    *h.add(7) = h7;
    *h.add(8) = h8;
    *h.add(9) = h9;
}

/* h = f - g ; can overlap h with f or g.
 * `fe25519_sub_lazy` is `#define`d to this when HAVE_TI_MODE is unset. */
unsafe fn fe25519_sub(h: *mut i32, f: *const i32, g: *const i32) {
    let h0 = (*f.add(0)).wrapping_sub(*g.add(0));
    let h1 = (*f.add(1)).wrapping_sub(*g.add(1));
    let h2 = (*f.add(2)).wrapping_sub(*g.add(2));
    let h3 = (*f.add(3)).wrapping_sub(*g.add(3));
    let h4 = (*f.add(4)).wrapping_sub(*g.add(4));
    let h5 = (*f.add(5)).wrapping_sub(*g.add(5));
    let h6 = (*f.add(6)).wrapping_sub(*g.add(6));
    let h7 = (*f.add(7)).wrapping_sub(*g.add(7));
    let h8 = (*f.add(8)).wrapping_sub(*g.add(8));
    let h9 = (*f.add(9)).wrapping_sub(*g.add(9));

    *h.add(0) = h0;
    *h.add(1) = h1;
    *h.add(2) = h2;
    *h.add(3) = h3;
    *h.add(4) = h4;
    *h.add(5) = h5;
    *h.add(6) = h6;
    *h.add(7) = h7;
    *h.add(8) = h8;
    *h.add(9) = h9;
}

/* `fe25519_sub_lazy` — plain `fe25519_sub` (no HAVE_TI_MODE) */
#[inline(always)]
unsafe fn fe25519_sub_lazy(h: *mut i32, f: *const i32, g: *const i32) {
    fe25519_sub(h, f, g);
}

/* Replace (f,g) with (g,f) if b == 1; leave them alone if b == 0. */
unsafe fn fe25519_cswap(f: *mut i32, g: *mut i32, b: c_uint) {
    let mask: u32 = (0i64.wrapping_sub(b as i64)) as u32;

    let f0 = *f.add(0);
    let f1 = *f.add(1);
    let f2 = *f.add(2);
    let f3 = *f.add(3);
    let f4 = *f.add(4);
    let f5 = *f.add(5);
    let f6 = *f.add(6);
    let f7 = *f.add(7);
    let f8 = *f.add(8);
    let f9 = *f.add(9);

    let g0 = *g.add(0);
    let g1 = *g.add(1);
    let g2 = *g.add(2);
    let g3 = *g.add(3);
    let g4 = *g.add(4);
    let g5 = *g.add(5);
    let g6 = *g.add(6);
    let g7 = *g.add(7);
    let g8 = *g.add(8);
    let g9 = *g.add(9);

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

    x0 = ((x0 as u32) & mask) as i32;
    x1 = ((x1 as u32) & mask) as i32;
    x2 = ((x2 as u32) & mask) as i32;
    x3 = ((x3 as u32) & mask) as i32;
    x4 = ((x4 as u32) & mask) as i32;
    x5 = ((x5 as u32) & mask) as i32;
    x6 = ((x6 as u32) & mask) as i32;
    x7 = ((x7 as u32) & mask) as i32;
    x8 = ((x8 as u32) & mask) as i32;
    x9 = ((x9 as u32) & mask) as i32;

    *f.add(0) = f0 ^ x0;
    *f.add(1) = f1 ^ x1;
    *f.add(2) = f2 ^ x2;
    *f.add(3) = f3 ^ x3;
    *f.add(4) = f4 ^ x4;
    *f.add(5) = f5 ^ x5;
    *f.add(6) = f6 ^ x6;
    *f.add(7) = f7 ^ x7;
    *f.add(8) = f8 ^ x8;
    *f.add(9) = f9 ^ x9;

    *g.add(0) = g0 ^ x0;
    *g.add(1) = g1 ^ x1;
    *g.add(2) = g2 ^ x2;
    *g.add(3) = g3 ^ x3;
    *g.add(4) = g4 ^ x4;
    *g.add(5) = g5 ^ x5;
    *g.add(6) = g6 ^ x6;
    *g.add(7) = g7 ^ x7;
    *g.add(8) = g8 ^ x8;
    *g.add(9) = g9 ^ x9;
}

/* h = f */
#[inline]
unsafe fn fe25519_copy(h: *mut i32, f: *const i32) {
    core::ptr::copy_nonoverlapping(f, h, 10);
}

/* h = f * g ; can overlap h with f or g */
unsafe fn fe25519_mul(h: *mut i32, f: *const i32, g: *const i32) {
    let f0 = *f.add(0);
    let f1 = *f.add(1);
    let f2 = *f.add(2);
    let f3 = *f.add(3);
    let f4 = *f.add(4);
    let f5 = *f.add(5);
    let f6 = *f.add(6);
    let f7 = *f.add(7);
    let f8 = *f.add(8);
    let f9 = *f.add(9);

    let g0 = *g.add(0);
    let g1 = *g.add(1);
    let g2 = *g.add(2);
    let g3 = *g.add(3);
    let g4 = *g.add(4);
    let g5 = *g.add(5);
    let g6 = *g.add(6);
    let g7 = *g.add(7);
    let g8 = *g.add(8);
    let g9 = *g.add(9);

    let g1_19: i32 = 19i32.wrapping_mul(g1);
    let g2_19: i32 = 19i32.wrapping_mul(g2);
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

    let f0g0: i64 = (f0 as i64) * (g0 as i64);
    let f0g1: i64 = (f0 as i64) * (g1 as i64);
    let f0g2: i64 = (f0 as i64) * (g2 as i64);
    let f0g3: i64 = (f0 as i64) * (g3 as i64);
    let f0g4: i64 = (f0 as i64) * (g4 as i64);
    let f0g5: i64 = (f0 as i64) * (g5 as i64);
    let f0g6: i64 = (f0 as i64) * (g6 as i64);
    let f0g7: i64 = (f0 as i64) * (g7 as i64);
    let f0g8: i64 = (f0 as i64) * (g8 as i64);
    let f0g9: i64 = (f0 as i64) * (g9 as i64);
    let f1g0: i64 = (f1 as i64) * (g0 as i64);
    let f1g1_2: i64 = (f1_2 as i64) * (g1 as i64);
    let f1g2: i64 = (f1 as i64) * (g2 as i64);
    let f1g3_2: i64 = (f1_2 as i64) * (g3 as i64);
    let f1g4: i64 = (f1 as i64) * (g4 as i64);
    let f1g5_2: i64 = (f1_2 as i64) * (g5 as i64);
    let f1g6: i64 = (f1 as i64) * (g6 as i64);
    let f1g7_2: i64 = (f1_2 as i64) * (g7 as i64);
    let f1g8: i64 = (f1 as i64) * (g8 as i64);
    let f1g9_38: i64 = (f1_2 as i64) * (g9_19 as i64);
    let f2g0: i64 = (f2 as i64) * (g0 as i64);
    let f2g1: i64 = (f2 as i64) * (g1 as i64);
    let f2g2: i64 = (f2 as i64) * (g2 as i64);
    let f2g3: i64 = (f2 as i64) * (g3 as i64);
    let f2g4: i64 = (f2 as i64) * (g4 as i64);
    let f2g5: i64 = (f2 as i64) * (g5 as i64);
    let f2g6: i64 = (f2 as i64) * (g6 as i64);
    let f2g7: i64 = (f2 as i64) * (g7 as i64);
    let f2g8_19: i64 = (f2 as i64) * (g8_19 as i64);
    let f2g9_19: i64 = (f2 as i64) * (g9_19 as i64);
    let f3g0: i64 = (f3 as i64) * (g0 as i64);
    let f3g1_2: i64 = (f3_2 as i64) * (g1 as i64);
    let f3g2: i64 = (f3 as i64) * (g2 as i64);
    let f3g3_2: i64 = (f3_2 as i64) * (g3 as i64);
    let f3g4: i64 = (f3 as i64) * (g4 as i64);
    let f3g5_2: i64 = (f3_2 as i64) * (g5 as i64);
    let f3g6: i64 = (f3 as i64) * (g6 as i64);
    let f3g7_38: i64 = (f3_2 as i64) * (g7_19 as i64);
    let f3g8_19: i64 = (f3 as i64) * (g8_19 as i64);
    let f3g9_38: i64 = (f3_2 as i64) * (g9_19 as i64);
    let f4g0: i64 = (f4 as i64) * (g0 as i64);
    let f4g1: i64 = (f4 as i64) * (g1 as i64);
    let f4g2: i64 = (f4 as i64) * (g2 as i64);
    let f4g3: i64 = (f4 as i64) * (g3 as i64);
    let f4g4: i64 = (f4 as i64) * (g4 as i64);
    let f4g5: i64 = (f4 as i64) * (g5 as i64);
    let f4g6_19: i64 = (f4 as i64) * (g6_19 as i64);
    let f4g7_19: i64 = (f4 as i64) * (g7_19 as i64);
    let f4g8_19: i64 = (f4 as i64) * (g8_19 as i64);
    let f4g9_19: i64 = (f4 as i64) * (g9_19 as i64);
    let f5g0: i64 = (f5 as i64) * (g0 as i64);
    let f5g1_2: i64 = (f5_2 as i64) * (g1 as i64);
    let f5g2: i64 = (f5 as i64) * (g2 as i64);
    let f5g3_2: i64 = (f5_2 as i64) * (g3 as i64);
    let f5g4: i64 = (f5 as i64) * (g4 as i64);
    let f5g5_38: i64 = (f5_2 as i64) * (g5_19 as i64);
    let f5g6_19: i64 = (f5 as i64) * (g6_19 as i64);
    let f5g7_38: i64 = (f5_2 as i64) * (g7_19 as i64);
    let f5g8_19: i64 = (f5 as i64) * (g8_19 as i64);
    let f5g9_38: i64 = (f5_2 as i64) * (g9_19 as i64);
    let f6g0: i64 = (f6 as i64) * (g0 as i64);
    let f6g1: i64 = (f6 as i64) * (g1 as i64);
    let f6g2: i64 = (f6 as i64) * (g2 as i64);
    let f6g3: i64 = (f6 as i64) * (g3 as i64);
    let f6g4_19: i64 = (f6 as i64) * (g4_19 as i64);
    let f6g5_19: i64 = (f6 as i64) * (g5_19 as i64);
    let f6g6_19: i64 = (f6 as i64) * (g6_19 as i64);
    let f6g7_19: i64 = (f6 as i64) * (g7_19 as i64);
    let f6g8_19: i64 = (f6 as i64) * (g8_19 as i64);
    let f6g9_19: i64 = (f6 as i64) * (g9_19 as i64);
    let f7g0: i64 = (f7 as i64) * (g0 as i64);
    let f7g1_2: i64 = (f7_2 as i64) * (g1 as i64);
    let f7g2: i64 = (f7 as i64) * (g2 as i64);
    let f7g3_38: i64 = (f7_2 as i64) * (g3_19 as i64);
    let f7g4_19: i64 = (f7 as i64) * (g4_19 as i64);
    let f7g5_38: i64 = (f7_2 as i64) * (g5_19 as i64);
    let f7g6_19: i64 = (f7 as i64) * (g6_19 as i64);
    let f7g7_38: i64 = (f7_2 as i64) * (g7_19 as i64);
    let f7g8_19: i64 = (f7 as i64) * (g8_19 as i64);
    let f7g9_38: i64 = (f7_2 as i64) * (g9_19 as i64);
    let f8g0: i64 = (f8 as i64) * (g0 as i64);
    let f8g1: i64 = (f8 as i64) * (g1 as i64);
    let f8g2_19: i64 = (f8 as i64) * (g2_19 as i64);
    let f8g3_19: i64 = (f8 as i64) * (g3_19 as i64);
    let f8g4_19: i64 = (f8 as i64) * (g4_19 as i64);
    let f8g5_19: i64 = (f8 as i64) * (g5_19 as i64);
    let f8g6_19: i64 = (f8 as i64) * (g6_19 as i64);
    let f8g7_19: i64 = (f8 as i64) * (g7_19 as i64);
    let f8g8_19: i64 = (f8 as i64) * (g8_19 as i64);
    let f8g9_19: i64 = (f8 as i64) * (g9_19 as i64);
    let f9g0: i64 = (f9 as i64) * (g0 as i64);
    let f9g1_38: i64 = (f9_2 as i64) * (g1_19 as i64);
    let f9g2_19: i64 = (f9 as i64) * (g2_19 as i64);
    let f9g3_38: i64 = (f9_2 as i64) * (g3_19 as i64);
    let f9g4_19: i64 = (f9 as i64) * (g4_19 as i64);
    let f9g5_38: i64 = (f9_2 as i64) * (g5_19 as i64);
    let f9g6_19: i64 = (f9 as i64) * (g6_19 as i64);
    let f9g7_38: i64 = (f9_2 as i64) * (g7_19 as i64);
    let f9g8_19: i64 = (f9 as i64) * (g8_19 as i64);
    let f9g9_38: i64 = (f9_2 as i64) * (g9_19 as i64);

    let mut h0: i64 = f0g0 + f1g9_38 + f2g8_19 + f3g7_38 + f4g6_19 + f5g5_38
        + f6g4_19 + f7g3_38 + f8g2_19 + f9g1_38;
    let mut h1: i64 = f0g1 + f1g0 + f2g9_19 + f3g8_19 + f4g7_19 + f5g6_19
        + f6g5_19 + f7g4_19 + f8g3_19 + f9g2_19;
    let mut h2: i64 = f0g2 + f1g1_2 + f2g0 + f3g9_38 + f4g8_19 + f5g7_38
        + f6g6_19 + f7g5_38 + f8g4_19 + f9g3_38;
    let mut h3: i64 = f0g3 + f1g2 + f2g1 + f3g0 + f4g9_19 + f5g8_19 + f6g7_19
        + f7g6_19 + f8g5_19 + f9g4_19;
    let mut h4: i64 = f0g4 + f1g3_2 + f2g2 + f3g1_2 + f4g0 + f5g9_38 + f6g8_19
        + f7g7_38 + f8g6_19 + f9g5_38;
    let mut h5: i64 = f0g5 + f1g4 + f2g3 + f3g2 + f4g1 + f5g0 + f6g9_19
        + f7g8_19 + f8g7_19 + f9g6_19;
    let mut h6: i64 = f0g6 + f1g5_2 + f2g4 + f3g3_2 + f4g2 + f5g1_2 + f6g0
        + f7g9_38 + f8g8_19 + f9g7_38;
    let mut h7: i64 = f0g7 + f1g6 + f2g5 + f3g4 + f4g3 + f5g2 + f6g1 + f7g0
        + f8g9_19 + f9g8_19;
    let mut h8: i64 = f0g8 + f1g7_2 + f2g6 + f3g5_2 + f4g4 + f5g3_2 + f6g2
        + f7g1_2 + f8g0 + f9g9_38;
    let mut h9: i64 =
        f0g9 + f1g8 + f2g7 + f3g6 + f4g5 + f5g4 + f6g3 + f7g2 + f8g1 + f9g0;

    let mut carry0: i64;
    let mut carry1: i64;
    let mut carry2: i64;
    let mut carry3: i64;
    let mut carry4: i64;
    let mut carry5: i64;
    let mut carry6: i64;
    let mut carry7: i64;
    let mut carry8: i64;
    let mut carry9: i64;

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));

    carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));

    carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));

    carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));

    *h.add(0) = h0 as i32;
    *h.add(1) = h1 as i32;
    *h.add(2) = h2 as i32;
    *h.add(3) = h3 as i32;
    *h.add(4) = h4 as i32;
    *h.add(5) = h5 as i32;
    *h.add(6) = h6 as i32;
    *h.add(7) = h7 as i32;
    *h.add(8) = h8 as i32;
    *h.add(9) = h9 as i32;
}

/* h = f * f ; can overlap h with f */
unsafe fn fe25519_sq(h: *mut i32, f: *const i32) {
    let f0 = *f.add(0);
    let f1 = *f.add(1);
    let f2 = *f.add(2);
    let f3 = *f.add(3);
    let f4 = *f.add(4);
    let f5 = *f.add(5);
    let f6 = *f.add(6);
    let f7 = *f.add(7);
    let f8 = *f.add(8);
    let f9 = *f.add(9);

    let f0_2: i32 = 2i32.wrapping_mul(f0);
    let f1_2: i32 = 2i32.wrapping_mul(f1);
    let f2_2: i32 = 2i32.wrapping_mul(f2);
    let f3_2: i32 = 2i32.wrapping_mul(f3);
    let f4_2: i32 = 2i32.wrapping_mul(f4);
    let f5_2: i32 = 2i32.wrapping_mul(f5);
    let f6_2: i32 = 2i32.wrapping_mul(f6);
    let f7_2: i32 = 2i32.wrapping_mul(f7);
    let f5_38: i32 = 38i32.wrapping_mul(f5);
    let f6_19: i32 = 19i32.wrapping_mul(f6);
    let f7_38: i32 = 38i32.wrapping_mul(f7);
    let f8_19: i32 = 19i32.wrapping_mul(f8);
    let f9_38: i32 = 38i32.wrapping_mul(f9);

    let f0f0: i64 = (f0 as i64) * (f0 as i64);
    let f0f1_2: i64 = (f0_2 as i64) * (f1 as i64);
    let f0f2_2: i64 = (f0_2 as i64) * (f2 as i64);
    let f0f3_2: i64 = (f0_2 as i64) * (f3 as i64);
    let f0f4_2: i64 = (f0_2 as i64) * (f4 as i64);
    let f0f5_2: i64 = (f0_2 as i64) * (f5 as i64);
    let f0f6_2: i64 = (f0_2 as i64) * (f6 as i64);
    let f0f7_2: i64 = (f0_2 as i64) * (f7 as i64);
    let f0f8_2: i64 = (f0_2 as i64) * (f8 as i64);
    let f0f9_2: i64 = (f0_2 as i64) * (f9 as i64);
    let f1f1_2: i64 = (f1_2 as i64) * (f1 as i64);
    let f1f2_2: i64 = (f1_2 as i64) * (f2 as i64);
    let f1f3_4: i64 = (f1_2 as i64) * (f3_2 as i64);
    let f1f4_2: i64 = (f1_2 as i64) * (f4 as i64);
    let f1f5_4: i64 = (f1_2 as i64) * (f5_2 as i64);
    let f1f6_2: i64 = (f1_2 as i64) * (f6 as i64);
    let f1f7_4: i64 = (f1_2 as i64) * (f7_2 as i64);
    let f1f8_2: i64 = (f1_2 as i64) * (f8 as i64);
    let f1f9_76: i64 = (f1_2 as i64) * (f9_38 as i64);
    let f2f2: i64 = (f2 as i64) * (f2 as i64);
    let f2f3_2: i64 = (f2_2 as i64) * (f3 as i64);
    let f2f4_2: i64 = (f2_2 as i64) * (f4 as i64);
    let f2f5_2: i64 = (f2_2 as i64) * (f5 as i64);
    let f2f6_2: i64 = (f2_2 as i64) * (f6 as i64);
    let f2f7_2: i64 = (f2_2 as i64) * (f7 as i64);
    let f2f8_38: i64 = (f2_2 as i64) * (f8_19 as i64);
    let f2f9_38: i64 = (f2 as i64) * (f9_38 as i64);
    let f3f3_2: i64 = (f3_2 as i64) * (f3 as i64);
    let f3f4_2: i64 = (f3_2 as i64) * (f4 as i64);
    let f3f5_4: i64 = (f3_2 as i64) * (f5_2 as i64);
    let f3f6_2: i64 = (f3_2 as i64) * (f6 as i64);
    let f3f7_76: i64 = (f3_2 as i64) * (f7_38 as i64);
    let f3f8_38: i64 = (f3_2 as i64) * (f8_19 as i64);
    let f3f9_76: i64 = (f3_2 as i64) * (f9_38 as i64);
    let f4f4: i64 = (f4 as i64) * (f4 as i64);
    let f4f5_2: i64 = (f4_2 as i64) * (f5 as i64);
    let f4f6_38: i64 = (f4_2 as i64) * (f6_19 as i64);
    let f4f7_38: i64 = (f4 as i64) * (f7_38 as i64);
    let f4f8_38: i64 = (f4_2 as i64) * (f8_19 as i64);
    let f4f9_38: i64 = (f4 as i64) * (f9_38 as i64);
    let f5f5_38: i64 = (f5 as i64) * (f5_38 as i64);
    let f5f6_38: i64 = (f5_2 as i64) * (f6_19 as i64);
    let f5f7_76: i64 = (f5_2 as i64) * (f7_38 as i64);
    let f5f8_38: i64 = (f5_2 as i64) * (f8_19 as i64);
    let f5f9_76: i64 = (f5_2 as i64) * (f9_38 as i64);
    let f6f6_19: i64 = (f6 as i64) * (f6_19 as i64);
    let f6f7_38: i64 = (f6 as i64) * (f7_38 as i64);
    let f6f8_38: i64 = (f6_2 as i64) * (f8_19 as i64);
    let f6f9_38: i64 = (f6 as i64) * (f9_38 as i64);
    let f7f7_38: i64 = (f7 as i64) * (f7_38 as i64);
    let f7f8_38: i64 = (f7_2 as i64) * (f8_19 as i64);
    let f7f9_76: i64 = (f7_2 as i64) * (f9_38 as i64);
    let f8f8_19: i64 = (f8 as i64) * (f8_19 as i64);
    let f8f9_38: i64 = (f8 as i64) * (f9_38 as i64);
    let f9f9_38: i64 = (f9 as i64) * (f9_38 as i64);

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
    let mut carry1: i64;
    let mut carry2: i64;
    let mut carry3: i64;
    let mut carry4: i64;
    let mut carry5: i64;
    let mut carry6: i64;
    let mut carry7: i64;
    let mut carry8: i64;
    let mut carry9: i64;

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));

    carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));

    carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));

    carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));

    *h.add(0) = h0 as i32;
    *h.add(1) = h1 as i32;
    *h.add(2) = h2 as i32;
    *h.add(3) = h3 as i32;
    *h.add(4) = h4 as i32;
    *h.add(5) = h5 as i32;
    *h.add(6) = h6 as i32;
    *h.add(7) = h7 as i32;
    *h.add(8) = h8 as i32;
    *h.add(9) = h9 as i32;
}

/* h = f * n, n a small unsigned scalar */
#[inline]
unsafe fn fe25519_mul32(h: *mut i32, f: *const i32, n: u32) {
    let sn: i64 = n as i64;
    let f0 = *f.add(0);
    let f1 = *f.add(1);
    let f2 = *f.add(2);
    let f3 = *f.add(3);
    let f4 = *f.add(4);
    let f5 = *f.add(5);
    let f6 = *f.add(6);
    let f7 = *f.add(7);
    let f8 = *f.add(8);
    let f9 = *f.add(9);
    let mut h0: i64 = (f0 as i64) * sn;
    let mut h1: i64 = (f1 as i64) * sn;
    let mut h2: i64 = (f2 as i64) * sn;
    let mut h3: i64 = (f3 as i64) * sn;
    let mut h4: i64 = (f4 as i64) * sn;
    let mut h5: i64 = (f5 as i64) * sn;
    let mut h6: i64 = (f6 as i64) * sn;
    let mut h7: i64 = (f7 as i64) * sn;
    let mut h8: i64 = (f8 as i64) * sn;
    let mut h9: i64 = (f9 as i64) * sn;
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

    carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));
    carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));
    carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));
    carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 = h8.wrapping_sub(carry8.wrapping_mul(1i64 << 26));

    *h.add(0) = h0 as i32;
    *h.add(1) = h1 as i32;
    *h.add(2) = h2 as i32;
    *h.add(3) = h3 as i32;
    *h.add(4) = h4 as i32;
    *h.add(5) = h5 as i32;
    *h.add(6) = h6 as i32;
    *h.add(7) = h7 as i32;
    *h.add(8) = h8 as i32;
    *h.add(9) = h9 as i32;
}

/* -------------------------------------------------------------------------
 * x25519_ref10.c
 * ------------------------------------------------------------------------- */

/*
 * Reject small order points early to mitigate the implications of
 * unexpected optimizations that would affect the ref10 code.
 * See https://eprint.iacr.org/2017/806.pdf for reference.
 */
unsafe fn has_small_order(s: *const u8) -> c_int {
    /* CRYPTO_ALIGN(16) static const unsigned char blocklist[][32] */
    static blocklist: [[u8; 32]; 7] = [
        /* 0 (order 4) */
        [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        /* 1 (order 1) */
        [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        /* 325606250916557431795983626356110631294008115727848805560023387167927233504
           (order 8) */
        [
            0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3,
            0xfa, 0xf1, 0x9f, 0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32,
            0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49, 0xb8, 0x00,
        ],
        /* 39382357235489614581723060781553021112529911719440698176882885853963445705823
           (order 8) */
        [
            0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1,
            0x55, 0x9c, 0x83, 0xef, 0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c,
            0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f, 0x11, 0x57,
        ],
        /* p-1 (order 2) */
        [
            0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
        ],
        /* p (=0, order 4) */
        [
            0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
        ],
        /* p+1 (=1, order 1) */
        [
            0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
        ],
    ];
    let mut c = [0u8; 7];
    let mut k: c_uint;
    let mut i: usize;
    let mut j: usize;

    j = 0;
    while j < 31 {
        i = 0;
        while i < 7 {
            c[i] |= *s.add(j) ^ blocklist[i][j];
            i += 1;
        }
        j += 1;
    }
    /* `j` is 31 here — the last byte is masked with 0x7f. */
    i = 0;
    while i < 7 {
        c[i] |= (*s.add(j) & 0x7f) ^ blocklist[i][j];
        i += 1;
    }
    k = 0;
    i = 0;
    while i < 7 {
        k |= ((c[i] as c_int) - 1) as c_uint;
        i += 1;
    }
    ((k >> 8) & 1) as c_int
}

unsafe extern "C" fn crypto_scalarmult_curve25519_ref10(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let mut t = [0u8; 32];
    let mut i: c_uint;
    let mut x1: Fe = [0; 10];
    let mut x2: Fe = [0; 10];
    let mut x3: Fe = [0; 10];
    let mut z2: Fe = [0; 10];
    let mut z3: Fe = [0; 10];
    let mut a: Fe = [0; 10];
    let mut b: Fe = [0; 10];
    let mut aa: Fe = [0; 10];
    let mut bb: Fe = [0; 10];
    let mut e: Fe = [0; 10];
    let mut da: Fe = [0; 10];
    let mut cb: Fe = [0; 10];
    let mut pos: c_int;
    let mut swap: c_uint;
    let mut bit: c_uint;

    let x1p = x1.as_mut_ptr();
    let x2p = x2.as_mut_ptr();
    let x3p = x3.as_mut_ptr();
    let z2p = z2.as_mut_ptr();
    let z3p = z3.as_mut_ptr();
    let ap = a.as_mut_ptr();
    let bp = b.as_mut_ptr();
    let aap = aa.as_mut_ptr();
    let bbp = bb.as_mut_ptr();
    let ep = e.as_mut_ptr();
    let dap = da.as_mut_ptr();
    let cbp = cb.as_mut_ptr();

    if has_small_order(p) != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        t[i as usize] = *n.add(i as usize);
        i += 1;
    }
    t[0] &= 248;
    t[31] &= 127;
    t[31] |= 64;
    _sodium_fe25519_frombytes(x1p, p);
    fe25519_1(x2p);
    fe25519_0(z2p);
    fe25519_copy(x3p, x1p);
    fe25519_1(z3p);

    swap = 0;
    pos = 254;
    while pos >= 0 {
        bit = (t[(pos / 8) as usize] >> (pos & 7)) as c_uint;
        bit &= 1;
        swap ^= bit;
        fe25519_cswap(x2p, x3p, swap);
        fe25519_cswap(z2p, z3p, swap);
        swap = bit;
        fe25519_add(ap, x2p, z2p);
        fe25519_sub_lazy(bp, x2p, z2p);
        fe25519_sq(aap, ap);
        fe25519_sq(bbp, bp);
        fe25519_mul(x2p, aap, bbp);
        fe25519_sub_lazy(ep, aap, bbp);
        fe25519_sub_lazy(dap, x3p, z3p);
        fe25519_mul(dap, dap, ap);
        fe25519_add(cbp, x3p, z3p);
        fe25519_mul(cbp, cbp, bp);
        fe25519_add(x3p, dap, cbp);
        fe25519_sq(x3p, x3p);
        fe25519_sub_lazy(z3p, dap, cbp);
        fe25519_sq(z3p, z3p);
        fe25519_mul(z3p, z3p, x1p);
        fe25519_mul32(z2p, ep, 121666);
        fe25519_add(z2p, z2p, bbp);
        fe25519_mul(z2p, z2p, ep);
        pos -= 1;
    }
    fe25519_cswap(x2p, x3p, swap);
    fe25519_cswap(z2p, z3p, swap);

    _sodium_fe25519_invert(z2p, z2p);
    fe25519_mul(x2p, x2p, z2p);
    _sodium_fe25519_tobytes(q, x2p);

    sodium_memzero(t.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 32]>());

    0
}

unsafe fn edwards_to_montgomery(
    montgomeryX: *mut i32,
    edwardsY: *const i32,
    edwardsZ: *const i32,
) {
    let mut tempX: Fe = [0; 10];
    let mut tempZ: Fe = [0; 10];
    let tempXp = tempX.as_mut_ptr();
    let tempZp = tempZ.as_mut_ptr();

    fe25519_add(tempXp, edwardsZ, edwardsY);
    fe25519_sub(tempZp, edwardsZ, edwardsY);
    _sodium_fe25519_invert(tempZp, tempZp);
    fe25519_mul(montgomeryX, tempXp, tempZp);
}

unsafe extern "C" fn crypto_scalarmult_curve25519_ref10_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    let t: *mut u8 = q;
    let mut A = ge25519_p3 {
        X: [0; 10],
        Y: [0; 10],
        Z: [0; 10],
        T: [0; 10],
    };
    let mut pk: Fe = [0; 10];
    let mut i: c_uint;

    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    *t.add(0) &= 248;
    *t.add(31) &= 127;
    *t.add(31) |= 64;
    _sodium_ge25519_scalarmult_base(&mut A as *mut ge25519_p3, t);
    edwards_to_montgomery(pk.as_mut_ptr(), A.Y.as_ptr(), A.Z.as_ptr());
    _sodium_fe25519_tobytes(q, pk.as_ptr());

    0
}

/* `struct crypto_scalarmult_curve25519_implementation` from the private header
 * `crypto_scalarmult/curve25519/scalarmult_curve25519.h`. */
#[repr(C)]
pub struct crypto_scalarmult_curve25519_implementation {
    pub mult:
        unsafe extern "C" fn(q: *mut u8, n: *const u8, p: *const u8) -> c_int,
    pub mult_base: unsafe extern "C" fn(q: *mut u8, n: *const u8) -> c_int,
}

#[unsafe(no_mangle)]
pub static crypto_scalarmult_curve25519_ref10_implementation:
    crypto_scalarmult_curve25519_implementation =
    crypto_scalarmult_curve25519_implementation {
        mult: crypto_scalarmult_curve25519_ref10,
        mult_base: crypto_scalarmult_curve25519_ref10_base,
    };
