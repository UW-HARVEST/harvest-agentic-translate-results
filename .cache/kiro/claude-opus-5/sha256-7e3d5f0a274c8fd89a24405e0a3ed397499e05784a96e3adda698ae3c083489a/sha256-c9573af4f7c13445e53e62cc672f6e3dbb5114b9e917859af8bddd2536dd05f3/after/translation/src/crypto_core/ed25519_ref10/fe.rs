//! Translation of the fe_25_5 field arithmetic for ed25519_ref10.
//!
//! Sources:
//! * `include/sodium/private/ed25519_ref10_fe_25_5.h`
//! * `crypto_core/ed25519/ref10/fe_25_5/fe.h`
//! * `crypto_core/ed25519/ref10/ed25519_ref10.c` (lines 1..262)
//!
//! `HAVE_TI_MODE` is undefined, so `fe25519` is `int32_t[10]`.

use core::ffi::c_int;

use super::base::fe25519_sqrtm1;
use super::{load_3, load_4};

/*
h = 0
*/
pub unsafe fn fe25519_0(h: *mut i32) {
    for i in 0..10 {
        *h.add(i) = 0;
    }
}

/*
h = 1
*/
pub unsafe fn fe25519_1(h: *mut i32) {
    *h.add(0) = 1;
    *h.add(1) = 0;
    for i in 2..10 {
        *h.add(i) = 0;
    }
}

/*
h = f + g
*/
pub unsafe fn fe25519_add(h: *mut i32, f: *const i32, g: *const i32) {
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

/*
h = f - g
*/
pub unsafe fn fe25519_sub(h: *mut i32, f: *const i32, g: *const i32) {
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

/*
h = -f
*/
pub unsafe fn fe25519_neg(h: *mut i32, f: *const i32) {
    let h0 = (*f.add(0)).wrapping_neg();
    let h1 = (*f.add(1)).wrapping_neg();
    let h2 = (*f.add(2)).wrapping_neg();
    let h3 = (*f.add(3)).wrapping_neg();
    let h4 = (*f.add(4)).wrapping_neg();
    let h5 = (*f.add(5)).wrapping_neg();
    let h6 = (*f.add(6)).wrapping_neg();
    let h7 = (*f.add(7)).wrapping_neg();
    let h8 = (*f.add(8)).wrapping_neg();
    let h9 = (*f.add(9)).wrapping_neg();

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
*/
pub unsafe fn fe25519_cmov(f: *mut i32, g: *const i32, b: u32) {
    let mask: u32 = (-(b as i32)) as u32;

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

    let mut x0 = f0 ^ *g.add(0);
    let mut x1 = f1 ^ *g.add(1);
    let mut x2 = f2 ^ *g.add(2);
    let mut x3 = f3 ^ *g.add(3);
    let mut x4 = f4 ^ *g.add(4);
    let mut x5 = f5 ^ *g.add(5);
    let mut x6 = f6 ^ *g.add(6);
    let mut x7 = f7 ^ *g.add(7);
    let mut x8 = f8 ^ *g.add(8);
    let mut x9 = f9 ^ *g.add(9);

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

pub unsafe fn fe25519_cswap(f: *mut i32, g: *mut i32, b: u32) {
    let mask: u32 = (-(b as i64)) as u32;

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
    for i in 0..10 {
        *h.add(i) = *f.add(i);
    }
}

/*
return 1 if f is in {1,3,5,...,q-2}
return 0 if f is in {0,2,4,...,q-1}
*/
pub unsafe fn fe25519_isnegative(f: *const i32) -> c_int {
    let mut s: [u8; 32] = [0; 32];

    fe25519_tobytes(s.as_mut_ptr(), f);

    (s[0] & 1) as c_int
}

/*
return 1 if f == 0
return 0 if f != 0
*/
pub unsafe fn fe25519_iszero(f: *const i32) -> c_int {
    let mut s: [u8; 32] = [0; 32];

    fe25519_tobytes(s.as_mut_ptr(), f);

    crate::sodium_utils::sodium_is_zero(s.as_ptr(), 32)
}

/*
h = f * g
*/
pub unsafe fn fe25519_mul(h: *mut i32, f: *const i32, g: *const i32) {
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

    let mut h0 = f0g0
        .wrapping_add(f1g9_38)
        .wrapping_add(f2g8_19)
        .wrapping_add(f3g7_38)
        .wrapping_add(f4g6_19)
        .wrapping_add(f5g5_38)
        .wrapping_add(f6g4_19)
        .wrapping_add(f7g3_38)
        .wrapping_add(f8g2_19)
        .wrapping_add(f9g1_38);
    let mut h1 = f0g1
        .wrapping_add(f1g0)
        .wrapping_add(f2g9_19)
        .wrapping_add(f3g8_19)
        .wrapping_add(f4g7_19)
        .wrapping_add(f5g6_19)
        .wrapping_add(f6g5_19)
        .wrapping_add(f7g4_19)
        .wrapping_add(f8g3_19)
        .wrapping_add(f9g2_19);
    let mut h2 = f0g2
        .wrapping_add(f1g1_2)
        .wrapping_add(f2g0)
        .wrapping_add(f3g9_38)
        .wrapping_add(f4g8_19)
        .wrapping_add(f5g7_38)
        .wrapping_add(f6g6_19)
        .wrapping_add(f7g5_38)
        .wrapping_add(f8g4_19)
        .wrapping_add(f9g3_38);
    let mut h3 = f0g3
        .wrapping_add(f1g2)
        .wrapping_add(f2g1)
        .wrapping_add(f3g0)
        .wrapping_add(f4g9_19)
        .wrapping_add(f5g8_19)
        .wrapping_add(f6g7_19)
        .wrapping_add(f7g6_19)
        .wrapping_add(f8g5_19)
        .wrapping_add(f9g4_19);
    let mut h4 = f0g4
        .wrapping_add(f1g3_2)
        .wrapping_add(f2g2)
        .wrapping_add(f3g1_2)
        .wrapping_add(f4g0)
        .wrapping_add(f5g9_38)
        .wrapping_add(f6g8_19)
        .wrapping_add(f7g7_38)
        .wrapping_add(f8g6_19)
        .wrapping_add(f9g5_38);
    let mut h5 = f0g5
        .wrapping_add(f1g4)
        .wrapping_add(f2g3)
        .wrapping_add(f3g2)
        .wrapping_add(f4g1)
        .wrapping_add(f5g0)
        .wrapping_add(f6g9_19)
        .wrapping_add(f7g8_19)
        .wrapping_add(f8g7_19)
        .wrapping_add(f9g6_19);
    let mut h6 = f0g6
        .wrapping_add(f1g5_2)
        .wrapping_add(f2g4)
        .wrapping_add(f3g3_2)
        .wrapping_add(f4g2)
        .wrapping_add(f5g1_2)
        .wrapping_add(f6g0)
        .wrapping_add(f7g9_38)
        .wrapping_add(f8g8_19)
        .wrapping_add(f9g7_38);
    let mut h7 = f0g7
        .wrapping_add(f1g6)
        .wrapping_add(f2g5)
        .wrapping_add(f3g4)
        .wrapping_add(f4g3)
        .wrapping_add(f5g2)
        .wrapping_add(f6g1)
        .wrapping_add(f7g0)
        .wrapping_add(f8g9_19)
        .wrapping_add(f9g8_19);
    let mut h8 = f0g8
        .wrapping_add(f1g7_2)
        .wrapping_add(f2g6)
        .wrapping_add(f3g5_2)
        .wrapping_add(f4g4)
        .wrapping_add(f5g3_2)
        .wrapping_add(f6g2)
        .wrapping_add(f7g1_2)
        .wrapping_add(f8g0)
        .wrapping_add(f9g9_38);
    let mut h9 = f0g9
        .wrapping_add(f1g8)
        .wrapping_add(f2g7)
        .wrapping_add(f3g6)
        .wrapping_add(f4g5)
        .wrapping_add(f5g4)
        .wrapping_add(f6g3)
        .wrapping_add(f7g2)
        .wrapping_add(f8g1)
        .wrapping_add(f9g0);

    let mut carry0;
    let carry1;
    let carry2;
    let carry3;
    let mut carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 << 26) as i64));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 << 26) as i64));

    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 << 25) as i64));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 << 25) as i64));

    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 << 26) as i64));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 << 26) as i64));

    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 << 25) as i64));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 << 25) as i64));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 << 26) as i64));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 << 26) as i64));

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 << 25) as i64));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 << 26) as i64));

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
*/
pub unsafe fn fe25519_sq(h: *mut i32, f: *const i32) {
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

    let mut h0 = f0f0
        .wrapping_add(f1f9_76)
        .wrapping_add(f2f8_38)
        .wrapping_add(f3f7_76)
        .wrapping_add(f4f6_38)
        .wrapping_add(f5f5_38);
    let mut h1 = f0f1_2
        .wrapping_add(f2f9_38)
        .wrapping_add(f3f8_38)
        .wrapping_add(f4f7_38)
        .wrapping_add(f5f6_38);
    let mut h2 = f0f2_2
        .wrapping_add(f1f1_2)
        .wrapping_add(f3f9_76)
        .wrapping_add(f4f8_38)
        .wrapping_add(f5f7_76)
        .wrapping_add(f6f6_19);
    let mut h3 = f0f3_2
        .wrapping_add(f1f2_2)
        .wrapping_add(f4f9_38)
        .wrapping_add(f5f8_38)
        .wrapping_add(f6f7_38);
    let mut h4 = f0f4_2
        .wrapping_add(f1f3_4)
        .wrapping_add(f2f2)
        .wrapping_add(f5f9_76)
        .wrapping_add(f6f8_38)
        .wrapping_add(f7f7_38);
    let mut h5 = f0f5_2
        .wrapping_add(f1f4_2)
        .wrapping_add(f2f3_2)
        .wrapping_add(f6f9_38)
        .wrapping_add(f7f8_38);
    let mut h6 = f0f6_2
        .wrapping_add(f1f5_4)
        .wrapping_add(f2f4_2)
        .wrapping_add(f3f3_2)
        .wrapping_add(f7f9_76)
        .wrapping_add(f8f8_19);
    let mut h7 = f0f7_2
        .wrapping_add(f1f6_2)
        .wrapping_add(f2f5_2)
        .wrapping_add(f3f4_2)
        .wrapping_add(f8f9_38);
    let mut h8 = f0f8_2
        .wrapping_add(f1f7_4)
        .wrapping_add(f2f6_2)
        .wrapping_add(f3f5_4)
        .wrapping_add(f4f4)
        .wrapping_add(f9f9_38);
    let mut h9 = f0f9_2
        .wrapping_add(f1f8_2)
        .wrapping_add(f2f7_2)
        .wrapping_add(f3f6_2)
        .wrapping_add(f4f5_2);

    let mut carry0;
    let carry1;
    let carry2;
    let carry3;
    let mut carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 << 26) as i64));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 << 26) as i64));

    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 << 25) as i64));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 << 25) as i64));

    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 << 26) as i64));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 << 26) as i64));

    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 << 25) as i64));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 << 25) as i64));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 << 26) as i64));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 << 26) as i64));

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 << 25) as i64));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 << 26) as i64));

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
*/
pub unsafe fn fe25519_sq2(h: *mut i32, f: *const i32) {
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

    let mut h0 = f0f0
        .wrapping_add(f1f9_76)
        .wrapping_add(f2f8_38)
        .wrapping_add(f3f7_76)
        .wrapping_add(f4f6_38)
        .wrapping_add(f5f5_38);
    let mut h1 = f0f1_2
        .wrapping_add(f2f9_38)
        .wrapping_add(f3f8_38)
        .wrapping_add(f4f7_38)
        .wrapping_add(f5f6_38);
    let mut h2 = f0f2_2
        .wrapping_add(f1f1_2)
        .wrapping_add(f3f9_76)
        .wrapping_add(f4f8_38)
        .wrapping_add(f5f7_76)
        .wrapping_add(f6f6_19);
    let mut h3 = f0f3_2
        .wrapping_add(f1f2_2)
        .wrapping_add(f4f9_38)
        .wrapping_add(f5f8_38)
        .wrapping_add(f6f7_38);
    let mut h4 = f0f4_2
        .wrapping_add(f1f3_4)
        .wrapping_add(f2f2)
        .wrapping_add(f5f9_76)
        .wrapping_add(f6f8_38)
        .wrapping_add(f7f7_38);
    let mut h5 = f0f5_2
        .wrapping_add(f1f4_2)
        .wrapping_add(f2f3_2)
        .wrapping_add(f6f9_38)
        .wrapping_add(f7f8_38);
    let mut h6 = f0f6_2
        .wrapping_add(f1f5_4)
        .wrapping_add(f2f4_2)
        .wrapping_add(f3f3_2)
        .wrapping_add(f7f9_76)
        .wrapping_add(f8f8_19);
    let mut h7 = f0f7_2
        .wrapping_add(f1f6_2)
        .wrapping_add(f2f5_2)
        .wrapping_add(f3f4_2)
        .wrapping_add(f8f9_38);
    let mut h8 = f0f8_2
        .wrapping_add(f1f7_4)
        .wrapping_add(f2f6_2)
        .wrapping_add(f3f5_4)
        .wrapping_add(f4f4)
        .wrapping_add(f9f9_38);
    let mut h9 = f0f9_2
        .wrapping_add(f1f8_2)
        .wrapping_add(f2f7_2)
        .wrapping_add(f3f6_2)
        .wrapping_add(f4f5_2);

    let mut carry0;
    let carry1;
    let carry2;
    let carry3;
    let mut carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

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
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 << 26) as i64));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 << 26) as i64));

    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 << 25) as i64));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 << 25) as i64));

    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 << 26) as i64));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 << 26) as i64));

    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 << 25) as i64));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 << 25) as i64));

    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 << 26) as i64));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 << 26) as i64));

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 << 25) as i64));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 << 26) as i64));

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
    let mut h0 = (f0 as i64).wrapping_mul(sn);
    let mut h1 = (f1 as i64).wrapping_mul(sn);
    let mut h2 = (f2 as i64).wrapping_mul(sn);
    let mut h3 = (f3 as i64).wrapping_mul(sn);
    let mut h4 = (f4 as i64).wrapping_mul(sn);
    let mut h5 = (f5 as i64).wrapping_mul(sn);
    let mut h6 = (f6 as i64).wrapping_mul(sn);
    let mut h7 = (f7 as i64).wrapping_mul(sn);
    let mut h8 = (f8 as i64).wrapping_mul(sn);
    let mut h9 = (f9 as i64).wrapping_mul(sn);
    let carry0;
    let carry1;
    let carry2;
    let carry3;
    let carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

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

/*
Ignores top bit of s.
*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_frombytes(h: *mut i32, s: *const u8) {
    let mut h0 = load_4(s) as i64;
    let mut h1 = (load_3(s.add(4)) << 6) as i64;
    let mut h2 = (load_3(s.add(7)) << 5) as i64;
    let mut h3 = (load_3(s.add(10)) << 3) as i64;
    let mut h4 = (load_3(s.add(13)) << 2) as i64;
    let mut h5 = load_4(s.add(16)) as i64;
    let mut h6 = (load_3(s.add(20)) << 7) as i64;
    let mut h7 = (load_3(s.add(23)) << 5) as i64;
    let mut h8 = (load_3(s.add(26)) << 4) as i64;
    let mut h9 = ((load_3(s.add(29)) & 8388607) << 2) as i64;

    let carry0;
    let carry1;
    let carry2;
    let carry3;
    let carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

    carry9 = (h9.wrapping_add(1i64 << 24)) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 << 25) as i64));
    carry1 = (h1.wrapping_add(1i64 << 24)) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 << 25) as i64));
    carry3 = (h3.wrapping_add(1i64 << 24)) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 << 25) as i64));
    carry5 = (h5.wrapping_add(1i64 << 24)) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 << 25) as i64));
    carry7 = (h7.wrapping_add(1i64 << 24)) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 << 25) as i64));

    carry0 = (h0.wrapping_add(1i64 << 25)) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 << 26) as i64));
    carry2 = (h2.wrapping_add(1i64 << 25)) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 << 26) as i64));
    carry4 = (h4.wrapping_add(1i64 << 25)) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 << 26) as i64));
    carry6 = (h6.wrapping_add(1i64 << 25)) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 << 26) as i64));
    carry8 = (h8.wrapping_add(1i64 << 25)) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 << 26) as i64));

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

/// Convenience wrapper for internal callers using the C name.
#[inline]
pub unsafe fn fe25519_frombytes(h: *mut i32, s: *const u8) {
    _sodium_fe25519_frombytes(h, s)
}

pub unsafe fn fe25519_reduce(h: *mut i32, f: *const i32) {
    let mut h0 = *f.add(0);
    let mut h1 = *f.add(1);
    let mut h2 = *f.add(2);
    let mut h3 = *f.add(3);
    let mut h4 = *f.add(4);
    let mut h5 = *f.add(5);
    let mut h6 = *f.add(6);
    let mut h7 = *f.add(7);
    let mut h8 = *f.add(8);
    let mut h9 = *f.add(9);

    let mut q: i32;
    let carry0;
    let carry1;
    let carry2;
    let carry3;
    let carry4;
    let carry5;
    let carry6;
    let carry7;
    let carry8;
    let carry9;

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
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u32 << 26) as i32));
    carry1 = h1 >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u32 << 25) as i32));
    carry2 = h2 >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u32 << 26) as i32));
    carry3 = h3 >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u32 << 25) as i32));
    carry4 = h4 >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u32 << 26) as i32));
    carry5 = h5 >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u32 << 25) as i32));
    carry6 = h6 >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u32 << 26) as i32));
    carry7 = h7 >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u32 << 25) as i32));
    carry8 = h8 >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u32 << 26) as i32));
    carry9 = h9 >> 25;
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u32 << 25) as i32));

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32) {
    let mut t: [i32; 10] = [0; 10];

    fe25519_reduce(t.as_mut_ptr(), h);
    let t = &t;

    *s.add(0) = (t[0] >> 0) as u8;
    *s.add(1) = (t[0] >> 8) as u8;
    *s.add(2) = (t[0] >> 16) as u8;
    *s.add(3) = (((t[0] >> 24) as u32) | ((t[1] as u32).wrapping_mul(1u32 << 2))) as u8;
    *s.add(4) = (t[1] >> 6) as u8;
    *s.add(5) = (t[1] >> 14) as u8;
    *s.add(6) = (((t[1] >> 22) as u32) | ((t[2] as u32).wrapping_mul(1u32 << 3))) as u8;
    *s.add(7) = (t[2] >> 5) as u8;
    *s.add(8) = (t[2] >> 13) as u8;
    *s.add(9) = (((t[2] >> 21) as u32) | ((t[3] as u32).wrapping_mul(1u32 << 5))) as u8;
    *s.add(10) = (t[3] >> 3) as u8;
    *s.add(11) = (t[3] >> 11) as u8;
    *s.add(12) = (((t[3] >> 19) as u32) | ((t[4] as u32).wrapping_mul(1u32 << 6))) as u8;
    *s.add(13) = (t[4] >> 2) as u8;
    *s.add(14) = (t[4] >> 10) as u8;
    *s.add(15) = (t[4] >> 18) as u8;
    *s.add(16) = (t[5] >> 0) as u8;
    *s.add(17) = (t[5] >> 8) as u8;
    *s.add(18) = (t[5] >> 16) as u8;
    *s.add(19) = (((t[5] >> 24) as u32) | ((t[6] as u32).wrapping_mul(1u32 << 1))) as u8;
    *s.add(20) = (t[6] >> 7) as u8;
    *s.add(21) = (t[6] >> 15) as u8;
    *s.add(22) = (((t[6] >> 23) as u32) | ((t[7] as u32).wrapping_mul(1u32 << 3))) as u8;
    *s.add(23) = (t[7] >> 5) as u8;
    *s.add(24) = (t[7] >> 13) as u8;
    *s.add(25) = (((t[7] >> 21) as u32) | ((t[8] as u32).wrapping_mul(1u32 << 4))) as u8;
    *s.add(26) = (t[8] >> 4) as u8;
    *s.add(27) = (t[8] >> 12) as u8;
    *s.add(28) = (((t[8] >> 20) as u32) | ((t[9] as u32).wrapping_mul(1u32 << 6))) as u8;
    *s.add(29) = (t[9] >> 2) as u8;
    *s.add(30) = (t[9] >> 10) as u8;
    *s.add(31) = (t[9] >> 18) as u8;
}

/// Convenience wrapper for internal callers using the C name.
#[inline]
pub unsafe fn fe25519_tobytes(s: *mut u8, h: *const i32) {
    _sodium_fe25519_tobytes(s, h)
}

pub unsafe fn fe25519_sqmul(s: *mut i32, n: c_int, a: *const i32) {
    let mut i = 0;
    while i < n {
        fe25519_sq(s, s);
        i += 1;
    }
    fe25519_mul(s, s, a);
}

/*
 * Inversion - sets out to 0 if z=0
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_invert(out: *mut i32, z: *const i32) {
    let mut t0: [i32; 10] = [0; 10];
    let mut t1: [i32; 10] = [0; 10];
    let mut t2: [i32; 10] = [0; 10];
    let mut t3: [i32; 10] = [0; 10];
    let mut i: c_int;

    let t0 = t0.as_mut_ptr();
    let t1 = t1.as_mut_ptr();
    let t2 = t2.as_mut_ptr();
    let t3 = t3.as_mut_ptr();

    fe25519_sq(t0, z);
    fe25519_sq(t1, t0);
    fe25519_sq(t1, t1);
    fe25519_mul(t1, z, t1);
    fe25519_mul(t0, t0, t1);
    fe25519_sq(t2, t0);
    fe25519_mul(t1, t1, t2);
    fe25519_sq(t2, t1);
    i = 1;
    while i < 5 {
        fe25519_sq(t2, t2);
        i += 1;
    }
    fe25519_mul(t1, t2, t1);
    fe25519_sq(t2, t1);
    i = 1;
    while i < 10 {
        fe25519_sq(t2, t2);
        i += 1;
    }
    fe25519_mul(t2, t2, t1);
    fe25519_sq(t3, t2);
    i = 1;
    while i < 20 {
        fe25519_sq(t3, t3);
        i += 1;
    }
    fe25519_mul(t2, t3, t2);
    i = 1;
    while i < 11 {
        fe25519_sq(t2, t2);
        i += 1;
    }
    fe25519_mul(t1, t2, t1);
    fe25519_sq(t2, t1);
    i = 1;
    while i < 50 {
        fe25519_sq(t2, t2);
        i += 1;
    }
    fe25519_mul(t2, t2, t1);
    fe25519_sq(t3, t2);
    i = 1;
    while i < 100 {
        fe25519_sq(t3, t3);
        i += 1;
    }
    fe25519_mul(t2, t3, t2);
    i = 1;
    while i < 51 {
        fe25519_sq(t2, t2);
        i += 1;
    }
    fe25519_mul(t1, t2, t1);
    i = 1;
    while i < 6 {
        fe25519_sq(t1, t1);
        i += 1;
    }
    fe25519_mul(out, t1, t0);
}

/// Convenience wrapper for internal callers using the C name.
#[inline]
pub unsafe fn fe25519_invert(out: *mut i32, z: *const i32) {
    _sodium_fe25519_invert(out, z)
}

/*
 * returns z^((p-5)/8) = z^(2^252-3)
 */
pub unsafe fn fe25519_pow22523(out: *mut i32, z: *const i32) {
    let mut t0: [i32; 10] = [0; 10];
    let mut t1: [i32; 10] = [0; 10];
    let mut t2: [i32; 10] = [0; 10];
    let mut i: c_int;

    let t0 = t0.as_mut_ptr();
    let t1 = t1.as_mut_ptr();
    let t2 = t2.as_mut_ptr();

    fe25519_sq(t0, z);
    fe25519_sq(t1, t0);
    fe25519_sq(t1, t1);
    fe25519_mul(t1, z, t1);
    fe25519_mul(t0, t0, t1);
    fe25519_sq(t0, t0);
    fe25519_mul(t0, t1, t0);
    fe25519_sq(t1, t0);
    i = 1;
    while i < 5 {
        fe25519_sq(t1, t1);
        i += 1;
    }
    fe25519_mul(t0, t1, t0);
    fe25519_sq(t1, t0);
    i = 1;
    while i < 10 {
        fe25519_sq(t1, t1);
        i += 1;
    }
    fe25519_mul(t1, t1, t0);
    fe25519_sq(t2, t1);
    i = 1;
    while i < 20 {
        fe25519_sq(t2, t2);
        i += 1;
    }
    fe25519_mul(t1, t2, t1);
    i = 1;
    while i < 11 {
        fe25519_sq(t1, t1);
        i += 1;
    }
    fe25519_mul(t0, t1, t0);
    fe25519_sq(t1, t0);
    i = 1;
    while i < 50 {
        fe25519_sq(t1, t1);
        i += 1;
    }
    fe25519_mul(t1, t1, t0);
    fe25519_sq(t2, t1);
    i = 1;
    while i < 100 {
        fe25519_sq(t2, t2);
        i += 1;
    }
    fe25519_mul(t1, t2, t1);
    i = 1;
    while i < 51 {
        fe25519_sq(t1, t1);
        i += 1;
    }
    fe25519_mul(t0, t1, t0);
    fe25519_sq(t0, t0);
    fe25519_sq(t0, t0);
    fe25519_mul(out, t0, z);
}

pub unsafe fn fe25519_cneg(h: *mut i32, b: u32) {
    let mut negf: [i32; 10] = [0; 10];

    fe25519_neg(negf.as_mut_ptr(), h);
    fe25519_cmov(h, negf.as_ptr(), b);
}

pub unsafe fn fe25519_abs(h: *mut i32) {
    fe25519_cneg(h, fe25519_isnegative(h) as u32);
}

pub unsafe fn fe25519_unchecked_sqrt(x: *mut i32, x2: *const i32) {
    let mut p_root: [i32; 10] = [0; 10];
    let mut m_root: [i32; 10] = [0; 10];
    let mut m_root2: [i32; 10] = [0; 10];
    let mut e: [i32; 10] = [0; 10];

    let p_root = p_root.as_mut_ptr();
    let m_root = m_root.as_mut_ptr();
    let m_root2 = m_root2.as_mut_ptr();
    let e = e.as_mut_ptr();

    fe25519_pow22523(e, x2);
    fe25519_mul(p_root, e, x2);
    fe25519_mul(m_root, p_root, fe25519_sqrtm1.as_ptr());
    fe25519_sq(m_root2, m_root);
    fe25519_sub(e, x2, m_root2);
    fe25519_copy(x, p_root);
    fe25519_cmov(x, m_root, fe25519_iszero(e) as u32);
}

pub unsafe fn fe25519_sqrt(x: *mut i32, x2: *const i32) -> c_int {
    let mut check: [i32; 10] = [0; 10];
    let mut x2_copy: [i32; 10] = [0; 10];

    let check = check.as_mut_ptr();
    let x2_copy = x2_copy.as_mut_ptr();

    fe25519_copy(x2_copy, x2);
    fe25519_unchecked_sqrt(x, x2);
    fe25519_sq(check, x);
    fe25519_sub(check, check, x2_copy);

    fe25519_iszero(check) - 1
}

pub unsafe fn fe25519_notsquare(x: *const i32) -> c_int {
    let mut _10: [i32; 10] = [0; 10];
    let mut _11: [i32; 10] = [0; 10];
    let mut _1100: [i32; 10] = [0; 10];
    let mut _1111: [i32; 10] = [0; 10];
    let mut _11110000: [i32; 10] = [0; 10];
    let mut _11111111: [i32; 10] = [0; 10];
    let mut t: [i32; 10] = [0; 10];
    let mut u: [i32; 10] = [0; 10];
    let mut v: [i32; 10] = [0; 10];
    let mut s: [u8; 32] = [0; 32];

    let _10 = _10.as_mut_ptr();
    let _11 = _11.as_mut_ptr();
    let _1100 = _1100.as_mut_ptr();
    let _1111 = _1111.as_mut_ptr();
    let _11110000 = _11110000.as_mut_ptr();
    let _11111111 = _11111111.as_mut_ptr();
    let t = t.as_mut_ptr();
    let u = u.as_mut_ptr();
    let v = v.as_mut_ptr();

    /* Jacobi symbol - x^((p-1)/2) */
    fe25519_mul(_10, x, x);
    fe25519_mul(_11, x, _10);
    fe25519_sq(_1100, _11);
    fe25519_sq(_1100, _1100);
    fe25519_mul(_1111, _11, _1100);
    fe25519_sq(_11110000, _1111);
    fe25519_sq(_11110000, _11110000);
    fe25519_sq(_11110000, _11110000);
    fe25519_sq(_11110000, _11110000);
    fe25519_mul(_11111111, _1111, _11110000);
    fe25519_copy(t, _11111111);
    fe25519_sqmul(t, 2, _11);
    fe25519_copy(u, t);
    fe25519_sqmul(t, 10, u);
    fe25519_sqmul(t, 10, u);
    fe25519_copy(v, t);
    fe25519_sqmul(t, 30, v);
    fe25519_copy(v, t);
    fe25519_sqmul(t, 60, v);
    fe25519_copy(v, t);
    fe25519_sqmul(t, 120, v);
    fe25519_sqmul(t, 10, u);
    fe25519_sqmul(t, 3, _11);
    fe25519_sq(t, t);

    fe25519_tobytes(s.as_mut_ptr(), t);

    (s[1] & 1) as c_int
}
