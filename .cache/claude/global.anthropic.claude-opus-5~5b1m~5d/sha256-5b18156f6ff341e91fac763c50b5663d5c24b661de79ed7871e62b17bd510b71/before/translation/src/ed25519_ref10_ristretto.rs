//! Montgomery <-> Edwards conversion, Elligator2, and the Ristretto255 group,
//! for ed25519 ref10.
//!
//! Translated from
//! `c_src/libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c`, lines
//! 2590..2992 (from `ge25519_mont_to_ed` through the end of the file):
//! `ge25519_mont_to_ed`, `ge25519_xmont_to_ymont`, `ge25519_clear_cofactor`,
//! `ge25519_elligator2`, `ge25519_from_uniform`, `fe25519_reduce64`,
//! `ge25519_from_hash`, `ristretto255_sqrt_ratio_m1`,
//! `ristretto255_is_canonical`, `ristretto255_frombytes`,
//! `ristretto255_p3_tobytes`, `ristretto255_elligator`,
//! `ristretto255_from_hash`.
//!
//! `HAVE_TI_MODE` is not defined in the reference build, so `fe25519` is
//! `int32_t[10]` (`crate::types::fe25519`).
//!
//! Where the original C code aliases an output field with one of its own
//! input fields (e.g. `fe25519_invert(rr2, rr2)`), we copy the aliased input
//! to a local temporary first, since the C fe25519 helpers read all inputs
//! into locals before writing the output — identical behaviour, alias-safe
//! in Rust.

use crate::ed25519_ref10_fe::*;
use crate::ed25519_ref10_ge::*;
use crate::ed25519_ref10_tables::*;
use crate::types::{fe25519, ge25519_p1p1, ge25519_p2, ge25519_p3};
use core::ffi::c_int;

/// `static volatile unsigned char optblocker_u8;`
///
/// Purely a compiler optimization blocker in the C source (never written),
/// so its value is always 0. Reproduced as a plain (never-mutated) static,
/// matching the same pattern used in `ed25519_ref10_ge.rs`.
static OPTBLOCKER_U8: u8 = 0;

// ---------------------------------------------------------------------------
// montgomery <-> edwards
// ---------------------------------------------------------------------------

/// `static void ge25519_mont_to_ed(fe25519 xed, fe25519 yed, const fe25519 x, const fe25519 y)`
pub fn ge25519_mont_to_ed(xed: &mut fe25519, yed: &mut fe25519, x: &fe25519, y: &fe25519) {
    let mut one: fe25519 = [0; 10];
    let mut x_plus_one: fe25519 = [0; 10];
    let mut x_minus_one: fe25519 = [0; 10];
    let mut x_plus_one_y_inv: fe25519 = [0; 10];

    fe25519_1(&mut one);
    fe25519_add(&mut x_plus_one, x, &one);
    fe25519_sub(&mut x_minus_one, x, &one);

    /* xed = sqrt(-A-2)*x/y */
    fe25519_mul(&mut x_plus_one_y_inv, &x_plus_one, y);
    {
        let t = x_plus_one_y_inv;
        fe25519_invert(&mut x_plus_one_y_inv, &t); /* 1/((x+1)*y) */
    }
    fe25519_mul(xed, x, &ed25519_sqrtam2);
    fe25519_mul_ip(xed, &x_plus_one_y_inv); /* sqrt(-A-2)*x/((x+1)*y) */
    fe25519_mul_ip(xed, &x_plus_one);

    /* yed = (x-1)/(x+1) */
    fe25519_mul(yed, &x_plus_one_y_inv, y); /* 1/(x+1) */
    fe25519_mul_ip(yed, &x_minus_one);
    let iz = fe25519_iszero(&x_plus_one_y_inv) as u32;
    fe25519_cmov(yed, &one, iz);
}

/// `static int ge25519_xmont_to_ymont(fe25519 y, const fe25519 x)`
pub fn ge25519_xmont_to_ymont(y: &mut fe25519, x: &fe25519) -> c_int {
    let mut x2: fe25519 = [0; 10];
    let mut x3: fe25519 = [0; 10];

    fe25519_sq(&mut x2, x);
    fe25519_mul(&mut x3, x, &x2);
    {
        let t = x2;
        fe25519_mul32(&mut x2, &t, ed25519_A_32 as u32);
    }
    fe25519_add(y, &x3, x);
    {
        let t = *y;
        fe25519_add(y, &t, &x2);
    }

    let y2 = *y;
    fe25519_sqrt(y, &y2)
}

/// multiply by the cofactor
/// `void ge25519_clear_cofactor(ge25519_p3 *p3)`
pub fn ge25519_clear_cofactor(p3: &mut ge25519_p3) {
    let mut p1: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut p2: ge25519_p2 = ge25519_p2::zero();

    ge25519_p3_dbl(&mut p1, p3);
    ge25519_p1p1_to_p2(&mut p2, &p1);
    ge25519_p2_dbl(&mut p1, &p2);
    ge25519_p1p1_to_p2(&mut p2, &p1);
    ge25519_p2_dbl(&mut p1, &p2);
    ge25519_p1p1_to_p3(p3, &p1);
}

/// `static void ge25519_elligator2(fe25519 x, fe25519 y, const fe25519 r, int *notsquare_p)`
pub fn ge25519_elligator2(x: &mut fe25519, y: &mut fe25519, r: &fe25519, notsquare_p: &mut c_int) {
    let mut gx1: fe25519 = [0; 10];
    let mut rr2: fe25519 = [0; 10];
    let mut x2: fe25519 = [0; 10];
    let mut x3: fe25519 = [0; 10];
    let mut negx: fe25519 = [0; 10];
    let notsquare: c_int;

    fe25519_sq2(&mut rr2, r);
    rr2[0] = rr2[0].wrapping_add(1);
    {
        let t = rr2;
        fe25519_invert(&mut rr2, &t);
    }
    fe25519_mul32(x, &rr2, ed25519_A_32 as u32);
    {
        let t = *x;
        fe25519_neg(x, &t); /* x=x1 */
    }

    fe25519_sq(&mut x2, x);
    fe25519_mul(&mut x3, x, &x2);
    {
        let t = x2;
        fe25519_mul32(&mut x2, &t, ed25519_A_32 as u32); /* x2 = A*x1^2 */
    }
    fe25519_add(&mut gx1, &x3, x);
    {
        let t = gx1;
        fe25519_add(&mut gx1, &t, &x2); /* gx1 = x1^3 + A*x1^2 + x1 */
    }

    notsquare = fe25519_notsquare(&gx1);

    /* gx1 not a square  => x = -x1-A */
    fe25519_neg(&mut negx, x);
    fe25519_cmov(x, &negx, notsquare as u32);
    fe25519_0(&mut x2);
    fe25519_cmov(&mut x2, &ed25519_A, notsquare as u32);
    {
        let t = *x;
        fe25519_sub(x, &t, &x2);
    }

    /* y = sqrt(gx1) or sqrt(gx2) with gx2 = gx1 * (A+x1) / -x1 */
    /* but it is about as fast to just recompute from the curve equation. */
    if ge25519_xmont_to_ymont(y, x) != 0 {
        unsafe {
            crate::csys::abort(); /* LCOV_EXCL_LINE */
        }
    }
    *notsquare_p = notsquare;
}

/// `void ge25519_from_uniform(unsigned char s[32], const unsigned char r[32])`
pub unsafe fn ge25519_from_uniform(s: *mut u8, r: *const u8) {
    let mut p3: ge25519_p3 = ge25519_p3::zero();
    let mut x: fe25519 = [0; 10];
    let mut y: fe25519 = [0; 10];
    let mut negxed: fe25519 = [0; 10];
    let mut r_fe: fe25519 = [0; 10];
    let mut notsquare: c_int = 0;

    for i in 0..32usize {
        *s.add(i) = *r.add(i);
    }
    let x_sign: u8 = ((*s.add(31) >> 5) ^ OPTBLOCKER_U8) >> 2;
    *s.add(31) &= 0x7f;
    fe25519_frombytes(&mut r_fe, s);

    ge25519_elligator2(&mut x, &mut y, &r_fe, &mut notsquare);

    ge25519_mont_to_ed(&mut p3.X, &mut p3.Y, &x, &y);
    fe25519_neg(&mut negxed, &p3.X);
    let cond: u32 = (fe25519_isnegative(&p3.X) as u32) ^ (x_sign as u32);
    fe25519_cmov(&mut p3.X, &negxed, cond);

    fe25519_1(&mut p3.Z);
    fe25519_mul(&mut p3.T, &p3.X, &p3.Y);
    ge25519_clear_cofactor(&mut p3);
    ge25519_p3_tobytes(s, &p3);
}

/// `static void fe25519_reduce64(fe25519 fe_f, const unsigned char h[64])`
pub unsafe fn fe25519_reduce64(fe_f: &mut fe25519, h: *const u8) {
    let mut fl = [0u8; 32];
    let mut gl = [0u8; 32];
    let mut fe_g: fe25519 = [0; 10];

    for i in 0..32usize {
        fl[i] = *h.add(i);
        gl[i] = *h.add(32 + i);
    }
    fl[31] &= 0x7f;
    gl[31] &= 0x7f;
    fe25519_frombytes(fe_f, fl.as_ptr());
    fe25519_frombytes(&mut fe_g, gl.as_ptr());

    let h31 = *h.add(31);
    let h63 = *h.add(63);
    let term1: i32 = (((h31 >> 5) ^ OPTBLOCKER_U8) >> 2) as i32 * 19;
    let term2: i32 = (((h63 >> 5) ^ OPTBLOCKER_U8) >> 2) as i32 * 722;
    fe_f[0] = fe_f[0].wrapping_add(term1).wrapping_add(term2);

    for i in 0..10usize {
        fe_f[i] = fe_f[i].wrapping_add(38i32.wrapping_mul(fe_g[i]));
    }
    let t = *fe_f;
    fe25519_reduce(fe_f, &t);
}

/// `void ge25519_from_hash(unsigned char s[32], const unsigned char h[64])`
/* LCOV_EXCL_START */
pub unsafe fn ge25519_from_hash(s: *mut u8, h: *const u8) {
    let mut p3: ge25519_p3 = ge25519_p3::zero();
    let mut fe_f: fe25519 = [0; 10];
    let mut x: fe25519 = [0; 10];
    let mut y: fe25519 = [0; 10];
    let mut negy: fe25519 = [0; 10];
    let mut notsquare: c_int = 0;

    fe25519_reduce64(&mut fe_f, h);
    ge25519_elligator2(&mut x, &mut y, &fe_f, &mut notsquare);

    let y_sign: c_int = notsquare ^ 1;
    fe25519_neg(&mut negy, &y);
    let cond: u32 = (fe25519_isnegative(&y) ^ y_sign) as u32;
    fe25519_cmov(&mut y, &negy, cond);

    ge25519_mont_to_ed(&mut p3.X, &mut p3.Y, &x, &y);

    fe25519_1(&mut p3.Z);
    fe25519_mul(&mut p3.T, &p3.X, &p3.Y);
    ge25519_clear_cofactor(&mut p3);
    ge25519_p3_tobytes(s, &p3);
}
/* LCOV_EXCL_STOP */

// ---------------------------------------------------------------------------
// Ristretto group
// ---------------------------------------------------------------------------

/// `static int ristretto255_sqrt_ratio_m1(fe25519 x, const fe25519 u, const fe25519 v)`
pub fn ristretto255_sqrt_ratio_m1(x: &mut fe25519, u: &fe25519, v: &fe25519) -> c_int {
    let mut v3: fe25519 = [0; 10];
    let mut vxx: fe25519 = [0; 10];
    let mut m_root_check: fe25519 = [0; 10];
    let mut p_root_check: fe25519 = [0; 10];
    let mut f_root_check: fe25519 = [0; 10];
    let mut x_sqrtm1: fe25519 = [0; 10];

    fe25519_sq(&mut v3, v);
    fe25519_mul_ip(&mut v3, v); /* v3 = v^3 */
    fe25519_sq(x, &v3);
    fe25519_mul_ip(x, u);
    fe25519_mul_ip(x, v); /* x = uv^7 */

    {
        let t = *x;
        fe25519_pow22523(x, &t); /* x = (uv^7)^((q-5)/8) */
    }
    fe25519_mul_ip(x, &v3);
    fe25519_mul_ip(x, u); /* x = uv^3(uv^7)^((q-5)/8) */

    fe25519_sq(&mut vxx, x);
    fe25519_mul_ip(&mut vxx, v); /* vx^2 */
    fe25519_sub(&mut m_root_check, &vxx, u); /* vx^2-u */
    fe25519_add(&mut p_root_check, &vxx, u); /* vx^2+u */
    fe25519_mul(&mut f_root_check, u, &fe25519_sqrtm1); /* u*sqrt(-1) */
    {
        let t = f_root_check;
        fe25519_add(&mut f_root_check, &vxx, &t); /* vx^2+u*sqrt(-1) */
    }
    let has_m_root: c_int = fe25519_iszero(&m_root_check);
    let has_p_root: c_int = fe25519_iszero(&p_root_check);
    let has_f_root: c_int = fe25519_iszero(&f_root_check);
    fe25519_mul(&mut x_sqrtm1, x, &fe25519_sqrtm1); /* x*sqrt(-1) */

    fe25519_cmov(x, &x_sqrtm1, (has_p_root | has_f_root) as u32);
    fe25519_abs(x);

    has_m_root | has_p_root
}

/// `static int ristretto255_is_canonical(const unsigned char *s)`
pub unsafe fn ristretto255_is_canonical(s: *const u8) -> c_int {
    let mut c: u8 = (*s.add(31) & 0x7f) ^ 0x7f;
    let mut i: i32 = 30;
    while i > 0 {
        c |= *s.add(i as usize) ^ 0xff;
        i -= 1;
    }
    let c2: u8 = ((c as u32).wrapping_sub(1) >> 8) as u8;
    let d: u8 = ((0xedu32.wrapping_sub(1).wrapping_sub(*s as u32)) >> 8) as u8;
    let e: u8 = ((*s.add(31) >> 5) ^ OPTBLOCKER_U8) >> 2;

    1i32.wrapping_sub((((c2 & d) | e | *s) & 1) as i32)
}

/// `int ristretto255_frombytes(ge25519_p3 *h, const unsigned char *s)`
pub unsafe fn ristretto255_frombytes(h: &mut ge25519_p3, s: *const u8) -> c_int {
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
    fe25519_frombytes(&mut s_, s);
    fe25519_sq(&mut ss, &s_); /* ss = s^2 */

    fe25519_1(&mut u1);
    {
        let t = u1;
        fe25519_sub(&mut u1, &t, &ss); /* u1 = 1-ss */
    }
    fe25519_sq(&mut u1u1, &u1); /* u1u1 = u1^2 */

    fe25519_1(&mut u2);
    {
        let t = u2;
        fe25519_add(&mut u2, &t, &ss); /* u2 = 1+ss */
    }
    fe25519_sq(&mut u2u2, &u2); /* u2u2 = u2^2 */

    fe25519_mul(&mut v, &ed25519_d, &u1u1); /* v = d*u1^2 */
    {
        let t = v;
        fe25519_neg(&mut v, &t); /* v = -d*u1^2 */
    }
    {
        let t = v;
        fe25519_sub(&mut v, &t, &u2u2); /* v = -(d*u1^2)-u2^2 */
    }

    fe25519_mul(&mut v_u2u2, &v, &u2u2); /* v_u2u2 = v*u2^2 */

    fe25519_1(&mut one);
    notsquare = ristretto255_sqrt_ratio_m1(&mut inv_sqrt, &one, &v_u2u2);
    fe25519_mul(&mut h.X, &inv_sqrt, &u2);
    {
        let hx = h.X;
        fe25519_mul(&mut h.Y, &inv_sqrt, &hx);
    }
    {
        let hy = h.Y;
        fe25519_mul(&mut h.Y, &hy, &v);
    }

    {
        let hx = h.X;
        fe25519_mul(&mut h.X, &hx, &s_);
    }
    {
        let hx = h.X;
        fe25519_add(&mut h.X, &hx, &hx);
    }
    fe25519_abs(&mut h.X);
    {
        let hy = h.Y;
        fe25519_mul(&mut h.Y, &u1, &hy);
    }
    fe25519_1(&mut h.Z);
    fe25519_mul(&mut h.T, &h.X, &h.Y);

    -((1 - notsquare) | fe25519_isnegative(&h.T) | fe25519_iszero(&h.Y))
}

/// `void ristretto255_p3_tobytes(unsigned char *s, const ge25519_p3 *h)`
pub unsafe fn ristretto255_p3_tobytes(s: *mut u8, h: &ge25519_p3) {
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

    fe25519_add(&mut u1, &h.Z, &h.Y); /* u1 = Z+Y */
    fe25519_sub(&mut zmy, &h.Z, &h.Y); /* zmy = Z-Y */
    {
        let t = u1;
        fe25519_mul(&mut u1, &t, &zmy); /* u1 = (Z+Y)*(Z-Y) */
    }
    fe25519_mul(&mut u2, &h.X, &h.Y); /* u2 = X*Y */

    fe25519_sq(&mut u1_u2u2, &u2); /* u1_u2u2 = u2^2 */
    {
        let t = u1_u2u2;
        fe25519_mul(&mut u1_u2u2, &u1, &t); /* u1_u2u2 = u1*u2^2 */
    }

    fe25519_1(&mut one);
    let _ = ristretto255_sqrt_ratio_m1(&mut inv_sqrt, &one, &u1_u2u2);
    fe25519_mul(&mut den1, &inv_sqrt, &u1); /* den1 = inv_sqrt*u1 */
    fe25519_mul(&mut den2, &inv_sqrt, &u2); /* den2 = inv_sqrt*u2 */
    fe25519_mul(&mut z_inv, &den1, &den2); /* z_inv = den1*den2 */
    {
        let t = z_inv;
        fe25519_mul(&mut z_inv, &t, &h.T); /* z_inv = den1*den2*T */
    }

    fe25519_mul(&mut ix, &h.X, &fe25519_sqrtm1); /* ix = X*sqrt(-1) */
    fe25519_mul(&mut iy, &h.Y, &fe25519_sqrtm1); /* iy = Y*sqrt(-1) */
    fe25519_mul(&mut eden, &den1, &ed25519_invsqrtamd); /* eden = den1/sqrt(a-d) */

    fe25519_mul(&mut t_z_inv, &h.T, &z_inv); /* t_z_inv = T*z_inv */
    rotate = fe25519_isnegative(&t_z_inv);

    fe25519_copy(&mut x_, &h.X);
    fe25519_copy(&mut y_, &h.Y);
    fe25519_copy(&mut den_inv, &den2);

    fe25519_cmov(&mut x_, &iy, rotate as u32);
    fe25519_cmov(&mut y_, &ix, rotate as u32);
    fe25519_cmov(&mut den_inv, &eden, rotate as u32);

    fe25519_mul(&mut x_z_inv, &x_, &z_inv);
    let neg_cond = fe25519_isnegative(&x_z_inv) as u32;
    fe25519_cneg(&mut y_, neg_cond);

    fe25519_sub(&mut s_, &h.Z, &y_);
    {
        let t = s_;
        fe25519_mul(&mut s_, &den_inv, &t);
    }
    fe25519_abs(&mut s_);
    fe25519_tobytes(s, &s_);
}

/// `static void ristretto255_elligator(ge25519_p3 *p, const fe25519 t)`
pub fn ristretto255_elligator(p: &mut ge25519_p3, t: &fe25519) {
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

    fe25519_1(&mut one);
    fe25519_sq(&mut r, t); /* r = t^2 */
    {
        let t2 = r;
        fe25519_mul(&mut r, &fe25519_sqrtm1, &t2); /* r = sqrt(-1)*t^2 */
    }
    fe25519_add(&mut u, &r, &one); /* u = r+1 */
    {
        let t2 = u;
        fe25519_mul(&mut u, &t2, &ed25519_onemsqd); /* u = (r+1)*(1-d^2) */
    }
    fe25519_1(&mut c);
    {
        let t2 = c;
        fe25519_neg(&mut c, &t2); /* c = -1 */
    }
    fe25519_add(&mut rpd, &r, &ed25519_d); /* rpd = r+d */
    fe25519_mul(&mut v, &r, &ed25519_d); /* v = r*d */
    {
        let t2 = v;
        fe25519_sub(&mut v, &c, &t2); /* v = c-r*d */
    }
    {
        let t2 = v;
        fe25519_mul(&mut v, &t2, &rpd); /* v = (c-r*d)*(r+d) */
    }

    wasnt_square = 1 - ristretto255_sqrt_ratio_m1(&mut s, &u, &v);
    fe25519_mul(&mut s_prime, &s, t);
    fe25519_abs(&mut s_prime);
    {
        let t2 = s_prime;
        fe25519_neg(&mut s_prime, &t2); /* s_prime = -|s*t| */
    }
    fe25519_cmov(&mut s, &s_prime, wasnt_square as u32);
    fe25519_cmov(&mut c, &r, wasnt_square as u32);

    fe25519_sub(&mut n, &r, &one); /* n = r-1 */
    {
        let t2 = n;
        fe25519_mul(&mut n, &t2, &c); /* n = c*(r-1) */
    }
    {
        let t2 = n;
        fe25519_mul(&mut n, &t2, &ed25519_sqdmone); /* n = c*(r-1)*(d-1)^2 */
    }
    {
        let t2 = n;
        fe25519_sub(&mut n, &t2, &v); /* n =  c*(r-1)*(d-1)^2-v */
    }

    fe25519_add(&mut w0, &s, &s); /* w0 = 2s */
    {
        let t2 = w0;
        fe25519_mul(&mut w0, &t2, &v); /* w0 = 2s*v */
    }
    fe25519_mul(&mut w1, &n, &ed25519_sqrtadm1); /* w1 = n*sqrt(ad-1) */
    fe25519_sq(&mut ss, &s); /* ss = s^2 */
    fe25519_sub(&mut w2, &one, &ss); /* w2 = 1-s^2 */
    fe25519_add(&mut w3, &one, &ss); /* w3 = 1+s^2 */

    fe25519_mul(&mut p.X, &w0, &w3);
    fe25519_mul(&mut p.Y, &w2, &w1);
    fe25519_mul(&mut p.Z, &w1, &w3);
    fe25519_mul(&mut p.T, &w0, &w2);
}

/// `void ristretto255_from_hash(unsigned char s[32], const unsigned char h[64])`
pub unsafe fn ristretto255_from_hash(s: *mut u8, h: *const u8) {
    let mut r0: fe25519 = [0; 10];
    let mut r1: fe25519 = [0; 10];
    let mut p0: ge25519_p3 = ge25519_p3::zero();
    let mut p1: ge25519_p3 = ge25519_p3::zero();
    let mut p: ge25519_p3 = ge25519_p3::zero();

    fe25519_frombytes(&mut r0, h);
    fe25519_frombytes(&mut r1, h.add(32));
    ristretto255_elligator(&mut p0, &r0);
    ristretto255_elligator(&mut p1, &r1);
    ge25519_p3_add(&mut p, &p0, &p1);
    ristretto255_p3_tobytes(s, &p);
}

// ---------------------------------------------------------------------------
// Exported C symbols (see $W/_cbuild/persym.txt for
// crypto_core/ed25519/ref10/ed25519_ref10.c.o); only symbols whose defining
// function lies within lines 2590..2992 of ed25519_ref10.c are provided here.
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_clear_cofactor(p3: *mut ge25519_p3) {
    ge25519_clear_cofactor(&mut *p3);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8) {
    ge25519_from_uniform(s, r);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8) {
    ge25519_from_hash(s, h);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int {
    ristretto255_frombytes(&mut *h, s)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const ge25519_p3) {
    ristretto255_p3_tobytes(s, &*h);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ristretto255_from_hash(s: *mut u8, h: *const u8) {
    ristretto255_from_hash(s, h);
}
