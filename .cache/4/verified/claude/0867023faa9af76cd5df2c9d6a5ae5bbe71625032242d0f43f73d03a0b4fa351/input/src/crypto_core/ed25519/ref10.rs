//! `crypto_core/ed25519/ref10/ed25519_ref10.c`, lines 262..2992.
//!
//! `HAVE_TI_MODE` is **undefined** in the reference build, so `fe25519` is
//! `int32_t[10]` and only the `fe_25_5` tables are used.  `HAVE_INLINE_ASM` is
//! undefined too, so `equal()` / `negative()` take the portable
//! `optblocker_u8` branch.
//!
//! What lives elsewhere:
//!   * `load_3`, `load_4` and every `fe25519_*` helper (lines 1..260) are in
//!     [`crate::crypto_core::ed25519::fe25519`];
//!   * the constant tables (`base[32][8]`, `base2[8]`, `ed25519_d`, ...) are in
//!     [`crate::crypto_core::ed25519::ref10_tables`];
//!   * the three big scalar kernels `sc25519_mul` (line 1202),
//!     `sc25519_muladd` (line 1675) and `sc25519_reduce` (line 2250) are in
//!     [`crate::crypto_core::ed25519::ref10_sc`] (this file only re-exports
//!     them with C linkage).
//!
//! # Calling convention
//!
//! As in `fe25519.rs`, every function that has a dedicated output parameter
//! takes its inputs **by value** (all of `Fe25519`, `Ge25519P2`, `Ge25519P3`,
//! `Ge25519P1p1`, `Ge25519Precomp` and `Ge25519Cached` are `Copy`).  That makes
//! the aliasing the C code relies on (`fe25519_mul(x, x, y)`,
//! `ge25519_p3_add(r, r, q)`, `ge25519_scalarmult(h, a, h)`, ...) expressible
//! in safe Rust while staying bit-for-bit identical: none of the C functions
//! ever reads its output object.
//!
//! Every C truncation / wrap-around / integer promotion is spelled out.

use core::ffi::c_int;
use core::ptr;

use crate::crypto_core::ed25519::fe25519::*;
use crate::crypto_core::ed25519::ref10_sc::{sc25519_mul, sc25519_muladd, sc25519_reduce};
use crate::crypto_core::ed25519::ref10_tables::*;
use crate::crypto_core::ed25519::types::{
    Fe25519, Ge25519Cached, Ge25519P1p1, Ge25519P2, Ge25519P3, Ge25519Precomp,
};

/* ------------------------------------------------------------------ */
/* value-returning `fe25519` wrappers                                  */
/*                                                                     */
/* None of the C `fe25519_*` primitives reads `h` before writing it, so */
/* returning a fresh value is exactly equivalent to the C in/out form.  */
/* ------------------------------------------------------------------ */

#[inline(always)]
fn f_add(f: Fe25519, g: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_add(&mut h, f, g);
    h
}

#[inline(always)]
fn f_sub(f: Fe25519, g: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_sub(&mut h, f, g);
    h
}

#[inline(always)]
fn f_neg(f: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_neg(&mut h, f);
    h
}

#[inline(always)]
fn f_mul(f: Fe25519, g: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_mul(&mut h, f, g);
    h
}

#[inline(always)]
fn f_sq(f: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_sq(&mut h, f);
    h
}

#[inline(always)]
fn f_sq2(f: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_sq2(&mut h, f);
    h
}

#[inline(always)]
fn f_mul32(f: Fe25519, n: u32) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_mul32(&mut h, f, n);
    h
}

#[inline(always)]
fn f_invert(z: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_invert(&mut h, &z);
    h
}

#[inline(always)]
fn f_pow22523(z: Fe25519) -> Fe25519 {
    let mut h = Fe25519::ZERO;
    fe25519_pow22523(&mut h, z);
    h
}

/* ------------------------------------------------------------------ */
/* ge25519_add_cached (line 262)                                       */
/* ------------------------------------------------------------------ */

/// `r = p + q`
fn ge25519_add_cached(r: &mut Ge25519P1p1, p: Ge25519P3, q: Ge25519Cached) {
    let t0: Fe25519;

    r.X = f_add(p.Y, p.X);
    r.Y = f_sub(p.Y, p.X);
    r.Z = f_mul(r.X, q.YplusX);
    r.Y = f_mul(r.Y, q.YminusX);
    r.T = f_mul(q.T2d, p.T);
    r.X = f_mul(p.Z, q.Z);
    t0 = f_add(r.X, r.X);
    r.X = f_sub(r.Z, r.Y);
    r.Y = f_add(r.Z, r.Y);
    r.Z = f_add(t0, r.T);
    r.T = f_sub(t0, r.T);
}

/* ------------------------------------------------------------------ */
/* slide_vartime (line 280)                                            */
/* ------------------------------------------------------------------ */

fn slide_vartime(r: &mut [i8; 256], a: &[u8]) {
    let mut i: usize;
    let mut b: usize;
    let mut k: usize;
    let mut ribs: i32;
    let mut cmp: i32;

    i = 0;
    while i < 256 {
        r[i] = ((a[i >> 3] >> (i & 7)) & 1) as i8;
        i += 1;
    }
    i = 0;
    while i < 256 {
        if r[i] == 0 {
            i += 1;
            continue;
        }
        b = 1;
        while b <= 6 && i + b < 256 {
            if r[i + b] == 0 {
                b += 1;
                continue;
            }
            ribs = (r[i + b] as i32).wrapping_shl(b as u32);
            cmp = (r[i] as i32).wrapping_add(ribs);
            if cmp <= 15 {
                r[i] = cmp as i8;
                r[i + b] = 0;
            } else {
                cmp = (r[i] as i32).wrapping_sub(ribs);
                if cmp < -15 {
                    break;
                }
                r[i] = cmp as i8;
                k = i + b;
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
        i += 1;
    }
}

/* ------------------------------------------------------------------ */
/* optblocker_u8 (line 323)                                            */
/* ------------------------------------------------------------------ */

/// C: `static volatile unsigned char optblocker_u8;`
static mut OPTBLOCKER_U8: u8 = 0;

#[inline]
fn optblocker_u8() -> u8 {
    unsafe { ptr::read_volatile(&raw const OPTBLOCKER_U8) }
}

/* ------------------------------------------------------------------ */
/* ge25519_frombytes (line 325)                                        */
/* ------------------------------------------------------------------ */

pub fn ge25519_frombytes(h: &mut Ge25519P3, s: &[u8]) -> i32 {
    let mut u: Fe25519;
    let mut v: Fe25519;
    let mut vxx: Fe25519;
    let m_root_check: Fe25519;
    let p_root_check: Fe25519;
    let negx: Fe25519;
    let x_sqrtm1: Fe25519;
    let has_m_root: i32;
    let has_p_root: i32;

    fe25519_frombytes(&mut h.Y, s);
    fe25519_1(&mut h.Z);
    u = f_sq(h.Y);
    v = f_mul(u, ED25519_D);
    u = f_sub(u, h.Z); /* u = y^2-1 */
    v = f_add(v, h.Z); /* v = dy^2+1 */

    h.X = f_mul(u, v);
    h.X = f_pow22523(h.X);
    h.X = f_mul(u, h.X); /* u((uv)^((q-5)/8)) */

    vxx = f_sq(h.X);
    vxx = f_mul(vxx, v);
    m_root_check = f_sub(vxx, u); /* vx^2-u */
    p_root_check = f_add(vxx, u); /* vx^2+u */
    has_m_root = fe25519_iszero(m_root_check);
    has_p_root = fe25519_iszero(p_root_check);
    x_sqrtm1 = f_mul(h.X, FE25519_SQRTM1); /* x*sqrt(-1) */
    fe25519_cmov(&mut h.X, x_sqrtm1, (1 - has_m_root) as u32);

    negx = f_neg(h.X);
    let b = fe25519_isnegative(h.X)
        ^ ((((s[31] as i32) >> 5) ^ (optblocker_u8() as i32)) >> 2);
    fe25519_cmov(&mut h.X, negx, b as u32);
    h.T = f_mul(h.X, h.Y);

    (has_m_root | has_p_root) - 1
}

/* ------------------------------------------------------------------ */
/* ge25519_frombytes_negate_vartime (line 363)                         */
/* ------------------------------------------------------------------ */

pub fn ge25519_frombytes_negate_vartime(h: &mut Ge25519P3, s: &[u8]) -> i32 {
    let mut u: Fe25519;
    let mut v: Fe25519;
    let mut v3: Fe25519;
    let mut vxx: Fe25519;
    let m_root_check: Fe25519;
    let p_root_check: Fe25519;

    fe25519_frombytes(&mut h.Y, s);
    fe25519_1(&mut h.Z);
    u = f_sq(h.Y);
    v = f_mul(u, ED25519_D);
    u = f_sub(u, h.Z); /* u = y^2-1 */
    v = f_add(v, h.Z); /* v = dy^2+1 */

    v3 = f_sq(v);
    v3 = f_mul(v3, v); /* v3 = v^3 */
    h.X = f_sq(v3);
    h.X = f_mul(h.X, v);
    h.X = f_mul(h.X, u); /* x = uv^7 */

    h.X = f_pow22523(h.X); /* x = (uv^7)^((q-5)/8) */
    h.X = f_mul(h.X, v3);
    h.X = f_mul(h.X, u); /* x = uv^3(uv^7)^((q-5)/8) */

    vxx = f_sq(h.X);
    vxx = f_mul(vxx, v);
    m_root_check = f_sub(vxx, u); /* vx^2-u */
    if fe25519_iszero(m_root_check) == 0 {
        p_root_check = f_add(vxx, u); /* vx^2+u */
        if fe25519_iszero(p_root_check) == 0 {
            return -1;
        }
        h.X = f_mul(h.X, FE25519_SQRTM1);
    }

    if fe25519_isnegative(h.X) == ((s[31] as i32) >> 7) {
        h.X = f_neg(h.X);
    }
    h.T = f_mul(h.X, h.Y);

    0
}

/* ------------------------------------------------------------------ */
/* ge25519_add_precomp / ge25519_sub_precomp (lines 412, 433)           */
/* ------------------------------------------------------------------ */

/// `r = p + q`
fn ge25519_add_precomp(r: &mut Ge25519P1p1, p: Ge25519P3, q: Ge25519Precomp) {
    let t0: Fe25519;

    r.X = f_add(p.Y, p.X);
    r.Y = f_sub(p.Y, p.X);
    r.Z = f_mul(r.X, q.yplusx);
    r.Y = f_mul(r.Y, q.yminusx);
    r.T = f_mul(q.xy2d, p.T);
    t0 = f_add(p.Z, p.Z);
    r.X = f_sub(r.Z, r.Y);
    r.Y = f_add(r.Z, r.Y);
    r.Z = f_add(t0, r.T);
    r.T = f_sub(t0, r.T);
}

/// `r = p - q`
fn ge25519_sub_precomp(r: &mut Ge25519P1p1, p: Ge25519P3, q: Ge25519Precomp) {
    let t0: Fe25519;

    r.X = f_add(p.Y, p.X);
    r.Y = f_sub(p.Y, p.X);
    r.Z = f_mul(r.X, q.yminusx);
    r.Y = f_mul(r.Y, q.yplusx);
    r.T = f_mul(q.xy2d, p.T);
    t0 = f_add(p.Z, p.Z);
    r.X = f_sub(r.Z, r.Y);
    r.Y = f_add(r.Z, r.Y);
    r.Z = f_sub(t0, r.T);
    r.T = f_add(t0, r.T);
}

/* ------------------------------------------------------------------ */
/* conversions (lines 454..607)                                        */
/* ------------------------------------------------------------------ */

/// `r = p`
pub fn ge25519_p1p1_to_p2(r: &mut Ge25519P2, p: Ge25519P1p1) {
    r.X = f_mul(p.X, p.T);
    r.Y = f_mul(p.Y, p.Z);
    r.Z = f_mul(p.Z, p.T);
}

/// `r = p`
pub fn ge25519_p1p1_to_p3(r: &mut Ge25519P3, p: Ge25519P1p1) {
    r.X = f_mul(p.X, p.T);
    r.Y = f_mul(p.Y, p.Z);
    r.Z = f_mul(p.Z, p.T);
    r.T = f_mul(p.X, p.Y);
}

/// `r = p`
pub fn ge25519_p2_to_p3(r: &mut Ge25519P3, p: Ge25519P2) {
    r.X = p.X;
    r.Y = p.Y;
    r.Z = p.Z;
    r.T = f_mul(p.X, p.Y);
}

fn ge25519_p2_0(h: &mut Ge25519P2) {
    fe25519_0(&mut h.X);
    fe25519_1(&mut h.Y);
    fe25519_1(&mut h.Z);
}

/// `r = 2 * p`
fn ge25519_p2_dbl(r: &mut Ge25519P1p1, p: Ge25519P2) {
    let t0: Fe25519;

    r.X = f_sq(p.X);
    r.Z = f_sq(p.Y);
    r.T = f_sq2(p.Z);
    r.Y = f_add(p.X, p.Y);
    t0 = f_sq(r.Y);
    r.Y = f_add(r.Z, r.X);
    r.Z = f_sub(r.Z, r.X);
    r.X = f_sub(t0, r.Y);
    r.T = f_sub(r.T, r.Z);
}

fn ge25519_p3_0(h: &mut Ge25519P3) {
    fe25519_0(&mut h.X);
    fe25519_1(&mut h.Y);
    fe25519_1(&mut h.Z);
    fe25519_0(&mut h.T);
}

fn ge25519_cached_0(h: &mut Ge25519Cached) {
    fe25519_1(&mut h.YplusX);
    fe25519_1(&mut h.YminusX);
    fe25519_1(&mut h.Z);
    fe25519_0(&mut h.T2d);
}

/// `r = p`
fn ge25519_p3_to_cached(r: &mut Ge25519Cached, p: Ge25519P3) {
    r.YplusX = f_add(p.Y, p.X);
    r.YminusX = f_sub(p.Y, p.X);
    r.Z = p.Z;
    r.T2d = f_mul(p.T, ED25519_D2);
}

fn ge25519_p3_to_precomp(pi: &mut Ge25519Precomp, p: Ge25519P3) {
    let recip: Fe25519;
    let x: Fe25519;
    let y: Fe25519;
    let xy: Fe25519;

    recip = f_invert(p.Z);
    x = f_mul(p.X, recip);
    y = f_mul(p.Y, recip);
    pi.yplusx = f_add(y, x);
    pi.yminusx = f_sub(y, x);
    xy = f_mul(x, y);
    pi.xy2d = f_mul(xy, ED25519_D2);
}

/// `r = p`
fn ge25519_p3_to_p2(r: &mut Ge25519P2, p: Ge25519P3) {
    r.X = p.X;
    r.Y = p.Y;
    r.Z = p.Z;
}

pub fn ge25519_p3_tobytes(s: &mut [u8], h: &Ge25519P3) {
    let recip: Fe25519;
    let x: Fe25519;
    let y: Fe25519;

    recip = f_invert(h.Z);
    x = f_mul(h.X, recip);
    y = f_mul(h.Y, recip);
    fe25519_tobytes(s, &y);
    s[31] = ((s[31] as i32) ^ (fe25519_isnegative(x) << 7)) as u8;
}

/// `r = 2 * p`
fn ge25519_p3_dbl(r: &mut Ge25519P1p1, p: Ge25519P3) {
    let mut q = Ge25519P2::default();
    ge25519_p3_to_p2(&mut q, p);
    ge25519_p2_dbl(r, q);
}

fn ge25519_precomp_0(h: &mut Ge25519Precomp) {
    fe25519_1(&mut h.yplusx);
    fe25519_1(&mut h.yminusx);
    fe25519_0(&mut h.xy2d);
}

/* ------------------------------------------------------------------ */
/* equal / negative (lines 609, 631)                                   */
/* ------------------------------------------------------------------ */

fn equal(b: i8, c: i8) -> u8 {
    let x: u8 = (b as u8) ^ (c as u8); /* 0: yes; 1..255: no */
    let mut y: u32 = x as u32; /* 0: yes; 1..255: no */

    y = y.wrapping_sub(1);
    (((y >> 29) ^ (optblocker_u8() as u32)) >> 2) as u8 /* 1: yes; 0: no */
}

fn negative(b: i8) -> u8 {
    let x: u8 = b as u8; /* 0..127: no 128..255: yes */
    ((((x >> 5) as i32) ^ (optblocker_u8() as i32)) >> 2) as u8 /* 1: yes; 0: no */
}

/* ------------------------------------------------------------------ */
/* constant-time table lookups (lines 647..720)                        */
/* ------------------------------------------------------------------ */

fn ge25519_cmov(t: &mut Ge25519Precomp, u: Ge25519Precomp, b: u8) {
    fe25519_cmov(&mut t.yplusx, u.yplusx, b as u32);
    fe25519_cmov(&mut t.yminusx, u.yminusx, b as u32);
    fe25519_cmov(&mut t.xy2d, u.xy2d, b as u32);
}

fn ge25519_cmov_cached(t: &mut Ge25519Cached, u: Ge25519Cached, b: u8) {
    fe25519_cmov(&mut t.YplusX, u.YplusX, b as u32);
    fe25519_cmov(&mut t.YminusX, u.YminusX, b as u32);
    fe25519_cmov(&mut t.Z, u.Z, b as u32);
    fe25519_cmov(&mut t.T2d, u.T2d, b as u32);
}

fn ge25519_cmov8(t: &mut Ge25519Precomp, precomp: &[Ge25519Precomp; 8], b: i8) {
    let mut minust = Ge25519Precomp::default();
    let bnegative: u8 = negative(b);
    /* `b - (((-bnegative) & b) * ((signed char) 1 << 1))` evaluated in `int`
     * then truncated to `unsigned char`. */
    let babs: u8 = (b as i32)
        .wrapping_sub(((0i32.wrapping_sub(bnegative as i32)) & (b as i32)).wrapping_mul(2))
        as u8;

    ge25519_precomp_0(t);
    ge25519_cmov(t, precomp[0], equal(babs as i8, 1));
    ge25519_cmov(t, precomp[1], equal(babs as i8, 2));
    ge25519_cmov(t, precomp[2], equal(babs as i8, 3));
    ge25519_cmov(t, precomp[3], equal(babs as i8, 4));
    ge25519_cmov(t, precomp[4], equal(babs as i8, 5));
    ge25519_cmov(t, precomp[5], equal(babs as i8, 6));
    ge25519_cmov(t, precomp[6], equal(babs as i8, 7));
    ge25519_cmov(t, precomp[7], equal(babs as i8, 8));
    minust.yplusx = t.yminusx;
    minust.yminusx = t.yplusx;
    minust.xy2d = f_neg(t.xy2d);
    ge25519_cmov(t, minust, bnegative);
}

fn ge25519_cmov8_base(t: &mut Ge25519Precomp, pos: i32, b: i8) {
    /* base[i][j] = (j+1)*256^i*B */
    ge25519_cmov8(t, &BASE[pos as usize], b);
}

fn ge25519_cmov8_cached(t: &mut Ge25519Cached, cached: &[Ge25519Cached; 8], b: i8) {
    let mut minust = Ge25519Cached::default();
    let bnegative: u8 = negative(b);
    let babs: u8 = (b as i32)
        .wrapping_sub(((0i32.wrapping_sub(bnegative as i32)) & (b as i32)).wrapping_mul(2))
        as u8;

    ge25519_cached_0(t);
    ge25519_cmov_cached(t, cached[0], equal(babs as i8, 1));
    ge25519_cmov_cached(t, cached[1], equal(babs as i8, 2));
    ge25519_cmov_cached(t, cached[2], equal(babs as i8, 3));
    ge25519_cmov_cached(t, cached[3], equal(babs as i8, 4));
    ge25519_cmov_cached(t, cached[4], equal(babs as i8, 5));
    ge25519_cmov_cached(t, cached[5], equal(babs as i8, 6));
    ge25519_cmov_cached(t, cached[6], equal(babs as i8, 7));
    ge25519_cmov_cached(t, cached[7], equal(babs as i8, 8));
    minust.YplusX = t.YminusX;
    minust.YminusX = t.YplusX;
    minust.Z = t.Z;
    minust.T2d = f_neg(t.T2d);
    ge25519_cmov_cached(t, minust, bnegative);
}

/* ------------------------------------------------------------------ */
/* ge25519_sub_cached (line 726)                                       */
/* ------------------------------------------------------------------ */

/// `r = p - q`
fn ge25519_sub_cached(r: &mut Ge25519P1p1, p: Ge25519P3, q: Ge25519Cached) {
    let t0: Fe25519;

    r.X = f_add(p.Y, p.X);
    r.Y = f_sub(p.Y, p.X);
    r.Z = f_mul(r.X, q.YminusX);
    r.Y = f_mul(r.Y, q.YplusX);
    r.T = f_mul(q.T2d, p.T);
    r.X = f_mul(p.Z, q.Z);
    t0 = f_add(r.X, r.X);
    r.X = f_sub(r.Z, r.Y);
    r.Y = f_add(r.Z, r.Y);
    r.Z = f_sub(t0, r.T);
    r.T = f_add(t0, r.T);
}

/* ------------------------------------------------------------------ */
/* ge25519_tobytes (line 745)                                          */
/* ------------------------------------------------------------------ */

pub fn ge25519_tobytes(s: &mut [u8], h: &Ge25519P2) {
    let recip: Fe25519;
    let x: Fe25519;
    let y: Fe25519;

    recip = f_invert(h.Z);
    x = f_mul(h.X, recip);
    y = f_mul(h.Y, recip);
    fe25519_tobytes(s, &y);
    s[31] = ((s[31] as i32) ^ (fe25519_isnegative(x) << 7)) as u8;
}

/* ------------------------------------------------------------------ */
/* ge25519_double_scalarmult_vartime (line 769)                         */
/* ------------------------------------------------------------------ */

/// `r = a * A + b * B` where `B` is the Ed25519 base point.
pub fn ge25519_double_scalarmult_vartime(
    r: &mut Ge25519P2,
    a: &[u8],
    A: &Ge25519P3,
    b: &[u8],
) {
    let bi: &[Ge25519Precomp; 8] = &BASE2;
    let mut aslide = [0i8; 256];
    let mut bslide = [0i8; 256];
    let mut ai = [Ge25519Cached::default(); 8]; /* A,3A,5A,7A,9A,11A,13A,15A */
    let mut t = Ge25519P1p1::default();
    let mut u = Ge25519P3::default();
    let mut a2 = Ge25519P3::default();
    let mut i: i32;

    slide_vartime(&mut aslide, a);
    slide_vartime(&mut bslide, b);

    ge25519_p3_to_cached(&mut ai[0], *A);

    ge25519_p3_dbl(&mut t, *A);
    ge25519_p1p1_to_p3(&mut a2, t);

    ge25519_add_cached(&mut t, a2, ai[0]);
    ge25519_p1p1_to_p3(&mut u, t);
    ge25519_p3_to_cached(&mut ai[1], u);

    ge25519_add_cached(&mut t, a2, ai[1]);
    ge25519_p1p1_to_p3(&mut u, t);
    ge25519_p3_to_cached(&mut ai[2], u);

    ge25519_add_cached(&mut t, a2, ai[2]);
    ge25519_p1p1_to_p3(&mut u, t);
    ge25519_p3_to_cached(&mut ai[3], u);

    ge25519_add_cached(&mut t, a2, ai[3]);
    ge25519_p1p1_to_p3(&mut u, t);
    ge25519_p3_to_cached(&mut ai[4], u);

    ge25519_add_cached(&mut t, a2, ai[4]);
    ge25519_p1p1_to_p3(&mut u, t);
    ge25519_p3_to_cached(&mut ai[5], u);

    ge25519_add_cached(&mut t, a2, ai[5]);
    ge25519_p1p1_to_p3(&mut u, t);
    ge25519_p3_to_cached(&mut ai[6], u);

    ge25519_add_cached(&mut t, a2, ai[6]);
    ge25519_p1p1_to_p3(&mut u, t);
    ge25519_p3_to_cached(&mut ai[7], u);

    ge25519_p2_0(r);

    i = 255;
    while i >= 0 {
        if aslide[i as usize] != 0 || bslide[i as usize] != 0 {
            break;
        }
        i -= 1;
    }

    while i >= 0 {
        ge25519_p2_dbl(&mut t, *r);

        if aslide[i as usize] > 0 {
            ge25519_p1p1_to_p3(&mut u, t);
            ge25519_add_cached(&mut t, u, ai[(aslide[i as usize] / 2) as usize]);
        } else if aslide[i as usize] < 0 {
            ge25519_p1p1_to_p3(&mut u, t);
            ge25519_sub_cached(
                &mut t,
                u,
                ai[((aslide[i as usize].wrapping_neg()) / 2) as usize],
            );
        }

        if bslide[i as usize] > 0 {
            ge25519_p1p1_to_p3(&mut u, t);
            ge25519_add_precomp(&mut t, u, bi[(bslide[i as usize] / 2) as usize]);
        } else if bslide[i as usize] < 0 {
            ge25519_p1p1_to_p3(&mut u, t);
            ge25519_sub_precomp(
                &mut t,
                u,
                bi[((bslide[i as usize].wrapping_neg()) / 2) as usize],
            );
        }

        ge25519_p1p1_to_p2(r, t);
        i -= 1;
    }
}

/* ------------------------------------------------------------------ */
/* ge25519_scalarmult (line 865)                                       */
/* ------------------------------------------------------------------ */

/// `h = a * p`
pub fn ge25519_scalarmult(h: &mut Ge25519P3, a: &[u8], p: Ge25519P3) {
    let mut e = [0i8; 64];
    let mut carry: i8;
    let mut r = Ge25519P1p1::default();
    let mut s = Ge25519P2::default();
    let mut t2 = Ge25519P1p1::default();
    let mut t3 = Ge25519P1p1::default();
    let mut t4 = Ge25519P1p1::default();
    let mut t5 = Ge25519P1p1::default();
    let mut t6 = Ge25519P1p1::default();
    let mut t7 = Ge25519P1p1::default();
    let mut t8 = Ge25519P1p1::default();
    let mut p2 = Ge25519P3::default();
    let mut p3 = Ge25519P3::default();
    let mut p4 = Ge25519P3::default();
    let mut p5 = Ge25519P3::default();
    let mut p6 = Ge25519P3::default();
    let mut p7 = Ge25519P3::default();
    let mut p8 = Ge25519P3::default();
    let mut pi = [Ge25519Cached::default(); 8];
    let mut t = Ge25519Cached::default();
    let mut i: i32;

    ge25519_p3_to_cached(&mut pi[1 - 1], p); /* p */

    ge25519_p3_dbl(&mut t2, p);
    ge25519_p1p1_to_p3(&mut p2, t2);
    ge25519_p3_to_cached(&mut pi[2 - 1], p2); /* 2p = 2*p */

    ge25519_add_cached(&mut t3, p, pi[2 - 1]);
    ge25519_p1p1_to_p3(&mut p3, t3);
    ge25519_p3_to_cached(&mut pi[3 - 1], p3); /* 3p = 2p+p */

    ge25519_p3_dbl(&mut t4, p2);
    ge25519_p1p1_to_p3(&mut p4, t4);
    ge25519_p3_to_cached(&mut pi[4 - 1], p4); /* 4p = 2*2p */

    ge25519_add_cached(&mut t5, p, pi[4 - 1]);
    ge25519_p1p1_to_p3(&mut p5, t5);
    ge25519_p3_to_cached(&mut pi[5 - 1], p5); /* 5p = 4p+p */

    ge25519_p3_dbl(&mut t6, p3);
    ge25519_p1p1_to_p3(&mut p6, t6);
    ge25519_p3_to_cached(&mut pi[6 - 1], p6); /* 6p = 2*3p */

    ge25519_add_cached(&mut t7, p, pi[6 - 1]);
    ge25519_p1p1_to_p3(&mut p7, t7);
    ge25519_p3_to_cached(&mut pi[7 - 1], p7); /* 7p = 6p+p */

    ge25519_p3_dbl(&mut t8, p4);
    ge25519_p1p1_to_p3(&mut p8, t8);
    ge25519_p3_to_cached(&mut pi[8 - 1], p8); /* 8p = 2*4p */

    let mut j: usize = 0;
    while j < 32 {
        e[2 * j + 0] = ((a[j] >> 0) & 15) as i8;
        e[2 * j + 1] = ((a[j] >> 4) & 15) as i8;
        j += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    let mut j: usize = 0;
    while j < 63 {
        e[j] = e[j].wrapping_add(carry);
        carry = e[j].wrapping_add(8);
        carry >>= 4;
        e[j] = e[j].wrapping_sub(carry.wrapping_mul(1i8 << 4));
        j += 1;
    }
    e[63] = e[63].wrapping_add(carry);
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    i = 63;
    while i != 0 {
        ge25519_cmov8_cached(&mut t, &pi, e[i as usize]);
        ge25519_add_cached(&mut r, *h, t);

        ge25519_p1p1_to_p2(&mut s, r);
        ge25519_p2_dbl(&mut r, s);
        ge25519_p1p1_to_p2(&mut s, r);
        ge25519_p2_dbl(&mut r, s);
        ge25519_p1p1_to_p2(&mut s, r);
        ge25519_p2_dbl(&mut r, s);
        ge25519_p1p1_to_p2(&mut s, r);
        ge25519_p2_dbl(&mut r, s);

        ge25519_p1p1_to_p3(h, r); /* *16 */
        i -= 1;
    }
    ge25519_cmov8_cached(&mut t, &pi, e[i as usize]);
    ge25519_add_cached(&mut r, *h, t);

    ge25519_p1p1_to_p3(h, r);
}

/* ------------------------------------------------------------------ */
/* ge25519_scalarmult_base (line 958)                                  */
/* ------------------------------------------------------------------ */

/// `h = a * B` (with precomputation)
pub fn ge25519_scalarmult_base(h: &mut Ge25519P3, a: &[u8]) {
    let mut e = [0i8; 64];
    let mut carry: i8;
    let mut r = Ge25519P1p1::default();
    let mut s = Ge25519P2::default();
    let mut t = Ge25519Precomp::default();
    let mut i: i32;

    let mut j: usize = 0;
    while j < 32 {
        e[2 * j + 0] = ((a[j] >> 0) & 15) as i8;
        e[2 * j + 1] = ((a[j] >> 4) & 15) as i8;
        j += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    let mut j: usize = 0;
    while j < 63 {
        e[j] = e[j].wrapping_add(carry);
        carry = e[j].wrapping_add(8);
        carry >>= 4;
        e[j] = e[j].wrapping_sub(carry.wrapping_mul(1i8 << 4));
        j += 1;
    }
    e[63] = e[63].wrapping_add(carry);
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    i = 1;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i as usize]);
        ge25519_add_precomp(&mut r, *h, t);
        ge25519_p1p1_to_p3(h, r);
        i += 2;
    }

    ge25519_p3_dbl(&mut r, *h);
    ge25519_p1p1_to_p2(&mut s, r);
    ge25519_p2_dbl(&mut r, s);
    ge25519_p1p1_to_p2(&mut s, r);
    ge25519_p2_dbl(&mut r, s);
    ge25519_p1p1_to_p2(&mut s, r);
    ge25519_p2_dbl(&mut r, s);
    ge25519_p1p1_to_p3(h, r);

    i = 0;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i as usize]);
        ge25519_add_precomp(&mut r, *h, t);
        ge25519_p1p1_to_p3(h, r);
        i += 2;
    }
}

/* ------------------------------------------------------------------ */
/* ge25519_p3p3_dbl .. ge25519_mul_l (lines 1010..1115)                 */
/* ------------------------------------------------------------------ */

/// `r = 2p`
fn ge25519_p3p3_dbl(r: &mut Ge25519P3, p: Ge25519P3) {
    let mut p1p1 = Ge25519P1p1::default();

    ge25519_p3_dbl(&mut p1p1, p);
    ge25519_p1p1_to_p3(r, p1p1);
}

/// `r = -p`
fn ge25519_p3_neg(r: &mut Ge25519P3, p: Ge25519P3) {
    r.X = f_neg(p.X);
    r.Y = p.Y;
    r.Z = p.Z;
    r.T = f_neg(p.T);
}

/// `r = p+q`
pub fn ge25519_p3_add(r: &mut Ge25519P3, p: Ge25519P3, q: Ge25519P3) {
    let mut q_cached = Ge25519Cached::default();
    let mut p1p1 = Ge25519P1p1::default();

    ge25519_p3_to_cached(&mut q_cached, q);
    ge25519_add_cached(&mut p1p1, p, q_cached);
    ge25519_p1p1_to_p3(r, p1p1);
}

/// `r = p-q`
pub fn ge25519_p3_sub(r: &mut Ge25519P3, p: Ge25519P3, q: Ge25519P3) {
    let mut q_neg = Ge25519P3::default();

    ge25519_p3_neg(&mut q_neg, q);
    ge25519_p3_add(r, p, q_neg);
}

/// `r = r*(2^n)+q`
fn ge25519_p3_dbladd(r: &mut Ge25519P3, n: i32, q: Ge25519P3) {
    let mut p2 = Ge25519P2::default();
    let mut p1p1 = Ge25519P1p1::default();
    let mut i: i32;

    ge25519_p3_to_p2(&mut p2, *r);
    i = 0;
    while i < n {
        ge25519_p2_dbl(&mut p1p1, p2);
        ge25519_p1p1_to_p2(&mut p2, p1p1);
        i += 1;
    }
    ge25519_p1p1_to_p3(r, p1p1);
    let rv = *r;
    ge25519_p3_add(r, rv, q);
}

/// multiply by the order of the main subgroup
/// `l = 2^252+27742317777372353535851937790883648493`
fn ge25519_mul_l(r: &mut Ge25519P3, p: Ge25519P3) {
    let mut _10 = Ge25519P3::default();
    let mut _11 = Ge25519P3::default();
    let mut _100 = Ge25519P3::default();
    let mut _110 = Ge25519P3::default();
    let mut _1000 = Ge25519P3::default();
    let mut _1011 = Ge25519P3::default();
    let mut _10000 = Ge25519P3::default();
    let mut _100000 = Ge25519P3::default();
    let mut _100110 = Ge25519P3::default();
    let mut _1000000 = Ge25519P3::default();
    let mut _1010000 = Ge25519P3::default();
    let mut _1010011 = Ge25519P3::default();
    let mut _1100011 = Ge25519P3::default();
    let mut _1100111 = Ge25519P3::default();
    let mut _1101011 = Ge25519P3::default();
    let mut _10010011 = Ge25519P3::default();
    let mut _10010111 = Ge25519P3::default();
    let mut _10111101 = Ge25519P3::default();
    let mut _11010011 = Ge25519P3::default();
    let mut _11100111 = Ge25519P3::default();
    let mut _11101101 = Ge25519P3::default();
    let mut _11110101 = Ge25519P3::default();

    ge25519_p3p3_dbl(&mut _10, p);
    ge25519_p3_add(&mut _11, p, _10);
    ge25519_p3_add(&mut _100, p, _11);
    ge25519_p3_add(&mut _110, _10, _100);
    ge25519_p3_add(&mut _1000, _10, _110);
    ge25519_p3_add(&mut _1011, _11, _1000);
    ge25519_p3p3_dbl(&mut _10000, _1000);
    ge25519_p3p3_dbl(&mut _100000, _10000);
    ge25519_p3_add(&mut _100110, _110, _100000);
    ge25519_p3p3_dbl(&mut _1000000, _100000);
    ge25519_p3_add(&mut _1010000, _10000, _1000000);
    ge25519_p3_add(&mut _1010011, _11, _1010000);
    ge25519_p3_add(&mut _1100011, _10000, _1010011);
    ge25519_p3_add(&mut _1100111, _100, _1100011);
    ge25519_p3_add(&mut _1101011, _100, _1100111);
    ge25519_p3_add(&mut _10010011, _1000000, _1010011);
    ge25519_p3_add(&mut _10010111, _100, _10010011);
    ge25519_p3_add(&mut _10111101, _100110, _10010111);
    ge25519_p3_add(&mut _11010011, _1000000, _10010011);
    ge25519_p3_add(&mut _11100111, _1010000, _10010111);
    ge25519_p3_add(&mut _11101101, _110, _11100111);
    ge25519_p3_add(&mut _11110101, _1000, _11101101);

    ge25519_p3_add(r, _1011, _11110101);
    ge25519_p3_dbladd(r, 126, _1010011);
    ge25519_p3_dbladd(r, 9, _10);
    let rv = *r;
    ge25519_p3_add(r, rv, _11110101);
    ge25519_p3_dbladd(r, 7, _1100111);
    ge25519_p3_dbladd(r, 9, _11110101);
    ge25519_p3_dbladd(r, 11, _10111101);
    ge25519_p3_dbladd(r, 8, _11100111);
    ge25519_p3_dbladd(r, 9, _1101011);
    ge25519_p3_dbladd(r, 6, _1011);
    ge25519_p3_dbladd(r, 14, _10010011);
    ge25519_p3_dbladd(r, 10, _1100011);
    ge25519_p3_dbladd(r, 9, _10010111);
    ge25519_p3_dbladd(r, 10, _11110101);
    ge25519_p3_dbladd(r, 8, _11010011);
    ge25519_p3_dbladd(r, 8, _11101101);
}

/* ------------------------------------------------------------------ */
/* point validation (lines 1117..1189)                                 */
/* ------------------------------------------------------------------ */

pub fn ge25519_is_on_curve(p: &Ge25519P3) -> i32 {
    let x2: Fe25519;
    let y2: Fe25519;
    let z2: Fe25519;
    let z4: Fe25519;
    let mut t0: Fe25519;
    let mut t1: Fe25519;

    x2 = f_sq(p.X);
    y2 = f_sq(p.Y);
    z2 = f_sq(p.Z);
    t0 = f_sub(y2, x2);
    t0 = f_mul(t0, z2);

    t1 = f_mul(x2, y2);
    t1 = f_mul(t1, ED25519_D);
    z4 = f_sq(z2);
    t1 = f_add(t1, z4);
    t0 = f_sub(t0, t1);

    fe25519_iszero(t0)
}

pub fn ge25519_is_on_main_subgroup(p: &Ge25519P3) -> i32 {
    let mut pl = Ge25519P3::default();
    let t: Fe25519;

    ge25519_mul_l(&mut pl, *p);

    t = f_sub(pl.Y, pl.Z);

    fe25519_iszero(pl.X) & fe25519_iszero(t)
}

pub fn ge25519_is_canonical(s: &[u8]) -> i32 {
    let mut c: u8;
    let d: u8;
    let mut i: u32;

    c = ((s[31] & 0x7f) ^ 0x7f) as u8;
    i = 30;
    while i > 0 {
        c |= s[i as usize] ^ 0xff;
        i -= 1;
    }
    c = (((c as u32).wrapping_sub(1u32)) >> 8) as u8;
    d = ((0xedu32.wrapping_sub(1u32).wrapping_sub(s[0] as u32)) >> 8) as u8;

    1 - ((c & d & 1) as i32)
}

pub fn ge25519_has_small_order(p: &Ge25519P3) -> i32 {
    let y_sqrtm1: Fe25519;
    let mut c: Fe25519;
    let mut ret: i32 = 0;

    ret |= fe25519_iszero(p.X);
    ret |= fe25519_iszero(p.Y);
    ret |= fe25519_iszero(p.Z);
    y_sqrtm1 = f_mul(p.Y, FE25519_SQRTM1);
    c = f_sub(y_sqrtm1, p.X);
    ret |= fe25519_iszero(c);
    c = f_add(y_sqrtm1, p.X);
    ret |= fe25519_iszero(c);

    ret
}

/* ------------------------------------------------------------------ */
/* sc25519_sq / sc25519_sqmul / sc25519_invert (lines 2160..2237)       */
/* ------------------------------------------------------------------ */

/// C: `static inline void sc25519_sq(unsigned char *s, const unsigned char *a)`
///
/// `s` and `a` never alias at this call site (see [`sc25519_sq_ip`] for the
/// `sc25519_sq(s, s)` form used by `sc25519_sqmul`).
#[inline]
fn sc25519_sq(s: &mut [u8], a: &[u8]) {
    sc25519_mul(s, a, a);
}

/// C: `sc25519_sq(s, s)` -- `sc25519_mul()` reads all of `a`/`b` before it
/// writes `s`, so the in-place form is well defined.
#[inline]
fn sc25519_sq_ip(s: &mut [u8]) {
    let mut a = [0u8; 32];
    a.copy_from_slice(&s[..32]);
    sc25519_mul(s, &a, &a);
}

/// `s = s^(2^n) * a`
fn sc25519_sqmul(s: &mut [u8], n: i32, a: &[u8]) {
    let mut i: i32 = 0;

    while i < n {
        sc25519_sq_ip(s);
        i += 1;
    }
    let mut t = [0u8; 32];
    t.copy_from_slice(&s[..32]);
    sc25519_mul(s, &t, a);
}

pub fn sc25519_invert(recip: &mut [u8], s: &[u8]) {
    let mut _10 = [0u8; 32];
    let mut _100 = [0u8; 32];
    let mut _1000 = [0u8; 32];
    let mut _10000 = [0u8; 32];
    let mut _100000 = [0u8; 32];
    let mut _1000000 = [0u8; 32];
    let mut _10010011 = [0u8; 32];
    let mut _10010111 = [0u8; 32];
    let mut _100110 = [0u8; 32];
    let mut _1010 = [0u8; 32];
    let mut _1010000 = [0u8; 32];
    let mut _1010011 = [0u8; 32];
    let mut _1011 = [0u8; 32];
    let mut _10110 = [0u8; 32];
    let mut _10111101 = [0u8; 32];
    let mut _11 = [0u8; 32];
    let mut _1100011 = [0u8; 32];
    let mut _1100111 = [0u8; 32];
    let mut _11010011 = [0u8; 32];
    let mut _1101011 = [0u8; 32];
    let mut _11100111 = [0u8; 32];
    let mut _11101011 = [0u8; 32];
    let mut _11110101 = [0u8; 32];

    sc25519_sq(&mut _10, s);
    sc25519_mul(&mut _11, s, &_10);
    sc25519_mul(&mut _100, s, &_11);
    sc25519_sq(&mut _1000, &_100);
    sc25519_mul(&mut _1010, &_10, &_1000);
    sc25519_mul(&mut _1011, s, &_1010);
    sc25519_sq(&mut _10000, &_1000);
    sc25519_sq(&mut _10110, &_1011);
    sc25519_mul(&mut _100000, &_1010, &_10110);
    sc25519_mul(&mut _100110, &_10000, &_10110);
    sc25519_sq(&mut _1000000, &_100000);
    sc25519_mul(&mut _1010000, &_10000, &_1000000);
    sc25519_mul(&mut _1010011, &_11, &_1010000);
    sc25519_mul(&mut _1100011, &_10000, &_1010011);
    sc25519_mul(&mut _1100111, &_100, &_1100011);
    sc25519_mul(&mut _1101011, &_100, &_1100111);
    sc25519_mul(&mut _10010011, &_1000000, &_1010011);
    sc25519_mul(&mut _10010111, &_100, &_10010011);
    sc25519_mul(&mut _10111101, &_100110, &_10010111);
    sc25519_mul(&mut _11010011, &_10110, &_10111101);
    sc25519_mul(&mut _11100111, &_1010000, &_10010111);
    sc25519_mul(&mut _11101011, &_100, &_11100111);
    sc25519_mul(&mut _11110101, &_1010, &_11101011);

    sc25519_mul(recip, &_1011, &_11110101);
    sc25519_sqmul(recip, 126, &_1010011);
    sc25519_sqmul(recip, 9, &_10);
    {
        let mut t = [0u8; 32];
        t.copy_from_slice(&recip[..32]);
        sc25519_mul(recip, &t, &_11110101);
    }
    sc25519_sqmul(recip, 7, &_1100111);
    sc25519_sqmul(recip, 9, &_11110101);
    sc25519_sqmul(recip, 11, &_10111101);
    sc25519_sqmul(recip, 8, &_11100111);
    sc25519_sqmul(recip, 9, &_1101011);
    sc25519_sqmul(recip, 6, &_1011);
    sc25519_sqmul(recip, 14, &_10010011);
    sc25519_sqmul(recip, 10, &_1100011);
    sc25519_sqmul(recip, 9, &_10010111);
    sc25519_sqmul(recip, 10, &_11110101);
    sc25519_sqmul(recip, 8, &_11010011);
    sc25519_sqmul(recip, 8, &_11101011);
}

/* ------------------------------------------------------------------ */
/* sc25519_is_canonical (line 2573)                                    */
/* ------------------------------------------------------------------ */

pub fn sc25519_is_canonical(s: &[u8]) -> i32 {
    /* 2^252+27742317777372353535851937790883648493 */
    static L: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
        0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x10,
    ];
    let mut c: u8 = 0;
    let mut n: u8 = 1;
    let mut i: u32 = 32;

    loop {
        i -= 1;
        let si = s[i as usize] as i32;
        let li = L[i as usize] as i32;
        c |= ((si.wrapping_sub(li) >> 8) & (n as i32)) as u8;
        n &= (((si ^ li).wrapping_sub(1)) >> 8) as u8;
        if i == 0 {
            break;
        }
    }

    (c != 0) as i32
}

/* ------------------------------------------------------------------ */
/* Montgomery <-> Edwards, elligator2 (lines 2596..2760)               */
/* ------------------------------------------------------------------ */

/// montgomery to edwards
fn ge25519_mont_to_ed(xed: &mut Fe25519, yed: &mut Fe25519, x: Fe25519, y: Fe25519) {
    let mut one = Fe25519::ZERO;
    let x_plus_one: Fe25519;
    let x_minus_one: Fe25519;
    let mut x_plus_one_y_inv: Fe25519;

    fe25519_1(&mut one);
    x_plus_one = f_add(x, one);
    x_minus_one = f_sub(x, one);

    /* xed = sqrt(-A-2)*x/y */
    x_plus_one_y_inv = f_mul(x_plus_one, y);
    x_plus_one_y_inv = f_invert(x_plus_one_y_inv); /* 1/((x+1)*y) */
    *xed = f_mul(x, ED25519_SQRTAM2);
    *xed = f_mul(*xed, x_plus_one_y_inv); /* sqrt(-A-2)*x/((x+1)*y) */
    *xed = f_mul(*xed, x_plus_one);

    /* yed = (x-1)/(x+1) */
    *yed = f_mul(x_plus_one_y_inv, y); /* 1/(x+1) */
    *yed = f_mul(*yed, x_minus_one);
    fe25519_cmov(yed, one, fe25519_iszero(x_plus_one_y_inv) as u32);
}

/// montgomery -- recover `y = sqrt(x^3 + A*x^2 + x)`
fn ge25519_xmont_to_ymont(y: &mut Fe25519, x: Fe25519) -> i32 {
    let mut x2: Fe25519;
    let x3: Fe25519;

    x2 = f_sq(x);
    x3 = f_mul(x, x2);
    x2 = f_mul32(x2, ED25519_A_32 as u32);
    *y = f_add(x3, x);
    *y = f_add(*y, x2);

    let yv = *y;
    fe25519_sqrt(y, yv)
}

/// multiply by the cofactor
pub fn ge25519_clear_cofactor(p3: &mut Ge25519P3) {
    let mut p1 = Ge25519P1p1::default();
    let mut p2 = Ge25519P2::default();

    ge25519_p3_dbl(&mut p1, *p3);
    ge25519_p1p1_to_p2(&mut p2, p1);
    ge25519_p2_dbl(&mut p1, p2);
    ge25519_p1p1_to_p2(&mut p2, p1);
    ge25519_p2_dbl(&mut p1, p2);
    ge25519_p1p1_to_p3(p3, p1);
}

fn ge25519_elligator2(x: &mut Fe25519, y: &mut Fe25519, r: Fe25519, notsquare_p: &mut i32) {
    let mut gx1: Fe25519;
    let mut rr2: Fe25519;
    let mut x2: Fe25519;
    let x3: Fe25519;
    let negx: Fe25519;
    let notsquare: i32;

    rr2 = f_sq2(r);
    rr2[0] = rr2[0].wrapping_add(1);
    rr2 = f_invert(rr2);
    *x = f_mul32(rr2, ED25519_A_32 as u32);
    *x = f_neg(*x); /* x=x1 */

    x2 = f_sq(*x);
    x3 = f_mul(*x, x2);
    x2 = f_mul32(x2, ED25519_A_32 as u32); /* x2 = A*x1^2 */
    gx1 = f_add(x3, *x);
    gx1 = f_add(gx1, x2); /* gx1 = x1^3 + A*x1^2 + x1 */

    notsquare = fe25519_notsquare(gx1);

    /* gx1 not a square  => x = -x1-A */
    negx = f_neg(*x);
    fe25519_cmov(x, negx, notsquare as u32);
    fe25519_0(&mut x2);
    fe25519_cmov(&mut x2, ED25519_A, notsquare as u32);
    *x = f_sub(*x, x2);

    /* y = sqrt(gx1) or sqrt(gx2) with gx2 = gx1 * (A+x1) / -x1 */
    /* but it is about as fast to just recompute from the curve equation. */
    if ge25519_xmont_to_ymont(y, *x) != 0 {
        std::process::abort(); /* LCOV_EXCL_LINE */
    }
    *notsquare_p = notsquare;
}

pub fn ge25519_from_uniform(s: &mut [u8], r: &[u8]) {
    let mut p3 = Ge25519P3::default();
    let mut x = Fe25519::ZERO;
    let mut y = Fe25519::ZERO;
    let negxed: Fe25519;
    let mut r_fe = Fe25519::ZERO;
    let mut notsquare: i32 = 0;
    let x_sign: u8;

    s[..32].copy_from_slice(&r[..32]);
    x_sign = ((((s[31] as i32) >> 5) ^ (optblocker_u8() as i32)) >> 2) as u8;
    s[31] &= 0x7f;
    fe25519_frombytes(&mut r_fe, &s[..32]);

    ge25519_elligator2(&mut x, &mut y, r_fe, &mut notsquare);

    ge25519_mont_to_ed(&mut p3.X, &mut p3.Y, x, y);
    negxed = f_neg(p3.X);
    let b = fe25519_isnegative(p3.X) ^ (x_sign as i32);
    fe25519_cmov(&mut p3.X, negxed, b as u32);

    fe25519_1(&mut p3.Z);
    p3.T = f_mul(p3.X, p3.Y);
    ge25519_clear_cofactor(&mut p3);
    ge25519_p3_tobytes(s, &p3);
}

fn fe25519_reduce64(fe_f: &mut Fe25519, h: &[u8]) {
    let mut fl = [0u8; 32];
    let mut gl = [0u8; 32];
    let mut fe_g = Fe25519::ZERO;
    let mut i: usize;

    fl.copy_from_slice(&h[0..32]);
    gl.copy_from_slice(&h[32..64]);
    fl[31] &= 0x7f;
    gl[31] &= 0x7f;
    fe25519_frombytes(fe_f, &fl);
    fe25519_frombytes(&mut fe_g, &gl);
    fe_f[0] = fe_f[0].wrapping_add(
        ((((h[31] as i32) >> 5) ^ (optblocker_u8() as i32)) >> 2)
            .wrapping_mul(19)
            .wrapping_add(
                ((((h[63] as i32) >> 5) ^ (optblocker_u8() as i32)) >> 2).wrapping_mul(722),
            ),
    );
    i = 0;
    while i < 10 {
        fe_f[i] = fe_f[i].wrapping_add(38i32.wrapping_mul(fe_g[i]));
        i += 1;
    }
    let t = *fe_f;
    fe25519_reduce(fe_f, t);
}

pub fn ge25519_from_hash(s: &mut [u8], h: &[u8]) {
    let mut p3 = Ge25519P3::default();
    let mut fe_f = Fe25519::ZERO;
    let mut x = Fe25519::ZERO;
    let mut y = Fe25519::ZERO;
    let negy: Fe25519;
    let mut notsquare: i32 = 0;
    let y_sign: u8;

    fe25519_reduce64(&mut fe_f, h);
    ge25519_elligator2(&mut x, &mut y, fe_f, &mut notsquare);

    y_sign = (notsquare ^ 1) as u8;
    negy = f_neg(y);
    let b = fe25519_isnegative(y) ^ (y_sign as i32);
    fe25519_cmov(&mut y, negy, b as u32);

    ge25519_mont_to_ed(&mut p3.X, &mut p3.Y, x, y);

    fe25519_1(&mut p3.Z);
    p3.T = f_mul(p3.X, p3.Y);
    ge25519_clear_cofactor(&mut p3);
    ge25519_p3_tobytes(s, &p3);
}

/* ------------------------------------------------------------------ */
/* Ristretto group (lines 2765..2992)                                  */
/* ------------------------------------------------------------------ */

fn ristretto255_sqrt_ratio_m1(x: &mut Fe25519, u: Fe25519, v: Fe25519) -> i32 {
    let mut v3: Fe25519;
    let mut vxx: Fe25519;
    let m_root_check: Fe25519;
    let p_root_check: Fe25519;
    let mut f_root_check: Fe25519;
    let x_sqrtm1: Fe25519;
    let has_m_root: i32;
    let has_p_root: i32;
    let has_f_root: i32;

    v3 = f_sq(v);
    v3 = f_mul(v3, v); /* v3 = v^3 */
    *x = f_sq(v3);
    *x = f_mul(*x, u);
    *x = f_mul(*x, v); /* x = uv^7 */

    *x = f_pow22523(*x); /* x = (uv^7)^((q-5)/8) */
    *x = f_mul(*x, v3);
    *x = f_mul(*x, u); /* x = uv^3(uv^7)^((q-5)/8) */

    vxx = f_sq(*x);
    vxx = f_mul(vxx, v); /* vx^2 */
    m_root_check = f_sub(vxx, u); /* vx^2-u */
    p_root_check = f_add(vxx, u); /* vx^2+u */
    f_root_check = f_mul(u, FE25519_SQRTM1); /* u*sqrt(-1) */
    f_root_check = f_add(vxx, f_root_check); /* vx^2+u*sqrt(-1) */
    has_m_root = fe25519_iszero(m_root_check);
    has_p_root = fe25519_iszero(p_root_check);
    has_f_root = fe25519_iszero(f_root_check);
    x_sqrtm1 = f_mul(*x, FE25519_SQRTM1); /* x*sqrt(-1) */

    fe25519_cmov(x, x_sqrtm1, (has_p_root | has_f_root) as u32);
    fe25519_abs(x);

    has_m_root | has_p_root
}

fn ristretto255_is_canonical(s: &[u8]) -> i32 {
    let mut c: u8;
    let d: u8;
    let e: u8;
    let mut i: u32;

    c = ((s[31] & 0x7f) ^ 0x7f) as u8;
    i = 30;
    while i > 0 {
        c |= s[i as usize] ^ 0xff;
        i -= 1;
    }
    c = (((c as u32).wrapping_sub(1u32)) >> 8) as u8;
    d = ((0xedu32.wrapping_sub(1u32).wrapping_sub(s[0] as u32)) >> 8) as u8;
    e = ((((s[31] as i32) >> 5) ^ (optblocker_u8() as i32)) >> 2) as u8;

    1 - ((((c & d) | e | s[0]) & 1) as i32)
}

pub fn ristretto255_frombytes(h: &mut Ge25519P3, s: &[u8]) -> i32 {
    let mut inv_sqrt = Fe25519::ZERO;
    let mut one = Fe25519::ZERO;
    let mut s_ = Fe25519::ZERO;
    let ss: Fe25519;
    let mut u1 = Fe25519::ZERO;
    let mut u2 = Fe25519::ZERO;
    let u1u1: Fe25519;
    let u2u2: Fe25519;
    let mut v: Fe25519;
    let v_u2u2: Fe25519;
    let notsquare: i32;

    if ristretto255_is_canonical(s) == 0 {
        return -1;
    }
    fe25519_frombytes(&mut s_, s);
    ss = f_sq(s_); /* ss = s^2 */

    fe25519_1(&mut u1);
    u1 = f_sub(u1, ss); /* u1 = 1-ss */
    u1u1 = f_sq(u1); /* u1u1 = u1^2 */

    fe25519_1(&mut u2);
    u2 = f_add(u2, ss); /* u2 = 1+ss */
    u2u2 = f_sq(u2); /* u2u2 = u2^2 */

    v = f_mul(ED25519_D, u1u1); /* v = d*u1^2 */
    v = f_neg(v); /* v = -d*u1^2 */
    v = f_sub(v, u2u2); /* v = -(d*u1^2)-u2^2 */

    v_u2u2 = f_mul(v, u2u2); /* v_u2u2 = v*u2^2 */

    fe25519_1(&mut one);
    notsquare = ristretto255_sqrt_ratio_m1(&mut inv_sqrt, one, v_u2u2);
    h.X = f_mul(inv_sqrt, u2);
    h.Y = f_mul(inv_sqrt, h.X);
    h.Y = f_mul(h.Y, v);

    h.X = f_mul(h.X, s_);
    h.X = f_add(h.X, h.X);
    fe25519_abs(&mut h.X);
    h.Y = f_mul(u1, h.Y);
    fe25519_1(&mut h.Z);
    h.T = f_mul(h.X, h.Y);

    -((1 - notsquare) | fe25519_isnegative(h.T) | fe25519_iszero(h.Y))
}

pub fn ristretto255_p3_tobytes(s: &mut [u8], h: &Ge25519P3) {
    let den1: Fe25519;
    let den2: Fe25519;
    let mut den_inv: Fe25519;
    let eden: Fe25519;
    let mut inv_sqrt = Fe25519::ZERO;
    let ix: Fe25519;
    let iy: Fe25519;
    let mut one = Fe25519::ZERO;
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
    let rotate: i32;

    u1 = f_add(h.Z, h.Y); /* u1 = Z+Y */
    zmy = f_sub(h.Z, h.Y); /* zmy = Z-Y */
    u1 = f_mul(u1, zmy); /* u1 = (Z+Y)*(Z-Y) */
    u2 = f_mul(h.X, h.Y); /* u2 = X*Y */

    u1_u2u2 = f_sq(u2); /* u1_u2u2 = u2^2 */
    u1_u2u2 = f_mul(u1, u1_u2u2); /* u1_u2u2 = u1*u2^2 */

    fe25519_1(&mut one);
    let _ = ristretto255_sqrt_ratio_m1(&mut inv_sqrt, one, u1_u2u2);
    den1 = f_mul(inv_sqrt, u1); /* den1 = inv_sqrt*u1 */
    den2 = f_mul(inv_sqrt, u2); /* den2 = inv_sqrt*u2 */
    z_inv = f_mul(den1, den2); /* z_inv = den1*den2 */
    z_inv = f_mul(z_inv, h.T); /* z_inv = den1*den2*T */

    ix = f_mul(h.X, FE25519_SQRTM1); /* ix = X*sqrt(-1) */
    iy = f_mul(h.Y, FE25519_SQRTM1); /* iy = Y*sqrt(-1) */
    eden = f_mul(den1, ED25519_INVSQRTAMD); /* eden = den1/sqrt(a-d) */

    t_z_inv = f_mul(h.T, z_inv); /* t_z_inv = T*z_inv */
    rotate = fe25519_isnegative(t_z_inv);

    x_ = h.X;
    y_ = h.Y;
    den_inv = den2;

    fe25519_cmov(&mut x_, iy, rotate as u32);
    fe25519_cmov(&mut y_, ix, rotate as u32);
    fe25519_cmov(&mut den_inv, eden, rotate as u32);

    x_z_inv = f_mul(x_, z_inv);
    fe25519_cneg(&mut y_, fe25519_isnegative(x_z_inv) as u32);

    s_ = f_sub(h.Z, y_);
    s_ = f_mul(den_inv, s_);
    fe25519_abs(&mut s_);
    fe25519_tobytes(s, &s_);
}

fn ristretto255_elligator(p: &mut Ge25519P3, t: Fe25519) {
    let mut c = Fe25519::ZERO;
    let mut n: Fe25519;
    let mut one = Fe25519::ZERO;
    let mut r: Fe25519;
    let rpd: Fe25519;
    let mut s = Fe25519::ZERO;
    let mut s_prime: Fe25519;
    let ss: Fe25519;
    let mut u: Fe25519;
    let mut v: Fe25519;
    let mut w0: Fe25519;
    let w1: Fe25519;
    let w2: Fe25519;
    let w3: Fe25519;
    let wasnt_square: i32;

    fe25519_1(&mut one);
    r = f_sq(t); /* r = t^2 */
    r = f_mul(FE25519_SQRTM1, r); /* r = sqrt(-1)*t^2 */
    u = f_add(r, one); /* u = r+1 */
    u = f_mul(u, ED25519_ONEMSQD); /* u = (r+1)*(1-d^2) */
    fe25519_1(&mut c);
    c = f_neg(c); /* c = -1 */
    rpd = f_add(r, ED25519_D); /* rpd = r+d */
    v = f_mul(r, ED25519_D); /* v = r*d */
    v = f_sub(c, v); /* v = c-r*d */
    v = f_mul(v, rpd); /* v = (c-r*d)*(r+d) */

    wasnt_square = 1 - ristretto255_sqrt_ratio_m1(&mut s, u, v);
    s_prime = f_mul(s, t);
    fe25519_abs(&mut s_prime);
    s_prime = f_neg(s_prime); /* s_prime = -|s*t| */
    fe25519_cmov(&mut s, s_prime, wasnt_square as u32);
    fe25519_cmov(&mut c, r, wasnt_square as u32);

    n = f_sub(r, one); /* n = r-1 */
    n = f_mul(n, c); /* n = c*(r-1) */
    n = f_mul(n, ED25519_SQDMONE); /* n = c*(r-1)*(d-1)^2 */
    n = f_sub(n, v); /* n =  c*(r-1)*(d-1)^2-v */

    w0 = f_add(s, s); /* w0 = 2s */
    w0 = f_mul(w0, v); /* w0 = 2s*v */
    w1 = f_mul(n, ED25519_SQRTADM1); /* w1 = n*sqrt(ad-1) */
    ss = f_sq(s); /* ss = s^2 */
    w2 = f_sub(one, ss); /* w2 = 1-s^2 */
    w3 = f_add(one, ss); /* w3 = 1+s^2 */

    p.X = f_mul(w0, w3);
    p.Y = f_mul(w2, w1);
    p.Z = f_mul(w1, w3);
    p.T = f_mul(w0, w2);
}

pub fn ristretto255_from_hash(s: &mut [u8], h: &[u8]) {
    let mut r0 = Fe25519::ZERO;
    let mut r1 = Fe25519::ZERO;
    let mut p0 = Ge25519P3::default();
    let mut p1 = Ge25519P3::default();
    let mut p = Ge25519P3::default();

    fe25519_frombytes(&mut r0, &h[0..32]);
    fe25519_frombytes(&mut r1, &h[32..64]);
    ristretto255_elligator(&mut p0, r0);
    ristretto255_elligator(&mut p1, r1);
    ge25519_p3_add(&mut p, p0, p1);
    ristretto255_p3_tobytes(s, &p);
}

/* ================================================================== */
/* C ABI (private/quirks.h renames)                                    */
/* ================================================================== */

/// C: `void ge25519_tobytes(unsigned char *s, const ge25519_p2 *h)`
///
/// # Safety
/// `s` must point to 32 writable bytes, `h` to a readable `ge25519_p2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_tobytes(s: *mut u8, h: *const Ge25519P2) {
    let hv = unsafe { ptr::read(h) };
    let mut buf = [0u8; 32];

    ge25519_tobytes(&mut buf, &hv);
    unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), s, 32) };
}

/// C: `void ge25519_p3_tobytes(unsigned char *s, const ge25519_p3 *h)`
///
/// # Safety
/// `s` must point to 32 writable bytes, `h` to a readable `ge25519_p3`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const Ge25519P3) {
    let hv = unsafe { ptr::read(h) };
    let mut buf = [0u8; 32];

    ge25519_p3_tobytes(&mut buf, &hv);
    unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), s, 32) };
}

/// C: `int ge25519_frombytes(ge25519_p3 *h, const unsigned char *s)`
///
/// # Safety
/// `h` must point to a writable `ge25519_p3`, `s` to 32 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes(h: *mut Ge25519P3, s: *const u8) -> c_int {
    let sl = unsafe { core::slice::from_raw_parts(s, 32) };
    let mut hv = unsafe { ptr::read(h) };

    let ret = ge25519_frombytes(&mut hv, sl);
    unsafe { ptr::write(h, hv) };

    ret as c_int
}

/// C: `int ge25519_frombytes_negate_vartime(ge25519_p3 *h, const unsigned char *s)`
///
/// # Safety
/// `h` must point to a writable `ge25519_p3`, `s` to 32 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes_negate_vartime(
    h: *mut Ge25519P3,
    s: *const u8,
) -> c_int {
    let sl = unsafe { core::slice::from_raw_parts(s, 32) };
    let mut hv = unsafe { ptr::read(h) };

    let ret = ge25519_frombytes_negate_vartime(&mut hv, sl);
    unsafe { ptr::write(h, hv) };

    ret as c_int
}

/// C: `void ge25519_p1p1_to_p2(ge25519_p2 *r, const ge25519_p1p1 *p)`
///
/// # Safety
/// Both pointers must reference valid objects of the corresponding type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p2(r: *mut Ge25519P2, p: *const Ge25519P1p1) {
    let pv = unsafe { ptr::read(p) };
    let mut rv = unsafe { ptr::read(r) };

    ge25519_p1p1_to_p2(&mut rv, pv);
    unsafe { ptr::write(r, rv) };
}

/// C: `void ge25519_p1p1_to_p3(ge25519_p3 *r, const ge25519_p1p1 *p)`
///
/// # Safety
/// Both pointers must reference valid objects of the corresponding type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p3(r: *mut Ge25519P3, p: *const Ge25519P1p1) {
    let pv = unsafe { ptr::read(p) };
    let mut rv = unsafe { ptr::read(r) };

    ge25519_p1p1_to_p3(&mut rv, pv);
    unsafe { ptr::write(r, rv) };
}

/// C: `void ge25519_p2_to_p3(ge25519_p3 *r, const ge25519_p2 *p)`
///
/// # Safety
/// Both pointers must reference valid objects of the corresponding type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p2_to_p3(r: *mut Ge25519P3, p: *const Ge25519P2) {
    let pv = unsafe { ptr::read(p) };
    let mut rv = unsafe { ptr::read(r) };

    ge25519_p2_to_p3(&mut rv, pv);
    unsafe { ptr::write(r, rv) };
}

/// C: `void ge25519_p3_add(ge25519_p3 *r, const ge25519_p3 *p, const ge25519_p3 *q)`
///
/// # Safety
/// All three pointers must reference valid `ge25519_p3` objects; they may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_add(
    r: *mut Ge25519P3,
    p: *const Ge25519P3,
    q: *const Ge25519P3,
) {
    let pv = unsafe { ptr::read(p) };
    let qv = unsafe { ptr::read(q) };
    let mut rv = unsafe { ptr::read(r) };

    ge25519_p3_add(&mut rv, pv, qv);
    unsafe { ptr::write(r, rv) };
}

/// C: `void ge25519_p3_sub(ge25519_p3 *r, const ge25519_p3 *p, const ge25519_p3 *q)`
///
/// # Safety
/// All three pointers must reference valid `ge25519_p3` objects; they may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_sub(
    r: *mut Ge25519P3,
    p: *const Ge25519P3,
    q: *const Ge25519P3,
) {
    let pv = unsafe { ptr::read(p) };
    let qv = unsafe { ptr::read(q) };
    let mut rv = unsafe { ptr::read(r) };

    ge25519_p3_sub(&mut rv, pv, qv);
    unsafe { ptr::write(r, rv) };
}

/// C: `void ge25519_scalarmult_base(ge25519_p3 *h, const unsigned char *a)`
///
/// # Safety
/// `h` must point to a writable `ge25519_p3`, `a` to 32 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult_base(h: *mut Ge25519P3, a: *const u8) {
    let al = unsafe { core::slice::from_raw_parts(a, 32) };
    let mut hv = Ge25519P3::default();

    ge25519_scalarmult_base(&mut hv, al);
    unsafe { ptr::write(h, hv) };
}

/// C: `void ge25519_double_scalarmult_vartime(ge25519_p2 *r, const unsigned char *a,
///                                            const ge25519_p3 *A, const unsigned char *b)`
///
/// # Safety
/// `a` and `b` must point to 32 readable bytes each, `A` to a readable
/// `ge25519_p3` and `r` to a writable `ge25519_p2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_double_scalarmult_vartime(
    r: *mut Ge25519P2,
    a: *const u8,
    A: *const Ge25519P3,
    b: *const u8,
) {
    let al = unsafe { core::slice::from_raw_parts(a, 32) };
    let bl = unsafe { core::slice::from_raw_parts(b, 32) };
    let av = unsafe { ptr::read(A) };
    let mut rv = Ge25519P2::default();

    ge25519_double_scalarmult_vartime(&mut rv, al, &av, bl);
    unsafe { ptr::write(r, rv) };
}

/// C: `void ge25519_scalarmult(ge25519_p3 *h, const unsigned char *a, const ge25519_p3 *p)`
///
/// # Safety
/// `h` must point to a writable `ge25519_p3`, `a` to 32 readable bytes and `p`
/// to a readable `ge25519_p3`; `h` and `p` may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult(
    h: *mut Ge25519P3,
    a: *const u8,
    p: *const Ge25519P3,
) {
    let al = unsafe { core::slice::from_raw_parts(a, 32) };
    let pv = unsafe { ptr::read(p) };
    let mut hv = Ge25519P3::default();

    ge25519_scalarmult(&mut hv, al, pv);
    unsafe { ptr::write(h, hv) };
}

/// C: `void ge25519_clear_cofactor(ge25519_p3 *p3)`
///
/// # Safety
/// `p3` must point to a readable and writable `ge25519_p3`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_clear_cofactor(p3: *mut Ge25519P3) {
    let mut pv = unsafe { ptr::read(p3) };

    ge25519_clear_cofactor(&mut pv);
    unsafe { ptr::write(p3, pv) };
}

/// C: `int ge25519_is_canonical(const unsigned char *s)`
///
/// # Safety
/// `s` must point to 32 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int {
    let sl = unsafe { core::slice::from_raw_parts(s, 32) };

    ge25519_is_canonical(sl) as c_int
}

/// C: `int ge25519_is_on_curve(const ge25519_p3 *p)`
///
/// # Safety
/// `p` must point to a readable `ge25519_p3`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_curve(p: *const Ge25519P3) -> c_int {
    let pv = unsafe { ptr::read(p) };

    ge25519_is_on_curve(&pv) as c_int
}

/// C: `int ge25519_is_on_main_subgroup(const ge25519_p3 *p)`
///
/// # Safety
/// `p` must point to a readable `ge25519_p3`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_main_subgroup(p: *const Ge25519P3) -> c_int {
    let pv = unsafe { ptr::read(p) };

    ge25519_is_on_main_subgroup(&pv) as c_int
}

/// C: `int ge25519_has_small_order(const ge25519_p3 *p)`
///
/// # Safety
/// `p` must point to a readable `ge25519_p3`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_has_small_order(p: *const Ge25519P3) -> c_int {
    let pv = unsafe { ptr::read(p) };

    ge25519_has_small_order(&pv) as c_int
}

/// C: `void ge25519_from_uniform(unsigned char s[32], const unsigned char r[32])`
///
/// # Safety
/// `s` must point to 32 writable bytes and `r` to 32 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8) {
    let mut rbuf = [0u8; 32];
    let mut sbuf = [0u8; 32];

    unsafe { ptr::copy_nonoverlapping(r, rbuf.as_mut_ptr(), 32) };
    ge25519_from_uniform(&mut sbuf, &rbuf);
    unsafe { ptr::copy_nonoverlapping(sbuf.as_ptr(), s, 32) };
}

/// C: `void ge25519_from_hash(unsigned char s[32], const unsigned char h[64])`
///
/// # Safety
/// `s` must point to 32 writable bytes and `h` to 64 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8) {
    let mut hbuf = [0u8; 64];
    let mut sbuf = [0u8; 32];

    unsafe { ptr::copy_nonoverlapping(h, hbuf.as_mut_ptr(), 64) };
    ge25519_from_hash(&mut sbuf, &hbuf);
    unsafe { ptr::copy_nonoverlapping(sbuf.as_ptr(), s, 32) };
}

/// C: `int ristretto255_frombytes(ge25519_p3 *h, const unsigned char *s)`
///
/// # Safety
/// `h` must point to a readable and writable `ge25519_p3`, `s` to 32 readable
/// bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_frombytes(
    h: *mut Ge25519P3,
    s: *const u8,
) -> c_int {
    let sl = unsafe { core::slice::from_raw_parts(s, 32) };
    let mut hv = unsafe { ptr::read(h) };

    let ret = ristretto255_frombytes(&mut hv, sl);
    unsafe { ptr::write(h, hv) };

    ret as c_int
}

/// C: `void ristretto255_p3_tobytes(unsigned char *s, const ge25519_p3 *h)`
///
/// # Safety
/// `s` must point to 32 writable bytes, `h` to a readable `ge25519_p3`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const Ge25519P3) {
    let hv = unsafe { ptr::read(h) };
    let mut buf = [0u8; 32];

    ristretto255_p3_tobytes(&mut buf, &hv);
    unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), s, 32) };
}

/// C: `void ristretto255_from_hash(unsigned char s[32], const unsigned char h[64])`
///
/// # Safety
/// `s` must point to 32 writable bytes and `h` to 64 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_from_hash(s: *mut u8, h: *const u8) {
    let mut hbuf = [0u8; 64];
    let mut sbuf = [0u8; 32];

    unsafe { ptr::copy_nonoverlapping(h, hbuf.as_mut_ptr(), 64) };
    ristretto255_from_hash(&mut sbuf, &hbuf);
    unsafe { ptr::copy_nonoverlapping(sbuf.as_ptr(), s, 32) };
}

/// C: `void sc25519_invert(unsigned char recip[32], const unsigned char s[32])`
///
/// # Safety
/// `recip` must point to 32 writable bytes and `s` to 32 readable bytes; they
/// may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_invert(recip: *mut u8, s: *const u8) {
    let mut sbuf = [0u8; 32];
    let mut rbuf = [0u8; 32];

    unsafe { ptr::copy_nonoverlapping(s, sbuf.as_mut_ptr(), 32) };
    sc25519_invert(&mut rbuf, &sbuf);
    unsafe { ptr::copy_nonoverlapping(rbuf.as_ptr(), recip, 32) };
}

/// C: `void sc25519_reduce(unsigned char s[64])`
///
/// # Safety
/// `s` must point to 64 readable and writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_reduce(s: *mut u8) {
    let mut buf = [0u8; 64];

    unsafe { ptr::copy_nonoverlapping(s, buf.as_mut_ptr(), 64) };
    sc25519_reduce(&mut buf);
    unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), s, 64) };
}

/// C: `void sc25519_mul(unsigned char s[32], const unsigned char a[32],
///                      const unsigned char b[32])`
///
/// # Safety
/// `s` must point to 32 writable bytes, `a` and `b` to 32 readable bytes each;
/// all three may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_mul(s: *mut u8, a: *const u8, b: *const u8) {
    let mut abuf = [0u8; 32];
    let mut bbuf = [0u8; 32];
    let mut sbuf = [0u8; 32];

    unsafe { ptr::copy_nonoverlapping(a, abuf.as_mut_ptr(), 32) };
    unsafe { ptr::copy_nonoverlapping(b, bbuf.as_mut_ptr(), 32) };
    sc25519_mul(&mut sbuf, &abuf, &bbuf);
    unsafe { ptr::copy_nonoverlapping(sbuf.as_ptr(), s, 32) };
}

/// C: `void sc25519_muladd(unsigned char s[32], const unsigned char a[32],
///                         const unsigned char b[32], const unsigned char c[32])`
///
/// # Safety
/// `s` must point to 32 writable bytes, `a`, `b` and `c` to 32 readable bytes
/// each; all four may alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_muladd(
    s: *mut u8,
    a: *const u8,
    b: *const u8,
    c: *const u8,
) {
    let mut abuf = [0u8; 32];
    let mut bbuf = [0u8; 32];
    let mut cbuf = [0u8; 32];
    let mut sbuf = [0u8; 32];

    unsafe { ptr::copy_nonoverlapping(a, abuf.as_mut_ptr(), 32) };
    unsafe { ptr::copy_nonoverlapping(b, bbuf.as_mut_ptr(), 32) };
    unsafe { ptr::copy_nonoverlapping(c, cbuf.as_mut_ptr(), 32) };
    sc25519_muladd(&mut sbuf, &abuf, &bbuf, &cbuf);
    unsafe { ptr::copy_nonoverlapping(sbuf.as_ptr(), s, 32) };
}

/// C: `int sc25519_is_canonical(const unsigned char s[32])`
///
/// # Safety
/// `s` must point to 32 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int {
    let sl = unsafe { core::slice::from_raw_parts(s, 32) };

    sc25519_is_canonical(sl) as c_int
}
