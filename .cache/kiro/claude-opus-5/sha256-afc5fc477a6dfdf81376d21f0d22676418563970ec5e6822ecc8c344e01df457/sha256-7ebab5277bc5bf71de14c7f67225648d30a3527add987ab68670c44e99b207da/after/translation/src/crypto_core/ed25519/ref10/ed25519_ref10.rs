//! Translation of c_src/libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use core::ffi::c_int;

use crate::crypto_core::ed25519::ref10::ed25519_ref10_tables::{BASE, BI};
use crate::fe25519::{
    fe25519, fe25519_0, fe25519_1, fe25519_add, fe25519_cmov, fe25519_copy, fe25519_isnegative,
    fe25519_iszero, fe25519_mul, fe25519_mul32, fe25519_neg, fe25519_sq, fe25519_sq2, fe25519_sub,
    ge25519_cached, ge25519_p1p1, ge25519_p2, ge25519_p3, ge25519_precomp,
};

extern "C" {
    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
    fn abort() -> !;
}

#[inline]
unsafe fn load_3(in_: *const u8) -> u64 {
    let mut result: u64;
    result = *in_.add(0) as u64;
    result |= (*in_.add(1) as u64) << 8;
    result |= (*in_.add(2) as u64) << 16;
    result
}

#[inline]
unsafe fn load_4(in_: *const u8) -> u64 {
    let mut result: u64;
    result = *in_.add(0) as u64;
    result |= (*in_.add(1) as u64) << 8;
    result |= (*in_.add(2) as u64) << 16;
    result |= (*in_.add(3) as u64) << 24;
    result
}

// HAVE_TI_MODE undefined: 32-bit limb variant (fe_25_5/constants.h)

/* sqrt(-1) */
static fe25519_sqrtm1: fe25519 = [
    -32595792, -7943725, 9377950, 3500415, 12389472, -272473, -25146209, -2005654, 326686, 11406482,
];

/* sqrt(-486664) */
static ed25519_sqrtam2: fe25519 = [
    -12222970, -8312128, -11511410, 9067497, -15300785, -241793, 25456130, 14121551, -12187136,
    3972024,
];

/* 37095705934669439343138083508754565189542113879843219016388785533085940283555 */
static ed25519_d: fe25519 = [
    -10913610, 13857413, -15372611, 6949391, 114729, -8787816, -6275908, -3247719, -18696448,
    -12055116,
];

/* 2 * d */
static ed25519_d2: fe25519 = [
    -21827239, -5839606, -30745221, 13898782, 229458, 15978800, -12551817, -6495438, 29715968,
    9444199,
];

/* A = 486662 */
const ed25519_A_32: u32 = 486662;
static ed25519_A: fe25519 = [ed25519_A_32 as i32, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/* sqrt(ad - 1) with a = -1 (mod p) */
static ed25519_sqrtadm1: fe25519 = [
    24849947, -153582, -23613485, 6347715, -21072328, -667138, -25271143, -15367704, -870347,
    14525639,
];

/* 1 / sqrt(a - d) */
static ed25519_invsqrtamd: fe25519 = [
    6111485, 4156064, -27798727, 12243468, -25904040, 120897, 20826367, -7060776, 6093568,
    -1986012,
];

/* 1 - d ^ 2 */
static ed25519_onemsqd: fe25519 = [
    6275446, -16617371, -22938544, -3773710, 11667077, 7397348, -27922721, 1766195, -24433858,
    672203,
];

/* (d - 1) ^ 2 */
static ed25519_sqdmone: fe25519 = [
    15551795, -11097455, -13425098, -10125071, -11896535, 10178284, -26634327, 4729244, -5282110,
    -10116402,
];

// ---------------------------------------------------------------------------
// fe_25_5/fe.h (#included by this .c file)
// ---------------------------------------------------------------------------

/*
 Ignores top bit of s.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_fe25519_frombytes(h: *mut i32, s: *const u8) {
    let mut h0: i64 = load_4(s) as i64;
    let mut h1: i64 = (load_3(s.add(4)) << 6) as i64;
    let mut h2: i64 = (load_3(s.add(7)) << 5) as i64;
    let mut h3: i64 = (load_3(s.add(10)) << 3) as i64;
    let mut h4: i64 = (load_3(s.add(13)) << 2) as i64;
    let mut h5: i64 = load_4(s.add(16)) as i64;
    let mut h6: i64 = (load_3(s.add(20)) << 7) as i64;
    let mut h7: i64 = (load_3(s.add(23)) << 5) as i64;
    let mut h8: i64 = (load_3(s.add(26)) << 4) as i64;
    let mut h9: i64 = ((load_3(s.add(29)) & 8388607) << 2) as i64;

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

    carry9 = (h9 + (1i64 << 24)) >> 25;
    h0 += carry9 * 19;
    h9 -= carry9 * ((1u64 << 25) as i64);
    carry1 = (h1 + (1i64 << 24)) >> 25;
    h2 += carry1;
    h1 -= carry1 * ((1u64 << 25) as i64);
    carry3 = (h3 + (1i64 << 24)) >> 25;
    h4 += carry3;
    h3 -= carry3 * ((1u64 << 25) as i64);
    carry5 = (h5 + (1i64 << 24)) >> 25;
    h6 += carry5;
    h5 -= carry5 * ((1u64 << 25) as i64);
    carry7 = (h7 + (1i64 << 24)) >> 25;
    h8 += carry7;
    h7 -= carry7 * ((1u64 << 25) as i64);

    carry0 = (h0 + (1i64 << 25)) >> 26;
    h1 += carry0;
    h0 -= carry0 * ((1u64 << 26) as i64);
    carry2 = (h2 + (1i64 << 25)) >> 26;
    h3 += carry2;
    h2 -= carry2 * ((1u64 << 26) as i64);
    carry4 = (h4 + (1i64 << 25)) >> 26;
    h5 += carry4;
    h4 -= carry4 * ((1u64 << 26) as i64);
    carry6 = (h6 + (1i64 << 25)) >> 26;
    h7 += carry6;
    h6 -= carry6 * ((1u64 << 26) as i64);
    carry8 = (h8 + (1i64 << 25)) >> 26;
    h9 += carry8;
    h8 -= carry8 * ((1u64 << 26) as i64);

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

unsafe fn fe25519_reduce(h: *mut i32, f: *const i32) {
    let mut h0: i32 = *f.add(0);
    let mut h1: i32 = *f.add(1);
    let mut h2: i32 = *f.add(2);
    let mut h3: i32 = *f.add(3);
    let mut h4: i32 = *f.add(4);
    let mut h5: i32 = *f.add(5);
    let mut h6: i32 = *f.add(6);
    let mut h7: i32 = *f.add(7);
    let mut h8: i32 = *f.add(8);
    let mut h9: i32 = *f.add(9);

    let mut q: i32;
    let mut carry0: i32;
    let mut carry1: i32;
    let mut carry2: i32;
    let mut carry3: i32;
    let mut carry4: i32;
    let mut carry5: i32;
    let mut carry6: i32;
    let mut carry7: i32;
    let mut carry8: i32;
    let mut carry9: i32;

    q = ((19i32.wrapping_mul(h9)).wrapping_add((1u32 << 24) as i32)) >> 25;
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

    h0 = h0.wrapping_add(19i32.wrapping_mul(q));

    carry0 = h0 >> 26;
    h1 = h1.wrapping_add(carry0);
    h0 -= carry0 * ((1u32 << 26) as i32);
    carry1 = h1 >> 25;
    h2 = h2.wrapping_add(carry1);
    h1 -= carry1 * ((1u32 << 25) as i32);
    carry2 = h2 >> 26;
    h3 = h3.wrapping_add(carry2);
    h2 -= carry2 * ((1u32 << 26) as i32);
    carry3 = h3 >> 25;
    h4 = h4.wrapping_add(carry3);
    h3 -= carry3 * ((1u32 << 25) as i32);
    carry4 = h4 >> 26;
    h5 = h5.wrapping_add(carry4);
    h4 -= carry4 * ((1u32 << 26) as i32);
    carry5 = h5 >> 25;
    h6 = h6.wrapping_add(carry5);
    h5 -= carry5 * ((1u32 << 25) as i32);
    carry6 = h6 >> 26;
    h7 = h7.wrapping_add(carry6);
    h6 -= carry6 * ((1u32 << 26) as i32);
    carry7 = h7 >> 25;
    h8 = h8.wrapping_add(carry7);
    h7 -= carry7 * ((1u32 << 25) as i32);
    carry8 = h8 >> 26;
    h9 = h9.wrapping_add(carry8);
    h8 -= carry8 * ((1u32 << 26) as i32);
    carry9 = h9 >> 25;
    h9 -= carry9 * ((1u32 << 25) as i32);

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
    let mut t: fe25519 = [0; 10];

    fe25519_reduce(t.as_mut_ptr(), h);
    *s.add(0) = (t[0] >> 0) as u8;
    *s.add(1) = (t[0] >> 8) as u8;
    *s.add(2) = (t[0] >> 16) as u8;
    *s.add(3) = ((t[0] >> 24) | (t[1].wrapping_mul((1u32 << 2) as i32))) as u8;
    *s.add(4) = (t[1] >> 6) as u8;
    *s.add(5) = (t[1] >> 14) as u8;
    *s.add(6) = ((t[1] >> 22) | (t[2].wrapping_mul((1u32 << 3) as i32))) as u8;
    *s.add(7) = (t[2] >> 5) as u8;
    *s.add(8) = (t[2] >> 13) as u8;
    *s.add(9) = ((t[2] >> 21) | (t[3].wrapping_mul((1u32 << 5) as i32))) as u8;
    *s.add(10) = (t[3] >> 3) as u8;
    *s.add(11) = (t[3] >> 11) as u8;
    *s.add(12) = ((t[3] >> 19) | (t[4].wrapping_mul((1u32 << 6) as i32))) as u8;
    *s.add(13) = (t[4] >> 2) as u8;
    *s.add(14) = (t[4] >> 10) as u8;
    *s.add(15) = (t[4] >> 18) as u8;
    *s.add(16) = (t[5] >> 0) as u8;
    *s.add(17) = (t[5] >> 8) as u8;
    *s.add(18) = (t[5] >> 16) as u8;
    *s.add(19) = ((t[5] >> 24) | (t[6].wrapping_mul((1u32 << 1) as i32))) as u8;
    *s.add(20) = (t[6] >> 7) as u8;
    *s.add(21) = (t[6] >> 15) as u8;
    *s.add(22) = ((t[6] >> 23) | (t[7].wrapping_mul((1u32 << 3) as i32))) as u8;
    *s.add(23) = (t[7] >> 5) as u8;
    *s.add(24) = (t[7] >> 13) as u8;
    *s.add(25) = ((t[7] >> 21) | (t[8].wrapping_mul((1u32 << 4) as i32))) as u8;
    *s.add(26) = (t[8] >> 4) as u8;
    *s.add(27) = (t[8] >> 12) as u8;
    *s.add(28) = ((t[8] >> 20) | (t[9].wrapping_mul((1u32 << 6) as i32))) as u8;
    *s.add(29) = (t[9] >> 2) as u8;
    *s.add(30) = (t[9] >> 10) as u8;
    *s.add(31) = (t[9] >> 18) as u8;
}

// ---------------------------------------------------------------------------
// Field arithmetic (rest of ed25519_ref10.c)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn fe25519_sqmul(s: *mut i32, n: c_int, a: *const i32) {
    let mut i: c_int = 0;
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
    let mut t0: fe25519 = [0; 10];
    let mut t1: fe25519 = [0; 10];
    let mut t2: fe25519 = [0; 10];
    let mut t3: fe25519 = [0; 10];
    let mut i: c_int;

    fe25519_sq(t0.as_mut_ptr(), z);
    fe25519_sq(t1.as_mut_ptr(), t0.as_ptr());
    fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
    fe25519_mul(t1.as_mut_ptr(), z, t1.as_ptr());
    fe25519_mul(t0.as_mut_ptr(), t0.as_ptr(), t1.as_ptr());
    fe25519_sq(t2.as_mut_ptr(), t0.as_ptr());
    fe25519_mul(t1.as_mut_ptr(), t1.as_ptr(), t2.as_ptr());
    fe25519_sq(t2.as_mut_ptr(), t1.as_ptr());
    i = 1;
    while i < 5 {
        fe25519_sq(t2.as_mut_ptr(), t2.as_ptr());
        i += 1;
    }
    fe25519_mul(t1.as_mut_ptr(), t2.as_ptr(), t1.as_ptr());
    fe25519_sq(t2.as_mut_ptr(), t1.as_ptr());
    i = 1;
    while i < 10 {
        fe25519_sq(t2.as_mut_ptr(), t2.as_ptr());
        i += 1;
    }
    fe25519_mul(t2.as_mut_ptr(), t2.as_ptr(), t1.as_ptr());
    fe25519_sq(t3.as_mut_ptr(), t2.as_ptr());
    i = 1;
    while i < 20 {
        fe25519_sq(t3.as_mut_ptr(), t3.as_ptr());
        i += 1;
    }
    fe25519_mul(t2.as_mut_ptr(), t3.as_ptr(), t2.as_ptr());
    i = 1;
    while i < 11 {
        fe25519_sq(t2.as_mut_ptr(), t2.as_ptr());
        i += 1;
    }
    fe25519_mul(t1.as_mut_ptr(), t2.as_ptr(), t1.as_ptr());
    fe25519_sq(t2.as_mut_ptr(), t1.as_ptr());
    i = 1;
    while i < 50 {
        fe25519_sq(t2.as_mut_ptr(), t2.as_ptr());
        i += 1;
    }
    fe25519_mul(t2.as_mut_ptr(), t2.as_ptr(), t1.as_ptr());
    fe25519_sq(t3.as_mut_ptr(), t2.as_ptr());
    i = 1;
    while i < 100 {
        fe25519_sq(t3.as_mut_ptr(), t3.as_ptr());
        i += 1;
    }
    fe25519_mul(t2.as_mut_ptr(), t3.as_ptr(), t2.as_ptr());
    i = 1;
    while i < 51 {
        fe25519_sq(t2.as_mut_ptr(), t2.as_ptr());
        i += 1;
    }
    fe25519_mul(t1.as_mut_ptr(), t2.as_ptr(), t1.as_ptr());
    i = 1;
    while i < 6 {
        fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
        i += 1;
    }
    fe25519_mul(out, t1.as_ptr(), t0.as_ptr());
}

/*
 * returns z^((p-5)/8) = z^(2^252-3)
 */
unsafe fn fe25519_pow22523(out: *mut i32, z: *const i32) {
    let mut t0: fe25519 = [0; 10];
    let mut t1: fe25519 = [0; 10];
    let mut t2: fe25519 = [0; 10];
    let mut i: c_int;

    fe25519_sq(t0.as_mut_ptr(), z);
    fe25519_sq(t1.as_mut_ptr(), t0.as_ptr());
    fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
    fe25519_mul(t1.as_mut_ptr(), z, t1.as_ptr());
    fe25519_mul(t0.as_mut_ptr(), t0.as_ptr(), t1.as_ptr());
    fe25519_sq(t0.as_mut_ptr(), t0.as_ptr());
    fe25519_mul(t0.as_mut_ptr(), t1.as_ptr(), t0.as_ptr());
    fe25519_sq(t1.as_mut_ptr(), t0.as_ptr());
    i = 1;
    while i < 5 {
        fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
        i += 1;
    }
    fe25519_mul(t0.as_mut_ptr(), t1.as_ptr(), t0.as_ptr());
    fe25519_sq(t1.as_mut_ptr(), t0.as_ptr());
    i = 1;
    while i < 10 {
        fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
        i += 1;
    }
    fe25519_mul(t1.as_mut_ptr(), t1.as_ptr(), t0.as_ptr());
    fe25519_sq(t2.as_mut_ptr(), t1.as_ptr());
    i = 1;
    while i < 20 {
        fe25519_sq(t2.as_mut_ptr(), t2.as_ptr());
        i += 1;
    }
    fe25519_mul(t1.as_mut_ptr(), t2.as_ptr(), t1.as_ptr());
    i = 1;
    while i < 11 {
        fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
        i += 1;
    }
    fe25519_mul(t0.as_mut_ptr(), t1.as_ptr(), t0.as_ptr());
    fe25519_sq(t1.as_mut_ptr(), t0.as_ptr());
    i = 1;
    while i < 50 {
        fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
        i += 1;
    }
    fe25519_mul(t1.as_mut_ptr(), t1.as_ptr(), t0.as_ptr());
    fe25519_sq(t2.as_mut_ptr(), t1.as_ptr());
    i = 1;
    while i < 100 {
        fe25519_sq(t2.as_mut_ptr(), t2.as_ptr());
        i += 1;
    }
    fe25519_mul(t1.as_mut_ptr(), t2.as_ptr(), t1.as_ptr());
    i = 1;
    while i < 51 {
        fe25519_sq(t1.as_mut_ptr(), t1.as_ptr());
        i += 1;
    }
    fe25519_mul(t0.as_mut_ptr(), t1.as_ptr(), t0.as_ptr());
    fe25519_sq(t0.as_mut_ptr(), t0.as_ptr());
    fe25519_sq(t0.as_mut_ptr(), t0.as_ptr());
    fe25519_mul(out, t0.as_ptr(), z);
}

#[inline]
unsafe fn fe25519_cneg(h: *mut i32, b: c_int) {
    let mut negf: fe25519 = [0; 10];

    fe25519_neg(negf.as_mut_ptr(), h);
    fe25519_cmov(h, negf.as_ptr(), b as core::ffi::c_uint);
}

#[inline]
unsafe fn fe25519_abs(h: *mut i32) {
    fe25519_cneg(h, fe25519_isnegative(h));
}

unsafe fn fe25519_unchecked_sqrt(x: *mut i32, x2: *const i32) {
    let mut p_root: fe25519 = [0; 10];
    let mut m_root: fe25519 = [0; 10];
    let mut m_root2: fe25519 = [0; 10];
    let mut e: fe25519 = [0; 10];

    fe25519_pow22523(e.as_mut_ptr(), x2);
    fe25519_mul(p_root.as_mut_ptr(), e.as_ptr(), x2);
    fe25519_mul(m_root.as_mut_ptr(), p_root.as_ptr(), fe25519_sqrtm1.as_ptr());
    fe25519_sq(m_root2.as_mut_ptr(), m_root.as_ptr());
    fe25519_sub(e.as_mut_ptr(), x2, m_root2.as_ptr());
    fe25519_copy(x, p_root.as_ptr());
    fe25519_cmov(x, m_root.as_ptr(), fe25519_iszero(e.as_ptr()) as core::ffi::c_uint);
}

unsafe fn fe25519_sqrt(x: *mut i32, x2: *const i32) -> c_int {
    let mut check: fe25519 = [0; 10];
    let mut x2_copy: fe25519 = [0; 10];

    fe25519_copy(x2_copy.as_mut_ptr(), x2);
    fe25519_unchecked_sqrt(x, x2);
    fe25519_sq(check.as_mut_ptr(), x);
    fe25519_sub(check.as_mut_ptr(), check.as_ptr(), x2_copy.as_ptr());

    fe25519_iszero(check.as_ptr()) - 1
}

unsafe fn fe25519_notsquare(x: *const i32) -> c_int {
    let mut _10: fe25519 = [0; 10];
    let mut _11: fe25519 = [0; 10];
    let mut _1100: fe25519 = [0; 10];
    let mut _1111: fe25519 = [0; 10];
    let mut _11110000: fe25519 = [0; 10];
    let mut _11111111: fe25519 = [0; 10];
    let mut t: fe25519 = [0; 10];
    let mut u: fe25519 = [0; 10];
    let mut v: fe25519 = [0; 10];
    let mut s: [u8; 32] = [0; 32];

    /* Jacobi symbol - x^((p-1)/2) */
    fe25519_mul(_10.as_mut_ptr(), x, x);
    fe25519_mul(_11.as_mut_ptr(), x, _10.as_ptr());
    fe25519_sq(_1100.as_mut_ptr(), _11.as_ptr());
    fe25519_sq(_1100.as_mut_ptr(), _1100.as_ptr());
    fe25519_mul(_1111.as_mut_ptr(), _11.as_ptr(), _1100.as_ptr());
    fe25519_sq(_11110000.as_mut_ptr(), _1111.as_ptr());
    fe25519_sq(_11110000.as_mut_ptr(), _11110000.as_ptr());
    fe25519_sq(_11110000.as_mut_ptr(), _11110000.as_ptr());
    fe25519_sq(_11110000.as_mut_ptr(), _11110000.as_ptr());
    fe25519_mul(_11111111.as_mut_ptr(), _1111.as_ptr(), _11110000.as_ptr());
    fe25519_copy(t.as_mut_ptr(), _11111111.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 2, _11.as_ptr());
    fe25519_copy(u.as_mut_ptr(), t.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 10, u.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 10, u.as_ptr());
    fe25519_copy(v.as_mut_ptr(), t.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 30, v.as_ptr());
    fe25519_copy(v.as_mut_ptr(), t.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 60, v.as_ptr());
    fe25519_copy(v.as_mut_ptr(), t.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 120, v.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 10, u.as_ptr());
    fe25519_sqmul(t.as_mut_ptr(), 3, _11.as_ptr());
    fe25519_sq(t.as_mut_ptr(), t.as_ptr());

    _sodium_fe25519_tobytes(s.as_mut_ptr(), t.as_ptr());

    (s[1] & 1) as c_int
}

/*
 r = p + q
 */
unsafe fn ge25519_add_cached(r: *mut ge25519_p1p1, p: *const ge25519_p3, q: *const ge25519_cached) {
    let mut t0: fe25519 = [0; 10];

    fe25519_add((*r).X.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_sub((*r).Y.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_mul((*r).Z.as_mut_ptr(), (*r).X.as_ptr(), (*q).YplusX.as_ptr());
    fe25519_mul((*r).Y.as_mut_ptr(), (*r).Y.as_ptr(), (*q).YminusX.as_ptr());
    fe25519_mul((*r).T.as_mut_ptr(), (*q).T2d.as_ptr(), (*p).T.as_ptr());
    fe25519_mul((*r).X.as_mut_ptr(), (*p).Z.as_ptr(), (*q).Z.as_ptr());
    fe25519_add(t0.as_mut_ptr(), (*r).X.as_ptr(), (*r).X.as_ptr());
    fe25519_sub((*r).X.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_add((*r).Y.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_add((*r).Z.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
    fe25519_sub((*r).T.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
}

unsafe fn slide_vartime(r: *mut i8, a: *const u8) {
    let mut i: c_int;
    let mut b: c_int;
    let mut k: c_int;
    let mut ribs: c_int;
    let mut cmp: c_int;

    i = 0;
    while i < 256 {
        *r.add(i as usize) =
            (1 & ((*a.add((i >> 3) as usize) as c_int) >> (i & 7))) as i8;
        i += 1;
    }
    i = 0;
    while i < 256 {
        if *r.add(i as usize) == 0 {
            i += 1;
            continue;
        }
        b = 1;
        while b <= 6 && i + b < 256 {
            if *r.add((i + b) as usize) == 0 {
                b += 1;
                continue;
            }
            ribs = (*r.add((i + b) as usize) as c_int) << b;
            cmp = (*r.add(i as usize) as c_int) + ribs;
            if cmp <= 15 {
                *r.add(i as usize) = cmp as i8;
                *r.add((i + b) as usize) = 0;
            } else {
                cmp = (*r.add(i as usize) as c_int) - ribs;
                if cmp < -15 {
                    break;
                }
                *r.add(i as usize) = cmp as i8;
                k = i + b;
                while k < 256 {
                    if *r.add(k as usize) == 0 {
                        *r.add(k as usize) = 1;
                        break;
                    }
                    *r.add(k as usize) = 0;
                    k += 1;
                }
            }
            b += 1;
        }
        i += 1;
    }
}

static mut optblocker_u8: u8 = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int {
    let mut u: fe25519 = [0; 10];
    let mut v: fe25519 = [0; 10];
    let mut vxx: fe25519 = [0; 10];
    let mut m_root_check: fe25519 = [0; 10];
    let mut p_root_check: fe25519 = [0; 10];
    let mut negx: fe25519 = [0; 10];
    let mut x_sqrtm1: fe25519 = [0; 10];
    let has_m_root: c_int;
    let has_p_root: c_int;

    _sodium_fe25519_frombytes((*h).Y.as_mut_ptr(), s);
    fe25519_1((*h).Z.as_mut_ptr());
    fe25519_sq(u.as_mut_ptr(), (*h).Y.as_ptr());
    fe25519_mul(v.as_mut_ptr(), u.as_ptr(), ed25519_d.as_ptr());
    fe25519_sub(u.as_mut_ptr(), u.as_ptr(), (*h).Z.as_ptr()); /* u = y^2-1 */
    fe25519_add(v.as_mut_ptr(), v.as_ptr(), (*h).Z.as_ptr()); /* v = dy^2+1 */

    fe25519_mul((*h).X.as_mut_ptr(), u.as_ptr(), v.as_ptr());
    fe25519_pow22523((*h).X.as_mut_ptr(), (*h).X.as_ptr());
    fe25519_mul((*h).X.as_mut_ptr(), u.as_ptr(), (*h).X.as_ptr()); /* u((uv)^((q-5)/8)) */

    fe25519_sq(vxx.as_mut_ptr(), (*h).X.as_ptr());
    fe25519_mul(vxx.as_mut_ptr(), vxx.as_ptr(), v.as_ptr());
    fe25519_sub(m_root_check.as_mut_ptr(), vxx.as_ptr(), u.as_ptr()); /* vx^2-u */
    fe25519_add(p_root_check.as_mut_ptr(), vxx.as_ptr(), u.as_ptr()); /* vx^2+u */
    has_m_root = fe25519_iszero(m_root_check.as_ptr());
    has_p_root = fe25519_iszero(p_root_check.as_ptr());
    fe25519_mul(x_sqrtm1.as_mut_ptr(), (*h).X.as_ptr(), fe25519_sqrtm1.as_ptr()); /* x*sqrt(-1) */
    fe25519_cmov((*h).X.as_mut_ptr(), x_sqrtm1.as_ptr(), (1 - has_m_root) as core::ffi::c_uint);

    fe25519_neg(negx.as_mut_ptr(), (*h).X.as_ptr());
    fe25519_cmov(
        (*h).X.as_mut_ptr(),
        negx.as_ptr(),
        (fe25519_isnegative((*h).X.as_ptr())
            ^ ((((*s.add(31) as c_int) >> 5) ^ (optblocker_u8 as c_int)) >> 2))
            as core::ffi::c_uint,
    );
    fe25519_mul((*h).T.as_mut_ptr(), (*h).X.as_ptr(), (*h).Y.as_ptr());

    (has_m_root | has_p_root) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes_negate_vartime(
    h: *mut ge25519_p3,
    s: *const u8,
) -> c_int {
    let mut u: fe25519 = [0; 10];
    let mut v: fe25519 = [0; 10];
    let mut v3: fe25519 = [0; 10];
    let mut vxx: fe25519 = [0; 10];
    let mut m_root_check: fe25519 = [0; 10];
    let mut p_root_check: fe25519 = [0; 10];

    _sodium_fe25519_frombytes((*h).Y.as_mut_ptr(), s);
    fe25519_1((*h).Z.as_mut_ptr());
    fe25519_sq(u.as_mut_ptr(), (*h).Y.as_ptr());
    fe25519_mul(v.as_mut_ptr(), u.as_ptr(), ed25519_d.as_ptr());
    fe25519_sub(u.as_mut_ptr(), u.as_ptr(), (*h).Z.as_ptr()); /* u = y^2-1 */
    fe25519_add(v.as_mut_ptr(), v.as_ptr(), (*h).Z.as_ptr()); /* v = dy^2+1 */

    fe25519_sq(v3.as_mut_ptr(), v.as_ptr());
    fe25519_mul(v3.as_mut_ptr(), v3.as_ptr(), v.as_ptr()); /* v3 = v^3 */
    fe25519_sq((*h).X.as_mut_ptr(), v3.as_ptr());
    fe25519_mul((*h).X.as_mut_ptr(), (*h).X.as_ptr(), v.as_ptr());
    fe25519_mul((*h).X.as_mut_ptr(), (*h).X.as_ptr(), u.as_ptr()); /* x = uv^7 */

    fe25519_pow22523((*h).X.as_mut_ptr(), (*h).X.as_ptr()); /* x = (uv^7)^((q-5)/8) */
    fe25519_mul((*h).X.as_mut_ptr(), (*h).X.as_ptr(), v3.as_ptr());
    fe25519_mul((*h).X.as_mut_ptr(), (*h).X.as_ptr(), u.as_ptr()); /* x = uv^3(uv^7)^((q-5)/8) */

    fe25519_sq(vxx.as_mut_ptr(), (*h).X.as_ptr());
    fe25519_mul(vxx.as_mut_ptr(), vxx.as_ptr(), v.as_ptr());
    fe25519_sub(m_root_check.as_mut_ptr(), vxx.as_ptr(), u.as_ptr()); /* vx^2-u */
    if fe25519_iszero(m_root_check.as_ptr()) == 0 {
        fe25519_add(p_root_check.as_mut_ptr(), vxx.as_ptr(), u.as_ptr()); /* vx^2+u */
        if fe25519_iszero(p_root_check.as_ptr()) == 0 {
            return -1;
        }
        fe25519_mul((*h).X.as_mut_ptr(), (*h).X.as_ptr(), fe25519_sqrtm1.as_ptr());
    }

    if fe25519_isnegative((*h).X.as_ptr()) == ((*s.add(31) as c_int) >> 7) {
        fe25519_neg((*h).X.as_mut_ptr(), (*h).X.as_ptr());
    }
    fe25519_mul((*h).T.as_mut_ptr(), (*h).X.as_ptr(), (*h).Y.as_ptr());

    0
}

/*
 r = p + q
 */
unsafe fn ge25519_add_precomp(r: *mut ge25519_p1p1, p: *const ge25519_p3, q: *const ge25519_precomp) {
    let mut t0: fe25519 = [0; 10];

    fe25519_add((*r).X.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_sub((*r).Y.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_mul((*r).Z.as_mut_ptr(), (*r).X.as_ptr(), (*q).yplusx.as_ptr());
    fe25519_mul((*r).Y.as_mut_ptr(), (*r).Y.as_ptr(), (*q).yminusx.as_ptr());
    fe25519_mul((*r).T.as_mut_ptr(), (*q).xy2d.as_ptr(), (*p).T.as_ptr());
    fe25519_add(t0.as_mut_ptr(), (*p).Z.as_ptr(), (*p).Z.as_ptr());
    fe25519_sub((*r).X.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_add((*r).Y.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_add((*r).Z.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
    fe25519_sub((*r).T.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
}

/*
 r = p - q
 */
unsafe fn ge25519_sub_precomp(r: *mut ge25519_p1p1, p: *const ge25519_p3, q: *const ge25519_precomp) {
    let mut t0: fe25519 = [0; 10];

    fe25519_add((*r).X.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_sub((*r).Y.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_mul((*r).Z.as_mut_ptr(), (*r).X.as_ptr(), (*q).yminusx.as_ptr());
    fe25519_mul((*r).Y.as_mut_ptr(), (*r).Y.as_ptr(), (*q).yplusx.as_ptr());
    fe25519_mul((*r).T.as_mut_ptr(), (*q).xy2d.as_ptr(), (*p).T.as_ptr());
    fe25519_add(t0.as_mut_ptr(), (*p).Z.as_ptr(), (*p).Z.as_ptr());
    fe25519_sub((*r).X.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_add((*r).Y.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_sub((*r).Z.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
    fe25519_add((*r).T.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
}

/*
 r = p
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p2(r: *mut ge25519_p2, p: *const ge25519_p1p1) {
    fe25519_mul((*r).X.as_mut_ptr(), (*p).X.as_ptr(), (*p).T.as_ptr());
    fe25519_mul((*r).Y.as_mut_ptr(), (*p).Y.as_ptr(), (*p).Z.as_ptr());
    fe25519_mul((*r).Z.as_mut_ptr(), (*p).Z.as_ptr(), (*p).T.as_ptr());
}

/*
 r = p
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p3(r: *mut ge25519_p3, p: *const ge25519_p1p1) {
    fe25519_mul((*r).X.as_mut_ptr(), (*p).X.as_ptr(), (*p).T.as_ptr());
    fe25519_mul((*r).Y.as_mut_ptr(), (*p).Y.as_ptr(), (*p).Z.as_ptr());
    fe25519_mul((*r).Z.as_mut_ptr(), (*p).Z.as_ptr(), (*p).T.as_ptr());
    fe25519_mul((*r).T.as_mut_ptr(), (*p).X.as_ptr(), (*p).Y.as_ptr());
}

/*
 r = p
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p2_to_p3(r: *mut ge25519_p3, p: *const ge25519_p2) {
    fe25519_copy((*r).X.as_mut_ptr(), (*p).X.as_ptr());
    fe25519_copy((*r).Y.as_mut_ptr(), (*p).Y.as_ptr());
    fe25519_copy((*r).Z.as_mut_ptr(), (*p).Z.as_ptr());
    fe25519_mul((*r).T.as_mut_ptr(), (*p).X.as_ptr(), (*p).Y.as_ptr());
}

unsafe fn ge25519_p2_0(h: *mut ge25519_p2) {
    fe25519_0((*h).X.as_mut_ptr());
    fe25519_1((*h).Y.as_mut_ptr());
    fe25519_1((*h).Z.as_mut_ptr());
}

/*
 r = 2 * p
 */
unsafe fn ge25519_p2_dbl(r: *mut ge25519_p1p1, p: *const ge25519_p2) {
    let mut t0: fe25519 = [0; 10];

    fe25519_sq((*r).X.as_mut_ptr(), (*p).X.as_ptr());
    fe25519_sq((*r).Z.as_mut_ptr(), (*p).Y.as_ptr());
    fe25519_sq2((*r).T.as_mut_ptr(), (*p).Z.as_ptr());
    fe25519_add((*r).Y.as_mut_ptr(), (*p).X.as_ptr(), (*p).Y.as_ptr());
    fe25519_sq(t0.as_mut_ptr(), (*r).Y.as_ptr());
    fe25519_add((*r).Y.as_mut_ptr(), (*r).Z.as_ptr(), (*r).X.as_ptr());
    fe25519_sub((*r).Z.as_mut_ptr(), (*r).Z.as_ptr(), (*r).X.as_ptr());
    fe25519_sub((*r).X.as_mut_ptr(), t0.as_ptr(), (*r).Y.as_ptr());
    fe25519_sub((*r).T.as_mut_ptr(), (*r).T.as_ptr(), (*r).Z.as_ptr());
}

unsafe fn ge25519_p3_0(h: *mut ge25519_p3) {
    fe25519_0((*h).X.as_mut_ptr());
    fe25519_1((*h).Y.as_mut_ptr());
    fe25519_1((*h).Z.as_mut_ptr());
    fe25519_0((*h).T.as_mut_ptr());
}

unsafe fn ge25519_cached_0(h: *mut ge25519_cached) {
    fe25519_1((*h).YplusX.as_mut_ptr());
    fe25519_1((*h).YminusX.as_mut_ptr());
    fe25519_1((*h).Z.as_mut_ptr());
    fe25519_0((*h).T2d.as_mut_ptr());
}

/*
 r = p
 */
unsafe fn ge25519_p3_to_cached(r: *mut ge25519_cached, p: *const ge25519_p3) {
    fe25519_add((*r).YplusX.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_sub((*r).YminusX.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_copy((*r).Z.as_mut_ptr(), (*p).Z.as_ptr());
    fe25519_mul((*r).T2d.as_mut_ptr(), (*p).T.as_ptr(), ed25519_d2.as_ptr());
}

unsafe fn ge25519_p3_to_precomp(pi: *mut ge25519_precomp, p: *const ge25519_p3) {
    let mut recip: fe25519 = [0; 10];
    let mut x: fe25519 = [0; 10];
    let mut y: fe25519 = [0; 10];
    let mut xy: fe25519 = [0; 10];

    _sodium_fe25519_invert(recip.as_mut_ptr(), (*p).Z.as_ptr());
    fe25519_mul(x.as_mut_ptr(), (*p).X.as_ptr(), recip.as_ptr());
    fe25519_mul(y.as_mut_ptr(), (*p).Y.as_ptr(), recip.as_ptr());
    fe25519_add((*pi).yplusx.as_mut_ptr(), y.as_ptr(), x.as_ptr());
    fe25519_sub((*pi).yminusx.as_mut_ptr(), y.as_ptr(), x.as_ptr());
    fe25519_mul(xy.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    fe25519_mul((*pi).xy2d.as_mut_ptr(), xy.as_ptr(), ed25519_d2.as_ptr());
}

/*
 r = p
 */
unsafe fn ge25519_p3_to_p2(r: *mut ge25519_p2, p: *const ge25519_p3) {
    fe25519_copy((*r).X.as_mut_ptr(), (*p).X.as_ptr());
    fe25519_copy((*r).Y.as_mut_ptr(), (*p).Y.as_ptr());
    fe25519_copy((*r).Z.as_mut_ptr(), (*p).Z.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3) {
    let mut recip: fe25519 = [0; 10];
    let mut x: fe25519 = [0; 10];
    let mut y: fe25519 = [0; 10];

    _sodium_fe25519_invert(recip.as_mut_ptr(), (*h).Z.as_ptr());
    fe25519_mul(x.as_mut_ptr(), (*h).X.as_ptr(), recip.as_ptr());
    fe25519_mul(y.as_mut_ptr(), (*h).Y.as_ptr(), recip.as_ptr());
    _sodium_fe25519_tobytes(s, y.as_ptr());
    *s.add(31) ^= (fe25519_isnegative(x.as_ptr()) << 7) as u8;
}

/*
 r = 2 * p
 */
unsafe fn ge25519_p3_dbl(r: *mut ge25519_p1p1, p: *const ge25519_p3) {
    let mut q: ge25519_p2 = ge25519_p2::default();
    ge25519_p3_to_p2(&mut q, p);
    ge25519_p2_dbl(r, &q);
}

unsafe fn ge25519_precomp_0(h: *mut ge25519_precomp) {
    fe25519_1((*h).yplusx.as_mut_ptr());
    fe25519_1((*h).yminusx.as_mut_ptr());
    fe25519_0((*h).xy2d.as_mut_ptr());
}

unsafe fn equal(b: i8, c: i8) -> u8 {
    // HAVE_INLINE_ASM undefined: portable branch
    let x: u8 = (b as u8) ^ (c as u8); /* 0: yes; 1..255: no */
    let mut y: u32 = x as u32; /* 0: yes; 1..255: no */

    y = y.wrapping_sub(1);
    (((y >> 29) ^ (optblocker_u8 as u32)) >> 2) as u8 /* 1: yes; 0: no */
}

unsafe fn negative(b: i8) -> u8 {
    // HAVE_INLINE_ASM undefined: portable branch
    let x: u8 = b as u8; /* 0..127: no 128..255: yes */
    (((x >> 5) ^ optblocker_u8) >> 2) as u8 /* 1: yes; 0: no */
}

unsafe fn ge25519_cmov(t: *mut ge25519_precomp, u: *const ge25519_precomp, b: u8) {
    fe25519_cmov((*t).yplusx.as_mut_ptr(), (*u).yplusx.as_ptr(), b as core::ffi::c_uint);
    fe25519_cmov((*t).yminusx.as_mut_ptr(), (*u).yminusx.as_ptr(), b as core::ffi::c_uint);
    fe25519_cmov((*t).xy2d.as_mut_ptr(), (*u).xy2d.as_ptr(), b as core::ffi::c_uint);
}

unsafe fn ge25519_cmov_cached(t: *mut ge25519_cached, u: *const ge25519_cached, b: u8) {
    fe25519_cmov((*t).YplusX.as_mut_ptr(), (*u).YplusX.as_ptr(), b as core::ffi::c_uint);
    fe25519_cmov((*t).YminusX.as_mut_ptr(), (*u).YminusX.as_ptr(), b as core::ffi::c_uint);
    fe25519_cmov((*t).Z.as_mut_ptr(), (*u).Z.as_ptr(), b as core::ffi::c_uint);
    fe25519_cmov((*t).T2d.as_mut_ptr(), (*u).T2d.as_ptr(), b as core::ffi::c_uint);
}

unsafe fn ge25519_cmov8(t: *mut ge25519_precomp, precomp: *const ge25519_precomp, b: i8) {
    let mut minust: ge25519_precomp = ge25519_precomp::default();
    let bnegative: u8 = negative(b);
    let babs: u8 =
        (b.wrapping_sub(((bnegative as i8).wrapping_neg() & b).wrapping_mul((1i8) << 1))) as u8;

    ge25519_precomp_0(t);
    ge25519_cmov(t, precomp.add(0), equal(babs as i8, 1));
    ge25519_cmov(t, precomp.add(1), equal(babs as i8, 2));
    ge25519_cmov(t, precomp.add(2), equal(babs as i8, 3));
    ge25519_cmov(t, precomp.add(3), equal(babs as i8, 4));
    ge25519_cmov(t, precomp.add(4), equal(babs as i8, 5));
    ge25519_cmov(t, precomp.add(5), equal(babs as i8, 6));
    ge25519_cmov(t, precomp.add(6), equal(babs as i8, 7));
    ge25519_cmov(t, precomp.add(7), equal(babs as i8, 8));
    fe25519_copy(minust.yplusx.as_mut_ptr(), (*t).yminusx.as_ptr());
    fe25519_copy(minust.yminusx.as_mut_ptr(), (*t).yplusx.as_ptr());
    fe25519_neg(minust.xy2d.as_mut_ptr(), (*t).xy2d.as_ptr());
    ge25519_cmov(t, &minust, bnegative);
}

unsafe fn ge25519_cmov8_base(t: *mut ge25519_precomp, pos: c_int, b: i8) {
    // base[i][j] = (j+1)*256^i*B  -- from BASE table (fe_25_5/base.h)
    let base: *const ge25519_precomp =
        &BASE[pos as usize][0] as *const [fe25519; 3] as *const ge25519_precomp;
    ge25519_cmov8(t, base, b);
}

unsafe fn ge25519_cmov8_cached(t: *mut ge25519_cached, cached: *const ge25519_cached, b: i8) {
    let mut minust: ge25519_cached = ge25519_cached::default();
    let bnegative: u8 = negative(b);
    let babs: u8 =
        (b.wrapping_sub(((bnegative as i8).wrapping_neg() & b).wrapping_mul((1i8) << 1))) as u8;

    ge25519_cached_0(t);
    ge25519_cmov_cached(t, cached.add(0), equal(babs as i8, 1));
    ge25519_cmov_cached(t, cached.add(1), equal(babs as i8, 2));
    ge25519_cmov_cached(t, cached.add(2), equal(babs as i8, 3));
    ge25519_cmov_cached(t, cached.add(3), equal(babs as i8, 4));
    ge25519_cmov_cached(t, cached.add(4), equal(babs as i8, 5));
    ge25519_cmov_cached(t, cached.add(5), equal(babs as i8, 6));
    ge25519_cmov_cached(t, cached.add(6), equal(babs as i8, 7));
    ge25519_cmov_cached(t, cached.add(7), equal(babs as i8, 8));
    fe25519_copy(minust.YplusX.as_mut_ptr(), (*t).YminusX.as_ptr());
    fe25519_copy(minust.YminusX.as_mut_ptr(), (*t).YplusX.as_ptr());
    fe25519_copy(minust.Z.as_mut_ptr(), (*t).Z.as_ptr());
    fe25519_neg(minust.T2d.as_mut_ptr(), (*t).T2d.as_ptr());
    ge25519_cmov_cached(t, &minust, bnegative);
}

/*
 r = p - q
 */
unsafe fn ge25519_sub_cached(r: *mut ge25519_p1p1, p: *const ge25519_p3, q: *const ge25519_cached) {
    let mut t0: fe25519 = [0; 10];

    fe25519_add((*r).X.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_sub((*r).Y.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_mul((*r).Z.as_mut_ptr(), (*r).X.as_ptr(), (*q).YminusX.as_ptr());
    fe25519_mul((*r).Y.as_mut_ptr(), (*r).Y.as_ptr(), (*q).YplusX.as_ptr());
    fe25519_mul((*r).T.as_mut_ptr(), (*q).T2d.as_ptr(), (*p).T.as_ptr());
    fe25519_mul((*r).X.as_mut_ptr(), (*p).Z.as_ptr(), (*q).Z.as_ptr());
    fe25519_add(t0.as_mut_ptr(), (*r).X.as_ptr(), (*r).X.as_ptr());
    fe25519_sub((*r).X.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_add((*r).Y.as_mut_ptr(), (*r).Z.as_ptr(), (*r).Y.as_ptr());
    fe25519_sub((*r).Z.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
    fe25519_add((*r).T.as_mut_ptr(), t0.as_ptr(), (*r).T.as_ptr());
}

/* LCOV_EXCL_START */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_tobytes(s: *mut u8, h: *const ge25519_p2) {
    let mut recip: fe25519 = [0; 10];
    let mut x: fe25519 = [0; 10];
    let mut y: fe25519 = [0; 10];

    _sodium_fe25519_invert(recip.as_mut_ptr(), (*h).Z.as_ptr());
    fe25519_mul(x.as_mut_ptr(), (*h).X.as_ptr(), recip.as_ptr());
    fe25519_mul(y.as_mut_ptr(), (*h).Y.as_ptr(), recip.as_ptr());
    _sodium_fe25519_tobytes(s, y.as_ptr());
    *s.add(31) ^= (fe25519_isnegative(x.as_ptr()) << 7) as u8;
}
/* LCOV_EXCL_STOP */

/*
 r = a * A + b * B
 Only used for signatures verification.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_double_scalarmult_vartime(
    r: *mut ge25519_p2,
    a: *const u8,
    A: *const ge25519_p3,
    b: *const u8,
) {
    // Bi[8] from fe_25_5/base2.h -> BI table
    let bi: *const ge25519_precomp =
        &BI[0] as *const [fe25519; 3] as *const ge25519_precomp;
    let mut aslide: [i8; 256] = [0; 256];
    let mut bslide: [i8; 256] = [0; 256];
    let mut Ai: [ge25519_cached; 8] = [ge25519_cached::default(); 8]; /* A,3A,5A,7A,9A,11A,13A,15A */
    let mut t: ge25519_p1p1 = ge25519_p1p1::default();
    let mut u: ge25519_p3 = ge25519_p3::default();
    let mut A2: ge25519_p3 = ge25519_p3::default();
    let mut i: c_int;

    slide_vartime(aslide.as_mut_ptr(), a);
    slide_vartime(bslide.as_mut_ptr(), b);

    ge25519_p3_to_cached(&mut Ai[0], A);

    ge25519_p3_dbl(&mut t, A);
    _sodium_ge25519_p1p1_to_p3(&mut A2, &t);

    ge25519_add_cached(&mut t, &A2, &Ai[0]);
    _sodium_ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[1], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[1]);
    _sodium_ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[2], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[2]);
    _sodium_ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[3], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[3]);
    _sodium_ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[4], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[4]);
    _sodium_ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[5], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[5]);
    _sodium_ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[6], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[6]);
    _sodium_ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[7], &u);

    ge25519_p2_0(r);

    i = 255;
    while i >= 0 {
        if aslide[i as usize] != 0 || bslide[i as usize] != 0 {
            break;
        }
        i -= 1;
    }

    while i >= 0 {
        ge25519_p2_dbl(&mut t, r);

        if aslide[i as usize] > 0 {
            _sodium_ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_add_cached(&mut t, &u, &Ai[(aslide[i as usize] / 2) as usize]);
        } else if aslide[i as usize] < 0 {
            _sodium_ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_sub_cached(&mut t, &u, &Ai[((-aslide[i as usize]) / 2) as usize]);
        }

        if bslide[i as usize] > 0 {
            _sodium_ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_add_precomp(&mut t, &u, bi.add((bslide[i as usize] / 2) as usize));
        } else if bslide[i as usize] < 0 {
            _sodium_ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_sub_precomp(&mut t, &u, bi.add(((-bslide[i as usize]) / 2) as usize));
        }

        _sodium_ge25519_p1p1_to_p2(r, &t);
        i -= 1;
    }
}

/*
 h = a * p
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult(
    h: *mut ge25519_p3,
    a: *const u8,
    p: *const ge25519_p3,
) {
    let mut e: [i8; 64] = [0; 64];
    let mut carry: i8;
    let mut r: ge25519_p1p1 = ge25519_p1p1::default();
    let mut s: ge25519_p2 = ge25519_p2::default();
    let mut t2: ge25519_p1p1 = ge25519_p1p1::default();
    let mut t3: ge25519_p1p1 = ge25519_p1p1::default();
    let mut t4: ge25519_p1p1 = ge25519_p1p1::default();
    let mut t5: ge25519_p1p1 = ge25519_p1p1::default();
    let mut t6: ge25519_p1p1 = ge25519_p1p1::default();
    let mut t7: ge25519_p1p1 = ge25519_p1p1::default();
    let mut t8: ge25519_p1p1 = ge25519_p1p1::default();
    let mut p2: ge25519_p3 = ge25519_p3::default();
    let mut p3: ge25519_p3 = ge25519_p3::default();
    let mut p4: ge25519_p3 = ge25519_p3::default();
    let mut p5: ge25519_p3 = ge25519_p3::default();
    let mut p6: ge25519_p3 = ge25519_p3::default();
    let mut p7: ge25519_p3 = ge25519_p3::default();
    let mut p8: ge25519_p3 = ge25519_p3::default();
    let mut pi: [ge25519_cached; 8] = [ge25519_cached::default(); 8];
    let mut t: ge25519_cached = ge25519_cached::default();
    let mut i: c_int;

    ge25519_p3_to_cached(&mut pi[1 - 1], p); /* p */

    ge25519_p3_dbl(&mut t2, p);
    _sodium_ge25519_p1p1_to_p3(&mut p2, &t2);
    ge25519_p3_to_cached(&mut pi[2 - 1], &p2); /* 2p = 2*p */

    ge25519_add_cached(&mut t3, p, &pi[2 - 1]);
    _sodium_ge25519_p1p1_to_p3(&mut p3, &t3);
    ge25519_p3_to_cached(&mut pi[3 - 1], &p3); /* 3p = 2p+p */

    ge25519_p3_dbl(&mut t4, &p2);
    _sodium_ge25519_p1p1_to_p3(&mut p4, &t4);
    ge25519_p3_to_cached(&mut pi[4 - 1], &p4); /* 4p = 2*2p */

    ge25519_add_cached(&mut t5, p, &pi[4 - 1]);
    _sodium_ge25519_p1p1_to_p3(&mut p5, &t5);
    ge25519_p3_to_cached(&mut pi[5 - 1], &p5); /* 5p = 4p+p */

    ge25519_p3_dbl(&mut t6, &p3);
    _sodium_ge25519_p1p1_to_p3(&mut p6, &t6);
    ge25519_p3_to_cached(&mut pi[6 - 1], &p6); /* 6p = 2*3p */

    ge25519_add_cached(&mut t7, p, &pi[6 - 1]);
    _sodium_ge25519_p1p1_to_p3(&mut p7, &t7);
    ge25519_p3_to_cached(&mut pi[7 - 1], &p7); /* 7p = 6p+p */

    ge25519_p3_dbl(&mut t8, &p4);
    _sodium_ge25519_p1p1_to_p3(&mut p8, &t8);
    ge25519_p3_to_cached(&mut pi[8 - 1], &p8); /* 8p = 2*4p */

    i = 0;
    while i < 32 {
        e[(2 * i + 0) as usize] = (((*a.add(i as usize) as c_int) >> 0) & 15) as i8;
        e[(2 * i + 1) as usize] = (((*a.add(i as usize) as c_int) >> 4) & 15) as i8;
        i += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    i = 0;
    while i < 63 {
        e[i as usize] = e[i as usize].wrapping_add(carry);
        carry = ((e[i as usize] as c_int) + 8) as i8;
        carry = ((carry as c_int) >> 4) as i8;
        e[i as usize] =
            e[i as usize].wrapping_sub((carry as c_int).wrapping_mul((1i8 << 4) as c_int) as i8);
        i += 1;
    }
    e[63] = e[63].wrapping_add(carry);
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    i = 63;
    while i != 0 {
        ge25519_cmov8_cached(&mut t, pi.as_ptr(), e[i as usize]);
        ge25519_add_cached(&mut r, h, &t);

        _sodium_ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);
        _sodium_ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);
        _sodium_ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);
        _sodium_ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);

        _sodium_ge25519_p1p1_to_p3(h, &r); /* *16 */
        i -= 1;
    }
    ge25519_cmov8_cached(&mut t, pi.as_ptr(), e[i as usize]);
    ge25519_add_cached(&mut r, h, &t);

    _sodium_ge25519_p1p1_to_p3(h, &r);
}

/*
 h = a * B (with precomputation)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8) {
    let mut e: [i8; 64] = [0; 64];
    let mut carry: i8;
    let mut r: ge25519_p1p1 = ge25519_p1p1::default();
    let mut s: ge25519_p2 = ge25519_p2::default();
    let mut t: ge25519_precomp = ge25519_precomp::default();
    let mut i: c_int;

    i = 0;
    while i < 32 {
        e[(2 * i + 0) as usize] = (((*a.add(i as usize) as c_int) >> 0) & 15) as i8;
        e[(2 * i + 1) as usize] = (((*a.add(i as usize) as c_int) >> 4) & 15) as i8;
        i += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    i = 0;
    while i < 63 {
        e[i as usize] = e[i as usize].wrapping_add(carry);
        carry = ((e[i as usize] as c_int) + 8) as i8;
        carry = ((carry as c_int) >> 4) as i8;
        e[i as usize] =
            e[i as usize].wrapping_sub((carry as c_int).wrapping_mul((1i8 << 4) as c_int) as i8);
        i += 1;
    }
    e[63] = e[63].wrapping_add(carry);
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    i = 1;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i as usize]);
        ge25519_add_precomp(&mut r, h, &t);
        _sodium_ge25519_p1p1_to_p3(h, &r);
        i += 2;
    }

    ge25519_p3_dbl(&mut r, h);
    _sodium_ge25519_p1p1_to_p2(&mut s, &r);
    ge25519_p2_dbl(&mut r, &s);
    _sodium_ge25519_p1p1_to_p2(&mut s, &r);
    ge25519_p2_dbl(&mut r, &s);
    _sodium_ge25519_p1p1_to_p2(&mut s, &r);
    ge25519_p2_dbl(&mut r, &s);
    _sodium_ge25519_p1p1_to_p3(h, &r);

    i = 0;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i as usize]);
        ge25519_add_precomp(&mut r, h, &t);
        _sodium_ge25519_p1p1_to_p3(h, &r);
        i += 2;
    }
}

/* r = 2p */
unsafe fn ge25519_p3p3_dbl(r: *mut ge25519_p3, p: *const ge25519_p3) {
    let mut p1p1: ge25519_p1p1 = ge25519_p1p1::default();

    ge25519_p3_dbl(&mut p1p1, p);
    _sodium_ge25519_p1p1_to_p3(r, &p1p1);
}

/* r = -p */
unsafe fn ge25519_p3_neg(r: *mut ge25519_p3, p: *const ge25519_p3) {
    fe25519_neg((*r).X.as_mut_ptr(), (*p).X.as_ptr());
    fe25519_copy((*r).Y.as_mut_ptr(), (*p).Y.as_ptr());
    fe25519_copy((*r).Z.as_mut_ptr(), (*p).Z.as_ptr());
    fe25519_neg((*r).T.as_mut_ptr(), (*p).T.as_ptr());
}

/* r = p+q */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_add(
    r: *mut ge25519_p3,
    p: *const ge25519_p3,
    q: *const ge25519_p3,
) {
    let mut q_cached: ge25519_cached = ge25519_cached::default();
    let mut p1p1: ge25519_p1p1 = ge25519_p1p1::default();

    ge25519_p3_to_cached(&mut q_cached, q);
    ge25519_add_cached(&mut p1p1, p, &q_cached);
    _sodium_ge25519_p1p1_to_p3(r, &p1p1);
}

/* r = p-q */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_sub(
    r: *mut ge25519_p3,
    p: *const ge25519_p3,
    q: *const ge25519_p3,
) {
    let mut q_neg: ge25519_p3 = ge25519_p3::default();

    ge25519_p3_neg(&mut q_neg, q);
    _sodium_ge25519_p3_add(r, p, &q_neg);
}

/* r = r*(2^n)+q */
unsafe fn ge25519_p3_dbladd(r: *mut ge25519_p3, n: c_int, q: *const ge25519_p3) {
    let mut p2: ge25519_p2 = ge25519_p2::default();
    let mut p1p1: ge25519_p1p1 = ge25519_p1p1::default();
    let mut i: c_int;

    ge25519_p3_to_p2(&mut p2, r);
    i = 0;
    while i < n {
        ge25519_p2_dbl(&mut p1p1, &p2);
        _sodium_ge25519_p1p1_to_p2(&mut p2, &p1p1);
        i += 1;
    }
    _sodium_ge25519_p1p1_to_p3(r, &p1p1);
    _sodium_ge25519_p3_add(r, r, q);
}

/* multiply by the order of the main subgroup l = 2^252+27742317777372353535851937790883648493 */
unsafe fn ge25519_mul_l(r: *mut ge25519_p3, p: *const ge25519_p3) {
    let mut _10: ge25519_p3 = ge25519_p3::default();
    let mut _11: ge25519_p3 = ge25519_p3::default();
    let mut _100: ge25519_p3 = ge25519_p3::default();
    let mut _110: ge25519_p3 = ge25519_p3::default();
    let mut _1000: ge25519_p3 = ge25519_p3::default();
    let mut _1011: ge25519_p3 = ge25519_p3::default();
    let mut _10000: ge25519_p3 = ge25519_p3::default();
    let mut _100000: ge25519_p3 = ge25519_p3::default();
    let mut _100110: ge25519_p3 = ge25519_p3::default();
    let mut _1000000: ge25519_p3 = ge25519_p3::default();
    let mut _1010000: ge25519_p3 = ge25519_p3::default();
    let mut _1010011: ge25519_p3 = ge25519_p3::default();
    let mut _1100011: ge25519_p3 = ge25519_p3::default();
    let mut _1100111: ge25519_p3 = ge25519_p3::default();
    let mut _1101011: ge25519_p3 = ge25519_p3::default();
    let mut _10010011: ge25519_p3 = ge25519_p3::default();
    let mut _10010111: ge25519_p3 = ge25519_p3::default();
    let mut _10111101: ge25519_p3 = ge25519_p3::default();
    let mut _11010011: ge25519_p3 = ge25519_p3::default();
    let mut _11100111: ge25519_p3 = ge25519_p3::default();
    let mut _11101101: ge25519_p3 = ge25519_p3::default();
    let mut _11110101: ge25519_p3 = ge25519_p3::default();

    ge25519_p3p3_dbl(&mut _10, p);
    _sodium_ge25519_p3_add(&mut _11, p, &_10);
    _sodium_ge25519_p3_add(&mut _100, p, &_11);
    _sodium_ge25519_p3_add(&mut _110, &_10, &_100);
    _sodium_ge25519_p3_add(&mut _1000, &_10, &_110);
    _sodium_ge25519_p3_add(&mut _1011, &_11, &_1000);
    ge25519_p3p3_dbl(&mut _10000, &_1000);
    ge25519_p3p3_dbl(&mut _100000, &_10000);
    _sodium_ge25519_p3_add(&mut _100110, &_110, &_100000);
    ge25519_p3p3_dbl(&mut _1000000, &_100000);
    _sodium_ge25519_p3_add(&mut _1010000, &_10000, &_1000000);
    _sodium_ge25519_p3_add(&mut _1010011, &_11, &_1010000);
    _sodium_ge25519_p3_add(&mut _1100011, &_10000, &_1010011);
    _sodium_ge25519_p3_add(&mut _1100111, &_100, &_1100011);
    _sodium_ge25519_p3_add(&mut _1101011, &_100, &_1100111);
    _sodium_ge25519_p3_add(&mut _10010011, &_1000000, &_1010011);
    _sodium_ge25519_p3_add(&mut _10010111, &_100, &_10010011);
    _sodium_ge25519_p3_add(&mut _10111101, &_100110, &_10010111);
    _sodium_ge25519_p3_add(&mut _11010011, &_1000000, &_10010011);
    _sodium_ge25519_p3_add(&mut _11100111, &_1010000, &_10010111);
    _sodium_ge25519_p3_add(&mut _11101101, &_110, &_11100111);
    _sodium_ge25519_p3_add(&mut _11110101, &_1000, &_11101101);

    _sodium_ge25519_p3_add(r, &_1011, &_11110101);
    ge25519_p3_dbladd(r, 126, &_1010011);
    ge25519_p3_dbladd(r, 9, &_10);
    _sodium_ge25519_p3_add(r, r, &_11110101);
    ge25519_p3_dbladd(r, 7, &_1100111);
    ge25519_p3_dbladd(r, 9, &_11110101);
    ge25519_p3_dbladd(r, 11, &_10111101);
    ge25519_p3_dbladd(r, 8, &_11100111);
    ge25519_p3_dbladd(r, 9, &_1101011);
    ge25519_p3_dbladd(r, 6, &_1011);
    ge25519_p3_dbladd(r, 14, &_10010011);
    ge25519_p3_dbladd(r, 10, &_1100011);
    ge25519_p3_dbladd(r, 9, &_10010111);
    ge25519_p3_dbladd(r, 10, &_11110101);
    ge25519_p3_dbladd(r, 8, &_11010011);
    ge25519_p3_dbladd(r, 8, &_11101101);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_curve(p: *const ge25519_p3) -> c_int {
    let mut x2: fe25519 = [0; 10];
    let mut y2: fe25519 = [0; 10];
    let mut z2: fe25519 = [0; 10];
    let mut z4: fe25519 = [0; 10];
    let mut t0: fe25519 = [0; 10];
    let mut t1: fe25519 = [0; 10];

    fe25519_sq(x2.as_mut_ptr(), (*p).X.as_ptr());
    fe25519_sq(y2.as_mut_ptr(), (*p).Y.as_ptr());
    fe25519_sq(z2.as_mut_ptr(), (*p).Z.as_ptr());
    fe25519_sub(t0.as_mut_ptr(), y2.as_ptr(), x2.as_ptr());
    fe25519_mul(t0.as_mut_ptr(), t0.as_ptr(), z2.as_ptr());

    fe25519_mul(t1.as_mut_ptr(), x2.as_ptr(), y2.as_ptr());
    fe25519_mul(t1.as_mut_ptr(), t1.as_ptr(), ed25519_d.as_ptr());
    fe25519_sq(z4.as_mut_ptr(), z2.as_ptr());
    fe25519_add(t1.as_mut_ptr(), t1.as_ptr(), z4.as_ptr());
    fe25519_sub(t0.as_mut_ptr(), t0.as_ptr(), t1.as_ptr());

    fe25519_iszero(t0.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int {
    let mut pl: ge25519_p3 = ge25519_p3::default();
    let mut t: fe25519 = [0; 10];

    ge25519_mul_l(&mut pl, p);

    fe25519_sub(t.as_mut_ptr(), pl.Y.as_ptr(), pl.Z.as_ptr());

    fe25519_iszero(pl.X.as_ptr()) & fe25519_iszero(t.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int {
    let mut c: u8;
    let d: u8;
    let mut i: core::ffi::c_uint;

    c = (*s.add(31) & 0x7f) ^ 0x7f;
    i = 30;
    while i > 0 {
        c |= *s.add(i as usize) ^ 0xff;
        i -= 1;
    }
    c = ((((c as core::ffi::c_uint).wrapping_sub(1u32)) >> 8)) as u8;
    d = ((0xed_u32.wrapping_sub(1u32).wrapping_sub(*s.add(0) as core::ffi::c_uint)) >> 8) as u8;

    1 - (c & d & 1) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> c_int {
    let mut y_sqrtm1: fe25519 = [0; 10];
    let mut c: fe25519 = [0; 10];
    let mut ret: c_int = 0;

    ret |= fe25519_iszero((*p).X.as_ptr());
    ret |= fe25519_iszero((*p).Y.as_ptr());
    ret |= fe25519_iszero((*p).Z.as_ptr());
    fe25519_mul(y_sqrtm1.as_mut_ptr(), (*p).Y.as_ptr(), fe25519_sqrtm1.as_ptr());
    fe25519_sub(c.as_mut_ptr(), y_sqrtm1.as_ptr(), (*p).X.as_ptr());
    ret |= fe25519_iszero(c.as_ptr());
    fe25519_add(c.as_mut_ptr(), y_sqrtm1.as_ptr(), (*p).X.as_ptr());
    ret |= fe25519_iszero(c.as_ptr());

    ret
}

/*
 Output: s = (ab) mod l
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_mul(s: *mut u8, a: *const u8, b: *const u8) {
    let a0: i64 = 2097151 & load_3(a) as i64;
    let a1: i64 = 2097151 & (load_4(a.add(2)) >> 5) as i64;
    let a2: i64 = 2097151 & (load_3(a.add(5)) >> 2) as i64;
    let a3: i64 = 2097151 & (load_4(a.add(7)) >> 7) as i64;
    let a4: i64 = 2097151 & (load_4(a.add(10)) >> 4) as i64;
    let a5: i64 = 2097151 & (load_3(a.add(13)) >> 1) as i64;
    let a6: i64 = 2097151 & (load_4(a.add(15)) >> 6) as i64;
    let a7: i64 = 2097151 & (load_3(a.add(18)) >> 3) as i64;
    let a8: i64 = 2097151 & load_3(a.add(21)) as i64;
    let a9: i64 = 2097151 & (load_4(a.add(23)) >> 5) as i64;
    let a10: i64 = 2097151 & (load_3(a.add(26)) >> 2) as i64;
    let a11: i64 = (load_4(a.add(28)) >> 7) as i64;

    let b0: i64 = 2097151 & load_3(b) as i64;
    let b1: i64 = 2097151 & (load_4(b.add(2)) >> 5) as i64;
    let b2: i64 = 2097151 & (load_3(b.add(5)) >> 2) as i64;
    let b3: i64 = 2097151 & (load_4(b.add(7)) >> 7) as i64;
    let b4: i64 = 2097151 & (load_4(b.add(10)) >> 4) as i64;
    let b5: i64 = 2097151 & (load_3(b.add(13)) >> 1) as i64;
    let b6: i64 = 2097151 & (load_4(b.add(15)) >> 6) as i64;
    let b7: i64 = 2097151 & (load_3(b.add(18)) >> 3) as i64;
    let b8: i64 = 2097151 & load_3(b.add(21)) as i64;
    let b9: i64 = 2097151 & (load_4(b.add(23)) >> 5) as i64;
    let b10: i64 = 2097151 & (load_3(b.add(26)) >> 2) as i64;
    let b11: i64 = (load_4(b.add(28)) >> 7) as i64;

    let mut s0: i64;
    let mut s1: i64;
    let mut s2: i64;
    let mut s3: i64;
    let mut s4: i64;
    let mut s5: i64;
    let mut s6: i64;
    let mut s7: i64;
    let mut s8: i64;
    let mut s9: i64;
    let mut s10: i64;
    let mut s11: i64;
    let mut s12: i64;
    let mut s13: i64;
    let mut s14: i64;
    let mut s15: i64;
    let mut s16: i64;
    let mut s17: i64;
    let mut s18: i64;
    let mut s19: i64;
    let mut s20: i64;
    let mut s21: i64;
    let mut s22: i64;
    let mut s23: i64;

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
    let mut carry10: i64;
    let mut carry11: i64;
    let mut carry12: i64;
    let mut carry13: i64;
    let mut carry14: i64;
    let mut carry15: i64;
    let mut carry16: i64;
    let mut carry17: i64;
    let mut carry18: i64;
    let mut carry19: i64;
    let mut carry20: i64;
    let mut carry21: i64;
    let mut carry22: i64;

    s0 = a0 * b0;
    s1 = a0 * b1 + a1 * b0;
    s2 = a0 * b2 + a1 * b1 + a2 * b0;
    s3 = a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0;
    s4 = a0 * b4 + a1 * b3 + a2 * b2 + a3 * b1 + a4 * b0;
    s5 = a0 * b5 + a1 * b4 + a2 * b3 + a3 * b2 + a4 * b1 + a5 * b0;
    s6 = a0 * b6 + a1 * b5 + a2 * b4 + a3 * b3 + a4 * b2 + a5 * b1 + a6 * b0;
    s7 = a0 * b7 + a1 * b6 + a2 * b5 + a3 * b4 + a4 * b3 + a5 * b2 + a6 * b1 + a7 * b0;
    s8 = a0 * b8 + a1 * b7 + a2 * b6 + a3 * b5 + a4 * b4 + a5 * b3 + a6 * b2 + a7 * b1 + a8 * b0;
    s9 = a0 * b9 + a1 * b8 + a2 * b7 + a3 * b6 + a4 * b5 + a5 * b4 + a6 * b3 + a7 * b2 + a8 * b1
        + a9 * b0;
    s10 = a0 * b10 + a1 * b9 + a2 * b8 + a3 * b7 + a4 * b6 + a5 * b5 + a6 * b4 + a7 * b3 + a8 * b2
        + a9 * b1
        + a10 * b0;
    s11 = a0 * b11 + a1 * b10 + a2 * b9 + a3 * b8 + a4 * b7 + a5 * b6 + a6 * b5 + a7 * b4 + a8 * b3
        + a9 * b2
        + a10 * b1
        + a11 * b0;
    s12 = a1 * b11 + a2 * b10 + a3 * b9 + a4 * b8 + a5 * b7 + a6 * b6 + a7 * b5 + a8 * b4 + a9 * b3
        + a10 * b2
        + a11 * b1;
    s13 = a2 * b11 + a3 * b10 + a4 * b9 + a5 * b8 + a6 * b7 + a7 * b6 + a8 * b5 + a9 * b4 + a10 * b3
        + a11 * b2;
    s14 = a3 * b11 + a4 * b10 + a5 * b9 + a6 * b8 + a7 * b7 + a8 * b6 + a9 * b5 + a10 * b4
        + a11 * b3;
    s15 = a4 * b11 + a5 * b10 + a6 * b9 + a7 * b8 + a8 * b7 + a9 * b6 + a10 * b5 + a11 * b4;
    s16 = a5 * b11 + a6 * b10 + a7 * b9 + a8 * b8 + a9 * b7 + a10 * b6 + a11 * b5;
    s17 = a6 * b11 + a7 * b10 + a8 * b9 + a9 * b8 + a10 * b7 + a11 * b6;
    s18 = a7 * b11 + a8 * b10 + a9 * b9 + a10 * b8 + a11 * b7;
    s19 = a8 * b11 + a9 * b10 + a10 * b9 + a11 * b8;
    s20 = a9 * b11 + a10 * b10 + a11 * b9;
    s21 = a10 * b11 + a11 * b10;
    s22 = a11 * b11;
    s23 = 0;

    carry0 = (s0 + (1i64 << 20)) >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry2 = (s2 + (1i64 << 20)) >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry4 = (s4 + (1i64 << 20)) >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry12 = (s12 + (1i64 << 20)) >> 21;
    s13 += carry12;
    s12 -= carry12 * ((1u64 << 21) as i64);
    carry14 = (s14 + (1i64 << 20)) >> 21;
    s15 += carry14;
    s14 -= carry14 * ((1u64 << 21) as i64);
    carry16 = (s16 + (1i64 << 20)) >> 21;
    s17 += carry16;
    s16 -= carry16 * ((1u64 << 21) as i64);
    carry18 = (s18 + (1i64 << 20)) >> 21;
    s19 += carry18;
    s18 -= carry18 * ((1u64 << 21) as i64);
    carry20 = (s20 + (1i64 << 20)) >> 21;
    s21 += carry20;
    s20 -= carry20 * ((1u64 << 21) as i64);
    carry22 = (s22 + (1i64 << 20)) >> 21;
    s23 += carry22;
    s22 -= carry22 * ((1u64 << 21) as i64);

    carry1 = (s1 + (1i64 << 20)) >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry3 = (s3 + (1i64 << 20)) >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry5 = (s5 + (1i64 << 20)) >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);
    carry13 = (s13 + (1i64 << 20)) >> 21;
    s14 += carry13;
    s13 -= carry13 * ((1u64 << 21) as i64);
    carry15 = (s15 + (1i64 << 20)) >> 21;
    s16 += carry15;
    s15 -= carry15 * ((1u64 << 21) as i64);
    carry17 = (s17 + (1i64 << 20)) >> 21;
    s18 += carry17;
    s17 -= carry17 * ((1u64 << 21) as i64);
    carry19 = (s19 + (1i64 << 20)) >> 21;
    s20 += carry19;
    s19 -= carry19 * ((1u64 << 21) as i64);
    carry21 = (s21 + (1i64 << 20)) >> 21;
    s22 += carry21;
    s21 -= carry21 * ((1u64 << 21) as i64);

    s11 += s23 * 666643;
    s12 += s23 * 470296;
    s13 += s23 * 654183;
    s14 -= s23 * 997805;
    s15 += s23 * 136657;
    s16 -= s23 * 683901;

    s10 += s22 * 666643;
    s11 += s22 * 470296;
    s12 += s22 * 654183;
    s13 -= s22 * 997805;
    s14 += s22 * 136657;
    s15 -= s22 * 683901;

    s9 += s21 * 666643;
    s10 += s21 * 470296;
    s11 += s21 * 654183;
    s12 -= s21 * 997805;
    s13 += s21 * 136657;
    s14 -= s21 * 683901;

    s8 += s20 * 666643;
    s9 += s20 * 470296;
    s10 += s20 * 654183;
    s11 -= s20 * 997805;
    s12 += s20 * 136657;
    s13 -= s20 * 683901;

    s7 += s19 * 666643;
    s8 += s19 * 470296;
    s9 += s19 * 654183;
    s10 -= s19 * 997805;
    s11 += s19 * 136657;
    s12 -= s19 * 683901;

    s6 += s18 * 666643;
    s7 += s18 * 470296;
    s8 += s18 * 654183;
    s9 -= s18 * 997805;
    s10 += s18 * 136657;
    s11 -= s18 * 683901;

    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry12 = (s12 + (1i64 << 20)) >> 21;
    s13 += carry12;
    s12 -= carry12 * ((1u64 << 21) as i64);
    carry14 = (s14 + (1i64 << 20)) >> 21;
    s15 += carry14;
    s14 -= carry14 * ((1u64 << 21) as i64);
    carry16 = (s16 + (1i64 << 20)) >> 21;
    s17 += carry16;
    s16 -= carry16 * ((1u64 << 21) as i64);

    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);
    carry13 = (s13 + (1i64 << 20)) >> 21;
    s14 += carry13;
    s13 -= carry13 * ((1u64 << 21) as i64);
    carry15 = (s15 + (1i64 << 20)) >> 21;
    s16 += carry15;
    s15 -= carry15 * ((1u64 << 21) as i64);

    s5 += s17 * 666643;
    s6 += s17 * 470296;
    s7 += s17 * 654183;
    s8 -= s17 * 997805;
    s9 += s17 * 136657;
    s10 -= s17 * 683901;

    s4 += s16 * 666643;
    s5 += s16 * 470296;
    s6 += s16 * 654183;
    s7 -= s16 * 997805;
    s8 += s16 * 136657;
    s9 -= s16 * 683901;

    s3 += s15 * 666643;
    s4 += s15 * 470296;
    s5 += s15 * 654183;
    s6 -= s15 * 997805;
    s7 += s15 * 136657;
    s8 -= s15 * 683901;

    s2 += s14 * 666643;
    s3 += s14 * 470296;
    s4 += s14 * 654183;
    s5 -= s14 * 997805;
    s6 += s14 * 136657;
    s7 -= s14 * 683901;

    s1 += s13 * 666643;
    s2 += s13 * 470296;
    s3 += s13 * 654183;
    s4 -= s13 * 997805;
    s5 += s13 * 136657;
    s6 -= s13 * 683901;

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;
    s12 = 0;

    carry0 = (s0 + (1i64 << 20)) >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry2 = (s2 + (1i64 << 20)) >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry4 = (s4 + (1i64 << 20)) >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);

    carry1 = (s1 + (1i64 << 20)) >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry3 = (s3 + (1i64 << 20)) >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry5 = (s5 + (1i64 << 20)) >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;
    s12 = 0;

    carry0 = s0 >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry1 = s1 >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry2 = s2 >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry3 = s3 >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry4 = s4 >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry5 = s5 >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry6 = s6 >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry7 = s7 >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry8 = s8 >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry9 = s9 >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry10 = s10 >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry11 = s11 >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;

    carry0 = s0 >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry1 = s1 >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry2 = s2 >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry3 = s3 >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry4 = s4 >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry5 = s5 >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry6 = s6 >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry7 = s7 >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry8 = s8 >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry9 = s9 >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry10 = s10 >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);

    *s.add(0) = (s0 >> 0) as u8;
    *s.add(1) = (s0 >> 8) as u8;
    *s.add(2) = ((s0 >> 16) | (s1 * ((1u64 << 5) as i64))) as u8;
    *s.add(3) = (s1 >> 3) as u8;
    *s.add(4) = (s1 >> 11) as u8;
    *s.add(5) = ((s1 >> 19) | (s2 * ((1u64 << 2) as i64))) as u8;
    *s.add(6) = (s2 >> 6) as u8;
    *s.add(7) = ((s2 >> 14) | (s3 * ((1u64 << 7) as i64))) as u8;
    *s.add(8) = (s3 >> 1) as u8;
    *s.add(9) = (s3 >> 9) as u8;
    *s.add(10) = ((s3 >> 17) | (s4 * ((1u64 << 4) as i64))) as u8;
    *s.add(11) = (s4 >> 4) as u8;
    *s.add(12) = (s4 >> 12) as u8;
    *s.add(13) = ((s4 >> 20) | (s5 * ((1u64 << 1) as i64))) as u8;
    *s.add(14) = (s5 >> 7) as u8;
    *s.add(15) = ((s5 >> 15) | (s6 * ((1u64 << 6) as i64))) as u8;
    *s.add(16) = (s6 >> 2) as u8;
    *s.add(17) = (s6 >> 10) as u8;
    *s.add(18) = ((s6 >> 18) | (s7 * ((1u64 << 3) as i64))) as u8;
    *s.add(19) = (s7 >> 5) as u8;
    *s.add(20) = (s7 >> 13) as u8;
    *s.add(21) = (s8 >> 0) as u8;
    *s.add(22) = (s8 >> 8) as u8;
    *s.add(23) = ((s8 >> 16) | (s9 * ((1u64 << 5) as i64))) as u8;
    *s.add(24) = (s9 >> 3) as u8;
    *s.add(25) = (s9 >> 11) as u8;
    *s.add(26) = ((s9 >> 19) | (s10 * ((1u64 << 2) as i64))) as u8;
    *s.add(27) = (s10 >> 6) as u8;
    *s.add(28) = ((s10 >> 14) | (s11 * ((1u64 << 7) as i64))) as u8;
    *s.add(29) = (s11 >> 1) as u8;
    *s.add(30) = (s11 >> 9) as u8;
    *s.add(31) = (s11 >> 17) as u8;
}

/*
 Output: s = (ab+c) mod l
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_muladd(
    s: *mut u8,
    a: *const u8,
    b: *const u8,
    c: *const u8,
) {
    let a0: i64 = 2097151 & load_3(a) as i64;
    let a1: i64 = 2097151 & (load_4(a.add(2)) >> 5) as i64;
    let a2: i64 = 2097151 & (load_3(a.add(5)) >> 2) as i64;
    let a3: i64 = 2097151 & (load_4(a.add(7)) >> 7) as i64;
    let a4: i64 = 2097151 & (load_4(a.add(10)) >> 4) as i64;
    let a5: i64 = 2097151 & (load_3(a.add(13)) >> 1) as i64;
    let a6: i64 = 2097151 & (load_4(a.add(15)) >> 6) as i64;
    let a7: i64 = 2097151 & (load_3(a.add(18)) >> 3) as i64;
    let a8: i64 = 2097151 & load_3(a.add(21)) as i64;
    let a9: i64 = 2097151 & (load_4(a.add(23)) >> 5) as i64;
    let a10: i64 = 2097151 & (load_3(a.add(26)) >> 2) as i64;
    let a11: i64 = (load_4(a.add(28)) >> 7) as i64;

    let b0: i64 = 2097151 & load_3(b) as i64;
    let b1: i64 = 2097151 & (load_4(b.add(2)) >> 5) as i64;
    let b2: i64 = 2097151 & (load_3(b.add(5)) >> 2) as i64;
    let b3: i64 = 2097151 & (load_4(b.add(7)) >> 7) as i64;
    let b4: i64 = 2097151 & (load_4(b.add(10)) >> 4) as i64;
    let b5: i64 = 2097151 & (load_3(b.add(13)) >> 1) as i64;
    let b6: i64 = 2097151 & (load_4(b.add(15)) >> 6) as i64;
    let b7: i64 = 2097151 & (load_3(b.add(18)) >> 3) as i64;
    let b8: i64 = 2097151 & load_3(b.add(21)) as i64;
    let b9: i64 = 2097151 & (load_4(b.add(23)) >> 5) as i64;
    let b10: i64 = 2097151 & (load_3(b.add(26)) >> 2) as i64;
    let b11: i64 = (load_4(b.add(28)) >> 7) as i64;

    let c0: i64 = 2097151 & load_3(c) as i64;
    let c1: i64 = 2097151 & (load_4(c.add(2)) >> 5) as i64;
    let c2: i64 = 2097151 & (load_3(c.add(5)) >> 2) as i64;
    let c3: i64 = 2097151 & (load_4(c.add(7)) >> 7) as i64;
    let c4: i64 = 2097151 & (load_4(c.add(10)) >> 4) as i64;
    let c5: i64 = 2097151 & (load_3(c.add(13)) >> 1) as i64;
    let c6: i64 = 2097151 & (load_4(c.add(15)) >> 6) as i64;
    let c7: i64 = 2097151 & (load_3(c.add(18)) >> 3) as i64;
    let c8: i64 = 2097151 & load_3(c.add(21)) as i64;
    let c9: i64 = 2097151 & (load_4(c.add(23)) >> 5) as i64;
    let c10: i64 = 2097151 & (load_3(c.add(26)) >> 2) as i64;
    let c11: i64 = (load_4(c.add(28)) >> 7) as i64;

    let mut s0: i64;
    let mut s1: i64;
    let mut s2: i64;
    let mut s3: i64;
    let mut s4: i64;
    let mut s5: i64;
    let mut s6: i64;
    let mut s7: i64;
    let mut s8: i64;
    let mut s9: i64;
    let mut s10: i64;
    let mut s11: i64;
    let mut s12: i64;
    let mut s13: i64;
    let mut s14: i64;
    let mut s15: i64;
    let mut s16: i64;
    let mut s17: i64;
    let mut s18: i64;
    let mut s19: i64;
    let mut s20: i64;
    let mut s21: i64;
    let mut s22: i64;
    let mut s23: i64;

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
    let mut carry10: i64;
    let mut carry11: i64;
    let mut carry12: i64;
    let mut carry13: i64;
    let mut carry14: i64;
    let mut carry15: i64;
    let mut carry16: i64;
    let mut carry17: i64;
    let mut carry18: i64;
    let mut carry19: i64;
    let mut carry20: i64;
    let mut carry21: i64;
    let mut carry22: i64;

    s0 = c0 + a0 * b0;
    s1 = c1 + a0 * b1 + a1 * b0;
    s2 = c2 + a0 * b2 + a1 * b1 + a2 * b0;
    s3 = c3 + a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0;
    s4 = c4 + a0 * b4 + a1 * b3 + a2 * b2 + a3 * b1 + a4 * b0;
    s5 = c5 + a0 * b5 + a1 * b4 + a2 * b3 + a3 * b2 + a4 * b1 + a5 * b0;
    s6 = c6 + a0 * b6 + a1 * b5 + a2 * b4 + a3 * b3 + a4 * b2 + a5 * b1 + a6 * b0;
    s7 = c7 + a0 * b7 + a1 * b6 + a2 * b5 + a3 * b4 + a4 * b3 + a5 * b2 + a6 * b1 + a7 * b0;
    s8 = c8 + a0 * b8 + a1 * b7 + a2 * b6 + a3 * b5 + a4 * b4 + a5 * b3 + a6 * b2 + a7 * b1
        + a8 * b0;
    s9 = c9 + a0 * b9 + a1 * b8 + a2 * b7 + a3 * b6 + a4 * b5 + a5 * b4 + a6 * b3 + a7 * b2
        + a8 * b1
        + a9 * b0;
    s10 = c10 + a0 * b10 + a1 * b9 + a2 * b8 + a3 * b7 + a4 * b6 + a5 * b5 + a6 * b4 + a7 * b3
        + a8 * b2
        + a9 * b1
        + a10 * b0;
    s11 = c11 + a0 * b11 + a1 * b10 + a2 * b9 + a3 * b8 + a4 * b7 + a5 * b6 + a6 * b5 + a7 * b4
        + a8 * b3
        + a9 * b2
        + a10 * b1
        + a11 * b0;
    s12 = a1 * b11 + a2 * b10 + a3 * b9 + a4 * b8 + a5 * b7 + a6 * b6 + a7 * b5 + a8 * b4 + a9 * b3
        + a10 * b2
        + a11 * b1;
    s13 = a2 * b11 + a3 * b10 + a4 * b9 + a5 * b8 + a6 * b7 + a7 * b6 + a8 * b5 + a9 * b4 + a10 * b3
        + a11 * b2;
    s14 = a3 * b11 + a4 * b10 + a5 * b9 + a6 * b8 + a7 * b7 + a8 * b6 + a9 * b5 + a10 * b4
        + a11 * b3;
    s15 = a4 * b11 + a5 * b10 + a6 * b9 + a7 * b8 + a8 * b7 + a9 * b6 + a10 * b5 + a11 * b4;
    s16 = a5 * b11 + a6 * b10 + a7 * b9 + a8 * b8 + a9 * b7 + a10 * b6 + a11 * b5;
    s17 = a6 * b11 + a7 * b10 + a8 * b9 + a9 * b8 + a10 * b7 + a11 * b6;
    s18 = a7 * b11 + a8 * b10 + a9 * b9 + a10 * b8 + a11 * b7;
    s19 = a8 * b11 + a9 * b10 + a10 * b9 + a11 * b8;
    s20 = a9 * b11 + a10 * b10 + a11 * b9;
    s21 = a10 * b11 + a11 * b10;
    s22 = a11 * b11;
    s23 = 0;

    carry0 = (s0 + (1i64 << 20)) >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry2 = (s2 + (1i64 << 20)) >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry4 = (s4 + (1i64 << 20)) >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry12 = (s12 + (1i64 << 20)) >> 21;
    s13 += carry12;
    s12 -= carry12 * ((1u64 << 21) as i64);
    carry14 = (s14 + (1i64 << 20)) >> 21;
    s15 += carry14;
    s14 -= carry14 * ((1u64 << 21) as i64);
    carry16 = (s16 + (1i64 << 20)) >> 21;
    s17 += carry16;
    s16 -= carry16 * ((1u64 << 21) as i64);
    carry18 = (s18 + (1i64 << 20)) >> 21;
    s19 += carry18;
    s18 -= carry18 * ((1u64 << 21) as i64);
    carry20 = (s20 + (1i64 << 20)) >> 21;
    s21 += carry20;
    s20 -= carry20 * ((1u64 << 21) as i64);
    carry22 = (s22 + (1i64 << 20)) >> 21;
    s23 += carry22;
    s22 -= carry22 * ((1u64 << 21) as i64);

    carry1 = (s1 + (1i64 << 20)) >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry3 = (s3 + (1i64 << 20)) >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry5 = (s5 + (1i64 << 20)) >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);
    carry13 = (s13 + (1i64 << 20)) >> 21;
    s14 += carry13;
    s13 -= carry13 * ((1u64 << 21) as i64);
    carry15 = (s15 + (1i64 << 20)) >> 21;
    s16 += carry15;
    s15 -= carry15 * ((1u64 << 21) as i64);
    carry17 = (s17 + (1i64 << 20)) >> 21;
    s18 += carry17;
    s17 -= carry17 * ((1u64 << 21) as i64);
    carry19 = (s19 + (1i64 << 20)) >> 21;
    s20 += carry19;
    s19 -= carry19 * ((1u64 << 21) as i64);
    carry21 = (s21 + (1i64 << 20)) >> 21;
    s22 += carry21;
    s21 -= carry21 * ((1u64 << 21) as i64);

    s11 += s23 * 666643;
    s12 += s23 * 470296;
    s13 += s23 * 654183;
    s14 -= s23 * 997805;
    s15 += s23 * 136657;
    s16 -= s23 * 683901;

    s10 += s22 * 666643;
    s11 += s22 * 470296;
    s12 += s22 * 654183;
    s13 -= s22 * 997805;
    s14 += s22 * 136657;
    s15 -= s22 * 683901;

    s9 += s21 * 666643;
    s10 += s21 * 470296;
    s11 += s21 * 654183;
    s12 -= s21 * 997805;
    s13 += s21 * 136657;
    s14 -= s21 * 683901;

    s8 += s20 * 666643;
    s9 += s20 * 470296;
    s10 += s20 * 654183;
    s11 -= s20 * 997805;
    s12 += s20 * 136657;
    s13 -= s20 * 683901;

    s7 += s19 * 666643;
    s8 += s19 * 470296;
    s9 += s19 * 654183;
    s10 -= s19 * 997805;
    s11 += s19 * 136657;
    s12 -= s19 * 683901;

    s6 += s18 * 666643;
    s7 += s18 * 470296;
    s8 += s18 * 654183;
    s9 -= s18 * 997805;
    s10 += s18 * 136657;
    s11 -= s18 * 683901;

    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry12 = (s12 + (1i64 << 20)) >> 21;
    s13 += carry12;
    s12 -= carry12 * ((1u64 << 21) as i64);
    carry14 = (s14 + (1i64 << 20)) >> 21;
    s15 += carry14;
    s14 -= carry14 * ((1u64 << 21) as i64);
    carry16 = (s16 + (1i64 << 20)) >> 21;
    s17 += carry16;
    s16 -= carry16 * ((1u64 << 21) as i64);

    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);
    carry13 = (s13 + (1i64 << 20)) >> 21;
    s14 += carry13;
    s13 -= carry13 * ((1u64 << 21) as i64);
    carry15 = (s15 + (1i64 << 20)) >> 21;
    s16 += carry15;
    s15 -= carry15 * ((1u64 << 21) as i64);

    s5 += s17 * 666643;
    s6 += s17 * 470296;
    s7 += s17 * 654183;
    s8 -= s17 * 997805;
    s9 += s17 * 136657;
    s10 -= s17 * 683901;

    s4 += s16 * 666643;
    s5 += s16 * 470296;
    s6 += s16 * 654183;
    s7 -= s16 * 997805;
    s8 += s16 * 136657;
    s9 -= s16 * 683901;

    s3 += s15 * 666643;
    s4 += s15 * 470296;
    s5 += s15 * 654183;
    s6 -= s15 * 997805;
    s7 += s15 * 136657;
    s8 -= s15 * 683901;

    s2 += s14 * 666643;
    s3 += s14 * 470296;
    s4 += s14 * 654183;
    s5 -= s14 * 997805;
    s6 += s14 * 136657;
    s7 -= s14 * 683901;

    s1 += s13 * 666643;
    s2 += s13 * 470296;
    s3 += s13 * 654183;
    s4 -= s13 * 997805;
    s5 += s13 * 136657;
    s6 -= s13 * 683901;

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;
    s12 = 0;

    carry0 = (s0 + (1i64 << 20)) >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry2 = (s2 + (1i64 << 20)) >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry4 = (s4 + (1i64 << 20)) >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);

    carry1 = (s1 + (1i64 << 20)) >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry3 = (s3 + (1i64 << 20)) >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry5 = (s5 + (1i64 << 20)) >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;
    s12 = 0;

    carry0 = s0 >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry1 = s1 >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry2 = s2 >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry3 = s3 >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry4 = s4 >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry5 = s5 >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry6 = s6 >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry7 = s7 >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry8 = s8 >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry9 = s9 >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry10 = s10 >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry11 = s11 >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;

    carry0 = s0 >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry1 = s1 >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry2 = s2 >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry3 = s3 >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry4 = s4 >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry5 = s5 >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry6 = s6 >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry7 = s7 >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry8 = s8 >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry9 = s9 >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry10 = s10 >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);

    *s.add(0) = (s0 >> 0) as u8;
    *s.add(1) = (s0 >> 8) as u8;
    *s.add(2) = ((s0 >> 16) | (s1 * ((1u64 << 5) as i64))) as u8;
    *s.add(3) = (s1 >> 3) as u8;
    *s.add(4) = (s1 >> 11) as u8;
    *s.add(5) = ((s1 >> 19) | (s2 * ((1u64 << 2) as i64))) as u8;
    *s.add(6) = (s2 >> 6) as u8;
    *s.add(7) = ((s2 >> 14) | (s3 * ((1u64 << 7) as i64))) as u8;
    *s.add(8) = (s3 >> 1) as u8;
    *s.add(9) = (s3 >> 9) as u8;
    *s.add(10) = ((s3 >> 17) | (s4 * ((1u64 << 4) as i64))) as u8;
    *s.add(11) = (s4 >> 4) as u8;
    *s.add(12) = (s4 >> 12) as u8;
    *s.add(13) = ((s4 >> 20) | (s5 * ((1u64 << 1) as i64))) as u8;
    *s.add(14) = (s5 >> 7) as u8;
    *s.add(15) = ((s5 >> 15) | (s6 * ((1u64 << 6) as i64))) as u8;
    *s.add(16) = (s6 >> 2) as u8;
    *s.add(17) = (s6 >> 10) as u8;
    *s.add(18) = ((s6 >> 18) | (s7 * ((1u64 << 3) as i64))) as u8;
    *s.add(19) = (s7 >> 5) as u8;
    *s.add(20) = (s7 >> 13) as u8;
    *s.add(21) = (s8 >> 0) as u8;
    *s.add(22) = (s8 >> 8) as u8;
    *s.add(23) = ((s8 >> 16) | (s9 * ((1u64 << 5) as i64))) as u8;
    *s.add(24) = (s9 >> 3) as u8;
    *s.add(25) = (s9 >> 11) as u8;
    *s.add(26) = ((s9 >> 19) | (s10 * ((1u64 << 2) as i64))) as u8;
    *s.add(27) = (s10 >> 6) as u8;
    *s.add(28) = ((s10 >> 14) | (s11 * ((1u64 << 7) as i64))) as u8;
    *s.add(29) = (s11 >> 1) as u8;
    *s.add(30) = (s11 >> 9) as u8;
    *s.add(31) = (s11 >> 17) as u8;
}

#[inline]
unsafe fn sc25519_sq(s: *mut u8, a: *const u8) {
    _sodium_sc25519_mul(s, a, a);
}

#[inline]
unsafe fn sc25519_sqmul(s: *mut u8, n: c_int, a: *const u8) {
    let mut i: c_int = 0;
    while i < n {
        sc25519_sq(s, s);
        i += 1;
    }
    _sodium_sc25519_mul(s, s, a);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_invert(recip: *mut u8, s: *const u8) {
    let mut _10: [u8; 32] = [0; 32];
    let mut _100: [u8; 32] = [0; 32];
    let mut _1000: [u8; 32] = [0; 32];
    let mut _10000: [u8; 32] = [0; 32];
    let mut _100000: [u8; 32] = [0; 32];
    let mut _1000000: [u8; 32] = [0; 32];
    let mut _10010011: [u8; 32] = [0; 32];
    let mut _10010111: [u8; 32] = [0; 32];
    let mut _100110: [u8; 32] = [0; 32];
    let mut _1010: [u8; 32] = [0; 32];
    let mut _1010000: [u8; 32] = [0; 32];
    let mut _1010011: [u8; 32] = [0; 32];
    let mut _1011: [u8; 32] = [0; 32];
    let mut _10110: [u8; 32] = [0; 32];
    let mut _10111101: [u8; 32] = [0; 32];
    let mut _11: [u8; 32] = [0; 32];
    let mut _1100011: [u8; 32] = [0; 32];
    let mut _1100111: [u8; 32] = [0; 32];
    let mut _11010011: [u8; 32] = [0; 32];
    let mut _1101011: [u8; 32] = [0; 32];
    let mut _11100111: [u8; 32] = [0; 32];
    let mut _11101011: [u8; 32] = [0; 32];
    let mut _11110101: [u8; 32] = [0; 32];

    sc25519_sq(_10.as_mut_ptr(), s);
    _sodium_sc25519_mul(_11.as_mut_ptr(), s, _10.as_ptr());
    _sodium_sc25519_mul(_100.as_mut_ptr(), s, _11.as_ptr());
    sc25519_sq(_1000.as_mut_ptr(), _100.as_ptr());
    _sodium_sc25519_mul(_1010.as_mut_ptr(), _10.as_ptr(), _1000.as_ptr());
    _sodium_sc25519_mul(_1011.as_mut_ptr(), s, _1010.as_ptr());
    sc25519_sq(_10000.as_mut_ptr(), _1000.as_ptr());
    sc25519_sq(_10110.as_mut_ptr(), _1011.as_ptr());
    _sodium_sc25519_mul(_100000.as_mut_ptr(), _1010.as_ptr(), _10110.as_ptr());
    _sodium_sc25519_mul(_100110.as_mut_ptr(), _10000.as_ptr(), _10110.as_ptr());
    sc25519_sq(_1000000.as_mut_ptr(), _100000.as_ptr());
    _sodium_sc25519_mul(_1010000.as_mut_ptr(), _10000.as_ptr(), _1000000.as_ptr());
    _sodium_sc25519_mul(_1010011.as_mut_ptr(), _11.as_ptr(), _1010000.as_ptr());
    _sodium_sc25519_mul(_1100011.as_mut_ptr(), _10000.as_ptr(), _1010011.as_ptr());
    _sodium_sc25519_mul(_1100111.as_mut_ptr(), _100.as_ptr(), _1100011.as_ptr());
    _sodium_sc25519_mul(_1101011.as_mut_ptr(), _100.as_ptr(), _1100111.as_ptr());
    _sodium_sc25519_mul(_10010011.as_mut_ptr(), _1000000.as_ptr(), _1010011.as_ptr());
    _sodium_sc25519_mul(_10010111.as_mut_ptr(), _100.as_ptr(), _10010011.as_ptr());
    _sodium_sc25519_mul(_10111101.as_mut_ptr(), _100110.as_ptr(), _10010111.as_ptr());
    _sodium_sc25519_mul(_11010011.as_mut_ptr(), _10110.as_ptr(), _10111101.as_ptr());
    _sodium_sc25519_mul(_11100111.as_mut_ptr(), _1010000.as_ptr(), _10010111.as_ptr());
    _sodium_sc25519_mul(_11101011.as_mut_ptr(), _100.as_ptr(), _11100111.as_ptr());
    _sodium_sc25519_mul(_11110101.as_mut_ptr(), _1010.as_ptr(), _11101011.as_ptr());

    _sodium_sc25519_mul(recip, _1011.as_ptr(), _11110101.as_ptr());
    sc25519_sqmul(recip, 126, _1010011.as_ptr());
    sc25519_sqmul(recip, 9, _10.as_ptr());
    _sodium_sc25519_mul(recip, recip, _11110101.as_ptr());
    sc25519_sqmul(recip, 7, _1100111.as_ptr());
    sc25519_sqmul(recip, 9, _11110101.as_ptr());
    sc25519_sqmul(recip, 11, _10111101.as_ptr());
    sc25519_sqmul(recip, 8, _11100111.as_ptr());
    sc25519_sqmul(recip, 9, _1101011.as_ptr());
    sc25519_sqmul(recip, 6, _1011.as_ptr());
    sc25519_sqmul(recip, 14, _10010011.as_ptr());
    sc25519_sqmul(recip, 10, _1100011.as_ptr());
    sc25519_sqmul(recip, 9, _10010111.as_ptr());
    sc25519_sqmul(recip, 10, _11110101.as_ptr());
    sc25519_sqmul(recip, 8, _11010011.as_ptr());
    sc25519_sqmul(recip, 8, _11101011.as_ptr());
}

/*
 Output: s = s mod l  (input 64 bytes, overwrites in place)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_reduce(s: *mut u8) {
    let mut s0: i64 = 2097151 & load_3(s) as i64;
    let mut s1: i64 = 2097151 & (load_4(s.add(2)) >> 5) as i64;
    let mut s2: i64 = 2097151 & (load_3(s.add(5)) >> 2) as i64;
    let mut s3: i64 = 2097151 & (load_4(s.add(7)) >> 7) as i64;
    let mut s4: i64 = 2097151 & (load_4(s.add(10)) >> 4) as i64;
    let mut s5: i64 = 2097151 & (load_3(s.add(13)) >> 1) as i64;
    let mut s6: i64 = 2097151 & (load_4(s.add(15)) >> 6) as i64;
    let mut s7: i64 = 2097151 & (load_3(s.add(18)) >> 3) as i64;
    let mut s8: i64 = 2097151 & load_3(s.add(21)) as i64;
    let mut s9: i64 = 2097151 & (load_4(s.add(23)) >> 5) as i64;
    let mut s10: i64 = 2097151 & (load_3(s.add(26)) >> 2) as i64;
    let mut s11: i64 = 2097151 & (load_4(s.add(28)) >> 7) as i64;
    let mut s12: i64 = 2097151 & (load_4(s.add(31)) >> 4) as i64;
    let mut s13: i64 = 2097151 & (load_3(s.add(34)) >> 1) as i64;
    let mut s14: i64 = 2097151 & (load_4(s.add(36)) >> 6) as i64;
    let mut s15: i64 = 2097151 & (load_3(s.add(39)) >> 3) as i64;
    let mut s16: i64 = 2097151 & load_3(s.add(42)) as i64;
    let mut s17: i64 = 2097151 & (load_4(s.add(44)) >> 5) as i64;
    let mut s18: i64 = 2097151 & (load_3(s.add(47)) >> 2) as i64;
    let mut s19: i64 = 2097151 & (load_4(s.add(49)) >> 7) as i64;
    let mut s20: i64 = 2097151 & (load_4(s.add(52)) >> 4) as i64;
    let mut s21: i64 = 2097151 & (load_3(s.add(55)) >> 1) as i64;
    let mut s22: i64 = 2097151 & (load_4(s.add(57)) >> 6) as i64;
    let s23: i64 = (load_4(s.add(60)) >> 3) as i64;

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
    let mut carry10: i64;
    let mut carry11: i64;
    let mut carry12: i64;
    let mut carry13: i64;
    let mut carry14: i64;
    let mut carry15: i64;
    let mut carry16: i64;

    s11 += s23 * 666643;
    s12 += s23 * 470296;
    s13 += s23 * 654183;
    s14 -= s23 * 997805;
    s15 += s23 * 136657;
    s16 -= s23 * 683901;

    s10 += s22 * 666643;
    s11 += s22 * 470296;
    s12 += s22 * 654183;
    s13 -= s22 * 997805;
    s14 += s22 * 136657;
    s15 -= s22 * 683901;

    s9 += s21 * 666643;
    s10 += s21 * 470296;
    s11 += s21 * 654183;
    s12 -= s21 * 997805;
    s13 += s21 * 136657;
    s14 -= s21 * 683901;

    s8 += s20 * 666643;
    s9 += s20 * 470296;
    s10 += s20 * 654183;
    s11 -= s20 * 997805;
    s12 += s20 * 136657;
    s13 -= s20 * 683901;

    s7 += s19 * 666643;
    s8 += s19 * 470296;
    s9 += s19 * 654183;
    s10 -= s19 * 997805;
    s11 += s19 * 136657;
    s12 -= s19 * 683901;

    s6 += s18 * 666643;
    s7 += s18 * 470296;
    s8 += s18 * 654183;
    s9 -= s18 * 997805;
    s10 += s18 * 136657;
    s11 -= s18 * 683901;

    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry12 = (s12 + (1i64 << 20)) >> 21;
    s13 += carry12;
    s12 -= carry12 * ((1u64 << 21) as i64);
    carry14 = (s14 + (1i64 << 20)) >> 21;
    s15 += carry14;
    s14 -= carry14 * ((1u64 << 21) as i64);
    carry16 = (s16 + (1i64 << 20)) >> 21;
    s17 += carry16;
    s16 -= carry16 * ((1u64 << 21) as i64);

    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);
    carry13 = (s13 + (1i64 << 20)) >> 21;
    s14 += carry13;
    s13 -= carry13 * ((1u64 << 21) as i64);
    carry15 = (s15 + (1i64 << 20)) >> 21;
    s16 += carry15;
    s15 -= carry15 * ((1u64 << 21) as i64);

    s5 += s17 * 666643;
    s6 += s17 * 470296;
    s7 += s17 * 654183;
    s8 -= s17 * 997805;
    s9 += s17 * 136657;
    s10 -= s17 * 683901;

    s4 += s16 * 666643;
    s5 += s16 * 470296;
    s6 += s16 * 654183;
    s7 -= s16 * 997805;
    s8 += s16 * 136657;
    s9 -= s16 * 683901;

    s3 += s15 * 666643;
    s4 += s15 * 470296;
    s5 += s15 * 654183;
    s6 -= s15 * 997805;
    s7 += s15 * 136657;
    s8 -= s15 * 683901;

    s2 += s14 * 666643;
    s3 += s14 * 470296;
    s4 += s14 * 654183;
    s5 -= s14 * 997805;
    s6 += s14 * 136657;
    s7 -= s14 * 683901;

    s1 += s13 * 666643;
    s2 += s13 * 470296;
    s3 += s13 * 654183;
    s4 -= s13 * 997805;
    s5 += s13 * 136657;
    s6 -= s13 * 683901;

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;
    s12 = 0;

    carry0 = (s0 + (1i64 << 20)) >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry2 = (s2 + (1i64 << 20)) >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry4 = (s4 + (1i64 << 20)) >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry6 = (s6 + (1i64 << 20)) >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry8 = (s8 + (1i64 << 20)) >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry10 = (s10 + (1i64 << 20)) >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);

    carry1 = (s1 + (1i64 << 20)) >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry3 = (s3 + (1i64 << 20)) >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry5 = (s5 + (1i64 << 20)) >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry7 = (s7 + (1i64 << 20)) >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry9 = (s9 + (1i64 << 20)) >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry11 = (s11 + (1i64 << 20)) >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;
    s12 = 0;

    carry0 = s0 >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry1 = s1 >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry2 = s2 >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry3 = s3 >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry4 = s4 >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry5 = s5 >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry6 = s6 >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry7 = s7 >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry8 = s8 >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry9 = s9 >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry10 = s10 >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);
    carry11 = s11 >> 21;
    s12 += carry11;
    s11 -= carry11 * ((1u64 << 21) as i64);

    s0 += s12 * 666643;
    s1 += s12 * 470296;
    s2 += s12 * 654183;
    s3 -= s12 * 997805;
    s4 += s12 * 136657;
    s5 -= s12 * 683901;

    carry0 = s0 >> 21;
    s1 += carry0;
    s0 -= carry0 * ((1u64 << 21) as i64);
    carry1 = s1 >> 21;
    s2 += carry1;
    s1 -= carry1 * ((1u64 << 21) as i64);
    carry2 = s2 >> 21;
    s3 += carry2;
    s2 -= carry2 * ((1u64 << 21) as i64);
    carry3 = s3 >> 21;
    s4 += carry3;
    s3 -= carry3 * ((1u64 << 21) as i64);
    carry4 = s4 >> 21;
    s5 += carry4;
    s4 -= carry4 * ((1u64 << 21) as i64);
    carry5 = s5 >> 21;
    s6 += carry5;
    s5 -= carry5 * ((1u64 << 21) as i64);
    carry6 = s6 >> 21;
    s7 += carry6;
    s6 -= carry6 * ((1u64 << 21) as i64);
    carry7 = s7 >> 21;
    s8 += carry7;
    s7 -= carry7 * ((1u64 << 21) as i64);
    carry8 = s8 >> 21;
    s9 += carry8;
    s8 -= carry8 * ((1u64 << 21) as i64);
    carry9 = s9 >> 21;
    s10 += carry9;
    s9 -= carry9 * ((1u64 << 21) as i64);
    carry10 = s10 >> 21;
    s11 += carry10;
    s10 -= carry10 * ((1u64 << 21) as i64);

    *s.add(0) = (s0 >> 0) as u8;
    *s.add(1) = (s0 >> 8) as u8;
    *s.add(2) = ((s0 >> 16) | (s1 * ((1u64 << 5) as i64))) as u8;
    *s.add(3) = (s1 >> 3) as u8;
    *s.add(4) = (s1 >> 11) as u8;
    *s.add(5) = ((s1 >> 19) | (s2 * ((1u64 << 2) as i64))) as u8;
    *s.add(6) = (s2 >> 6) as u8;
    *s.add(7) = ((s2 >> 14) | (s3 * ((1u64 << 7) as i64))) as u8;
    *s.add(8) = (s3 >> 1) as u8;
    *s.add(9) = (s3 >> 9) as u8;
    *s.add(10) = ((s3 >> 17) | (s4 * ((1u64 << 4) as i64))) as u8;
    *s.add(11) = (s4 >> 4) as u8;
    *s.add(12) = (s4 >> 12) as u8;
    *s.add(13) = ((s4 >> 20) | (s5 * ((1u64 << 1) as i64))) as u8;
    *s.add(14) = (s5 >> 7) as u8;
    *s.add(15) = ((s5 >> 15) | (s6 * ((1u64 << 6) as i64))) as u8;
    *s.add(16) = (s6 >> 2) as u8;
    *s.add(17) = (s6 >> 10) as u8;
    *s.add(18) = ((s6 >> 18) | (s7 * ((1u64 << 3) as i64))) as u8;
    *s.add(19) = (s7 >> 5) as u8;
    *s.add(20) = (s7 >> 13) as u8;
    *s.add(21) = (s8 >> 0) as u8;
    *s.add(22) = (s8 >> 8) as u8;
    *s.add(23) = ((s8 >> 16) | (s9 * ((1u64 << 5) as i64))) as u8;
    *s.add(24) = (s9 >> 3) as u8;
    *s.add(25) = (s9 >> 11) as u8;
    *s.add(26) = ((s9 >> 19) | (s10 * ((1u64 << 2) as i64))) as u8;
    *s.add(27) = (s10 >> 6) as u8;
    *s.add(28) = ((s10 >> 14) | (s11 * ((1u64 << 7) as i64))) as u8;
    *s.add(29) = (s11 >> 1) as u8;
    *s.add(30) = (s11 >> 9) as u8;
    *s.add(31) = (s11 >> 17) as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int {
    /* 2^252+27742317777372353535851937790883648493 */
    static L: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];
    let mut c: u8 = 0;
    let mut n: u8 = 1;
    let mut i: core::ffi::c_uint = 32;

    loop {
        i -= 1;
        /* C: `s[i] - L[i]` promotes both operands to `int`, so the difference
           can be negative and `>> 8` sign-extends to -1. */
        c |= ((((*s.add(i as usize)) as c_int) - (L[i as usize] as c_int)) >> 8) as u8 & n;
        n &= (((((*s.add(i as usize)) ^ L[i as usize]) as c_int) - 1) >> 8) as u8;
        if i == 0 {
            break;
        }
    }

    (c != 0) as c_int
}

/* montgomery to edwards */
unsafe fn ge25519_mont_to_ed(xed: *mut i32, yed: *mut i32, x: *const i32, y: *const i32) {
    let mut one: fe25519 = [0; 10];
    let mut x_plus_one: fe25519 = [0; 10];
    let mut x_minus_one: fe25519 = [0; 10];
    let mut x_plus_one_y_inv: fe25519 = [0; 10];

    fe25519_1(one.as_mut_ptr());
    fe25519_add(x_plus_one.as_mut_ptr(), x, one.as_ptr());
    fe25519_sub(x_minus_one.as_mut_ptr(), x, one.as_ptr());

    /* xed = sqrt(-A-2)*x/y */
    fe25519_mul(x_plus_one_y_inv.as_mut_ptr(), x_plus_one.as_ptr(), y);
    _sodium_fe25519_invert(x_plus_one_y_inv.as_mut_ptr(), x_plus_one_y_inv.as_ptr()); /* 1/((x+1)*y) */
    fe25519_mul(xed, x, ed25519_sqrtam2.as_ptr());
    fe25519_mul(xed, xed, x_plus_one_y_inv.as_ptr()); /* sqrt(-A-2)*x/((x+1)*y) */
    fe25519_mul(xed, xed, x_plus_one.as_ptr());

    /* yed = (x-1)/(x+1) */
    fe25519_mul(yed, x_plus_one_y_inv.as_ptr(), y); /* 1/(x+1) */
    fe25519_mul(yed, yed, x_minus_one.as_ptr());
    fe25519_cmov(
        yed,
        one.as_ptr(),
        fe25519_iszero(x_plus_one_y_inv.as_ptr()) as core::ffi::c_uint,
    );
}

/* montgomery -- recover y = sqrt(x^3 + A*x^2 + x) */
unsafe fn ge25519_xmont_to_ymont(y: *mut i32, x: *const i32) -> c_int {
    let mut x2: fe25519 = [0; 10];
    let mut x3: fe25519 = [0; 10];

    fe25519_sq(x2.as_mut_ptr(), x);
    fe25519_mul(x3.as_mut_ptr(), x, x2.as_ptr());
    fe25519_mul32(x2.as_mut_ptr(), x2.as_ptr(), ed25519_A_32);
    fe25519_add(y, x3.as_ptr(), x);
    fe25519_add(y, y, x2.as_ptr());

    fe25519_sqrt(y, y)
}

/* multiply by the cofactor */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_clear_cofactor(p3: *mut ge25519_p3) {
    let mut p1: ge25519_p1p1 = ge25519_p1p1::default();
    let mut p2: ge25519_p2 = ge25519_p2::default();

    ge25519_p3_dbl(&mut p1, p3);
    _sodium_ge25519_p1p1_to_p2(&mut p2, &p1);
    ge25519_p2_dbl(&mut p1, &p2);
    _sodium_ge25519_p1p1_to_p2(&mut p2, &p1);
    ge25519_p2_dbl(&mut p1, &p2);
    _sodium_ge25519_p1p1_to_p3(p3, &p1);
}

unsafe fn ge25519_elligator2(x: *mut i32, y: *mut i32, r: *const i32, notsquare_p: *mut c_int) {
    let mut gx1: fe25519 = [0; 10];
    let mut rr2: fe25519 = [0; 10];
    let mut x2: fe25519 = [0; 10];
    let mut x3: fe25519 = [0; 10];
    let mut negx: fe25519 = [0; 10];
    let notsquare: c_int;

    fe25519_sq2(rr2.as_mut_ptr(), r);
    rr2[0] += 1;
    _sodium_fe25519_invert(rr2.as_mut_ptr(), rr2.as_ptr());
    fe25519_mul32(x, rr2.as_ptr(), ed25519_A_32);
    fe25519_neg(x, x); /* x=x1 */

    fe25519_sq(x2.as_mut_ptr(), x);
    fe25519_mul(x3.as_mut_ptr(), x, x2.as_ptr());
    fe25519_mul32(x2.as_mut_ptr(), x2.as_ptr(), ed25519_A_32); /* x2 = A*x1^2 */
    fe25519_add(gx1.as_mut_ptr(), x3.as_ptr(), x);
    fe25519_add(gx1.as_mut_ptr(), gx1.as_ptr(), x2.as_ptr()); /* gx1 = x1^3 + A*x1^2 + x1 */

    notsquare = fe25519_notsquare(gx1.as_ptr());

    /* gx1 not a square  => x = -x1-A */
    fe25519_neg(negx.as_mut_ptr(), x);
    fe25519_cmov(x, negx.as_ptr(), notsquare as core::ffi::c_uint);
    fe25519_0(x2.as_mut_ptr());
    fe25519_cmov(x2.as_mut_ptr(), ed25519_A.as_ptr(), notsquare as core::ffi::c_uint);
    fe25519_sub(x, x, x2.as_ptr());

    /* y = sqrt(gx1) or sqrt(gx2) with gx2 = gx1 * (A+x1) / -x1 */
    if ge25519_xmont_to_ymont(y, x) != 0 {
        abort(); /* LCOV_EXCL_LINE */
    }
    *notsquare_p = notsquare;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8) {
    let mut p3: ge25519_p3 = ge25519_p3::default();
    let mut x: fe25519 = [0; 10];
    let mut y: fe25519 = [0; 10];
    let mut negxed: fe25519 = [0; 10];
    let mut r_fe: fe25519 = [0; 10];
    let mut notsquare: c_int = 0;
    let x_sign: u8;

    core::ptr::copy_nonoverlapping(r, s, 32);
    x_sign = (((*s.add(31) >> 5) ^ optblocker_u8) >> 2) as u8;
    *s.add(31) &= 0x7f;
    _sodium_fe25519_frombytes(r_fe.as_mut_ptr(), s);

    ge25519_elligator2(x.as_mut_ptr(), y.as_mut_ptr(), r_fe.as_ptr(), &mut notsquare);

    ge25519_mont_to_ed(p3.X.as_mut_ptr(), p3.Y.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    fe25519_neg(negxed.as_mut_ptr(), p3.X.as_ptr());
    fe25519_cmov(
        p3.X.as_mut_ptr(),
        negxed.as_ptr(),
        (fe25519_isnegative(p3.X.as_ptr()) ^ (x_sign as c_int)) as core::ffi::c_uint,
    );

    fe25519_1(p3.Z.as_mut_ptr());
    fe25519_mul(p3.T.as_mut_ptr(), p3.X.as_ptr(), p3.Y.as_ptr());
    _sodium_ge25519_clear_cofactor(&mut p3);
    _sodium_ge25519_p3_tobytes(s, &p3);
}

unsafe fn fe25519_reduce64(fe_f: *mut i32, h: *const u8) {
    let mut fl: [u8; 32] = [0; 32];
    let mut gl: [u8; 32] = [0; 32];
    let mut fe_g: fe25519 = [0; 10];
    let mut i: usize;

    core::ptr::copy_nonoverlapping(h, fl.as_mut_ptr(), 32);
    core::ptr::copy_nonoverlapping(h.add(32), gl.as_mut_ptr(), 32);
    fl[31] &= 0x7f;
    gl[31] &= 0x7f;
    _sodium_fe25519_frombytes(fe_f, fl.as_ptr());
    _sodium_fe25519_frombytes(fe_g.as_mut_ptr(), gl.as_ptr());
    *fe_f.add(0) += (((((*h.add(31)) >> 5) ^ optblocker_u8) >> 2) as c_int) * 19
        + (((((*h.add(63)) >> 5) ^ optblocker_u8) >> 2) as c_int) * 722;
    i = 0;
    while i < core::mem::size_of::<fe25519>() / core::mem::size_of::<i32>() {
        *fe_f.add(i) += 38 * fe_g[i];
        i += 1;
    }
    fe25519_reduce(fe_f, fe_f);
}

/* LCOV_EXCL_START */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8) {
    let mut p3: ge25519_p3 = ge25519_p3::default();
    let mut fe_f: fe25519 = [0; 10];
    let mut x: fe25519 = [0; 10];
    let mut y: fe25519 = [0; 10];
    let mut negy: fe25519 = [0; 10];
    let mut notsquare: c_int = 0;
    let y_sign: u8;

    fe25519_reduce64(fe_f.as_mut_ptr(), h);
    ge25519_elligator2(x.as_mut_ptr(), y.as_mut_ptr(), fe_f.as_ptr(), &mut notsquare);

    y_sign = (notsquare ^ 1) as u8;
    fe25519_neg(negy.as_mut_ptr(), y.as_ptr());
    fe25519_cmov(
        y.as_mut_ptr(),
        negy.as_ptr(),
        (fe25519_isnegative(y.as_ptr()) ^ (y_sign as c_int)) as core::ffi::c_uint,
    );

    ge25519_mont_to_ed(p3.X.as_mut_ptr(), p3.Y.as_mut_ptr(), x.as_ptr(), y.as_ptr());

    fe25519_1(p3.Z.as_mut_ptr());
    fe25519_mul(p3.T.as_mut_ptr(), p3.X.as_ptr(), p3.Y.as_ptr());
    _sodium_ge25519_clear_cofactor(&mut p3);
    _sodium_ge25519_p3_tobytes(s, &p3);
}
/* LCOV_EXCL_STOP */

/* Ristretto group */

unsafe fn ristretto255_sqrt_ratio_m1(x: *mut i32, u: *const i32, v: *const i32) -> c_int {
    let mut v3: fe25519 = [0; 10];
    let mut vxx: fe25519 = [0; 10];
    let mut m_root_check: fe25519 = [0; 10];
    let mut p_root_check: fe25519 = [0; 10];
    let mut f_root_check: fe25519 = [0; 10];
    let mut x_sqrtm1: fe25519 = [0; 10];
    let has_m_root: c_int;
    let has_p_root: c_int;
    let has_f_root: c_int;

    fe25519_sq(v3.as_mut_ptr(), v);
    fe25519_mul(v3.as_mut_ptr(), v3.as_ptr(), v); /* v3 = v^3 */
    fe25519_sq(x, v3.as_ptr());
    fe25519_mul(x, x, u);
    fe25519_mul(x, x, v); /* x = uv^7 */

    fe25519_pow22523(x, x); /* x = (uv^7)^((q-5)/8) */
    fe25519_mul(x, x, v3.as_ptr());
    fe25519_mul(x, x, u); /* x = uv^3(uv^7)^((q-5)/8) */

    fe25519_sq(vxx.as_mut_ptr(), x);
    fe25519_mul(vxx.as_mut_ptr(), vxx.as_ptr(), v); /* vx^2 */
    fe25519_sub(m_root_check.as_mut_ptr(), vxx.as_ptr(), u); /* vx^2-u */
    fe25519_add(p_root_check.as_mut_ptr(), vxx.as_ptr(), u); /* vx^2+u */
    fe25519_mul(f_root_check.as_mut_ptr(), u, fe25519_sqrtm1.as_ptr()); /* u*sqrt(-1) */
    fe25519_add(f_root_check.as_mut_ptr(), vxx.as_ptr(), f_root_check.as_ptr()); /* vx^2+u*sqrt(-1) */
    has_m_root = fe25519_iszero(m_root_check.as_ptr());
    has_p_root = fe25519_iszero(p_root_check.as_ptr());
    has_f_root = fe25519_iszero(f_root_check.as_ptr());
    fe25519_mul(x_sqrtm1.as_mut_ptr(), x, fe25519_sqrtm1.as_ptr()); /* x*sqrt(-1) */

    fe25519_cmov(x, x_sqrtm1.as_ptr(), (has_p_root | has_f_root) as core::ffi::c_uint);
    fe25519_abs(x);

    has_m_root | has_p_root
}

unsafe fn ristretto255_is_canonical(s: *const u8) -> c_int {
    let mut c: u8;
    let d: u8;
    let e: u8;
    let mut i: core::ffi::c_uint;

    c = (*s.add(31) & 0x7f) ^ 0x7f;
    i = 30;
    while i > 0 {
        c |= *s.add(i as usize) ^ 0xff;
        i -= 1;
    }
    c = ((((c as core::ffi::c_uint).wrapping_sub(1u32)) >> 8)) as u8;
    d = ((0xed_u32.wrapping_sub(1u32).wrapping_sub(*s.add(0) as core::ffi::c_uint)) >> 8) as u8;
    e = (((*s.add(31) >> 5) ^ optblocker_u8) >> 2) as u8;

    1 - (((c & d) | e | *s.add(0)) & 1) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int {
    let mut inv_sqrt: fe25519 = [0; 10];
    let mut one: fe25519 = [0; 10];
    let mut s_: fe25519 = [0; 10];
    let mut ss: fe25519 = [0; 10];
    let mut u1: fe25519 = [0; 10];
    let mut u2: fe25519 = [0; 10];
    let mut u1u1: fe25519 = [0; 10];
    let mut u2u2: fe25519 = [0; 10];
    let mut v: fe25519 = [0; 10];
    let mut v_u2u2: fe25519 = [0; 10];
    let notsquare: c_int;

    if ristretto255_is_canonical(s) == 0 {
        return -1;
    }
    _sodium_fe25519_frombytes(s_.as_mut_ptr(), s);
    fe25519_sq(ss.as_mut_ptr(), s_.as_ptr()); /* ss = s^2 */

    fe25519_1(u1.as_mut_ptr());
    fe25519_sub(u1.as_mut_ptr(), u1.as_ptr(), ss.as_ptr()); /* u1 = 1-ss */
    fe25519_sq(u1u1.as_mut_ptr(), u1.as_ptr()); /* u1u1 = u1^2 */

    fe25519_1(u2.as_mut_ptr());
    fe25519_add(u2.as_mut_ptr(), u2.as_ptr(), ss.as_ptr()); /* u2 = 1+ss */
    fe25519_sq(u2u2.as_mut_ptr(), u2.as_ptr()); /* u2u2 = u2^2 */

    fe25519_mul(v.as_mut_ptr(), ed25519_d.as_ptr(), u1u1.as_ptr()); /* v = d*u1^2 */
    fe25519_neg(v.as_mut_ptr(), v.as_ptr()); /* v = -d*u1^2 */
    fe25519_sub(v.as_mut_ptr(), v.as_ptr(), u2u2.as_ptr()); /* v = -(d*u1^2)-u2^2 */

    fe25519_mul(v_u2u2.as_mut_ptr(), v.as_ptr(), u2u2.as_ptr()); /* v_u2u2 = v*u2^2 */

    fe25519_1(one.as_mut_ptr());
    notsquare = ristretto255_sqrt_ratio_m1(inv_sqrt.as_mut_ptr(), one.as_ptr(), v_u2u2.as_ptr());
    fe25519_mul((*h).X.as_mut_ptr(), inv_sqrt.as_ptr(), u2.as_ptr());
    fe25519_mul((*h).Y.as_mut_ptr(), inv_sqrt.as_ptr(), (*h).X.as_ptr());
    fe25519_mul((*h).Y.as_mut_ptr(), (*h).Y.as_ptr(), v.as_ptr());

    fe25519_mul((*h).X.as_mut_ptr(), (*h).X.as_ptr(), s_.as_ptr());
    fe25519_add((*h).X.as_mut_ptr(), (*h).X.as_ptr(), (*h).X.as_ptr());
    fe25519_abs((*h).X.as_mut_ptr());
    fe25519_mul((*h).Y.as_mut_ptr(), u1.as_ptr(), (*h).Y.as_ptr());
    fe25519_1((*h).Z.as_mut_ptr());
    fe25519_mul((*h).T.as_mut_ptr(), (*h).X.as_ptr(), (*h).Y.as_ptr());

    -((1 - notsquare) | fe25519_isnegative((*h).T.as_ptr()) | fe25519_iszero((*h).Y.as_ptr()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const ge25519_p3) {
    let mut den1: fe25519 = [0; 10];
    let mut den2: fe25519 = [0; 10];
    let mut den_inv: fe25519 = [0; 10];
    let mut eden: fe25519 = [0; 10];
    let mut inv_sqrt: fe25519 = [0; 10];
    let mut ix: fe25519 = [0; 10];
    let mut iy: fe25519 = [0; 10];
    let mut one: fe25519 = [0; 10];
    let mut s_: fe25519 = [0; 10];
    let mut t_z_inv: fe25519 = [0; 10];
    let mut u1: fe25519 = [0; 10];
    let mut u2: fe25519 = [0; 10];
    let mut u1_u2u2: fe25519 = [0; 10];
    let mut x_: fe25519 = [0; 10];
    let mut y_: fe25519 = [0; 10];
    let mut x_z_inv: fe25519 = [0; 10];
    let mut z_inv: fe25519 = [0; 10];
    let mut zmy: fe25519 = [0; 10];
    let rotate: c_int;

    fe25519_add(u1.as_mut_ptr(), (*h).Z.as_ptr(), (*h).Y.as_ptr()); /* u1 = Z+Y */
    fe25519_sub(zmy.as_mut_ptr(), (*h).Z.as_ptr(), (*h).Y.as_ptr()); /* zmy = Z-Y */
    fe25519_mul(u1.as_mut_ptr(), u1.as_ptr(), zmy.as_ptr()); /* u1 = (Z+Y)*(Z-Y) */
    fe25519_mul(u2.as_mut_ptr(), (*h).X.as_ptr(), (*h).Y.as_ptr()); /* u2 = X*Y */

    fe25519_sq(u1_u2u2.as_mut_ptr(), u2.as_ptr()); /* u1_u2u2 = u2^2 */
    fe25519_mul(u1_u2u2.as_mut_ptr(), u1.as_ptr(), u1_u2u2.as_ptr()); /* u1_u2u2 = u1*u2^2 */

    fe25519_1(one.as_mut_ptr());
    let _ = ristretto255_sqrt_ratio_m1(inv_sqrt.as_mut_ptr(), one.as_ptr(), u1_u2u2.as_ptr());
    fe25519_mul(den1.as_mut_ptr(), inv_sqrt.as_ptr(), u1.as_ptr()); /* den1 = inv_sqrt*u1 */
    fe25519_mul(den2.as_mut_ptr(), inv_sqrt.as_ptr(), u2.as_ptr()); /* den2 = inv_sqrt*u2 */
    fe25519_mul(z_inv.as_mut_ptr(), den1.as_ptr(), den2.as_ptr()); /* z_inv = den1*den2 */
    fe25519_mul(z_inv.as_mut_ptr(), z_inv.as_ptr(), (*h).T.as_ptr()); /* z_inv = den1*den2*T */

    fe25519_mul(ix.as_mut_ptr(), (*h).X.as_ptr(), fe25519_sqrtm1.as_ptr()); /* ix = X*sqrt(-1) */
    fe25519_mul(iy.as_mut_ptr(), (*h).Y.as_ptr(), fe25519_sqrtm1.as_ptr()); /* iy = Y*sqrt(-1) */
    fe25519_mul(eden.as_mut_ptr(), den1.as_ptr(), ed25519_invsqrtamd.as_ptr()); /* eden = den1/sqrt(a-d) */

    fe25519_mul(t_z_inv.as_mut_ptr(), (*h).T.as_ptr(), z_inv.as_ptr()); /* t_z_inv = T*z_inv */
    rotate = fe25519_isnegative(t_z_inv.as_ptr());

    fe25519_copy(x_.as_mut_ptr(), (*h).X.as_ptr());
    fe25519_copy(y_.as_mut_ptr(), (*h).Y.as_ptr());
    fe25519_copy(den_inv.as_mut_ptr(), den2.as_ptr());

    fe25519_cmov(x_.as_mut_ptr(), iy.as_ptr(), rotate as core::ffi::c_uint);
    fe25519_cmov(y_.as_mut_ptr(), ix.as_ptr(), rotate as core::ffi::c_uint);
    fe25519_cmov(den_inv.as_mut_ptr(), eden.as_ptr(), rotate as core::ffi::c_uint);

    fe25519_mul(x_z_inv.as_mut_ptr(), x_.as_ptr(), z_inv.as_ptr());
    fe25519_cneg(y_.as_mut_ptr(), fe25519_isnegative(x_z_inv.as_ptr()));

    fe25519_sub(s_.as_mut_ptr(), (*h).Z.as_ptr(), y_.as_ptr());
    fe25519_mul(s_.as_mut_ptr(), den_inv.as_ptr(), s_.as_ptr());
    fe25519_abs(s_.as_mut_ptr());
    _sodium_fe25519_tobytes(s, s_.as_ptr());
}

unsafe fn ristretto255_elligator(p: *mut ge25519_p3, t: *const i32) {
    let mut c: fe25519 = [0; 10];
    let mut n: fe25519 = [0; 10];
    let mut one: fe25519 = [0; 10];
    let mut r: fe25519 = [0; 10];
    let mut rpd: fe25519 = [0; 10];
    let mut s: fe25519 = [0; 10];
    let mut s_prime: fe25519 = [0; 10];
    let mut ss: fe25519 = [0; 10];
    let mut u: fe25519 = [0; 10];
    let mut v: fe25519 = [0; 10];
    let mut w0: fe25519 = [0; 10];
    let mut w1: fe25519 = [0; 10];
    let mut w2: fe25519 = [0; 10];
    let mut w3: fe25519 = [0; 10];
    let wasnt_square: c_int;

    fe25519_1(one.as_mut_ptr());
    fe25519_sq(r.as_mut_ptr(), t); /* r = t^2 */
    fe25519_mul(r.as_mut_ptr(), fe25519_sqrtm1.as_ptr(), r.as_ptr()); /* r = sqrt(-1)*t^2 */
    fe25519_add(u.as_mut_ptr(), r.as_ptr(), one.as_ptr()); /* u = r+1 */
    fe25519_mul(u.as_mut_ptr(), u.as_ptr(), ed25519_onemsqd.as_ptr()); /* u = (r+1)*(1-d^2) */
    fe25519_1(c.as_mut_ptr());
    fe25519_neg(c.as_mut_ptr(), c.as_ptr()); /* c = -1 */
    fe25519_add(rpd.as_mut_ptr(), r.as_ptr(), ed25519_d.as_ptr()); /* rpd = r+d */
    fe25519_mul(v.as_mut_ptr(), r.as_ptr(), ed25519_d.as_ptr()); /* v = r*d */
    fe25519_sub(v.as_mut_ptr(), c.as_ptr(), v.as_ptr()); /* v = c-r*d */
    fe25519_mul(v.as_mut_ptr(), v.as_ptr(), rpd.as_ptr()); /* v = (c-r*d)*(r+d) */

    wasnt_square = 1 - ristretto255_sqrt_ratio_m1(s.as_mut_ptr(), u.as_ptr(), v.as_ptr());
    fe25519_mul(s_prime.as_mut_ptr(), s.as_ptr(), t);
    fe25519_abs(s_prime.as_mut_ptr());
    fe25519_neg(s_prime.as_mut_ptr(), s_prime.as_ptr()); /* s_prime = -|s*t| */
    fe25519_cmov(s.as_mut_ptr(), s_prime.as_ptr(), wasnt_square as core::ffi::c_uint);
    fe25519_cmov(c.as_mut_ptr(), r.as_ptr(), wasnt_square as core::ffi::c_uint);

    fe25519_sub(n.as_mut_ptr(), r.as_ptr(), one.as_ptr()); /* n = r-1 */
    fe25519_mul(n.as_mut_ptr(), n.as_ptr(), c.as_ptr()); /* n = c*(r-1) */
    fe25519_mul(n.as_mut_ptr(), n.as_ptr(), ed25519_sqdmone.as_ptr()); /* n = c*(r-1)*(d-1)^2 */
    fe25519_sub(n.as_mut_ptr(), n.as_ptr(), v.as_ptr()); /* n =  c*(r-1)*(d-1)^2-v */

    fe25519_add(w0.as_mut_ptr(), s.as_ptr(), s.as_ptr()); /* w0 = 2s */
    fe25519_mul(w0.as_mut_ptr(), w0.as_ptr(), v.as_ptr()); /* w0 = 2s*v */
    fe25519_mul(w1.as_mut_ptr(), n.as_ptr(), ed25519_sqrtadm1.as_ptr()); /* w1 = n*sqrt(ad-1) */
    fe25519_sq(ss.as_mut_ptr(), s.as_ptr()); /* ss = s^2 */
    fe25519_sub(w2.as_mut_ptr(), one.as_ptr(), ss.as_ptr()); /* w2 = 1-s^2 */
    fe25519_add(w3.as_mut_ptr(), one.as_ptr(), ss.as_ptr()); /* w3 = 1+s^2 */

    fe25519_mul((*p).X.as_mut_ptr(), w0.as_ptr(), w3.as_ptr());
    fe25519_mul((*p).Y.as_mut_ptr(), w2.as_ptr(), w1.as_ptr());
    fe25519_mul((*p).Z.as_mut_ptr(), w1.as_ptr(), w3.as_ptr());
    fe25519_mul((*p).T.as_mut_ptr(), w0.as_ptr(), w2.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_from_hash(s: *mut u8, h: *const u8) {
    let mut r0: fe25519 = [0; 10];
    let mut r1: fe25519 = [0; 10];
    let mut p0: ge25519_p3 = ge25519_p3::default();
    let mut p1: ge25519_p3 = ge25519_p3::default();
    let mut p: ge25519_p3 = ge25519_p3::default();

    _sodium_fe25519_frombytes(r0.as_mut_ptr(), h);
    _sodium_fe25519_frombytes(r1.as_mut_ptr(), h.add(32));
    ristretto255_elligator(&mut p0, r0.as_ptr());
    ristretto255_elligator(&mut p1, r1.as_ptr());
    _sodium_ge25519_p3_add(&mut p, &p0, &p1);
    _sodium_ristretto255_p3_tobytes(s, &p);
}
