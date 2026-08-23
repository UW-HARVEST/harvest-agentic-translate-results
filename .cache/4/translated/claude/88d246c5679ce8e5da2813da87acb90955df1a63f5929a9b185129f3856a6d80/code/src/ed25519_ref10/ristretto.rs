//! Ristretto group part of
//! `crypto_core/ed25519/ref10/ed25519_ref10.c` (lines ~2764..2992).
//!
//! Contains `ristretto255_sqrt_ratio_m1`, `ristretto255_is_canonical`,
//! `ristretto255_frombytes`, `ristretto255_p3_tobytes`,
//! `ristretto255_elligator` and `ristretto255_from_hash`.

use core::ffi::c_int;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::ed25519_ref10::fe;
use crate::ed25519_ref10::ge;
use crate::ed25519_ref10::tables::{
    ED25519_D, ED25519_INVSQRTAMD, ED25519_ONEMSQD, ED25519_SQDMONE, ED25519_SQRTADM1,
    FE25519_SQRTM1,
};
use crate::ed25519_ref10::{Fe25519, Ge25519P3};

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
fn fcmov(f: &mut Fe25519, g: &Fe25519, b: u32) {
    fe::fe25519_cmov(f, g, b);
}

#[inline(always)]
fn fcneg(h: &mut Fe25519, b: u32) {
    fe::fe25519_cneg(h, b);
}

#[inline(always)]
fn fabs(h: &mut Fe25519) {
    fe::fe25519_abs(h);
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
fn fpow22523(z: &Fe25519) -> Fe25519 {
    let mut out = FE_ZERO;
    fe::fe25519_pow22523(&mut out, z);
    out
}

#[inline(always)]
unsafe fn ffrombytes(s: *const u8) -> Fe25519 {
    fe::fe_frombytes_ptr(s)
}

#[inline(always)]
unsafe fn ftobytes(s: *mut u8, h: &Fe25519) {
    fe::fe25519_tobytes(&mut *(s as *mut [u8; 32]), h);
}

#[inline(always)]
fn p3_add(r: &mut Ge25519P3, p: &Ge25519P3, q: &Ge25519P3) {
    ge::ge25519_p3_add(r, p, q);
}

/* ------------------------------------------------------------------ */
/*  Ristretto group                                                    */
/* ------------------------------------------------------------------ */

fn ristretto255_sqrt_ratio_m1(x: &mut Fe25519, u: &Fe25519, v: &Fe25519) -> c_int {
    let mut v3: Fe25519;
    let mut vxx: Fe25519;
    let m_root_check: Fe25519;
    let p_root_check: Fe25519;
    let mut f_root_check: Fe25519;
    let x_sqrtm1: Fe25519;
    let has_m_root: c_int;
    let has_p_root: c_int;
    let has_f_root: c_int;

    v3 = fsq(v);
    v3 = fmul(&v3, v); /* v3 = v^3 */
    *x = fsq(&v3);
    *x = fmul(x, u);
    *x = fmul(x, v); /* x = uv^7 */

    *x = fpow22523(x); /* x = (uv^7)^((q-5)/8) */
    *x = fmul(x, &v3);
    *x = fmul(x, u); /* x = uv^3(uv^7)^((q-5)/8) */

    vxx = fsq(x);
    vxx = fmul(&vxx, v); /* vx^2 */
    m_root_check = fsub(&vxx, u); /* vx^2-u */
    p_root_check = fadd(&vxx, u); /* vx^2+u */
    f_root_check = fmul(u, &FE25519_SQRTM1); /* u*sqrt(-1) */
    f_root_check = fadd(&vxx, &f_root_check); /* vx^2+u*sqrt(-1) */
    has_m_root = fiszero(&m_root_check);
    has_p_root = fiszero(&p_root_check);
    has_f_root = fiszero(&f_root_check);
    x_sqrtm1 = fmul(x, &FE25519_SQRTM1); /* x*sqrt(-1) */

    fcmov(x, &x_sqrtm1, (has_p_root | has_f_root) as u32);
    fabs(x);

    has_m_root | has_p_root
}

/// `static int ristretto255_is_canonical(const unsigned char *s)`
///
/// Kept `pub` so sibling modules of the same C translation unit can reach it.
pub fn ristretto255_is_canonical(s: *const u8) -> i32 {
    let mut c: u8;
    let d: u8;
    let e: u8;
    let mut i: u32;

    unsafe {
        c = (*s.add(31) & 0x7f) ^ 0x7f;
        i = 30;
        while i > 0 {
            c |= *s.add(i as usize) ^ 0xff;
            i -= 1;
        }
        c = (((c as u32).wrapping_sub(1u32)) >> 8) as u8;
        d = ((0xedu32.wrapping_sub(1u32).wrapping_sub(*s.add(0) as u32)) >> 8) as u8;
        e = ((((*s.add(31) >> 5) as c_int) ^ (optblocker_u8() as c_int)) >> 2) as u8;

        1 - ((((c & d) | e | *s.add(0)) & 1) as i32)
    }
}

pub unsafe fn ristretto255_frombytes(h: &mut Ge25519P3, s: *const u8) -> c_int {
    let mut inv_sqrt: Fe25519 = FE_ZERO;
    let one: Fe25519;
    let s_: Fe25519;
    let ss: Fe25519;
    let mut u1: Fe25519;
    let mut u2: Fe25519;
    let u1u1: Fe25519;
    let u2u2: Fe25519;
    let mut v: Fe25519;
    let v_u2u2: Fe25519;
    let notsquare: c_int;

    if ristretto255_is_canonical(s) == 0 {
        return -1;
    }
    s_ = ffrombytes(s);
    ss = fsq(&s_); /* ss = s^2 */

    u1 = fe_1();
    u1 = fsub(&u1, &ss); /* u1 = 1-ss */
    u1u1 = fsq(&u1); /* u1u1 = u1^2 */

    u2 = fe_1();
    u2 = fadd(&u2, &ss); /* u2 = 1+ss */
    u2u2 = fsq(&u2); /* u2u2 = u2^2 */

    v = fmul(&ED25519_D, &u1u1); /* v = d*u1^2 */
    v = fneg(&v); /* v = -d*u1^2 */
    v = fsub(&v, &u2u2); /* v = -(d*u1^2)-u2^2 */

    v_u2u2 = fmul(&v, &u2u2); /* v_u2u2 = v*u2^2 */

    one = fe_1();
    notsquare = ristretto255_sqrt_ratio_m1(&mut inv_sqrt, &one, &v_u2u2);
    h.X = fmul(&inv_sqrt, &u2);
    let hx = h.X;
    h.Y = fmul(&inv_sqrt, &hx);
    let hy = h.Y;
    h.Y = fmul(&hy, &v);

    let hx = h.X;
    h.X = fmul(&hx, &s_);
    let hx = h.X;
    h.X = fadd(&hx, &hx);
    fabs(&mut h.X);
    let hy = h.Y;
    h.Y = fmul(&u1, &hy);
    h.Z = fe_1();
    h.T = fmul(&h.X, &h.Y);

    -((1 - notsquare) | fisnegative(&h.T) | fiszero(&h.Y))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_frombytes(h: *mut Ge25519P3, s: *const u8) -> c_int {
    ristretto255_frombytes(&mut *h, s)
}

pub unsafe fn ristretto255_p3_tobytes(s: *mut u8, h: &Ge25519P3) {
    let den1: Fe25519;
    let den2: Fe25519;
    let mut den_inv: Fe25519;
    let eden: Fe25519;
    let mut inv_sqrt: Fe25519 = FE_ZERO;
    let ix: Fe25519;
    let iy: Fe25519;
    let one: Fe25519;
    let mut s_: Fe25519;
    let t_z_inv: Fe25519;
    let mut u1: Fe25519;
    let u2: Fe25519;
    let mut u1_u2u2: Fe25519;
    let mut x_: Fe25519;
    let mut y_: Fe25519;
    let x_z_inv: Fe25519;
    let mut z_inv: Fe25519;
    let zmy: Fe25519;
    let rotate: c_int;

    u1 = fadd(&h.Z, &h.Y); /* u1 = Z+Y */
    zmy = fsub(&h.Z, &h.Y); /* zmy = Z-Y */
    u1 = fmul(&u1, &zmy); /* u1 = (Z+Y)*(Z-Y) */
    u2 = fmul(&h.X, &h.Y); /* u2 = X*Y */

    u1_u2u2 = fsq(&u2); /* u1_u2u2 = u2^2 */
    u1_u2u2 = fmul(&u1, &u1_u2u2); /* u1_u2u2 = u1*u2^2 */

    one = fe_1();
    let _ = ristretto255_sqrt_ratio_m1(&mut inv_sqrt, &one, &u1_u2u2);
    den1 = fmul(&inv_sqrt, &u1); /* den1 = inv_sqrt*u1 */
    den2 = fmul(&inv_sqrt, &u2); /* den2 = inv_sqrt*u2 */
    z_inv = fmul(&den1, &den2); /* z_inv = den1*den2 */
    z_inv = fmul(&z_inv, &h.T); /* z_inv = den1*den2*T */

    ix = fmul(&h.X, &FE25519_SQRTM1); /* ix = X*sqrt(-1) */
    iy = fmul(&h.Y, &FE25519_SQRTM1); /* iy = Y*sqrt(-1) */
    eden = fmul(&den1, &ED25519_INVSQRTAMD); /* eden = den1/sqrt(a-d) */

    t_z_inv = fmul(&h.T, &z_inv); /* t_z_inv = T*z_inv */
    rotate = fisnegative(&t_z_inv);

    /* fe25519_copy() is a plain limb-wise copy */
    x_ = h.X;
    y_ = h.Y;
    den_inv = den2;

    fcmov(&mut x_, &iy, rotate as u32);
    fcmov(&mut y_, &ix, rotate as u32);
    fcmov(&mut den_inv, &eden, rotate as u32);

    x_z_inv = fmul(&x_, &z_inv);
    fcneg(&mut y_, fisnegative(&x_z_inv) as u32);

    s_ = fsub(&h.Z, &y_);
    s_ = fmul(&den_inv, &s_);
    fabs(&mut s_);
    ftobytes(s, &s_);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const Ge25519P3) {
    ristretto255_p3_tobytes(s, &*h);
}

fn ristretto255_elligator(p: &mut Ge25519P3, t: &Fe25519) {
    let mut c: Fe25519;
    let mut n: Fe25519;
    let one: Fe25519;
    let mut r: Fe25519;
    let rpd: Fe25519;
    let mut s: Fe25519 = FE_ZERO;
    let mut s_prime: Fe25519;
    let ss: Fe25519;
    let mut u: Fe25519;
    let mut v: Fe25519;
    let mut w0: Fe25519;
    let w1: Fe25519;
    let w2: Fe25519;
    let w3: Fe25519;
    let wasnt_square: c_int;

    one = fe_1();
    r = fsq(t); /* r = t^2 */
    r = fmul(&FE25519_SQRTM1, &r); /* r = sqrt(-1)*t^2 */
    u = fadd(&r, &one); /* u = r+1 */
    u = fmul(&u, &ED25519_ONEMSQD); /* u = (r+1)*(1-d^2) */
    c = fe_1();
    c = fneg(&c); /* c = -1 */
    rpd = fadd(&r, &ED25519_D); /* rpd = r+d */
    v = fmul(&r, &ED25519_D); /* v = r*d */
    v = fsub(&c, &v); /* v = c-r*d */
    v = fmul(&v, &rpd); /* v = (c-r*d)*(r+d) */

    wasnt_square = 1 - ristretto255_sqrt_ratio_m1(&mut s, &u, &v);
    s_prime = fmul(&s, t);
    fabs(&mut s_prime);
    s_prime = fneg(&s_prime); /* s_prime = -|s*t| */
    fcmov(&mut s, &s_prime, wasnt_square as u32);
    fcmov(&mut c, &r, wasnt_square as u32);

    n = fsub(&r, &one); /* n = r-1 */
    n = fmul(&n, &c); /* n = c*(r-1) */
    n = fmul(&n, &ED25519_SQDMONE); /* n = c*(r-1)*(d-1)^2 */
    n = fsub(&n, &v); /* n =  c*(r-1)*(d-1)^2-v */

    w0 = fadd(&s, &s); /* w0 = 2s */
    w0 = fmul(&w0, &v); /* w0 = 2s*v */
    w1 = fmul(&n, &ED25519_SQRTADM1); /* w1 = n*sqrt(ad-1) */
    ss = fsq(&s); /* ss = s^2 */
    w2 = fsub(&one, &ss); /* w2 = 1-s^2 */
    w3 = fadd(&one, &ss); /* w3 = 1+s^2 */

    p.X = fmul(&w0, &w3);
    p.Y = fmul(&w2, &w1);
    p.Z = fmul(&w1, &w3);
    p.T = fmul(&w0, &w2);
}

pub unsafe fn ristretto255_from_hash(s: *mut u8, h: *const u8) {
    let r0: Fe25519;
    let r1: Fe25519;
    let mut p0 = Ge25519P3::zeroed();
    let mut p1 = Ge25519P3::zeroed();
    let mut p = Ge25519P3::zeroed();

    r0 = ffrombytes(h);
    r1 = ffrombytes(h.add(32));
    ristretto255_elligator(&mut p0, &r0);
    ristretto255_elligator(&mut p1, &r1);
    p3_add(&mut p, &p0, &p1);
    ristretto255_p3_tobytes(s, &p);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_from_hash(s: *mut u8, h: *const u8) {
    ristretto255_from_hash(s, h);
}
