//! Hash-to-curve / Elligator 2 part of
//! `crypto_core/ed25519/ref10/ed25519_ref10.c` (lines ~2595..2762).
//!
//! Contains `ge25519_mont_to_ed`, `ge25519_xmont_to_ymont`,
//! `ge25519_clear_cofactor`, `ge25519_elligator2`, `ge25519_from_uniform`,
//! `fe25519_reduce64` and `ge25519_from_hash`.

use core::ffi::c_int;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::common::memcpy;
use crate::ed25519_ref10::fe;
use crate::ed25519_ref10::ge;
use crate::ed25519_ref10::tables::{ED25519_A, ED25519_A_32, ED25519_SQRTAM2};
use crate::ed25519_ref10::{Fe25519, Ge25519P1p1, Ge25519P2, Ge25519P3};

/* ------------------------------------------------------------------ */
/*  `static volatile unsigned char optblocker_u8;` (always zero)       */
/* ------------------------------------------------------------------ */

static OPTBLOCKER_U8: AtomicU8 = AtomicU8::new(0);

#[inline(always)]
fn optblocker_u8() -> u8 {
    OPTBLOCKER_U8.load(Ordering::Relaxed)
}

/* ------------------------------------------------------------------ */
/*  Thin adapters over `fe.rs` / `ge.rs`.                              */
/*                                                                     */
/*  The C code freely aliases the output operand with an input operand */
/*  (`fe25519_mul(x, x, y)`); the value-returning wrappers below keep   */
/*  the exact same sequence of field operations while satisfying Rust's */
/*  borrow rules.                                                      */
/* ------------------------------------------------------------------ */

const FE_ZERO: Fe25519 = [0i32; 10];

#[inline(always)]
fn fe_0() -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_0(&mut h);
    h
}

#[inline(always)]
fn fe_1() -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_1(&mut h);
    h
}

#[inline(always)]
fn fadd(f: &Fe25519, g: &Fe25519) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_add(&mut h, f, g);
    h
}

#[inline(always)]
fn fsub(f: &Fe25519, g: &Fe25519) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_sub(&mut h, f, g);
    h
}

#[inline(always)]
fn fneg(f: &Fe25519) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_neg(&mut h, f);
    h
}

#[inline(always)]
fn fmul(f: &Fe25519, g: &Fe25519) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_mul(&mut h, f, g);
    h
}

#[inline(always)]
fn fsq(f: &Fe25519) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_sq(&mut h, f);
    h
}

#[inline(always)]
fn fsq2(f: &Fe25519) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_sq2(&mut h, f);
    h
}

#[inline(always)]
fn fmul32(f: &Fe25519, n: u32) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_mul32(&mut h, f, n);
    h
}

#[inline(always)]
fn fcmov(f: &mut Fe25519, g: &Fe25519, b: u32) {
    fe::fe25519_cmov(f, g, b);
}

#[inline(always)]
fn fisnegative(f: &Fe25519) -> c_int {
    fe::fe25519_isnegative(f)
}

#[inline(always)]
fn fiszero(f: &Fe25519) -> c_int {
    fe::fe25519_iszero(f)
}

#[inline(always)]
fn finvert(z: &Fe25519) -> Fe25519 {
    let mut out = FE_ZERO;
    fe::fe25519_invert(&mut out, z);
    out
}

#[inline(always)]
fn freduce(f: &Fe25519) -> Fe25519 {
    let mut h = FE_ZERO;
    fe::fe25519_reduce(&mut h, f);
    h
}

/// `fe25519_sqrt(x, x2)` — returns `(x, retval)`.
#[inline(always)]
fn fsqrt(x2: &Fe25519) -> (Fe25519, c_int) {
    let mut x = FE_ZERO;
    let r = fe::fe25519_sqrt(&mut x, x2);
    (x, r)
}

#[inline(always)]
fn fnotsquare(x: &Fe25519) -> c_int {
    fe::fe25519_notsquare(x)
}

#[inline(always)]
unsafe fn ffrombytes(s: *const u8) -> Fe25519 {
    fe::fe_frombytes_ptr(s)
}

#[inline(always)]
fn p3_dbl(r: &mut Ge25519P1p1, p: &Ge25519P3) {
    ge::ge25519_p3_dbl(r, p);
}

#[inline(always)]
fn p2_dbl(r: &mut Ge25519P1p1, p: &Ge25519P2) {
    ge::ge25519_p2_dbl(r, p);
}

#[inline(always)]
fn p1p1_to_p2(r: &mut Ge25519P2, p: &Ge25519P1p1) {
    ge::ge25519_p1p1_to_p2(r, p);
}

#[inline(always)]
fn p1p1_to_p3(r: &mut Ge25519P3, p: &Ge25519P1p1) {
    ge::ge25519_p1p1_to_p3(r, p);
}

#[inline(always)]
unsafe fn p3_tobytes(s: *mut u8, h: &Ge25519P3) {
    ge::ge25519_p3_tobytes(&mut *(s as *mut [u8; 32]), h);
}

/* ------------------------------------------------------------------ */
/*  montgomery to edwards                                              */
/* ------------------------------------------------------------------ */

fn ge25519_mont_to_ed(xed: &mut Fe25519, yed: &mut Fe25519, x: &Fe25519, y: &Fe25519) {
    let one: Fe25519;
    let x_plus_one: Fe25519;
    let x_minus_one: Fe25519;
    let mut x_plus_one_y_inv: Fe25519;

    one = fe_1();
    x_plus_one = fadd(x, &one);
    x_minus_one = fsub(x, &one);

    /* xed = sqrt(-A-2)*x/y */
    x_plus_one_y_inv = fmul(&x_plus_one, y);
    x_plus_one_y_inv = finvert(&x_plus_one_y_inv); /* 1/((x+1)*y) */
    *xed = fmul(x, &ED25519_SQRTAM2);
    *xed = fmul(xed, &x_plus_one_y_inv); /* sqrt(-A-2)*x/((x+1)*y) */
    *xed = fmul(xed, &x_plus_one);

    /* yed = (x-1)/(x+1) */
    *yed = fmul(&x_plus_one_y_inv, y); /* 1/(x+1) */
    *yed = fmul(yed, &x_minus_one);
    fcmov(yed, &one, fiszero(&x_plus_one_y_inv) as u32);
}

/* montgomery -- recover y = sqrt(x^3 + A*x^2 + x) */
fn ge25519_xmont_to_ymont(y: &mut Fe25519, x: &Fe25519) -> c_int {
    let mut x2: Fe25519;
    let x3: Fe25519;

    x2 = fsq(x);
    x3 = fmul(x, &x2);
    x2 = fmul32(&x2, ED25519_A_32 as u32);
    *y = fadd(&x3, x);
    *y = fadd(y, &x2);

    let (r, ret) = fsqrt(y);
    *y = r;

    ret
}

/* ------------------------------------------------------------------ */
/*  multiply by the cofactor                                           */
/* ------------------------------------------------------------------ */

pub fn ge25519_clear_cofactor(p3: &mut Ge25519P3) {
    let mut p1 = Ge25519P1p1::zeroed();
    let mut p2 = Ge25519P2::zeroed();

    p3_dbl(&mut p1, p3);
    p1p1_to_p2(&mut p2, &p1);
    p2_dbl(&mut p1, &p2);
    p1p1_to_p2(&mut p2, &p1);
    p2_dbl(&mut p1, &p2);
    p1p1_to_p3(p3, &p1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_clear_cofactor(p3: *mut Ge25519P3) {
    ge25519_clear_cofactor(&mut *p3);
}

/* ------------------------------------------------------------------ */
/*  Elligator 2                                                        */
/* ------------------------------------------------------------------ */

fn ge25519_elligator2(x: &mut Fe25519, y: &mut Fe25519, r: &Fe25519, notsquare_p: &mut c_int) {
    let mut gx1: Fe25519;
    let mut rr2: Fe25519;
    let mut x2: Fe25519;
    let x3: Fe25519;
    let negx: Fe25519;
    let notsquare: c_int;

    rr2 = fsq2(r);
    rr2[0] = rr2[0].wrapping_add(1);
    rr2 = finvert(&rr2);
    *x = fmul32(&rr2, ED25519_A_32 as u32);
    *x = fneg(x); /* x=x1 */

    x2 = fsq(x);
    x3 = fmul(x, &x2);
    x2 = fmul32(&x2, ED25519_A_32 as u32); /* x2 = A*x1^2 */
    gx1 = fadd(&x3, x);
    gx1 = fadd(&gx1, &x2); /* gx1 = x1^3 + A*x1^2 + x1 */

    notsquare = fnotsquare(&gx1);

    /* gx1 not a square  => x = -x1-A */
    negx = fneg(x);
    fcmov(x, &negx, notsquare as u32);
    x2 = fe_0();
    fcmov(&mut x2, &ED25519_A, notsquare as u32);
    *x = fsub(x, &x2);

    /* y = sqrt(gx1) or sqrt(gx2) with gx2 = gx1 * (A+x1) / -x1 */
    /* but it is about as fast to just recompute from the curve equation. */
    if ge25519_xmont_to_ymont(y, x) != 0 {
        std::process::abort(); /* LCOV_EXCL_LINE */
    }
    *notsquare_p = notsquare;
}

/* ------------------------------------------------------------------ */
/*  ge25519_from_uniform                                               */
/* ------------------------------------------------------------------ */

pub unsafe fn ge25519_from_uniform(s: *mut u8, r: *const u8) {
    let mut p3 = Ge25519P3::zeroed();
    let mut x = FE_ZERO;
    let mut y = FE_ZERO;
    let negxed: Fe25519;
    let r_fe: Fe25519;
    let mut notsquare: c_int = 0;
    let x_sign: u8;

    memcpy(s, r, 32);
    x_sign = ((((*s.add(31) >> 5) as c_int) ^ (optblocker_u8() as c_int)) >> 2) as u8;
    *s.add(31) &= 0x7f;
    r_fe = ffrombytes(s);

    ge25519_elligator2(&mut x, &mut y, &r_fe, &mut notsquare);

    ge25519_mont_to_ed(&mut p3.X, &mut p3.Y, &x, &y);
    negxed = fneg(&p3.X);
    let b = (fisnegative(&p3.X) ^ (x_sign as c_int)) as u32;
    fcmov(&mut p3.X, &negxed, b);

    p3.Z = fe_1();
    p3.T = fmul(&p3.X, &p3.Y);
    ge25519_clear_cofactor(&mut p3);
    p3_tobytes(s, &p3);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8) {
    ge25519_from_uniform(s, r);
}

/* ------------------------------------------------------------------ */
/*  fe25519_reduce64                                                   */
/* ------------------------------------------------------------------ */

unsafe fn fe25519_reduce64(fe_f: &mut Fe25519, h: *const u8) {
    let mut fl = [0u8; 32];
    let mut gl = [0u8; 32];
    let fe_g: Fe25519;
    let mut i: usize;

    memcpy(fl.as_mut_ptr(), h, 32);
    memcpy(gl.as_mut_ptr(), h.add(32), 32);
    fl[31] &= 0x7f;
    gl[31] &= 0x7f;
    *fe_f = ffrombytes(fl.as_ptr());
    fe_g = ffrombytes(gl.as_ptr());
    fe_f[0] = fe_f[0].wrapping_add(
        ((((*h.add(31) >> 5) as c_int) ^ (optblocker_u8() as c_int)) >> 2) * 19
            + ((((*h.add(63) >> 5) as c_int) ^ (optblocker_u8() as c_int)) >> 2) * 722,
    );
    i = 0;
    while i < 10 {
        fe_f[i] = fe_f[i].wrapping_add(38i32.wrapping_mul(fe_g[i]));
        i += 1;
    }
    let t = *fe_f;
    *fe_f = freduce(&t);
}

/* ------------------------------------------------------------------ */
/*  ge25519_from_hash                                                  */
/* ------------------------------------------------------------------ */

/* LCOV_EXCL_START */
pub unsafe fn ge25519_from_hash(s: *mut u8, h: *const u8) {
    let mut p3 = Ge25519P3::zeroed();
    let mut fe_f = FE_ZERO;
    let mut x = FE_ZERO;
    let mut y = FE_ZERO;
    let negy: Fe25519;
    let mut notsquare: c_int = 0;
    let y_sign: u8;

    fe25519_reduce64(&mut fe_f, h);
    ge25519_elligator2(&mut x, &mut y, &fe_f, &mut notsquare);

    y_sign = (notsquare ^ 1) as u8;
    negy = fneg(&y);
    let b = (fisnegative(&y) ^ (y_sign as c_int)) as u32;
    fcmov(&mut y, &negy, b);

    ge25519_mont_to_ed(&mut p3.X, &mut p3.Y, &x, &y);

    p3.Z = fe_1();
    p3.T = fmul(&p3.X, &p3.Y);
    ge25519_clear_cofactor(&mut p3);
    p3_tobytes(s, &p3);
}
/* LCOV_EXCL_STOP */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8) {
    ge25519_from_hash(s, h);
}
