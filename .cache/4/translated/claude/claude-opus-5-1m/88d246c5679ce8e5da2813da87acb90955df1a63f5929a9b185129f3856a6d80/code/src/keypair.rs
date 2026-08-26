//! Translation of `crypto_sign/ed25519/ref10/keypair.c`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* HAVE_TI_MODE is *not* defined in the reference build:
 *   typedef int32_t fe25519[10];
 */
pub type fe25519 = [i32; 10];

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ge25519_p3 {
    pub X: fe25519,
    pub Y: fe25519,
    pub Z: fe25519,
    pub T: fe25519,
}

/* #define crypto_hash_sha512_BYTES 64U */
const crypto_hash_sha512_BYTES: usize = 64;
/* #define crypto_scalarmult_curve25519_BYTES 32U */
const crypto_scalarmult_curve25519_BYTES: usize = 32;

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: c_ulonglong) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);

    /* private/ed25519_ref10.h -- names after private/quirks.h renaming */
    #[link_name = "_sodium_ge25519_scalarmult_base"]
    fn ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    #[link_name = "_sodium_ge25519_p3_tobytes"]
    fn ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    #[link_name = "_sodium_ge25519_frombytes_negate_vartime"]
    fn ge25519_frombytes_negate_vartime(h: *mut ge25519_p3, s: *const u8) -> c_int;
    #[link_name = "_sodium_ge25519_has_small_order"]
    fn ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_ge25519_is_on_main_subgroup"]
    fn ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    #[link_name = "_sodium_fe25519_invert"]
    fn fe25519_invert(out: *mut i32, z: *const i32);
    #[link_name = "_sodium_fe25519_tobytes"]
    fn fe25519_tobytes(s: *mut u8, h: *const i32);
}

/* ------------------------------------------------------------------------ */
/* `static inline` / `static` helpers from
 * include/sodium/private/ed25519_ref10_fe_25_5.h, duplicated here as the
 * contract requires.                                                       */
/* ------------------------------------------------------------------------ */

/* h = 1 */
#[inline]
fn fe25519_1(h: &mut fe25519) {
    h[0] = 1;
    h[1] = 0;
    h[2] = 0;
    h[3] = 0;
    h[4] = 0;
    h[5] = 0;
    h[6] = 0;
    h[7] = 0;
    h[8] = 0;
    h[9] = 0;
}

/* h = f + g */
#[inline]
fn fe25519_add(h: &mut fe25519, f: &fe25519, g: &fe25519) {
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

/* h = f - g */
fn fe25519_sub(h: &mut fe25519, f: &fe25519, g: &fe25519) {
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

/* h = f * g */
fn fe25519_mul(h: &mut fe25519, f: &fe25519, g: &fe25519) {
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
    let mut carry1: i64;
    let mut carry2: i64;
    let mut carry3: i64;
    let mut carry4: i64;
    let mut carry5: i64;
    let mut carry6: i64;
    let mut carry7: i64;
    let mut carry8: i64;
    let mut carry9: i64;

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = (h0 as u64).wrapping_sub((carry0 as u64).wrapping_mul(1u64 << 26)) as i64;
    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = (h4 as u64).wrapping_sub((carry4 as u64).wrapping_mul(1u64 << 26)) as i64;

    carry1 = h1.wrapping_add(1i64 << 24) >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 = (h1 as u64).wrapping_sub((carry1 as u64).wrapping_mul(1u64 << 25)) as i64;
    carry5 = h5.wrapping_add(1i64 << 24) >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 = (h5 as u64).wrapping_sub((carry5 as u64).wrapping_mul(1u64 << 25)) as i64;

    carry2 = h2.wrapping_add(1i64 << 25) >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 = (h2 as u64).wrapping_sub((carry2 as u64).wrapping_mul(1u64 << 26)) as i64;
    carry6 = h6.wrapping_add(1i64 << 25) >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 = (h6 as u64).wrapping_sub((carry6 as u64).wrapping_mul(1u64 << 26)) as i64;

    carry3 = h3.wrapping_add(1i64 << 24) >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 = (h3 as u64).wrapping_sub((carry3 as u64).wrapping_mul(1u64 << 25)) as i64;
    carry7 = h7.wrapping_add(1i64 << 24) >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 = (h7 as u64).wrapping_sub((carry7 as u64).wrapping_mul(1u64 << 25)) as i64;

    carry4 = h4.wrapping_add(1i64 << 25) >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 = (h4 as u64).wrapping_sub((carry4 as u64).wrapping_mul(1u64 << 26)) as i64;
    carry8 = h8.wrapping_add(1i64 << 25) >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 = (h8 as u64).wrapping_sub((carry8 as u64).wrapping_mul(1u64 << 26)) as i64;

    carry9 = h9.wrapping_add(1i64 << 24) >> 25;
    h0 = h0.wrapping_add(carry9.wrapping_mul(19));
    h9 = (h9 as u64).wrapping_sub((carry9 as u64).wrapping_mul(1u64 << 25)) as i64;

    carry0 = h0.wrapping_add(1i64 << 25) >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 = (h0 as u64).wrapping_sub((carry0 as u64).wrapping_mul(1u64 << 26)) as i64;

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

/* ------------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut A: ge25519_p3 = core::mem::zeroed();

    crypto_hash_sha512(sk, seed, 32);
    *sk.add(0) &= 248;
    *sk.add(31) &= 127;
    *sk.add(31) |= 64;

    ge25519_scalarmult_base(&mut A, sk);
    ge25519_p3_tobytes(pk, &A);

    memmove(sk, seed, 32);
    memmove(sk.add(32), pk, 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed: [u8; 32] = [0; 32];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));
    ret = crypto_sign_ed25519_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_pk_to_curve25519(
    curve25519_pk: *mut u8,
    ed25519_pk: *const u8,
) -> c_int {
    let mut A: ge25519_p3 = core::mem::zeroed();
    let mut x: fe25519 = [0; 10];
    let mut one_minus_y: fe25519 = [0; 10];

    if ge25519_frombytes_negate_vartime(&mut A, ed25519_pk) != 0
        || ge25519_has_small_order(&A) != 0
        || ge25519_is_on_main_subgroup(&A) == 0
    {
        return -1;
    }
    fe25519_1(&mut one_minus_y);
    /* assumes A.Z=1 */
    let a_y = A.Y;
    let tmp = one_minus_y;
    fe25519_sub(&mut one_minus_y, &tmp, &a_y);
    fe25519_1(&mut x);
    let tmp = x;
    fe25519_add(&mut x, &tmp, &a_y);
    fe25519_invert(one_minus_y.as_mut_ptr(), one_minus_y.as_ptr());
    let tmp = x;
    fe25519_mul(&mut x, &tmp, &one_minus_y);
    fe25519_tobytes(curve25519_pk, x.as_ptr());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_curve25519(
    curve25519_sk: *mut u8,
    ed25519_sk: *const u8,
) -> c_int {
    let mut h: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512(h.as_mut_ptr(), ed25519_sk, 32);
    h[0] &= 248;
    h[31] &= 127;
    h[31] |= 64;
    memcpy(curve25519_sk, h.as_ptr(), crypto_scalarmult_curve25519_BYTES);
    sodium_memzero(h.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&h));

    0
}
