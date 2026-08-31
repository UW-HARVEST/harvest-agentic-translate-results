//! Group arithmetic (`ge25519_*`) for ed25519 ref10.
//!
//! Translated from
//! `c_src/libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c`, lines 263..1200
//! (from `ge25519_add_cached` through `ge25519_has_small_order`).
//!
//! `HAVE_TI_MODE` is not defined in the reference build, so `fe25519` is
//! `int32_t[10]` (`crate::types::fe25519`), matching the `fe_25_5`
//! representation used by `crate::ed25519_ref10_fe`.
//!
//! Field-element helper functions take `&`/`&mut fe25519`. Where the original
//! C code aliases an output field with one of its own input fields (e.g.
//! `fe25519_mul(r->Y, r->Y, q->YminusX)`), we copy the aliased field to a
//! local temporary first and pass that, which is semantically identical since
//! the C helper functions read all inputs into locals before writing the
//! output.

#![allow(non_snake_case)]

use crate::ed25519_ref10_fe::*;
use crate::ed25519_ref10_tables::{base, base2, ed25519_d, ed25519_d2, fe25519_sqrtm1};
use crate::types::{fe25519, ge25519_cached, ge25519_p1p1, ge25519_p2, ge25519_p3, ge25519_precomp};
use core::ffi::c_int;

/// `static volatile unsigned char optblocker_u8;`
///
/// In the C source this is a volatile static used purely as a compiler
/// optimization blocker (it is never written anywhere in the file, so its
/// value is always 0). We reproduce it as a plain (never-mutated) static so
/// the arithmetic in `equal`/`negative`/`ge25519_frombytes` matches exactly,
/// without needing `unsafe` to read it.
static OPTBLOCKER_U8: u8 = 0;

// ---------------------------------------------------------------------------
// r = p + q
// ---------------------------------------------------------------------------

/// `static void ge25519_add_cached(ge25519_p1p1 *r, const ge25519_p3 *p, const ge25519_cached *q)`
pub fn ge25519_add_cached(r: &mut ge25519_p1p1, p: &ge25519_p3, q: &ge25519_cached) {
    let mut t0: fe25519 = [0i32; 10];

    fe25519_add(&mut r.X, &p.Y, &p.X);
    fe25519_sub(&mut r.Y, &p.Y, &p.X);
    fe25519_mul(&mut r.Z, &r.X, &q.YplusX);
    {
        let ry = r.Y;
        fe25519_mul(&mut r.Y, &ry, &q.YminusX);
    }
    fe25519_mul(&mut r.T, &q.T2d, &p.T);
    fe25519_mul(&mut r.X, &p.Z, &q.Z);
    fe25519_add(&mut t0, &r.X, &r.X);
    fe25519_sub(&mut r.X, &r.Z, &r.Y);
    {
        let ry = r.Y;
        fe25519_add(&mut r.Y, &r.Z, &ry);
    }
    fe25519_add(&mut r.Z, &t0, &r.T);
    {
        let rt = r.T;
        fe25519_sub(&mut r.T, &t0, &rt);
    }
}

/// `static void slide_vartime(signed char *r, const unsigned char *a)`
pub fn slide_vartime(r: &mut [i8; 256], a: &[u8; 32]) {
    for i in 0..256 {
        r[i] = (1 & (a[i >> 3] >> (i & 7))) as i8;
    }
    for i in 0..256 {
        if r[i] == 0 {
            continue;
        }
        let mut b = 1i32;
        while b <= 6 && i + (b as usize) < 256 {
            let bu = b as usize;
            if r[i + bu] == 0 {
                b += 1;
                continue;
            }
            let ribs: i32 = (r[i + bu] as i32).wrapping_shl(b as u32);
            let mut cmp: i32 = (r[i] as i32).wrapping_add(ribs);
            if cmp <= 15 {
                r[i] = cmp as i8;
                r[i + bu] = 0;
            } else {
                cmp = (r[i] as i32).wrapping_sub(ribs);
                if cmp < -15 {
                    break;
                }
                r[i] = cmp as i8;
                let mut k = i + bu;
                while k < 256 {
                    if r[k] == 0 {
                        r[k] = 1;
                        break;
                    }
                    r[k] = 0;
                    k += 1;
                }
            }
            b += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// ge25519_frombytes / ge25519_frombytes_negate_vartime
// ---------------------------------------------------------------------------

/// `int ge25519_frombytes(ge25519_p3 *h, const unsigned char *s)`
pub unsafe fn ge25519_frombytes(h: &mut ge25519_p3, s: *const u8) -> c_int {
    let mut u: fe25519 = [0i32; 10];
    let mut v: fe25519 = [0i32; 10];
    let mut vxx: fe25519 = [0i32; 10];
    let mut m_root_check: fe25519 = [0i32; 10];
    let mut p_root_check: fe25519 = [0i32; 10];
    let mut negx: fe25519 = [0i32; 10];
    let mut x_sqrtm1: fe25519 = [0i32; 10];

    fe25519_frombytes(&mut h.Y, s);
    fe25519_1(&mut h.Z);
    fe25519_sq(&mut u, &h.Y);
    fe25519_mul(&mut v, &u, &ed25519_d);
    {
        let uu = u;
        fe25519_sub(&mut u, &uu, &h.Z); /* u = y^2-1 */
    }
    {
        let vv = v;
        fe25519_add(&mut v, &vv, &h.Z); /* v = dy^2+1 */
    }

    fe25519_mul(&mut h.X, &u, &v);
    {
        let hx = h.X;
        fe25519_pow22523(&mut h.X, &hx);
    }
    {
        let hx = h.X;
        fe25519_mul(&mut h.X, &u, &hx); /* u((uv)^((q-5)/8)) */
    }

    fe25519_sq(&mut vxx, &h.X);
    {
        let vxx0 = vxx;
        fe25519_mul(&mut vxx, &vxx0, &v);
    }
    fe25519_sub(&mut m_root_check, &vxx, &u); /* vx^2-u */
    fe25519_add(&mut p_root_check, &vxx, &u); /* vx^2+u */
    let has_m_root: c_int = fe25519_iszero(&m_root_check);
    let has_p_root: c_int = fe25519_iszero(&p_root_check);
    fe25519_mul(&mut x_sqrtm1, &h.X, &fe25519_sqrtm1); /* x*sqrt(-1) */
    fe25519_cmov(&mut h.X, &x_sqrtm1, (1 - has_m_root) as u32);

    fe25519_neg(&mut negx, &h.X);
    let cond: u32 = (fe25519_isnegative(&h.X) ^ (((*s.add(31) >> 5) as c_int) ^ (OPTBLOCKER_U8 as c_int)) >> 2) as u32;
    fe25519_cmov(&mut h.X, &negx, cond);
    fe25519_mul(&mut h.T, &h.X, &h.Y);

    (has_m_root | has_p_root) - 1
}

/// `int ge25519_frombytes_negate_vartime(ge25519_p3 *h, const unsigned char *s)`
pub unsafe fn ge25519_frombytes_negate_vartime(h: &mut ge25519_p3, s: *const u8) -> c_int {
    let mut u: fe25519 = [0i32; 10];
    let mut v: fe25519 = [0i32; 10];
    let mut v3: fe25519 = [0i32; 10];
    let mut vxx: fe25519 = [0i32; 10];
    let mut m_root_check: fe25519 = [0i32; 10];
    let mut p_root_check: fe25519 = [0i32; 10];

    fe25519_frombytes(&mut h.Y, s);
    fe25519_1(&mut h.Z);
    fe25519_sq(&mut u, &h.Y);
    fe25519_mul(&mut v, &u, &ed25519_d);
    {
        let uu = u;
        fe25519_sub(&mut u, &uu, &h.Z); /* u = y^2-1 */
    }
    {
        let vv = v;
        fe25519_add(&mut v, &vv, &h.Z); /* v = dy^2+1 */
    }

    fe25519_sq(&mut v3, &v);
    {
        let v3v = v3;
        fe25519_mul(&mut v3, &v3v, &v); /* v3 = v^3 */
    }
    fe25519_sq(&mut h.X, &v3);
    {
        let hx = h.X;
        fe25519_mul(&mut h.X, &hx, &v);
    }
    {
        let hx = h.X;
        fe25519_mul(&mut h.X, &hx, &u); /* x = uv^7 */
    }

    {
        let hx = h.X;
        fe25519_pow22523(&mut h.X, &hx); /* x = (uv^7)^((q-5)/8) */
    }
    {
        let hx = h.X;
        fe25519_mul(&mut h.X, &hx, &v3);
    }
    {
        let hx = h.X;
        fe25519_mul(&mut h.X, &hx, &u); /* x = uv^3(uv^7)^((q-5)/8) */
    }

    fe25519_sq(&mut vxx, &h.X);
    {
        let vxx0 = vxx;
        fe25519_mul(&mut vxx, &vxx0, &v);
    }
    fe25519_sub(&mut m_root_check, &vxx, &u); /* vx^2-u */
    if fe25519_iszero(&m_root_check) == 0 {
        fe25519_add(&mut p_root_check, &vxx, &u); /* vx^2+u */
        if fe25519_iszero(&p_root_check) == 0 {
            return -1;
        }
        let hx = h.X;
        fe25519_mul(&mut h.X, &hx, &fe25519_sqrtm1);
    }

    if fe25519_isnegative(&h.X) == ((*s.add(31) >> 7) as c_int) {
        /* vartime function - compiler optimization is fine */
        let hx = h.X;
        fe25519_neg(&mut h.X, &hx);
    }
    fe25519_mul(&mut h.T, &h.X, &h.Y);

    0
}

// ---------------------------------------------------------------------------
// r = p + q / r = p - q (precomp)
// ---------------------------------------------------------------------------

/// `static void ge25519_add_precomp(ge25519_p1p1 *r, const ge25519_p3 *p, const ge25519_precomp *q)`
pub fn ge25519_add_precomp(r: &mut ge25519_p1p1, p: &ge25519_p3, q: &ge25519_precomp) {
    let mut t0: fe25519 = [0i32; 10];

    fe25519_add(&mut r.X, &p.Y, &p.X);
    fe25519_sub(&mut r.Y, &p.Y, &p.X);
    fe25519_mul(&mut r.Z, &r.X, &q.yplusx);
    {
        let ry = r.Y;
        fe25519_mul(&mut r.Y, &ry, &q.yminusx);
    }
    fe25519_mul(&mut r.T, &q.xy2d, &p.T);
    fe25519_add(&mut t0, &p.Z, &p.Z);
    fe25519_sub(&mut r.X, &r.Z, &r.Y);
    {
        let ry = r.Y;
        fe25519_add(&mut r.Y, &r.Z, &ry);
    }
    fe25519_add(&mut r.Z, &t0, &r.T);
    {
        let rt = r.T;
        fe25519_sub(&mut r.T, &t0, &rt);
    }
}

/// `static void ge25519_sub_precomp(ge25519_p1p1 *r, const ge25519_p3 *p, const ge25519_precomp *q)`
pub fn ge25519_sub_precomp(r: &mut ge25519_p1p1, p: &ge25519_p3, q: &ge25519_precomp) {
    let mut t0: fe25519 = [0i32; 10];

    fe25519_add(&mut r.X, &p.Y, &p.X);
    fe25519_sub(&mut r.Y, &p.Y, &p.X);
    fe25519_mul(&mut r.Z, &r.X, &q.yminusx);
    {
        let ry = r.Y;
        fe25519_mul(&mut r.Y, &ry, &q.yplusx);
    }
    fe25519_mul(&mut r.T, &q.xy2d, &p.T);
    fe25519_add(&mut t0, &p.Z, &p.Z);
    fe25519_sub(&mut r.X, &r.Z, &r.Y);
    {
        let ry = r.Y;
        fe25519_add(&mut r.Y, &r.Z, &ry);
    }
    fe25519_sub(&mut r.Z, &t0, &r.T);
    {
        let rt = r.T;
        fe25519_add(&mut r.T, &t0, &rt);
    }
}

// ---------------------------------------------------------------------------
// r = p (conversions)
// ---------------------------------------------------------------------------

/// `void ge25519_p1p1_to_p2(ge25519_p2 *r, const ge25519_p1p1 *p)`
pub fn ge25519_p1p1_to_p2(r: &mut ge25519_p2, p: &ge25519_p1p1) {
    fe25519_mul(&mut r.X, &p.X, &p.T);
    fe25519_mul(&mut r.Y, &p.Y, &p.Z);
    fe25519_mul(&mut r.Z, &p.Z, &p.T);
}

/// `void ge25519_p1p1_to_p3(ge25519_p3 *r, const ge25519_p1p1 *p)`
pub fn ge25519_p1p1_to_p3(r: &mut ge25519_p3, p: &ge25519_p1p1) {
    fe25519_mul(&mut r.X, &p.X, &p.T);
    fe25519_mul(&mut r.Y, &p.Y, &p.Z);
    fe25519_mul(&mut r.Z, &p.Z, &p.T);
    fe25519_mul(&mut r.T, &p.X, &p.Y);
}

/// `void ge25519_p2_to_p3(ge25519_p3 *r, const ge25519_p2 *p)`
pub fn ge25519_p2_to_p3(r: &mut ge25519_p3, p: &ge25519_p2) {
    fe25519_copy(&mut r.X, &p.X);
    fe25519_copy(&mut r.Y, &p.Y);
    fe25519_copy(&mut r.Z, &p.Z);
    fe25519_mul(&mut r.T, &p.X, &p.Y);
}

/// `static void ge25519_p2_0(ge25519_p2 *h)`
pub fn ge25519_p2_0(h: &mut ge25519_p2) {
    fe25519_0(&mut h.X);
    fe25519_1(&mut h.Y);
    fe25519_1(&mut h.Z);
}

/// `static void ge25519_p2_dbl(ge25519_p1p1 *r, const ge25519_p2 *p)` (r = 2*p)
pub fn ge25519_p2_dbl(r: &mut ge25519_p1p1, p: &ge25519_p2) {
    let mut t0: fe25519 = [0i32; 10];

    fe25519_sq(&mut r.X, &p.X);
    fe25519_sq(&mut r.Z, &p.Y);
    fe25519_sq2(&mut r.T, &p.Z);
    fe25519_add(&mut r.Y, &p.X, &p.Y);
    fe25519_sq(&mut t0, &r.Y);
    fe25519_add(&mut r.Y, &r.Z, &r.X);
    {
        let rz = r.Z;
        fe25519_sub(&mut r.Z, &rz, &r.X);
    }
    fe25519_sub(&mut r.X, &t0, &r.Y);
    {
        let rt = r.T;
        fe25519_sub(&mut r.T, &rt, &r.Z);
    }
}

/// `static void ge25519_p3_0(ge25519_p3 *h)`
pub fn ge25519_p3_0(h: &mut ge25519_p3) {
    fe25519_0(&mut h.X);
    fe25519_1(&mut h.Y);
    fe25519_1(&mut h.Z);
    fe25519_0(&mut h.T);
}

/// `static void ge25519_cached_0(ge25519_cached *h)`
pub fn ge25519_cached_0(h: &mut ge25519_cached) {
    fe25519_1(&mut h.YplusX);
    fe25519_1(&mut h.YminusX);
    fe25519_1(&mut h.Z);
    fe25519_0(&mut h.T2d);
}

/// `static void ge25519_p3_to_cached(ge25519_cached *r, const ge25519_p3 *p)`
pub fn ge25519_p3_to_cached(r: &mut ge25519_cached, p: &ge25519_p3) {
    fe25519_add(&mut r.YplusX, &p.Y, &p.X);
    fe25519_sub(&mut r.YminusX, &p.Y, &p.X);
    fe25519_copy(&mut r.Z, &p.Z);
    fe25519_mul(&mut r.T2d, &p.T, &ed25519_d2);
}

/// `static void ge25519_p3_to_precomp(ge25519_precomp *pi, const ge25519_p3 *p)`
pub fn ge25519_p3_to_precomp(pi: &mut ge25519_precomp, p: &ge25519_p3) {
    let mut recip: fe25519 = [0i32; 10];
    let mut x: fe25519 = [0i32; 10];
    let mut y: fe25519 = [0i32; 10];
    let mut xy: fe25519 = [0i32; 10];

    fe25519_invert(&mut recip, &p.Z);
    fe25519_mul(&mut x, &p.X, &recip);
    fe25519_mul(&mut y, &p.Y, &recip);
    fe25519_add(&mut pi.yplusx, &y, &x);
    fe25519_sub(&mut pi.yminusx, &y, &x);
    fe25519_mul(&mut xy, &x, &y);
    fe25519_mul(&mut pi.xy2d, &xy, &ed25519_d2);
}

/// `static void ge25519_p3_to_p2(ge25519_p2 *r, const ge25519_p3 *p)`
pub fn ge25519_p3_to_p2(r: &mut ge25519_p2, p: &ge25519_p3) {
    fe25519_copy(&mut r.X, &p.X);
    fe25519_copy(&mut r.Y, &p.Y);
    fe25519_copy(&mut r.Z, &p.Z);
}

/// `void ge25519_p3_tobytes(unsigned char *s, const ge25519_p3 *h)`
pub unsafe fn ge25519_p3_tobytes(s: *mut u8, h: &ge25519_p3) {
    let mut recip: fe25519 = [0i32; 10];
    let mut x: fe25519 = [0i32; 10];
    let mut y: fe25519 = [0i32; 10];

    fe25519_invert(&mut recip, &h.Z);
    fe25519_mul(&mut x, &h.X, &recip);
    fe25519_mul(&mut y, &h.Y, &recip);
    fe25519_tobytes(s, &y);
    *s.add(31) ^= (fe25519_isnegative(&x) << 7) as u8;
}

/// `static void ge25519_p3_dbl(ge25519_p1p1 *r, const ge25519_p3 *p)` (r = 2*p)
pub fn ge25519_p3_dbl(r: &mut ge25519_p1p1, p: &ge25519_p3) {
    let mut q: ge25519_p2 = ge25519_p2::zero();
    ge25519_p3_to_p2(&mut q, p);
    ge25519_p2_dbl(r, &q);
}

/// `static void ge25519_precomp_0(ge25519_precomp *h)`
pub fn ge25519_precomp_0(h: &mut ge25519_precomp) {
    fe25519_1(&mut h.yplusx);
    fe25519_1(&mut h.yminusx);
    fe25519_0(&mut h.xy2d);
}

// ---------------------------------------------------------------------------
// constant-time helpers
// ---------------------------------------------------------------------------

/// `static unsigned char equal(signed char b, signed char c)`
///
/// (non-x86_64/aarch64 inline-asm fallback path, since no `HAVE_INLINE_ASM`
/// is defined for this build configuration.)
pub fn equal(b: i8, c: i8) -> u8 {
    let x: u8 = (b as u8) ^ (c as u8); /* 0: yes; 1..255: no */
    let mut y: u32 = x as u32; /* 0: yes; 1..255: no */

    y = y.wrapping_sub(1);
    (((y >> 29) as u8 ^ OPTBLOCKER_U8) >> 2) as u8 /* 1: yes; 0: no */
}

/// `static unsigned char negative(signed char b)`
pub fn negative(b: i8) -> u8 {
    let x: u8 = b as u8; /* 0..127: no 128..255: yes */
    ((x >> 5) ^ OPTBLOCKER_U8) >> 2 /* 1: yes; 0: no */
}

/// `static void ge25519_cmov(ge25519_precomp *t, const ge25519_precomp *u, unsigned char b)`
pub fn ge25519_cmov(t: &mut ge25519_precomp, u: &ge25519_precomp, b: u8) {
    fe25519_cmov(&mut t.yplusx, &u.yplusx, b as u32);
    fe25519_cmov(&mut t.yminusx, &u.yminusx, b as u32);
    fe25519_cmov(&mut t.xy2d, &u.xy2d, b as u32);
}

/// `static void ge25519_cmov_cached(ge25519_cached *t, const ge25519_cached *u, unsigned char b)`
pub fn ge25519_cmov_cached(t: &mut ge25519_cached, u: &ge25519_cached, b: u8) {
    fe25519_cmov(&mut t.YplusX, &u.YplusX, b as u32);
    fe25519_cmov(&mut t.YminusX, &u.YminusX, b as u32);
    fe25519_cmov(&mut t.Z, &u.Z, b as u32);
    fe25519_cmov(&mut t.T2d, &u.T2d, b as u32);
}

/// `static void ge25519_cmov8(ge25519_precomp *t, const ge25519_precomp precomp[8], const signed char b)`
pub fn ge25519_cmov8(t: &mut ge25519_precomp, precomp: &[ge25519_precomp; 8], b: i8) {
    let mut minust: ge25519_precomp = ge25519_precomp::zero();
    let bnegative: u8 = negative(b);
    // babs = b - (((-bnegative) & b) * (1 << 1)), all in `signed char` arithmetic.
    let neg_bnegative: i8 = (bnegative as i8).wrapping_neg();
    let masked: i8 = neg_bnegative & b;
    let babs: u8 = (b as i32).wrapping_sub((masked as i32).wrapping_mul(2)) as u8;

    ge25519_precomp_0(t);
    ge25519_cmov(t, &precomp[0], equal(babs as i8, 1));
    ge25519_cmov(t, &precomp[1], equal(babs as i8, 2));
    ge25519_cmov(t, &precomp[2], equal(babs as i8, 3));
    ge25519_cmov(t, &precomp[3], equal(babs as i8, 4));
    ge25519_cmov(t, &precomp[4], equal(babs as i8, 5));
    ge25519_cmov(t, &precomp[5], equal(babs as i8, 6));
    ge25519_cmov(t, &precomp[6], equal(babs as i8, 7));
    ge25519_cmov(t, &precomp[7], equal(babs as i8, 8));
    fe25519_copy(&mut minust.yplusx, &t.yminusx);
    fe25519_copy(&mut minust.yminusx, &t.yplusx);
    fe25519_neg(&mut minust.xy2d, &t.xy2d);
    ge25519_cmov(t, &minust, bnegative);
}

/// `static void ge25519_cmov8_base(ge25519_precomp *t, const int pos, const signed char b)`
pub fn ge25519_cmov8_base(t: &mut ge25519_precomp, pos: usize, b: i8) {
    ge25519_cmov8(t, &base[pos], b);
}

/// `static void ge25519_cmov8_cached(ge25519_cached *t, const ge25519_cached cached[8], const signed char b)`
pub fn ge25519_cmov8_cached(t: &mut ge25519_cached, cached: &[ge25519_cached; 8], b: i8) {
    let mut minust: ge25519_cached = ge25519_cached::zero();
    let bnegative: u8 = negative(b);
    let neg_bnegative: i8 = (bnegative as i8).wrapping_neg();
    let masked: i8 = neg_bnegative & b;
    let babs: u8 = (b as i32).wrapping_sub((masked as i32).wrapping_mul(2)) as u8;

    ge25519_cached_0(t);
    ge25519_cmov_cached(t, &cached[0], equal(babs as i8, 1));
    ge25519_cmov_cached(t, &cached[1], equal(babs as i8, 2));
    ge25519_cmov_cached(t, &cached[2], equal(babs as i8, 3));
    ge25519_cmov_cached(t, &cached[3], equal(babs as i8, 4));
    ge25519_cmov_cached(t, &cached[4], equal(babs as i8, 5));
    ge25519_cmov_cached(t, &cached[5], equal(babs as i8, 6));
    ge25519_cmov_cached(t, &cached[6], equal(babs as i8, 7));
    ge25519_cmov_cached(t, &cached[7], equal(babs as i8, 8));
    fe25519_copy(&mut minust.YplusX, &t.YminusX);
    fe25519_copy(&mut minust.YminusX, &t.YplusX);
    fe25519_copy(&mut minust.Z, &t.Z);
    fe25519_neg(&mut minust.T2d, &t.T2d);
    ge25519_cmov_cached(t, &minust, bnegative);
}

/// `static void ge25519_sub_cached(ge25519_p1p1 *r, const ge25519_p3 *p, const ge25519_cached *q)` (r = p - q)
pub fn ge25519_sub_cached(r: &mut ge25519_p1p1, p: &ge25519_p3, q: &ge25519_cached) {
    let mut t0: fe25519 = [0i32; 10];

    fe25519_add(&mut r.X, &p.Y, &p.X);
    fe25519_sub(&mut r.Y, &p.Y, &p.X);
    fe25519_mul(&mut r.Z, &r.X, &q.YminusX);
    {
        let ry = r.Y;
        fe25519_mul(&mut r.Y, &ry, &q.YplusX);
    }
    fe25519_mul(&mut r.T, &q.T2d, &p.T);
    fe25519_mul(&mut r.X, &p.Z, &q.Z);
    fe25519_add(&mut t0, &r.X, &r.X);
    fe25519_sub(&mut r.X, &r.Z, &r.Y);
    {
        let ry = r.Y;
        fe25519_add(&mut r.Y, &r.Z, &ry);
    }
    fe25519_sub(&mut r.Z, &t0, &r.T);
    {
        let rt = r.T;
        fe25519_add(&mut r.T, &t0, &rt);
    }
}

/// `void ge25519_tobytes(unsigned char *s, const ge25519_p2 *h)`
pub unsafe fn ge25519_tobytes(s: *mut u8, h: &ge25519_p2) {
    let mut recip: fe25519 = [0i32; 10];
    let mut x: fe25519 = [0i32; 10];
    let mut y: fe25519 = [0i32; 10];

    fe25519_invert(&mut recip, &h.Z);
    fe25519_mul(&mut x, &h.X, &recip);
    fe25519_mul(&mut y, &h.Y, &recip);
    fe25519_tobytes(s, &y);
    *s.add(31) ^= (fe25519_isnegative(&x) << 7) as u8;
}

// ---------------------------------------------------------------------------
// r = a*A + b*B (double scalar multiplication, vartime)
// ---------------------------------------------------------------------------

/// `void ge25519_double_scalarmult_vartime(ge25519_p2 *r, const unsigned char *a, const ge25519_p3 *A, const unsigned char *b)`
pub unsafe fn ge25519_double_scalarmult_vartime(
    r: &mut ge25519_p2,
    a: *const u8,
    aa: &ge25519_p3,
    b: *const u8,
) {
    let mut aslide: [i8; 256] = [0; 256];
    let mut bslide: [i8; 256] = [0; 256];
    let mut ai: [ge25519_cached; 8] = [ge25519_cached::zero(); 8]; /* A,3A,5A,7A,9A,11A,13A,15A */
    let mut t: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut u: ge25519_p3 = ge25519_p3::zero();
    let mut a2: ge25519_p3 = ge25519_p3::zero();

    let mut a_bytes = [0u8; 32];
    let mut b_bytes = [0u8; 32];
    for i in 0..32 {
        a_bytes[i] = *a.add(i);
        b_bytes[i] = *b.add(i);
    }

    slide_vartime(&mut aslide, &a_bytes);
    slide_vartime(&mut bslide, &b_bytes);

    ge25519_p3_to_cached(&mut ai[0], aa);

    ge25519_p3_dbl(&mut t, aa);
    ge25519_p1p1_to_p3(&mut a2, &t);

    ge25519_add_cached(&mut t, &a2, &ai[0]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut ai[1], &u);

    ge25519_add_cached(&mut t, &a2, &ai[1]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut ai[2], &u);

    ge25519_add_cached(&mut t, &a2, &ai[2]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut ai[3], &u);

    ge25519_add_cached(&mut t, &a2, &ai[3]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut ai[4], &u);

    ge25519_add_cached(&mut t, &a2, &ai[4]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut ai[5], &u);

    ge25519_add_cached(&mut t, &a2, &ai[5]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut ai[6], &u);

    ge25519_add_cached(&mut t, &a2, &ai[6]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut ai[7], &u);

    ge25519_p2_0(r);

    let mut i: i32 = 255;
    while i >= 0 {
        if aslide[i as usize] != 0 || bslide[i as usize] != 0 {
            break;
        }
        i -= 1;
    }

    while i >= 0 {
        let iu = i as usize;
        ge25519_p2_dbl(&mut t, r);

        if aslide[iu] > 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_add_cached(&mut t, &u, &ai[(aslide[iu] / 2) as usize]);
        } else if aslide[iu] < 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_sub_cached(&mut t, &u, &ai[((-aslide[iu]) / 2) as usize]);
        }

        if bslide[iu] > 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_add_precomp(&mut t, &u, &base2[(bslide[iu] / 2) as usize]);
        } else if bslide[iu] < 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_sub_precomp(&mut t, &u, &base2[((-bslide[iu]) / 2) as usize]);
        }

        ge25519_p1p1_to_p2(r, &t);
        i -= 1;
    }
}

// ---------------------------------------------------------------------------
// h = a * p
// ---------------------------------------------------------------------------

/// `void ge25519_scalarmult(ge25519_p3 *h, const unsigned char *a, const ge25519_p3 *p)`
pub unsafe fn ge25519_scalarmult(h: &mut ge25519_p3, a: *const u8, p: &ge25519_p3) {
    let mut e: [i8; 64] = [0; 64];
    let mut carry: i8;
    let mut r: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut s: ge25519_p2 = ge25519_p2::zero();
    let mut t2: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut t3: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut t4: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut t5: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut t6: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut t7: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut t8: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut p2: ge25519_p3 = ge25519_p3::zero();
    let mut p3: ge25519_p3 = ge25519_p3::zero();
    let mut p4: ge25519_p3 = ge25519_p3::zero();
    let mut p5: ge25519_p3 = ge25519_p3::zero();
    let mut p6: ge25519_p3 = ge25519_p3::zero();
    let mut p7: ge25519_p3 = ge25519_p3::zero();
    let mut p8: ge25519_p3 = ge25519_p3::zero();
    let mut pi: [ge25519_cached; 8] = [ge25519_cached::zero(); 8];
    let mut t: ge25519_cached = ge25519_cached::zero();

    ge25519_p3_to_cached(&mut pi[1 - 1], p); /* p */

    ge25519_p3_dbl(&mut t2, p);
    ge25519_p1p1_to_p3(&mut p2, &t2);
    ge25519_p3_to_cached(&mut pi[2 - 1], &p2); /* 2p = 2*p */

    ge25519_add_cached(&mut t3, p, &pi[2 - 1]);
    ge25519_p1p1_to_p3(&mut p3, &t3);
    ge25519_p3_to_cached(&mut pi[3 - 1], &p3); /* 3p = 2p+p */

    ge25519_p3_dbl(&mut t4, &p2);
    ge25519_p1p1_to_p3(&mut p4, &t4);
    ge25519_p3_to_cached(&mut pi[4 - 1], &p4); /* 4p = 2*2p */

    ge25519_add_cached(&mut t5, p, &pi[4 - 1]);
    ge25519_p1p1_to_p3(&mut p5, &t5);
    ge25519_p3_to_cached(&mut pi[5 - 1], &p5); /* 5p = 4p+p */

    ge25519_p3_dbl(&mut t6, &p3);
    ge25519_p1p1_to_p3(&mut p6, &t6);
    ge25519_p3_to_cached(&mut pi[6 - 1], &p6); /* 6p = 2*3p */

    ge25519_add_cached(&mut t7, p, &pi[6 - 1]);
    ge25519_p1p1_to_p3(&mut p7, &t7);
    ge25519_p3_to_cached(&mut pi[7 - 1], &p7); /* 7p = 6p+p */

    ge25519_p3_dbl(&mut t8, &p4);
    ge25519_p1p1_to_p3(&mut p8, &t8);
    ge25519_p3_to_cached(&mut pi[8 - 1], &p8); /* 8p = 2*4p */

    for i in 0..32usize {
        let ai = *a.add(i);
        e[2 * i] = ((ai >> 0) & 15) as i8;
        e[2 * i + 1] = ((ai >> 4) & 15) as i8;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    for i in 0..63usize {
        e[i] = e[i].wrapping_add(carry);
        carry = (e[i] as i32).wrapping_add(8) as i8;
        carry >>= 4;
        e[i] = (e[i] as i32).wrapping_sub((carry as i32).wrapping_mul(1i32 << 4)) as i8;
    }
    e[63] = e[63].wrapping_add(carry);
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    let mut i: i32 = 63;
    while i != 0 {
        let iu = i as usize;
        ge25519_cmov8_cached(&mut t, &pi, e[iu]);
        ge25519_add_cached(&mut r, h, &t);

        ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);
        ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);
        ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);
        ge25519_p1p1_to_p2(&mut s, &r);
        ge25519_p2_dbl(&mut r, &s);

        ge25519_p1p1_to_p3(h, &r); /* *16 */
        i -= 1;
    }
    ge25519_cmov8_cached(&mut t, &pi, e[i as usize]);
    ge25519_add_cached(&mut r, h, &t);

    ge25519_p1p1_to_p3(h, &r);
}

// ---------------------------------------------------------------------------
// h = a * B (base-point scalar multiplication, with precomputation)
// ---------------------------------------------------------------------------

/// `void ge25519_scalarmult_base(ge25519_p3 *h, const unsigned char *a)`
pub unsafe fn ge25519_scalarmult_base(h: &mut ge25519_p3, a: *const u8) {
    let mut e: [i8; 64] = [0; 64];
    let mut carry: i8;
    let mut r: ge25519_p1p1 = ge25519_p1p1::zero();
    let mut s: ge25519_p2 = ge25519_p2::zero();
    let mut t: ge25519_precomp = ge25519_precomp::zero();

    for i in 0..32usize {
        let ai = *a.add(i);
        e[2 * i] = ((ai >> 0) & 15) as i8;
        e[2 * i + 1] = ((ai >> 4) & 15) as i8;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    for i in 0..63usize {
        e[i] = e[i].wrapping_add(carry);
        carry = (e[i] as i32).wrapping_add(8) as i8;
        carry >>= 4;
        e[i] = (e[i] as i32).wrapping_sub((carry as i32).wrapping_mul(1i32 << 4)) as i8;
    }
    e[63] = e[63].wrapping_add(carry);
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    let mut i: usize = 1;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i]);
        ge25519_add_precomp(&mut r, h, &t);
        ge25519_p1p1_to_p3(h, &r);
        i += 2;
    }

    ge25519_p3_dbl(&mut r, h);
    ge25519_p1p1_to_p2(&mut s, &r);
    ge25519_p2_dbl(&mut r, &s);
    ge25519_p1p1_to_p2(&mut s, &r);
    ge25519_p2_dbl(&mut r, &s);
    ge25519_p1p1_to_p2(&mut s, &r);
    ge25519_p2_dbl(&mut r, &s);
    ge25519_p1p1_to_p3(h, &r);

    let mut i: usize = 0;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i]);
        ge25519_add_precomp(&mut r, h, &t);
        ge25519_p1p1_to_p3(h, &r);
        i += 2;
    }
}

// ---------------------------------------------------------------------------
// ge25519_p3 group law helpers (used by ge25519_mul_l / subgroup checks)
// ---------------------------------------------------------------------------

/// `static void ge25519_p3p3_dbl(ge25519_p3 *r, const ge25519_p3 *p)` (r = 2p)
pub fn ge25519_p3p3_dbl(r: &mut ge25519_p3, p: &ge25519_p3) {
    let mut p1p1: ge25519_p1p1 = ge25519_p1p1::zero();

    ge25519_p3_dbl(&mut p1p1, p);
    ge25519_p1p1_to_p3(r, &p1p1);
}

/// `static void ge25519_p3_neg(ge25519_p3 *r, const ge25519_p3 *p)` (r = -p)
pub fn ge25519_p3_neg(r: &mut ge25519_p3, p: &ge25519_p3) {
    fe25519_neg(&mut r.X, &p.X);
    fe25519_copy(&mut r.Y, &p.Y);
    fe25519_copy(&mut r.Z, &p.Z);
    fe25519_neg(&mut r.T, &p.T);
}

/// `void ge25519_p3_add(ge25519_p3 *r, const ge25519_p3 *p, const ge25519_p3 *q)` (r = p+q)
pub fn ge25519_p3_add(r: &mut ge25519_p3, p: &ge25519_p3, q: &ge25519_p3) {
    let mut q_cached: ge25519_cached = ge25519_cached::zero();
    let mut p1p1: ge25519_p1p1 = ge25519_p1p1::zero();

    ge25519_p3_to_cached(&mut q_cached, q);
    ge25519_add_cached(&mut p1p1, p, &q_cached);
    ge25519_p1p1_to_p3(r, &p1p1);
}

/// `void ge25519_p3_sub(ge25519_p3 *r, const ge25519_p3 *p, const ge25519_p3 *q)` (r = p-q)
pub fn ge25519_p3_sub(r: &mut ge25519_p3, p: &ge25519_p3, q: &ge25519_p3) {
    let mut q_neg: ge25519_p3 = ge25519_p3::zero();

    ge25519_p3_neg(&mut q_neg, q);
    ge25519_p3_add(r, p, &q_neg);
}

/// `static void ge25519_p3_dbladd(ge25519_p3 *r, const int n, const ge25519_p3 *q)` (r = r*(2^n)+q)
pub fn ge25519_p3_dbladd(r: &mut ge25519_p3, n: c_int, q: &ge25519_p3) {
    let mut p2: ge25519_p2 = ge25519_p2::zero();
    let mut p1p1: ge25519_p1p1 = ge25519_p1p1::zero();

    ge25519_p3_to_p2(&mut p2, r);
    let mut i: c_int = 0;
    while i < n {
        ge25519_p2_dbl(&mut p1p1, &p2);
        ge25519_p1p1_to_p2(&mut p2, &p1p1);
        i += 1;
    }
    ge25519_p1p1_to_p3(r, &p1p1);
    let rcopy = *r;
    ge25519_p3_add(r, &rcopy, q);
}

/// multiply by the order of the main subgroup
/// l = 2^252+27742317777372353535851937790883648493
/// `static void ge25519_mul_l(ge25519_p3 *r, const ge25519_p3 *p)`
pub fn ge25519_mul_l(r: &mut ge25519_p3, p: &ge25519_p3) {
    let mut _10: ge25519_p3 = ge25519_p3::zero();
    let mut _11: ge25519_p3 = ge25519_p3::zero();
    let mut _100: ge25519_p3 = ge25519_p3::zero();
    let mut _110: ge25519_p3 = ge25519_p3::zero();
    let mut _1000: ge25519_p3 = ge25519_p3::zero();
    let mut _1011: ge25519_p3 = ge25519_p3::zero();
    let mut _10000: ge25519_p3 = ge25519_p3::zero();
    let mut _100000: ge25519_p3 = ge25519_p3::zero();
    let mut _100110: ge25519_p3 = ge25519_p3::zero();
    let mut _1000000: ge25519_p3 = ge25519_p3::zero();
    let mut _1010000: ge25519_p3 = ge25519_p3::zero();
    let mut _1010011: ge25519_p3 = ge25519_p3::zero();
    let mut _1100011: ge25519_p3 = ge25519_p3::zero();
    let mut _1100111: ge25519_p3 = ge25519_p3::zero();
    let mut _1101011: ge25519_p3 = ge25519_p3::zero();
    let mut _10010011: ge25519_p3 = ge25519_p3::zero();
    let mut _10010111: ge25519_p3 = ge25519_p3::zero();
    let mut _10111101: ge25519_p3 = ge25519_p3::zero();
    let mut _11010011: ge25519_p3 = ge25519_p3::zero();
    let mut _11100111: ge25519_p3 = ge25519_p3::zero();
    let mut _11101101: ge25519_p3 = ge25519_p3::zero();
    let mut _11110101: ge25519_p3 = ge25519_p3::zero();

    ge25519_p3p3_dbl(&mut _10, p);
    ge25519_p3_add(&mut _11, p, &_10);
    ge25519_p3_add(&mut _100, p, &_11);
    ge25519_p3_add(&mut _110, &_10, &_100);
    ge25519_p3_add(&mut _1000, &_10, &_110);
    ge25519_p3_add(&mut _1011, &_11, &_1000);
    ge25519_p3p3_dbl(&mut _10000, &_1000);
    ge25519_p3p3_dbl(&mut _100000, &_10000);
    ge25519_p3_add(&mut _100110, &_110, &_100000);
    ge25519_p3p3_dbl(&mut _1000000, &_100000);
    ge25519_p3_add(&mut _1010000, &_10000, &_1000000);
    ge25519_p3_add(&mut _1010011, &_11, &_1010000);
    ge25519_p3_add(&mut _1100011, &_10000, &_1010011);
    ge25519_p3_add(&mut _1100111, &_100, &_1100011);
    ge25519_p3_add(&mut _1101011, &_100, &_1100111);
    ge25519_p3_add(&mut _10010011, &_1000000, &_1010011);
    ge25519_p3_add(&mut _10010111, &_100, &_10010011);
    ge25519_p3_add(&mut _10111101, &_100110, &_10010111);
    ge25519_p3_add(&mut _11010011, &_1000000, &_10010011);
    ge25519_p3_add(&mut _11100111, &_1010000, &_10010111);
    ge25519_p3_add(&mut _11101101, &_110, &_11100111);
    ge25519_p3_add(&mut _11110101, &_1000, &_11101101);

    ge25519_p3_add(r, &_1011, &_11110101);
    ge25519_p3_dbladd(r, 126, &_1010011);
    ge25519_p3_dbladd(r, 9, &_10);
    let rcopy = *r;
    ge25519_p3_add(r, &rcopy, &_11110101);
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

// ---------------------------------------------------------------------------
// checks
// ---------------------------------------------------------------------------

/// `int ge25519_is_on_curve(const ge25519_p3 *p)`
pub fn ge25519_is_on_curve(p: &ge25519_p3) -> c_int {
    let mut x2: fe25519 = [0i32; 10];
    let mut y2: fe25519 = [0i32; 10];
    let mut z2: fe25519 = [0i32; 10];
    let mut z4: fe25519 = [0i32; 10];
    let mut t0: fe25519 = [0i32; 10];
    let mut t1: fe25519 = [0i32; 10];

    fe25519_sq(&mut x2, &p.X);
    fe25519_sq(&mut y2, &p.Y);
    fe25519_sq(&mut z2, &p.Z);
    fe25519_sub(&mut t0, &y2, &x2);
    {
        let t0v = t0;
        fe25519_mul(&mut t0, &t0v, &z2);
    }

    fe25519_mul(&mut t1, &x2, &y2);
    {
        let t1v = t1;
        fe25519_mul(&mut t1, &t1v, &ed25519_d);
    }
    fe25519_sq(&mut z4, &z2);
    {
        let t1v = t1;
        fe25519_add(&mut t1, &t1v, &z4);
    }
    {
        let t0v = t0;
        fe25519_sub(&mut t0, &t0v, &t1);
    }

    fe25519_iszero(&t0)
}

/// `int ge25519_is_on_main_subgroup(const ge25519_p3 *p)`
pub fn ge25519_is_on_main_subgroup(p: &ge25519_p3) -> c_int {
    let mut pl: ge25519_p3 = ge25519_p3::zero();
    let mut t: fe25519 = [0i32; 10];

    ge25519_mul_l(&mut pl, p);

    fe25519_sub(&mut t, &pl.Y, &pl.Z);

    fe25519_iszero(&pl.X) & fe25519_iszero(&t)
}

/// `int ge25519_is_canonical(const unsigned char *s)`
pub unsafe fn ge25519_is_canonical(s: *const u8) -> c_int {
    let mut c: u32 = ((*s.add(31) & 0x7f) ^ 0x7f) as u32;
    let mut i: i32 = 30;
    while i > 0 {
        c |= (*s.add(i as usize) ^ 0xff) as u32;
        i -= 1;
    }
    c = c.wrapping_sub(1) >> 8;
    let d: u32 = (0xedu32.wrapping_sub(1).wrapping_sub(*s as u32)) >> 8;

    (1u32.wrapping_sub(c & d & 1)) as c_int
}

/// `int ge25519_has_small_order(const ge25519_p3 *p)`
pub fn ge25519_has_small_order(p: &ge25519_p3) -> c_int {
    let mut y_sqrtm1: fe25519 = [0i32; 10];
    let mut c: fe25519 = [0i32; 10];
    let mut ret: c_int = 0;

    ret |= fe25519_iszero(&p.X);
    ret |= fe25519_iszero(&p.Y);
    ret |= fe25519_iszero(&p.Z);
    fe25519_mul(&mut y_sqrtm1, &p.Y, &fe25519_sqrtm1);
    fe25519_sub(&mut c, &y_sqrtm1, &p.X);
    ret |= fe25519_iszero(&c);
    fe25519_add(&mut c, &y_sqrtm1, &p.X);
    ret |= fe25519_iszero(&c);

    ret
}

// ---------------------------------------------------------------------------
// Exported C symbols (see $W/_cbuild/persym.txt for
// crypto_core/ed25519/ref10/ed25519_ref10.c.o); only symbols whose defining
// function lies within lines 263..1200 of ed25519_ref10.c are provided here.
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int {
    ge25519_frombytes(&mut *h, s)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_frombytes_negate_vartime(
    h: *mut ge25519_p3,
    s: *const u8,
) -> c_int {
    ge25519_frombytes_negate_vartime(&mut *h, s)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p2(r: *mut ge25519_p2, p: *const ge25519_p1p1) {
    ge25519_p1p1_to_p2(&mut *r, &*p);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p3(r: *mut ge25519_p3, p: *const ge25519_p1p1) {
    ge25519_p1p1_to_p3(&mut *r, &*p);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_p2_to_p3(r: *mut ge25519_p3, p: *const ge25519_p2) {
    ge25519_p2_to_p3(&mut *r, &*p);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3) {
    ge25519_p3_tobytes(s, &*h);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_tobytes(s: *mut u8, h: *const ge25519_p2) {
    ge25519_tobytes(s, &*h);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_double_scalarmult_vartime(
    r: *mut ge25519_p2,
    a: *const u8,
    aa: *const ge25519_p3,
    b: *const u8,
) {
    ge25519_double_scalarmult_vartime(&mut *r, a, &*aa, b);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult(
    h: *mut ge25519_p3,
    a: *const u8,
    p: *const ge25519_p3,
) {
    ge25519_scalarmult(&mut *h, a, &*p);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8) {
    ge25519_scalarmult_base(&mut *h, a);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_p3_add(
    r: *mut ge25519_p3,
    p: *const ge25519_p3,
    q: *const ge25519_p3,
) {
    ge25519_p3_add(&mut *r, &*p, &*q);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_p3_sub(
    r: *mut ge25519_p3,
    p: *const ge25519_p3,
    q: *const ge25519_p3,
) {
    ge25519_p3_sub(&mut *r, &*p, &*q);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_is_on_curve(p: *const ge25519_p3) -> c_int {
    ge25519_is_on_curve(&*p)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int {
    ge25519_is_on_main_subgroup(&*p)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int {
    ge25519_is_canonical(s)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> c_int {
    ge25519_has_small_order(&*p)
}
