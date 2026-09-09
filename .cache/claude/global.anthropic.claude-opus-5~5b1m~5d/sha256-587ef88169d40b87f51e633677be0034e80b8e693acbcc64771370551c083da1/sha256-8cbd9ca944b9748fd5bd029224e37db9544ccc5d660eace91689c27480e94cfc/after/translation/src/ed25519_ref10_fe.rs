//! Field arithmetic (fe25519, fe_25_5 representation) for ed25519 ref10.
//!
//! Translated from:
//!  - `c_src/libsodium/include/sodium/private/ed25519_ref10_fe_25_5.h`
//!  - `c_src/libsodium/crypto_core/ed25519/ref10/fe_25_5/fe.h`
//!  - `c_src/libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c` (lines 1-262)
//!
//! `HAVE_TI_MODE` is not defined in the reference build, so `fe25519` is
//! `int32_t[10]` (`crate::types::fe25519`).
//!
//! This module is shared: many other translated modules call these functions
//! via `use crate::ed25519_ref10_fe::*;`. All functions take `&`/`&mut`
//! references (not raw pointers) except `load_3`/`load_4` and the
//! `#[no_mangle]` extern wrappers, so that callers cannot violate aliasing
//! rules that the original C code relies on (C allows `fe25519_mul(h, h, g)`
//! because it reads all inputs into locals first; we mirror that by copying
//! inputs into locals before writing outputs).

use crate::ed25519_ref10_tables::*;
use crate::types::fe25519;

/// `static inline uint64_t load_3(const unsigned char *in)`
#[inline]
pub unsafe fn load_3(inp: *const u8) -> u64 {
    let mut result: u64 = *inp as u64;
    result |= (*inp.add(1) as u64) << 8;
    result |= (*inp.add(2) as u64) << 16;
    result
}

/// `static inline uint64_t load_4(const unsigned char *in)`
#[inline]
pub unsafe fn load_4(inp: *const u8) -> u64 {
    let mut result: u64 = *inp as u64;
    result |= (*inp.add(1) as u64) << 8;
    result |= (*inp.add(2) as u64) << 16;
    result |= (*inp.add(3) as u64) << 24;
    result
}

/// `h = 0`
#[inline]
pub fn fe25519_0(h: &mut fe25519) {
    for x in h.iter_mut() {
        *x = 0;
    }
}

/// `h = 1`
#[inline]
pub fn fe25519_1(h: &mut fe25519) {
    h[0] = 1;
    for x in h.iter_mut().skip(1) {
        *x = 0;
    }
}

/// `h = f + g`
#[inline]
pub fn fe25519_add(h: &mut fe25519, f: &fe25519, g: &fe25519) {
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

    let h0 = f0.wrapping_add(g0);
    let h1 = f1.wrapping_add(g1);
    let h2 = f2.wrapping_add(g2);
    let h3 = f3.wrapping_add(g3);
    let h4 = f4.wrapping_add(g4);
    let h5 = f5.wrapping_add(g5);
    let h6 = f6.wrapping_add(g6);
    let h7 = f7.wrapping_add(g7);
    let h8 = f8.wrapping_add(g8);
    let h9 = f9.wrapping_add(g9);

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
pub fn fe25519_sub(h: &mut fe25519, f: &fe25519, g: &fe25519) {
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

    let h0 = f0.wrapping_sub(g0);
    let h1 = f1.wrapping_sub(g1);
    let h2 = f2.wrapping_sub(g2);
    let h3 = f3.wrapping_sub(g3);
    let h4 = f4.wrapping_sub(g4);
    let h5 = f5.wrapping_sub(g5);
    let h6 = f6.wrapping_sub(g6);
    let h7 = f7.wrapping_sub(g7);
    let h8 = f8.wrapping_sub(g8);
    let h9 = f9.wrapping_sub(g9);

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
pub fn fe25519_neg(h: &mut fe25519, f: &fe25519) {
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

    let h0 = f0.wrapping_neg();
    let h1 = f1.wrapping_neg();
    let h2 = f2.wrapping_neg();
    let h3 = f3.wrapping_neg();
    let h4 = f4.wrapping_neg();
    let h5 = f5.wrapping_neg();
    let h6 = f6.wrapping_neg();
    let h7 = f7.wrapping_neg();
    let h8 = f8.wrapping_neg();
    let h9 = f9.wrapping_neg();

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

/// `fe25519_cmov`: Replace (f,g) with (g,g) if b == 1; (f,g) with (f,g) if b == 0.
#[inline]
pub fn fe25519_cmov(f: &mut fe25519, g: &fe25519, b: u32) {
    let mask: u32 = (b as i32).wrapping_neg() as u32;

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

/// `fe25519_cswap`
#[inline]
pub fn fe25519_cswap(f: &mut fe25519, g: &mut fe25519, b: u32) {
    let mask: u32 = (b as i64).wrapping_neg() as u32;

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
pub fn fe25519_copy(h: &mut fe25519, f: &fe25519) {
    h.copy_from_slice(f);
}

/// return 1 if f is in {1,3,5,...,q-2}, 0 if f is in {0,2,4,...,q-1}
#[inline]
pub fn fe25519_isnegative(f: &fe25519) -> core::ffi::c_int {
    let mut s = [0u8; 32];
    unsafe {
        fe25519_tobytes(s.as_mut_ptr(), f);
    }
    (s[0] & 1) as core::ffi::c_int
}

/// return 1 if f == 0, 0 if f != 0
#[inline]
pub fn fe25519_iszero(f: &fe25519) -> core::ffi::c_int {
    let mut s = [0u8; 32];
    unsafe {
        fe25519_tobytes(s.as_mut_ptr(), f);
    }
    let mut d: u32 = 0;
    for &b in s.iter() {
        d |= b as u32;
    }
    (1 & ((d.wrapping_sub(1)) >> 8)) as core::ffi::c_int
}

/// `h = f * g`
pub fn fe25519_mul(h: &mut fe25519, f: &fe25519, g: &fe25519) {
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
    let mut carry2: i64;
    let carry3: i64;
    let mut carry4: i64;
    let carry5: i64;
    let mut carry6: i64;
    let carry7: i64;
    let mut carry8: i64;
    let carry9: i64;

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 as i64) << 26));
    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 as i64) << 26));

    carry1 = h1.wrapping_add(1i64 << 24) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 as i64) << 25));
    carry5 = h5.wrapping_add(1i64 << 24) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 as i64) << 25));

    carry2 = h2.wrapping_add(1i64 << 25) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 as i64) << 26));
    carry6 = h6.wrapping_add(1i64 << 25) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 as i64) << 26));

    carry3 = h3.wrapping_add(1i64 << 24) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 as i64) << 25));
    carry7 = h7.wrapping_add(1i64 << 24) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 as i64) << 25));

    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 as i64) << 26));
    carry8 = h8.wrapping_add(1i64 << 25) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 as i64) << 26));

    carry9 = h9.wrapping_add(1i64 << 24) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 as i64) << 25));

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 as i64) << 26));

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

/// `h = h * g` (in-place-safe variant used when the caller needs h and f to
/// alias the same variable, mirroring C's `fe25519_mul(h, h, g)`).
#[inline]
pub fn fe25519_mul_ip(h: &mut fe25519, g: &fe25519) {
    let f = *h;
    fe25519_mul(h, &f, g);
}

/// `h = f * f`
pub fn fe25519_sq(h: &mut fe25519, f: &fe25519) {
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
    let f5_38: i32 = 38i32.wrapping_mul(f5);
    let f6_19: i32 = 19i32.wrapping_mul(f6);
    let f7_38: i32 = 38i32.wrapping_mul(f7);
    let f8_19: i32 = 19i32.wrapping_mul(f8);
    let f9_38: i32 = 38i32.wrapping_mul(f9);

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
    let mut carry2: i64;
    let carry3: i64;
    let mut carry4: i64;
    let carry5: i64;
    let mut carry6: i64;
    let carry7: i64;
    let mut carry8: i64;
    let carry9: i64;

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 as i64) << 26));
    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 as i64) << 26));

    carry1 = h1.wrapping_add(1i64 << 24) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 as i64) << 25));
    carry5 = h5.wrapping_add(1i64 << 24) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 as i64) << 25));

    carry2 = h2.wrapping_add(1i64 << 25) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 as i64) << 26));
    carry6 = h6.wrapping_add(1i64 << 25) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 as i64) << 26));

    carry3 = h3.wrapping_add(1i64 << 24) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 as i64) << 25));
    carry7 = h7.wrapping_add(1i64 << 24) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 as i64) << 25));

    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 as i64) << 26));
    carry8 = h8.wrapping_add(1i64 << 25) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 as i64) << 26));

    carry9 = h9.wrapping_add(1i64 << 24) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 as i64) << 25));

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 as i64) << 26));

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

/// `h = h^2` (in-place-safe variant used when the caller needs h and f to
/// alias the same variable, mirroring C's `fe25519_sq(h, h)`).
#[inline]
pub fn fe25519_sq_ip(h: &mut fe25519) {
    let f = *h;
    fe25519_sq(h, &f);
}

/// `h = 2 * f * f`
pub fn fe25519_sq2(h: &mut fe25519, f: &fe25519) {
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
    let f5_38: i32 = 38i32.wrapping_mul(f5);
    let f6_19: i32 = 19i32.wrapping_mul(f6);
    let f7_38: i32 = 38i32.wrapping_mul(f7);
    let f8_19: i32 = 19i32.wrapping_mul(f8);
    let f9_38: i32 = 38i32.wrapping_mul(f9);

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
    let mut carry2: i64;
    let carry3: i64;
    let mut carry4: i64;
    let carry5: i64;
    let mut carry6: i64;
    let carry7: i64;
    let mut carry8: i64;
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

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 as i64) << 26));
    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 as i64) << 26));

    carry1 = h1.wrapping_add(1i64 << 24) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 as i64) << 25));
    carry5 = h5.wrapping_add(1i64 << 24) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 as i64) << 25));

    carry2 = h2.wrapping_add(1i64 << 25) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 as i64) << 26));
    carry6 = h6.wrapping_add(1i64 << 25) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 as i64) << 26));

    carry3 = h3.wrapping_add(1i64 << 24) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 as i64) << 25));
    carry7 = h7.wrapping_add(1i64 << 24) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 as i64) << 25));

    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 as i64) << 26));
    carry8 = h8.wrapping_add(1i64 << 25) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 as i64) << 26));

    carry9 = h9.wrapping_add(1i64 << 24) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 as i64) << 25));

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 as i64) << 26));

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

/// `h = f * n`
pub fn fe25519_mul32(h: &mut fe25519, f: &fe25519, n: u32) {
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

    let carry9: i64;
    let carry1: i64;
    let carry3: i64;
    let carry5: i64;
    let carry7: i64;
    let carry0: i64;
    let carry2: i64;
    let carry4: i64;
    let carry6: i64;
    let carry8: i64;

    carry9 = h9.wrapping_add(1i64 << 24) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul(1i64 << 25));
    carry1 = h1.wrapping_add(1i64 << 24) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul(1i64 << 25));
    carry3 = h3.wrapping_add(1i64 << 24) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul(1i64 << 25));
    carry5 = h5.wrapping_add(1i64 << 24) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul(1i64 << 25));
    carry7 = h7.wrapping_add(1i64 << 24) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul(1i64 << 25));

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul(1i64 << 26));
    carry2 = h2.wrapping_add(1i64 << 25) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul(1i64 << 26));
    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul(1i64 << 26));
    carry6 = h6.wrapping_add(1i64 << 25) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul(1i64 << 26));
    carry8 = h8.wrapping_add(1i64 << 25) >> 26;
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

// ---------------------------------------------------------------------------
// fe_25_5/fe.h: fe25519_frombytes, fe25519_reduce, fe25519_tobytes
// ---------------------------------------------------------------------------

/// `void fe25519_frombytes(fe25519 h, const unsigned char *s)`
/// Ignores top bit of s.
#[inline]
pub unsafe fn fe25519_frombytes(h: &mut fe25519, s: *const u8) {
    let mut h0: i64 = load_4(s) as i64;
    let mut h1: i64 = (load_3(s.add(4)) as i64) << 6;
    let mut h2: i64 = (load_3(s.add(7)) as i64) << 5;
    let mut h3: i64 = (load_3(s.add(10)) as i64) << 3;
    let mut h4: i64 = (load_3(s.add(13)) as i64) << 2;
    let mut h5: i64 = load_4(s.add(16)) as i64;
    let mut h6: i64 = (load_3(s.add(20)) as i64) << 7;
    let mut h7: i64 = (load_3(s.add(23)) as i64) << 5;
    let mut h8: i64 = (load_3(s.add(26)) as i64) << 4;
    let mut h9: i64 = ((load_3(s.add(29)) & 8388607) as i64) << 2;

    let carry9: i64;
    let carry1: i64;
    let carry3: i64;
    let carry5: i64;
    let carry7: i64;
    let carry0: i64;
    let carry2: i64;
    let carry4: i64;
    let carry6: i64;
    let carry8: i64;

    carry9 = h9.wrapping_add(1i64 << 24) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u64 as i64) << 25));
    carry1 = h1.wrapping_add(1i64 << 24) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u64 as i64) << 25));
    carry3 = h3.wrapping_add(1i64 << 24) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u64 as i64) << 25));
    carry5 = h5.wrapping_add(1i64 << 24) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u64 as i64) << 25));
    carry7 = h7.wrapping_add(1i64 << 24) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u64 as i64) << 25));

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u64 as i64) << 26));
    carry2 = h2.wrapping_add(1i64 << 25) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u64 as i64) << 26));
    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u64 as i64) << 26));
    carry6 = h6.wrapping_add(1i64 << 25) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u64 as i64) << 26));
    carry8 = h8.wrapping_add(1i64 << 25) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u64 as i64) << 26));

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

/// `static void fe25519_reduce(fe25519 h, const fe25519 f)`
pub fn fe25519_reduce(h: &mut fe25519, f: &fe25519) {
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

    q = (19i32.wrapping_mul(h9).wrapping_add((1u32 as i32) << 24)) >> 25;
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
    h0 = h0.wrapping_sub(carry0.wrapping_mul((1u32 as i32) << 26));
    carry1 = h1 >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = h1.wrapping_sub(carry1.wrapping_mul((1u32 as i32) << 25));
    carry2 = h2 >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = h2.wrapping_sub(carry2.wrapping_mul((1u32 as i32) << 26));
    carry3 = h3 >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = h3.wrapping_sub(carry3.wrapping_mul((1u32 as i32) << 25));
    carry4 = h4 >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = h4.wrapping_sub(carry4.wrapping_mul((1u32 as i32) << 26));
    carry5 = h5 >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = h5.wrapping_sub(carry5.wrapping_mul((1u32 as i32) << 25));
    carry6 = h6 >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = h6.wrapping_sub(carry6.wrapping_mul((1u32 as i32) << 26));
    carry7 = h7 >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = h7.wrapping_sub(carry7.wrapping_mul((1u32 as i32) << 25));
    carry8 = h8 >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = h8.wrapping_sub(carry8.wrapping_mul((1u32 as i32) << 26));
    carry9 = h9 >> 25;
    h9 = h9.wrapping_sub(carry9.wrapping_mul((1u32 as i32) << 25));

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

/// `void fe25519_tobytes(unsigned char *s, const fe25519 h)`
#[inline]
pub unsafe fn fe25519_tobytes(s: *mut u8, h: &fe25519) {
    let mut t: fe25519 = [0i32; 10];
    fe25519_reduce(&mut t, h);

    *s = (t[0] >> 0) as u8;
    *s.add(1) = (t[0] >> 8) as u8;
    *s.add(2) = (t[0] >> 16) as u8;
    *s.add(3) = ((t[0] >> 24) | (t[1].wrapping_mul(1i32 << 2))) as u8;
    *s.add(4) = (t[1] >> 6) as u8;
    *s.add(5) = (t[1] >> 14) as u8;
    *s.add(6) = ((t[1] >> 22) | (t[2].wrapping_mul(1i32 << 3))) as u8;
    *s.add(7) = (t[2] >> 5) as u8;
    *s.add(8) = (t[2] >> 13) as u8;
    *s.add(9) = ((t[2] >> 21) | (t[3].wrapping_mul(1i32 << 5))) as u8;
    *s.add(10) = (t[3] >> 3) as u8;
    *s.add(11) = (t[3] >> 11) as u8;
    *s.add(12) = ((t[3] >> 19) | (t[4].wrapping_mul(1i32 << 6))) as u8;
    *s.add(13) = (t[4] >> 2) as u8;
    *s.add(14) = (t[4] >> 10) as u8;
    *s.add(15) = (t[4] >> 18) as u8;
    *s.add(16) = (t[5] >> 0) as u8;
    *s.add(17) = (t[5] >> 8) as u8;
    *s.add(18) = (t[5] >> 16) as u8;
    *s.add(19) = ((t[5] >> 24) | (t[6].wrapping_mul(1i32 << 1))) as u8;
    *s.add(20) = (t[6] >> 7) as u8;
    *s.add(21) = (t[6] >> 15) as u8;
    *s.add(22) = ((t[6] >> 23) | (t[7].wrapping_mul(1i32 << 3))) as u8;
    *s.add(23) = (t[7] >> 5) as u8;
    *s.add(24) = (t[7] >> 13) as u8;
    *s.add(25) = ((t[7] >> 21) | (t[8].wrapping_mul(1i32 << 4))) as u8;
    *s.add(26) = (t[8] >> 4) as u8;
    *s.add(27) = (t[8] >> 12) as u8;
    *s.add(28) = ((t[8] >> 20) | (t[9].wrapping_mul(1i32 << 6))) as u8;
    *s.add(29) = (t[9] >> 2) as u8;
    *s.add(30) = (t[9] >> 10) as u8;
    *s.add(31) = (t[9] >> 18) as u8;
}

// ---------------------------------------------------------------------------
// ed25519_ref10.c lines 1-262
// ---------------------------------------------------------------------------

/// `static inline void fe25519_sqmul(fe25519 s, const int n, const fe25519 a)`
#[inline]
pub fn fe25519_sqmul(s: &mut fe25519, n: core::ffi::c_int, a: &fe25519) {
    let mut i: core::ffi::c_int = 0;
    while i < n {
        fe25519_sq_ip(s);
        i += 1;
    }
    fe25519_mul_ip(s, a);
}

/// Inversion - sets out to 0 if z=0
/// `void fe25519_invert(fe25519 out, const fe25519 z)`
pub fn fe25519_invert(out: &mut fe25519, z: &fe25519) {
    let mut t0: fe25519 = [0i32; 10];
    let mut t1: fe25519 = [0i32; 10];
    let mut t2: fe25519 = [0i32; 10];
    let mut t3: fe25519 = [0i32; 10];
    let mut i: i32;

    fe25519_sq(&mut t0, z);
    fe25519_sq(&mut t1, &t0);
    fe25519_sq_ip(&mut t1);
    fe25519_mul_ip(&mut t1, z);
    fe25519_mul_ip(&mut t0, &t1);
    fe25519_sq(&mut t2, &t0);
    fe25519_mul_ip(&mut t1, &t2);
    fe25519_sq(&mut t2, &t1);
    i = 1;
    while i < 5 {
        fe25519_sq_ip(&mut t2);
        i += 1;
    }
    fe25519_mul_ip(&mut t1, &t2);
    fe25519_sq(&mut t2, &t1);
    i = 1;
    while i < 10 {
        fe25519_sq_ip(&mut t2);
        i += 1;
    }
    fe25519_mul_ip(&mut t2, &t1);
    fe25519_sq(&mut t3, &t2);
    i = 1;
    while i < 20 {
        fe25519_sq_ip(&mut t3);
        i += 1;
    }
    fe25519_mul_ip(&mut t2, &t3);
    i = 1;
    while i < 11 {
        fe25519_sq_ip(&mut t2);
        i += 1;
    }
    fe25519_mul_ip(&mut t1, &t2);
    fe25519_sq(&mut t2, &t1);
    i = 1;
    while i < 50 {
        fe25519_sq_ip(&mut t2);
        i += 1;
    }
    fe25519_mul_ip(&mut t2, &t1);
    fe25519_sq(&mut t3, &t2);
    i = 1;
    while i < 100 {
        fe25519_sq_ip(&mut t3);
        i += 1;
    }
    fe25519_mul_ip(&mut t2, &t3);
    i = 1;
    while i < 51 {
        fe25519_sq_ip(&mut t2);
        i += 1;
    }
    fe25519_mul_ip(&mut t1, &t2);
    i = 1;
    while i < 6 {
        fe25519_sq_ip(&mut t1);
        i += 1;
    }
    fe25519_mul(out, &t1, &t0);
}

/// returns z^((p-5)/8) = z^(2^252-3); used to compute square roots since
/// we have p=5 (mod 8); see Cohen and Frey.
/// `static void fe25519_pow22523(fe25519 out, const fe25519 z)`
pub fn fe25519_pow22523(out: &mut fe25519, z: &fe25519) {
    let mut t0: fe25519 = [0i32; 10];
    let mut t1: fe25519 = [0i32; 10];
    let mut t2: fe25519 = [0i32; 10];
    let mut i: i32;

    fe25519_sq(&mut t0, z);
    fe25519_sq(&mut t1, &t0);
    fe25519_sq_ip(&mut t1);
    fe25519_mul_ip(&mut t1, z);
    fe25519_mul_ip(&mut t0, &t1);
    fe25519_sq_ip(&mut t0);
    fe25519_mul_ip(&mut t0, &t1);
    fe25519_sq(&mut t1, &t0);
    i = 1;
    while i < 5 {
        fe25519_sq_ip(&mut t1);
        i += 1;
    }
    fe25519_mul_ip(&mut t0, &t1);
    fe25519_sq(&mut t1, &t0);
    i = 1;
    while i < 10 {
        fe25519_sq_ip(&mut t1);
        i += 1;
    }
    fe25519_mul_ip(&mut t1, &t0);
    fe25519_sq(&mut t2, &t1);
    i = 1;
    while i < 20 {
        fe25519_sq_ip(&mut t2);
        i += 1;
    }
    fe25519_mul_ip(&mut t1, &t2);
    i = 1;
    while i < 11 {
        fe25519_sq_ip(&mut t1);
        i += 1;
    }
    fe25519_mul_ip(&mut t0, &t1);
    fe25519_sq(&mut t1, &t0);
    i = 1;
    while i < 50 {
        fe25519_sq_ip(&mut t1);
        i += 1;
    }
    fe25519_mul_ip(&mut t1, &t0);
    fe25519_sq(&mut t2, &t1);
    i = 1;
    while i < 100 {
        fe25519_sq_ip(&mut t2);
        i += 1;
    }
    fe25519_mul_ip(&mut t1, &t2);
    i = 1;
    while i < 51 {
        fe25519_sq_ip(&mut t1);
        i += 1;
    }
    fe25519_mul_ip(&mut t0, &t1);
    fe25519_sq_ip(&mut t0);
    fe25519_sq_ip(&mut t0);
    fe25519_mul(out, &t0, z);
}

/// `static inline void fe25519_cneg(fe25519 h, unsigned int b)`
#[inline]
pub fn fe25519_cneg(h: &mut fe25519, b: u32) {
    let mut negf: fe25519 = [0i32; 10];
    fe25519_neg(&mut negf, h);
    fe25519_cmov(h, &negf, b);
}

/// `static inline void fe25519_abs(fe25519 h)`
#[inline]
pub fn fe25519_abs(h: &mut fe25519) {
    let b = fe25519_isnegative(h) as u32;
    fe25519_cneg(h, b);
}

/// `static void fe25519_unchecked_sqrt(fe25519 x, const fe25519 x2)`
pub fn fe25519_unchecked_sqrt(x: &mut fe25519, x2: &fe25519) {
    let mut p_root: fe25519 = [0i32; 10];
    let mut m_root: fe25519 = [0i32; 10];
    let mut m_root2: fe25519 = [0i32; 10];
    let mut e: fe25519 = [0i32; 10];

    fe25519_pow22523(&mut e, x2);
    fe25519_mul(&mut p_root, &e, x2);
    fe25519_mul(&mut m_root, &p_root, &fe25519_sqrtm1);
    fe25519_sq(&mut m_root2, &m_root);
    fe25519_sub(&mut e, x2, &m_root2);
    fe25519_copy(x, &p_root);
    let b = fe25519_iszero(&e) as u32;
    fe25519_cmov(x, &m_root, b);
}

/// `static int fe25519_sqrt(fe25519 x, const fe25519 x2)`
pub fn fe25519_sqrt(x: &mut fe25519, x2: &fe25519) -> core::ffi::c_int {
    let mut check: fe25519 = [0i32; 10];
    let mut x2_copy: fe25519 = [0i32; 10];

    fe25519_copy(&mut x2_copy, x2);
    fe25519_unchecked_sqrt(x, x2);
    fe25519_sq(&mut check, x);
    let check_copy = check;
    fe25519_sub(&mut check, &check_copy, &x2_copy);

    fe25519_iszero(&check) - 1
}

/// `static int fe25519_notsquare(const fe25519 x)`
pub fn fe25519_notsquare(x: &fe25519) -> core::ffi::c_int {
    let mut _10: fe25519 = [0i32; 10];
    let mut _11: fe25519 = [0i32; 10];
    let mut _1100: fe25519 = [0i32; 10];
    let mut _1111: fe25519 = [0i32; 10];
    let mut _11110000: fe25519 = [0i32; 10];
    let mut _11111111: fe25519 = [0i32; 10];
    let mut t: fe25519 = [0i32; 10];
    let mut u: fe25519 = [0i32; 10];
    let mut v: fe25519 = [0i32; 10];
    let mut s = [0u8; 32];

    /* Jacobi symbol - x^((p-1)/2) */
    fe25519_mul(&mut _10, x, x);
    fe25519_mul(&mut _11, x, &_10);
    fe25519_sq(&mut _1100, &_11);
    fe25519_sq_ip(&mut _1100);
    fe25519_mul(&mut _1111, &_11, &_1100);
    fe25519_sq(&mut _11110000, &_1111);
    fe25519_sq_ip(&mut _11110000);
    fe25519_sq_ip(&mut _11110000);
    fe25519_sq_ip(&mut _11110000);
    fe25519_mul(&mut _11111111, &_1111, &_11110000);
    fe25519_copy(&mut t, &_11111111);
    fe25519_sqmul(&mut t, 2, &_11);
    fe25519_copy(&mut u, &t);
    fe25519_sqmul(&mut t, 10, &u);
    fe25519_sqmul(&mut t, 10, &u);
    fe25519_copy(&mut v, &t);
    fe25519_sqmul(&mut t, 30, &v);
    fe25519_copy(&mut v, &t);
    fe25519_sqmul(&mut t, 60, &v);
    fe25519_copy(&mut v, &t);
    fe25519_sqmul(&mut t, 120, &v);
    fe25519_sqmul(&mut t, 10, &u);
    fe25519_sqmul(&mut t, 3, &_11);
    fe25519_sq_ip(&mut t);

    unsafe {
        fe25519_tobytes(s.as_mut_ptr(), &t);
    }

    (s[1] & 1) as core::ffi::c_int
}

// ---------------------------------------------------------------------------
// Exported C symbols (see $W/_cbuild/persym.txt for
// crypto_core/ed25519/ref10/ed25519_ref10.c.o)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _sodium_fe25519_frombytes(h: *mut i32, s: *const u8) {
    let href = &mut *(h as *mut fe25519);
    fe25519_frombytes(href, s);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32) {
    let href = &*(h as *const fe25519);
    fe25519_tobytes(s, href);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_fe25519_invert(out: *mut i32, z: *const i32) {
    let outref = &mut *(out as *mut fe25519);
    let zref = &*(z as *const fe25519);
    fe25519_invert(outref, zref);
}
