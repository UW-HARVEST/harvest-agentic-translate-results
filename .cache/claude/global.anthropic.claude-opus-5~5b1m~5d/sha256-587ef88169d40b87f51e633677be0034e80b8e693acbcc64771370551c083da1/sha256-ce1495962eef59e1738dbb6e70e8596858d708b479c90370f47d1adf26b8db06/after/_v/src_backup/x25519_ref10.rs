//! Translated from `crypto_scalarmult/curve25519/ref10/x25519_ref10.c`.
//!
//! `HAVE_TI_MODE` is not defined in the reference build, so `fe25519_sub_lazy`
//! is just `fe25519_sub` (see the `#else` branch in the C source).

use core::ffi::c_int;

use crate::ed25519_ref10_fe::*;
use crate::types::{fe25519, ge25519_p3};

extern "C" {
    #[link_name = "_sodium_ge25519_scalarmult_base"]
    fn ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);

    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
}

/// `#[repr(C)]` mirror of `crypto_scalarmult_curve25519_implementation`
/// (see `scalarmult_curve25519.h`).
#[repr(C)]
pub struct crypto_scalarmult_curve25519_implementation {
    pub mult: unsafe extern "C" fn(q: *mut u8, n: *const u8, p: *const u8) -> c_int,
    pub mult_base: unsafe extern "C" fn(q: *mut u8, n: *const u8) -> c_int,
}

/*
 * Reject small order points early to mitigate the implications of
 * unexpected optimizations that would affect the ref10 code.
 * See https://eprint.iacr.org/2017/806.pdf for reference.
 */
#[rustfmt::skip]
static BLOCKLIST: [[u8; 32]; 7] = [
    /* 0 (order 4) */
    [ 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 ],
    /* 1 (order 1) */
    [ 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 ],
    /* 325606250916557431795983626356110631294008115727848805560023387167927233504
       (order 8) */
    [ 0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3,
      0xfa, 0xf1, 0x9f, 0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32,
      0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49, 0xb8, 0x00 ],
    /* 39382357235489614581723060781553021112529911719440698176882885853963445705823
       (order 8) */
    [ 0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1,
      0x55, 0x9c, 0x83, 0xef, 0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c,
      0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f, 0x11, 0x57 ],
    /* p-1 (order 2) */
    [ 0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f ],
    /* p (=0, order 4) */
    [ 0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f ],
    /* p+1 (=1, order 1) */
    [ 0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f ],
];

/// `static int has_small_order(const unsigned char s[32])`
///
/// Never indexes past the fixed-size `s`/`BLOCKLIST` arrays.
unsafe fn has_small_order(s: *const u8) -> c_int {
    let mut c: [u8; 7] = [0; 7];
    let mut j: usize;

    j = 0;
    while j < 31 {
        let sj = *s.add(j);
        for i in 0..BLOCKLIST.len() {
            c[i] |= sj ^ BLOCKLIST[i][j];
        }
        j += 1;
    }
    let sj = *s.add(31);
    for i in 0..BLOCKLIST.len() {
        c[i] |= (sj & 0x7f) ^ BLOCKLIST[i][31];
    }
    let mut k: u32 = 0;
    for i in 0..BLOCKLIST.len() {
        k |= (c[i] as u32).wrapping_sub(1);
    }
    ((k >> 8) & 1) as c_int
}

/*
 * limbs are signed and fe25519_sub() is a plain per-limb subtraction
 * (HAVE_TI_MODE is not defined in the reference build)
 */
#[inline]
fn fe25519_sub_lazy(h: &mut fe25519, f: &fe25519, g: &fe25519) {
    fe25519_sub(h, f, g);
}

/// `static int crypto_scalarmult_curve25519_ref10(unsigned char *q, const
/// unsigned char *n, const unsigned char *p)`
unsafe fn crypto_scalarmult_curve25519_ref10(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    let mut t: [u8; 32] = [0; 32];
    let mut i: usize;
    let mut x1: fe25519 = [0; 10];
    let mut x2: fe25519 = [0; 10];
    let mut x3: fe25519 = [0; 10];
    let mut z2: fe25519 = [0; 10];
    let mut z3: fe25519 = [0; 10];
    let mut a: fe25519 = [0; 10];
    let mut b: fe25519 = [0; 10];
    let mut aa: fe25519 = [0; 10];
    let mut bb: fe25519 = [0; 10];
    let mut e: fe25519 = [0; 10];
    let mut da: fe25519 = [0; 10];
    let mut cb: fe25519 = [0; 10];
    let mut pos: i32;
    let mut swap: u32;
    let mut bit: u32;

    if has_small_order(p) != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        t[i] = *n.add(i);
        i += 1;
    }
    t[0] &= 248;
    t[31] &= 127;
    t[31] |= 64;
    fe25519_frombytes(&mut x1, p);
    fe25519_1(&mut x2);
    fe25519_0(&mut z2);
    fe25519_copy(&mut x3, &x1);
    fe25519_1(&mut z3);

    swap = 0;
    pos = 254;
    while pos >= 0 {
        bit = (t[(pos / 8) as usize] as u32) >> ((pos & 7) as u32);
        bit &= 1;
        swap ^= bit;
        fe25519_cswap(&mut x2, &mut x3, swap);
        fe25519_cswap(&mut z2, &mut z3, swap);
        swap = bit;
        fe25519_add(&mut a, &x2, &z2);
        fe25519_sub_lazy(&mut b, &x2, &z2);
        fe25519_sq(&mut aa, &a);
        fe25519_sq(&mut bb, &b);
        fe25519_mul(&mut x2, &aa, &bb);
        fe25519_sub_lazy(&mut e, &aa, &bb);
        fe25519_sub_lazy(&mut da, &x3, &z3);
        {
            let t0 = da;
            fe25519_mul(&mut da, &t0, &a);
        }
        fe25519_add(&mut cb, &x3, &z3);
        {
            let t0 = cb;
            fe25519_mul(&mut cb, &t0, &b);
        }
        fe25519_add(&mut x3, &da, &cb);
        fe25519_sq_ip(&mut x3);
        fe25519_sub_lazy(&mut z3, &da, &cb);
        fe25519_sq_ip(&mut z3);
        {
            let t0 = z3;
            fe25519_mul(&mut z3, &t0, &x1);
        }
        fe25519_mul32(&mut z2, &e, 121666);
        {
            let t0 = z2;
            fe25519_add(&mut z2, &t0, &bb);
        }
        {
            let t0 = z2;
            fe25519_mul(&mut z2, &t0, &e);
        }

        pos -= 1;
    }
    fe25519_cswap(&mut x2, &mut x3, swap);
    fe25519_cswap(&mut z2, &mut z3, swap);

    {
        let t0 = z2;
        fe25519_invert(&mut z2, &t0);
    }
    {
        let t0 = x2;
        fe25519_mul(&mut x2, &t0, &z2);
    }
    fe25519_tobytes(q, &x2);

    sodium_memzero(t.as_mut_ptr() as *mut core::ffi::c_void, t.len());

    0
}

/// `static void edwards_to_montgomery(fe25519 montgomeryX, const fe25519
/// edwardsY, const fe25519 edwardsZ)`
fn edwards_to_montgomery(montgomery_x: &mut fe25519, edwards_y: &fe25519, edwards_z: &fe25519) {
    let mut temp_x: fe25519 = [0; 10];
    let mut temp_z: fe25519 = [0; 10];

    fe25519_add(&mut temp_x, edwards_z, edwards_y);
    fe25519_sub(&mut temp_z, edwards_z, edwards_y);
    {
        let t0 = temp_z;
        fe25519_invert(&mut temp_z, &t0);
    }
    fe25519_mul(montgomery_x, &temp_x, &temp_z);
}

/// `static int crypto_scalarmult_curve25519_ref10_base(unsigned char *q,
/// const unsigned char *n)`
unsafe fn crypto_scalarmult_curve25519_ref10_base(q: *mut u8, n: *const u8) -> c_int {
    let t = q;
    let mut a: ge25519_p3 = core::mem::zeroed();
    let mut pk: fe25519 = [0; 10];
    let mut i: usize;

    i = 0;
    while i < 32 {
        *t.add(i) = *n.add(i);
        i += 1;
    }
    *t &= 248;
    *t.add(31) &= 127;
    *t.add(31) |= 64;
    ge25519_scalarmult_base(&mut a, t);
    edwards_to_montgomery(&mut pk, &a.Y, &a.Z);
    fe25519_tobytes(q, &pk);

    0
}

unsafe extern "C" fn mult_impl(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    crypto_scalarmult_curve25519_ref10(q, n, p)
}

unsafe extern "C" fn mult_base_impl(q: *mut u8, n: *const u8) -> c_int {
    crypto_scalarmult_curve25519_ref10_base(q, n)
}

/// `struct crypto_scalarmult_curve25519_implementation
/// crypto_scalarmult_curve25519_ref10_implementation`
#[no_mangle]
pub static crypto_scalarmult_curve25519_ref10_implementation: crypto_scalarmult_curve25519_implementation =
    crypto_scalarmult_curve25519_implementation {
        mult: mult_impl,
        mult_base: mult_base_impl,
    };
