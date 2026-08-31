//! Translation of c_src/libsodium/include/sodium/private/ed25519_ref10_fe_25_5.h
//!
//! HAVE_TI_MODE undefined: 32-bit limb variant (`fe25519 = int32_t[10]`).
//! This module is the shared home of the `fe25519` type, its inline field
//! operations, and the `ge25519` group-element struct definitions. Other
//! modules reach these via `crate::fe25519::...`.

use core::ffi::{c_int, c_uint};

// `fe25519_tobytes` is renamed to `_sodium_fe25519_tobytes` by
// private/quirks.h and is defined in another C file (an exported symbol).
// `sodium_is_zero` is likewise an exported symbol. Both are reached via
// `extern "C"` (contract rule 3).
extern "C" {
    fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> core::ffi::c_int;
}

/// fe means field element. Here the field is \Z/(2^255-19).
// HAVE_TI_MODE undefined: 32-bit limb variant.
pub type fe25519 = [i32; 10];

// ---------------------------------------------------------------------------
// Group element representations from private/ed25519_ref10.h (lines 43-76).
// ---------------------------------------------------------------------------

/// ge25519_p2 (projective): (X:Y:Z) satisfying x=X/Z, y=Y/Z
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p2 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
}

impl Default for ge25519_p2 {
    fn default() -> Self {
        ge25519_p2 {
            X: [0; 10],
            Y: [0; 10],
            Z: [0; 10],
        }
    }
}

/// ge25519_p3 (extended): (X:Y:Z:T) satisfying x=X/Z, y=Y/Z, XY=ZT
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p3 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

impl Default for ge25519_p3 {
    fn default() -> Self {
        ge25519_p3 {
            X: [0; 10],
            Y: [0; 10],
            Z: [0; 10],
            T: [0; 10],
        }
    }
}

/// ge25519_p1p1 (completed): ((X:Z),(Y:T)) satisfying x=X/Z, y=Y/T
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p1p1 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

impl Default for ge25519_p1p1 {
    fn default() -> Self {
        ge25519_p1p1 {
            X: [0; 10],
            Y: [0; 10],
            Z: [0; 10],
            T: [0; 10],
        }
    }
}

/// ge25519_precomp (Duif): (y+x,y-x,2dxy)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_precomp {
    pub yplusx: fe25519,
    pub yminusx: fe25519,
    pub xy2d: fe25519,
}

impl Default for ge25519_precomp {
    fn default() -> Self {
        ge25519_precomp {
            yplusx: [0; 10],
            yminusx: [0; 10],
            xy2d: [0; 10],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_cached {
    pub YplusX: fe25519,
    pub YminusX: fe25519,
    pub Z: fe25519,
    pub T2d: fe25519,
}

impl Default for ge25519_cached {
    fn default() -> Self {
        ge25519_cached {
            YplusX: [0; 10],
            YminusX: [0; 10],
            Z: [0; 10],
            T2d: [0; 10],
        }
    }
}

/*
 h = 0
 */
pub unsafe fn fe25519_0(h: *mut i32) {
    // memset(&h[0], 0, 10 * sizeof h[0]);
    core::ptr::write_bytes(h, 0, 10);
}

/*
 h = 1
 */
pub unsafe fn fe25519_1(h: *mut i32) {
    *h.add(0) = 1;
    *h.add(1) = 0;
    // memset(&h[2], 0, 8 * sizeof h[0]);
    core::ptr::write_bytes(h.add(2), 0, 8);
}

/*
 h = f + g
 Can overlap h with f or g.
 */
pub unsafe fn fe25519_add(h: *mut i32, f: *const i32, g: *const i32) {
    let h0: i32 = (*f.add(0)).wrapping_add(*g.add(0));
    let h1: i32 = (*f.add(1)).wrapping_add(*g.add(1));
    let h2: i32 = (*f.add(2)).wrapping_add(*g.add(2));
    let h3: i32 = (*f.add(3)).wrapping_add(*g.add(3));
    let h4: i32 = (*f.add(4)).wrapping_add(*g.add(4));
    let h5: i32 = (*f.add(5)).wrapping_add(*g.add(5));
    let h6: i32 = (*f.add(6)).wrapping_add(*g.add(6));
    let h7: i32 = (*f.add(7)).wrapping_add(*g.add(7));
    let h8: i32 = (*f.add(8)).wrapping_add(*g.add(8));
    let h9: i32 = (*f.add(9)).wrapping_add(*g.add(9));

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

/*
 h = f - g
 Can overlap h with f or g.
 */
pub unsafe fn fe25519_sub(h: *mut i32, f: *const i32, g: *const i32) {
    let h0: i32 = (*f.add(0)).wrapping_sub(*g.add(0));
    let h1: i32 = (*f.add(1)).wrapping_sub(*g.add(1));
    let h2: i32 = (*f.add(2)).wrapping_sub(*g.add(2));
    let h3: i32 = (*f.add(3)).wrapping_sub(*g.add(3));
    let h4: i32 = (*f.add(4)).wrapping_sub(*g.add(4));
    let h5: i32 = (*f.add(5)).wrapping_sub(*g.add(5));
    let h6: i32 = (*f.add(6)).wrapping_sub(*g.add(6));
    let h7: i32 = (*f.add(7)).wrapping_sub(*g.add(7));
    let h8: i32 = (*f.add(8)).wrapping_sub(*g.add(8));
    let h9: i32 = (*f.add(9)).wrapping_sub(*g.add(9));

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

/*
 h = -f
 */
pub unsafe fn fe25519_neg(h: *mut i32, f: *const i32) {
    let h0: i32 = (*f.add(0)).wrapping_neg();
    let h1: i32 = (*f.add(1)).wrapping_neg();
    let h2: i32 = (*f.add(2)).wrapping_neg();
    let h3: i32 = (*f.add(3)).wrapping_neg();
    let h4: i32 = (*f.add(4)).wrapping_neg();
    let h5: i32 = (*f.add(5)).wrapping_neg();
    let h6: i32 = (*f.add(6)).wrapping_neg();
    let h7: i32 = (*f.add(7)).wrapping_neg();
    let h8: i32 = (*f.add(8)).wrapping_neg();
    let h9: i32 = (*f.add(9)).wrapping_neg();

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

/*
 Replace (f,g) with (g,g) if b == 1;
 replace (f,g) with (f,g) if b == 0.

 Preconditions: b in {0,1}.
 */
pub unsafe fn fe25519_cmov(f: *mut i32, g: *const i32, b: c_uint) {
    // uint32_t mask = (uint32_t) (-(int32_t) b);
    let mask: u32 = (b as i32).wrapping_neg() as u32;
    let f0: i32;
    let f1: i32;
    let f2: i32;
    let f3: i32;
    let f4: i32;
    let f5: i32;
    let f6: i32;
    let f7: i32;
    let f8: i32;
    let f9: i32;
    let mut x0: i32;
    let mut x1: i32;
    let mut x2: i32;
    let mut x3: i32;
    let mut x4: i32;
    let mut x5: i32;
    let mut x6: i32;
    let mut x7: i32;
    let mut x8: i32;
    let mut x9: i32;

    f0 = *f.add(0);
    f1 = *f.add(1);
    f2 = *f.add(2);
    f3 = *f.add(3);
    f4 = *f.add(4);
    f5 = *f.add(5);
    f6 = *f.add(6);
    f7 = *f.add(7);
    f8 = *f.add(8);
    f9 = *f.add(9);

    x0 = f0 ^ *g.add(0);
    x1 = f1 ^ *g.add(1);
    x2 = f2 ^ *g.add(2);
    x3 = f3 ^ *g.add(3);
    x4 = f4 ^ *g.add(4);
    x5 = f5 ^ *g.add(5);
    x6 = f6 ^ *g.add(6);
    x7 = f7 ^ *g.add(7);
    x8 = f8 ^ *g.add(8);
    x9 = f9 ^ *g.add(9);

    // HAVE_INLINE_ASM undefined: no asm barrier.

    // x &= mask, where x is int32_t and mask is uint32_t: the C promotes
    // x to unsigned int for the &, then converts back to int32_t on assign.
    x0 &= mask as i32;
    x1 &= mask as i32;
    x2 &= mask as i32;
    x3 &= mask as i32;
    x4 &= mask as i32;
    x5 &= mask as i32;
    x6 &= mask as i32;
    x7 &= mask as i32;
    x8 &= mask as i32;
    x9 &= mask as i32;

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
}

pub unsafe fn fe25519_cswap(f: *mut i32, g: *mut i32, b: c_uint) {
    // uint32_t mask = (uint32_t) (-(int64_t) b);
    let mask: u32 = (b as i64).wrapping_neg() as u32;
    let f0: i32;
    let f1: i32;
    let f2: i32;
    let f3: i32;
    let f4: i32;
    let f5: i32;
    let f6: i32;
    let f7: i32;
    let f8: i32;
    let f9: i32;
    let g0: i32;
    let g1: i32;
    let g2: i32;
    let g3: i32;
    let g4: i32;
    let g5: i32;
    let g6: i32;
    let g7: i32;
    let g8: i32;
    let g9: i32;
    let mut x0: i32;
    let mut x1: i32;
    let mut x2: i32;
    let mut x3: i32;
    let mut x4: i32;
    let mut x5: i32;
    let mut x6: i32;
    let mut x7: i32;
    let mut x8: i32;
    let mut x9: i32;

    f0 = *f.add(0);
    f1 = *f.add(1);
    f2 = *f.add(2);
    f3 = *f.add(3);
    f4 = *f.add(4);
    f5 = *f.add(5);
    f6 = *f.add(6);
    f7 = *f.add(7);
    f8 = *f.add(8);
    f9 = *f.add(9);

    g0 = *g.add(0);
    g1 = *g.add(1);
    g2 = *g.add(2);
    g3 = *g.add(3);
    g4 = *g.add(4);
    g5 = *g.add(5);
    g6 = *g.add(6);
    g7 = *g.add(7);
    g8 = *g.add(8);
    g9 = *g.add(9);

    x0 = f0 ^ g0;
    x1 = f1 ^ g1;
    x2 = f2 ^ g2;
    x3 = f3 ^ g3;
    x4 = f4 ^ g4;
    x5 = f5 ^ g5;
    x6 = f6 ^ g6;
    x7 = f7 ^ g7;
    x8 = f8 ^ g8;
    x9 = f9 ^ g9;

    // HAVE_INLINE_ASM undefined: no asm barrier.

    x0 &= mask as i32;
    x1 &= mask as i32;
    x2 &= mask as i32;
    x3 &= mask as i32;
    x4 &= mask as i32;
    x5 &= mask as i32;
    x6 &= mask as i32;
    x7 &= mask as i32;
    x8 &= mask as i32;
    x9 &= mask as i32;

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

/*
 h = f
 */
pub unsafe fn fe25519_copy(h: *mut i32, f: *const i32) {
    // memcpy(h, f, 10 * sizeof h[0]);
    core::ptr::copy_nonoverlapping(f, h, 10);
}

/*
 return 1 if f is in {1,3,5,...,q-2}
 return 0 if f is in {0,2,4,...,q-1}
 */
pub unsafe fn fe25519_isnegative(f: *const i32) -> c_int {
    let mut s: [u8; 32] = [0; 32];

    // fe25519_tobytes is renamed _sodium_fe25519_tobytes (quirks.h).
    _sodium_fe25519_tobytes(s.as_mut_ptr(), f);

    (s[0] & 1) as c_int
}

/*
 return 1 if f == 0
 return 0 if f != 0
 */
pub unsafe fn fe25519_iszero(f: *const i32) -> c_int {
    let mut s: [u8; 32] = [0; 32];

    _sodium_fe25519_tobytes(s.as_mut_ptr(), f);

    sodium_is_zero(s.as_ptr(), 32)
}

/*
 h = f * g
 Can overlap h with f or g.
 */
pub unsafe fn fe25519_mul(h: *mut i32, f: *const i32, g: *const i32) {
    let f0: i32 = *f.add(0);
    let f1: i32 = *f.add(1);
    let f2: i32 = *f.add(2);
    let f3: i32 = *f.add(3);
    let f4: i32 = *f.add(4);
    let f5: i32 = *f.add(5);
    let f6: i32 = *f.add(6);
    let f7: i32 = *f.add(7);
    let f8: i32 = *f.add(8);
    let f9: i32 = *f.add(9);

    let g0: i32 = *g.add(0);
    let g1: i32 = *g.add(1);
    let g2: i32 = *g.add(2);
    let g3: i32 = *g.add(3);
    let g4: i32 = *g.add(4);
    let g5: i32 = *g.add(5);
    let g6: i32 = *g.add(6);
    let g7: i32 = *g.add(7);
    let g8: i32 = *g.add(8);
    let g9: i32 = *g.add(9);

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

/*
 h = f * f
 Can overlap h with f.
 */
pub unsafe fn fe25519_sq(h: *mut i32, f: *const i32) {
    let f0: i32 = *f.add(0);
    let f1: i32 = *f.add(1);
    let f2: i32 = *f.add(2);
    let f3: i32 = *f.add(3);
    let f4: i32 = *f.add(4);
    let f5: i32 = *f.add(5);
    let f6: i32 = *f.add(6);
    let f7: i32 = *f.add(7);
    let f8: i32 = *f.add(8);
    let f9: i32 = *f.add(9);

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

/*
 h = 2 * f * f
 Can overlap h with f.
 */
pub unsafe fn fe25519_sq2(h: *mut i32, f: *const i32) {
    let f0: i32 = *f.add(0);
    let f1: i32 = *f.add(1);
    let f2: i32 = *f.add(2);
    let f3: i32 = *f.add(3);
    let f4: i32 = *f.add(4);
    let f5: i32 = *f.add(5);
    let f6: i32 = *f.add(6);
    let f7: i32 = *f.add(7);
    let f8: i32 = *f.add(8);
    let f9: i32 = *f.add(9);

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

pub unsafe fn fe25519_mul32(h: *mut i32, f: *const i32, n: u32) {
    let sn: i64 = n as i64;
    let f0: i32 = *f.add(0);
    let f1: i32 = *f.add(1);
    let f2: i32 = *f.add(2);
    let f3: i32 = *f.add(3);
    let f4: i32 = *f.add(4);
    let f5: i32 = *f.add(5);
    let f6: i32 = *f.add(6);
    let f7: i32 = *f.add(7);
    let f8: i32 = *f.add(8);
    let f9: i32 = *f.add(9);
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
