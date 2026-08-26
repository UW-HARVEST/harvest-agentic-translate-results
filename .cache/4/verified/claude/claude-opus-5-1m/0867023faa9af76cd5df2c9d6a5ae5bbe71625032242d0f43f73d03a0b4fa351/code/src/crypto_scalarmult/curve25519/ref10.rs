//! Translation of `crypto_scalarmult/curve25519/ref10/x25519_ref10.c`.
//!
//! `HAVE_TI_MODE` is undefined in the reference build, so `fe25519` is
//! `int32_t[10]` and `fe25519_sub_lazy` is a plain `#define` for
//! `fe25519_sub()` (the `#else` branch of the `#ifdef HAVE_TI_MODE` block).

use core::ffi::{c_int, c_void};

use super::crypto_scalarmult_curve25519_implementation;
use crate::crypto_core::ed25519::fe25519::{
    fe25519_0, fe25519_1, fe25519_add, fe25519_copy, fe25519_cswap, fe25519_frombytes,
    fe25519_invert_in_place, fe25519_mul, fe25519_mul32, fe25519_sq, fe25519_sub, fe25519_tobytes,
};
use crate::crypto_core::ed25519::types::{Fe25519, Ge25519P3};
use crate::sodium::utils::sodium_memzero;

unsafe extern "C" {
    fn _sodium_ge25519_scalarmult_base(h: *mut Ge25519P3, a: *const u8);
}

/// `CRYPTO_ALIGN(16) static const unsigned char blocklist[][32]`
static blocklist: [[u8; 32]; 7] = [
    /* 0 (order 4) */
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
    /* 1 (order 1) */
    [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
    /* 325606250916557431795983626356110631294008115727848805560023387167927233504
       (order 8) */
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    /* 39382357235489614581723060781553021112529911719440698176882885853963445705823
       (order 8) */
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    /* p-1 (order 2) */
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    /* p (=0, order 4) */
    [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    /* p+1 (=1, order 1) */
    [
        0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
];

/// Reject small order points early to mitigate the implications of
/// unexpected optimizations that would affect the ref10 code.
/// See https://eprint.iacr.org/2017/806.pdf for reference.
unsafe fn has_small_order(s: *const u8) -> c_int {
    let mut c: [u8; 7] = [0u8; 7];
    let mut k: u32;
    let mut i: usize;
    let mut j: usize;

    j = 0;
    while j < 31 {
        i = 0;
        while i < blocklist.len() {
            c[i] |= unsafe { *s.add(j) } ^ blocklist[i][j];
            i += 1;
        }
        j += 1;
    }
    /* `j` is 31 here - the C code reuses the loop variable on purpose. */
    i = 0;
    while i < blocklist.len() {
        c[i] |= (unsafe { *s.add(j) } & 0x7f) ^ blocklist[i][j];
        i += 1;
    }
    k = 0;
    i = 0;
    while i < blocklist.len() {
        k |= ((c[i] as c_int).wrapping_sub(1)) as u32;
        i += 1;
    }

    ((k >> 8) & 1) as c_int
}

unsafe extern "C" fn crypto_scalarmult_curve25519_ref10(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let mut t: [u8; 32] = [0u8; 32];
    let mut i: u32;
    let mut x1 = Fe25519::ZERO;
    let mut x2 = Fe25519::ZERO;
    let mut x3 = Fe25519::ZERO;
    let mut z2 = Fe25519::ZERO;
    let mut z3 = Fe25519::ZERO;
    let mut a = Fe25519::ZERO;
    let mut b = Fe25519::ZERO;
    let mut aa = Fe25519::ZERO;
    let mut bb = Fe25519::ZERO;
    let mut e = Fe25519::ZERO;
    let mut da = Fe25519::ZERO;
    let mut cb = Fe25519::ZERO;
    let mut pos: c_int;
    let mut swap: u32;
    let mut bit: u32;

    if unsafe { has_small_order(p) } != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        t[i as usize] = unsafe { *n.add(i as usize) };
        i += 1;
    }
    t[0] &= 248;
    t[31] &= 127;
    t[31] |= 64;
    fe25519_frombytes(&mut x1, unsafe { core::slice::from_raw_parts(p, 32) });
    fe25519_1(&mut x2);
    fe25519_0(&mut z2);
    fe25519_copy(&mut x3, x1);
    fe25519_1(&mut z3);

    swap = 0;
    pos = 254;
    while pos >= 0 {
        bit = ((t[(pos / 8) as usize] as c_int) >> (pos & 7)) as u32;
        bit &= 1;
        swap ^= bit;
        fe25519_cswap(&mut x2, &mut x3, swap);
        fe25519_cswap(&mut z2, &mut z3, swap);
        swap = bit;
        fe25519_add(&mut a, x2, z2);
        fe25519_sub(&mut b, x2, z2);
        fe25519_sq(&mut aa, a);
        fe25519_sq(&mut bb, b);
        fe25519_mul(&mut x2, aa, bb);
        fe25519_sub(&mut e, aa, bb);
        fe25519_sub(&mut da, x3, z3);
        let da_v = da;
        fe25519_mul(&mut da, da_v, a);
        fe25519_add(&mut cb, x3, z3);
        let cb_v = cb;
        fe25519_mul(&mut cb, cb_v, b);
        fe25519_add(&mut x3, da, cb);
        let x3_v = x3;
        fe25519_sq(&mut x3, x3_v);
        fe25519_sub(&mut z3, da, cb);
        let z3_v = z3;
        fe25519_sq(&mut z3, z3_v);
        let z3_v = z3;
        fe25519_mul(&mut z3, z3_v, x1);
        fe25519_mul32(&mut z2, e, 121666);
        let z2_v = z2;
        fe25519_add(&mut z2, z2_v, bb);
        let z2_v = z2;
        fe25519_mul(&mut z2, z2_v, e);
        pos -= 1;
    }
    fe25519_cswap(&mut x2, &mut x3, swap);
    fe25519_cswap(&mut z2, &mut z3, swap);

    fe25519_invert_in_place(&mut z2);
    let x2_v = x2;
    fe25519_mul(&mut x2, x2_v, z2);
    fe25519_tobytes(unsafe { core::slice::from_raw_parts_mut(q, 32) }, &x2);

    unsafe {
        sodium_memzero(
            (&raw mut t) as *mut c_void,
            core::mem::size_of::<[u8; 32]>(),
        )
    };

    0
}

fn edwards_to_montgomery(montgomeryX: &mut Fe25519, edwardsY: Fe25519, edwardsZ: Fe25519) {
    let mut tempX = Fe25519::ZERO;
    let mut tempZ = Fe25519::ZERO;

    fe25519_add(&mut tempX, edwardsZ, edwardsY);
    fe25519_sub(&mut tempZ, edwardsZ, edwardsY);
    fe25519_invert_in_place(&mut tempZ);
    fe25519_mul(montgomeryX, tempX, tempZ);
}

unsafe extern "C" fn crypto_scalarmult_curve25519_ref10_base(q: *mut u8, n: *const u8) -> c_int {
    /* `unsigned char *t = q;` - `t` aliases the output buffer. */
    let t: *mut u8 = q;
    let mut A = Ge25519P3::default();
    let mut pk = Fe25519::ZERO;
    let mut i: u32;

    i = 0;
    while i < 32 {
        unsafe { *t.add(i as usize) = *n.add(i as usize) };
        i += 1;
    }
    unsafe {
        *t.add(0) &= 248;
        *t.add(31) &= 127;
        *t.add(31) |= 64;
    }
    unsafe { _sodium_ge25519_scalarmult_base(&raw mut A, t) };
    edwards_to_montgomery(&mut pk, A.Y, A.Z);
    fe25519_tobytes(unsafe { core::slice::from_raw_parts_mut(q, 32) }, &pk);

    0
}

#[unsafe(no_mangle)]
pub static mut crypto_scalarmult_curve25519_ref10_implementation:
    crypto_scalarmult_curve25519_implementation = crypto_scalarmult_curve25519_implementation {
    mult: Some(crypto_scalarmult_curve25519_ref10),
    mult_base: Some(crypto_scalarmult_curve25519_ref10_base),
};
