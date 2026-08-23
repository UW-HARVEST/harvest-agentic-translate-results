//! `ge25519` group-element section of
//! `crypto_core/ed25519/ref10/ed25519_ref10.c` (C lines 258..1200).
//!
//! ## Exported C symbols (`#[unsafe(no_mangle)] extern "C"`, names after
//! `private/quirks.h` renaming)
//!
//! ```text
//! _sodium_ge25519_frombytes                  _sodium_ge25519_p3_add
//! _sodium_ge25519_frombytes_negate_vartime   _sodium_ge25519_p3_sub
//! _sodium_ge25519_p1p1_to_p2                 _sodium_ge25519_double_scalarmult_vartime
//! _sodium_ge25519_p1p1_to_p3                 _sodium_ge25519_scalarmult
//! _sodium_ge25519_p2_to_p3                   _sodium_ge25519_scalarmult_base
//! _sodium_ge25519_p3_tobytes                 _sodium_ge25519_is_on_curve
//! _sodium_ge25519_tobytes                    _sodium_ge25519_is_on_main_subgroup
//! _sodium_ge25519_is_canonical               _sodium_ge25519_has_small_order
//! ```
//!
//! ## Rust-level `pub` API (for the sibling modules `h2c` and `ristretto`)
//!
//! ```text
//! // constructors
//! pub fn ge25519_p2_0(h: &mut Ge25519P2)
//! pub fn ge25519_p3_0(h: &mut Ge25519P3)
//! pub fn ge25519_precomp_0(h: &mut Ge25519Precomp)
//! pub fn ge25519_cached_0(h: &mut Ge25519Cached)
//! // representation changes
//! pub fn ge25519_p1p1_to_p2(r: &mut Ge25519P2, p: &Ge25519P1p1)
//! pub fn ge25519_p1p1_to_p3(r: &mut Ge25519P3, p: &Ge25519P1p1)
//! pub fn ge25519_p2_to_p3(r: &mut Ge25519P3, p: &Ge25519P2)
//! pub fn ge25519_p3_to_p2(r: &mut Ge25519P2, p: &Ge25519P3)
//! pub fn ge25519_p3_to_cached(r: &mut Ge25519Cached, p: &Ge25519P3)
//! pub fn ge25519_p3_to_precomp(pi: &mut Ge25519Precomp, p: &Ge25519P3)
//! // doubling / addition
//! pub fn ge25519_p2_dbl(r: &mut Ge25519P1p1, p: &Ge25519P2)
//! pub fn ge25519_p3_dbl(r: &mut Ge25519P1p1, p: &Ge25519P3)
//! pub fn ge25519_p3p3_dbl(r: &mut Ge25519P3, p: &Ge25519P3)
//! pub fn ge25519_add_precomp(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Precomp)
//! pub fn ge25519_sub_precomp(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Precomp)
//! pub fn ge25519_add_cached(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Cached)
//! pub fn ge25519_sub_cached(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Cached)
//! pub fn ge25519_p3_neg(r: &mut Ge25519P3, p: &Ge25519P3)
//! pub fn ge25519_p3_add(r: &mut Ge25519P3, p: &Ge25519P3, q: &Ge25519P3)
//! pub fn ge25519_p3_sub(r: &mut Ge25519P3, p: &Ge25519P3, q: &Ge25519P3)
//! pub fn ge25519_p3_dbladd(r: &mut Ge25519P3, n: c_int, q: &Ge25519P3)
//! pub fn ge25519_mul_l(r: &mut Ge25519P3, p: &Ge25519P3)
//! // serialization / deserialization
//! pub fn ge25519_frombytes(h: &mut Ge25519P3, s: &[u8; 32]) -> c_int
//! pub fn ge25519_frombytes_negate_vartime(h: &mut Ge25519P3, s: &[u8; 32]) -> c_int
//! pub fn ge25519_p3_tobytes(s: &mut [u8; 32], h: &Ge25519P3)
//! pub fn ge25519_tobytes(s: &mut [u8; 32], h: &Ge25519P2)
//! // scalar multiplication
//! pub fn ge25519_scalarmult(h: &mut Ge25519P3, a: &[u8; 32], p: &Ge25519P3)
//! pub fn ge25519_scalarmult_base(h: &mut Ge25519P3, a: &[u8; 32])
//! pub fn ge25519_double_scalarmult_vartime(r: &mut Ge25519P2, a: &[u8; 32],
//!                                          A: &Ge25519P3, b: &[u8; 32])
//! // predicates
//! pub fn ge25519_is_on_curve(p: &Ge25519P3) -> c_int
//! pub fn ge25519_is_on_main_subgroup(p: &Ge25519P3) -> c_int
//! pub fn ge25519_is_canonical(s: &[u8; 32]) -> c_int
//! pub fn ge25519_has_small_order(p: &Ge25519P3) -> c_int
//! ```
//!
//! Every routine whose C counterpart tolerates `r == p` aliasing works on local
//! copies of the field elements and only stores into `*r` at the very end, so the
//! `extern "C"` wrappers can safely be handed aliasing pointers.

use core::ffi::c_int;
use core::sync::atomic::{AtomicU8, Ordering};

use super::fe::*;
use super::tables::{BASE, BI, ED25519_D, ED25519_D2, FE25519_SQRTM1};
use super::{Fe25519, Ge25519Cached, Ge25519P1p1, Ge25519P2, Ge25519P3, Ge25519Precomp};

/* ------------------------------------------------------------------------- */
/* static volatile unsigned char optblocker_u8;                              */
/* ------------------------------------------------------------------------- */

static OPTBLOCKER_U8: AtomicU8 = AtomicU8::new(0);

#[inline(always)]
fn optblocker_u8() -> u8 {
    /* volatile read of a never-written zero byte */
    OPTBLOCKER_U8.load(Ordering::Relaxed)
}

/* ------------------------------------------------------------------------- */
/* r = p + q                                                                 */
/* ------------------------------------------------------------------------- */

pub fn ge25519_add_cached(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Cached) {
    let t0: Fe25519;
    let mut rx: Fe25519;
    let mut ry: Fe25519;
    let mut rz: Fe25519;
    let mut rt: Fe25519;

    rx = fe_add(&p.Y, &p.X);
    ry = fe_sub(&p.Y, &p.X);
    rz = fe_mul(&rx, &q.YplusX);
    ry = fe_mul(&ry, &q.YminusX);
    rt = fe_mul(&q.T2d, &p.T);
    rx = fe_mul(&p.Z, &q.Z);
    t0 = fe_add(&rx, &rx);
    rx = fe_sub(&rz, &ry);
    ry = fe_add(&rz, &ry);
    rz = fe_add(&t0, &rt);
    rt = fe_sub(&t0, &rt);

    r.X = rx;
    r.Y = ry;
    r.Z = rz;
    r.T = rt;
}

/* ------------------------------------------------------------------------- */
/* static void slide_vartime(signed char *r, const unsigned char *a)         */
/* ------------------------------------------------------------------------- */

fn slide_vartime(r: &mut [i8; 256], a: &[u8; 32]) {
    let mut i: usize;
    let mut b: usize;
    let mut k: usize;
    let mut ribs: i32;
    let mut cmp: i32;

    i = 0;
    while i < 256 {
        r[i] = (1 & ((a[i >> 3] as i32) >> (i & 7))) as i8;
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
            ribs = (r[i + b] as i32) << b;
            cmp = (r[i] as i32) + ribs;
            if cmp <= 15 {
                r[i] = cmp as i8;
                r[i + b] = 0;
            } else {
                cmp = (r[i] as i32) - ribs;
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

/* ------------------------------------------------------------------------- */
/* int ge25519_frombytes(ge25519_p3 *h, const unsigned char *s)              */
/* ------------------------------------------------------------------------- */

pub fn ge25519_frombytes(h: &mut Ge25519P3, s: &[u8; 32]) -> c_int {
    let mut u: Fe25519;
    let mut v: Fe25519;
    let mut vxx: Fe25519;
    let m_root_check: Fe25519;
    let p_root_check: Fe25519;
    let negx: Fe25519;
    let x_sqrtm1: Fe25519;
    let has_m_root: c_int;
    let has_p_root: c_int;

    fe25519_frombytes(&mut h.Y, s);
    fe25519_1(&mut h.Z);
    u = fe_sq(&h.Y);
    v = fe_mul(&u, &ED25519_D);
    u = fe_sub(&u, &h.Z); /* u = y^2-1 */
    v = fe_add(&v, &h.Z); /* v = dy^2+1 */

    h.X = fe_mul(&u, &v);
    {
        let t = h.X;
        fe25519_pow22523(&mut h.X, &t);
    }
    h.X = fe_mul(&u, &h.X); /* u((uv)^((q-5)/8)) */

    vxx = fe_sq(&h.X);
    vxx = fe_mul(&vxx, &v);
    m_root_check = fe_sub(&vxx, &u); /* vx^2-u */
    p_root_check = fe_add(&vxx, &u); /* vx^2+u */
    has_m_root = fe25519_iszero(&m_root_check);
    has_p_root = fe25519_iszero(&p_root_check);
    x_sqrtm1 = fe_mul(&h.X, &FE25519_SQRTM1); /* x*sqrt(-1) */
    fe25519_cmov(&mut h.X, &x_sqrtm1, (1 - has_m_root) as u32);

    negx = fe_neg(&h.X);
    let b = fe25519_isnegative(&h.X)
        ^ ((((s[31] as i32) >> 5) ^ (optblocker_u8() as i32)) >> 2);
    fe25519_cmov(&mut h.X, &negx, b as u32);
    h.T = fe_mul(&h.X, &h.Y);

    (has_m_root | has_p_root) - 1
}

/* ------------------------------------------------------------------------- */
/* int ge25519_frombytes_negate_vartime(ge25519_p3 *h, const unsigned char*) */
/* ------------------------------------------------------------------------- */

pub fn ge25519_frombytes_negate_vartime(h: &mut Ge25519P3, s: &[u8; 32]) -> c_int {
    let mut u: Fe25519;
    let mut v: Fe25519;
    let mut v3: Fe25519;
    let mut vxx: Fe25519;
    let m_root_check: Fe25519;
    let p_root_check: Fe25519;

    fe25519_frombytes(&mut h.Y, s);
    fe25519_1(&mut h.Z);
    u = fe_sq(&h.Y);
    v = fe_mul(&u, &ED25519_D);
    u = fe_sub(&u, &h.Z); /* u = y^2-1 */
    v = fe_add(&v, &h.Z); /* v = dy^2+1 */

    v3 = fe_sq(&v);
    v3 = fe_mul(&v3, &v); /* v3 = v^3 */
    h.X = fe_sq(&v3);
    h.X = fe_mul(&h.X, &v);
    h.X = fe_mul(&h.X, &u); /* x = uv^7 */

    {
        let t = h.X;
        fe25519_pow22523(&mut h.X, &t); /* x = (uv^7)^((q-5)/8) */
    }
    h.X = fe_mul(&h.X, &v3);
    h.X = fe_mul(&h.X, &u); /* x = uv^3(uv^7)^((q-5)/8) */

    vxx = fe_sq(&h.X);
    vxx = fe_mul(&vxx, &v);
    m_root_check = fe_sub(&vxx, &u); /* vx^2-u */
    if fe25519_iszero(&m_root_check) == 0 {
        p_root_check = fe_add(&vxx, &u); /* vx^2+u */
        if fe25519_iszero(&p_root_check) == 0 {
            return -1;
        }
        h.X = fe_mul(&h.X, &FE25519_SQRTM1);
    }

    if fe25519_isnegative(&h.X) == ((s[31] as i32) >> 7) {
        /* vartime function - compiler optimization is fine */
        h.X = fe_neg(&h.X);
    }
    h.T = fe_mul(&h.X, &h.Y);

    0
}

/* ------------------------------------------------------------------------- */
/* r = p + q  (precomputed q)                                                */
/* ------------------------------------------------------------------------- */

pub fn ge25519_add_precomp(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Precomp) {
    let t0: Fe25519;
    let mut rx: Fe25519;
    let mut ry: Fe25519;
    let mut rz: Fe25519;
    let mut rt: Fe25519;

    rx = fe_add(&p.Y, &p.X);
    ry = fe_sub(&p.Y, &p.X);
    rz = fe_mul(&rx, &q.yplusx);
    ry = fe_mul(&ry, &q.yminusx);
    rt = fe_mul(&q.xy2d, &p.T);
    t0 = fe_add(&p.Z, &p.Z);
    rx = fe_sub(&rz, &ry);
    ry = fe_add(&rz, &ry);
    rz = fe_add(&t0, &rt);
    rt = fe_sub(&t0, &rt);

    r.X = rx;
    r.Y = ry;
    r.Z = rz;
    r.T = rt;
}

/* ------------------------------------------------------------------------- */
/* r = p - q  (precomputed q)                                                */
/* ------------------------------------------------------------------------- */

pub fn ge25519_sub_precomp(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Precomp) {
    let t0: Fe25519;
    let mut rx: Fe25519;
    let mut ry: Fe25519;
    let mut rz: Fe25519;
    let mut rt: Fe25519;

    rx = fe_add(&p.Y, &p.X);
    ry = fe_sub(&p.Y, &p.X);
    rz = fe_mul(&rx, &q.yminusx);
    ry = fe_mul(&ry, &q.yplusx);
    rt = fe_mul(&q.xy2d, &p.T);
    t0 = fe_add(&p.Z, &p.Z);
    rx = fe_sub(&rz, &ry);
    ry = fe_add(&rz, &ry);
    rz = fe_sub(&t0, &rt);
    rt = fe_add(&t0, &rt);

    r.X = rx;
    r.Y = ry;
    r.Z = rz;
    r.T = rt;
}

/* ------------------------------------------------------------------------- */
/* representation changes                                                    */
/* ------------------------------------------------------------------------- */

pub fn ge25519_p1p1_to_p2(r: &mut Ge25519P2, p: &Ge25519P1p1) {
    let x = fe_mul(&p.X, &p.T);
    let y = fe_mul(&p.Y, &p.Z);
    let z = fe_mul(&p.Z, &p.T);

    r.X = x;
    r.Y = y;
    r.Z = z;
}

pub fn ge25519_p1p1_to_p3(r: &mut Ge25519P3, p: &Ge25519P1p1) {
    let x = fe_mul(&p.X, &p.T);
    let y = fe_mul(&p.Y, &p.Z);
    let z = fe_mul(&p.Z, &p.T);
    let t = fe_mul(&p.X, &p.Y);

    r.X = x;
    r.Y = y;
    r.Z = z;
    r.T = t;
}

pub fn ge25519_p2_to_p3(r: &mut Ge25519P3, p: &Ge25519P2) {
    let x = p.X;
    let y = p.Y;
    let z = p.Z;
    let t = fe_mul(&p.X, &p.Y);

    fe25519_copy(&mut r.X, &x);
    fe25519_copy(&mut r.Y, &y);
    fe25519_copy(&mut r.Z, &z);
    r.T = t;
}

pub fn ge25519_p2_0(h: &mut Ge25519P2) {
    fe25519_0(&mut h.X);
    fe25519_1(&mut h.Y);
    fe25519_1(&mut h.Z);
}

/* r = 2 * p */
pub fn ge25519_p2_dbl(r: &mut Ge25519P1p1, p: &Ge25519P2) {
    let t0: Fe25519;
    let mut rx: Fe25519;
    let mut ry: Fe25519;
    let mut rz: Fe25519;
    let mut rt: Fe25519;

    rx = fe_sq(&p.X);
    rz = fe_sq(&p.Y);
    rt = fe_sq2(&p.Z);
    ry = fe_add(&p.X, &p.Y);
    t0 = fe_sq(&ry);
    ry = fe_add(&rz, &rx);
    rz = fe_sub(&rz, &rx);
    rx = fe_sub(&t0, &ry);
    rt = fe_sub(&rt, &rz);

    r.X = rx;
    r.Y = ry;
    r.Z = rz;
    r.T = rt;
}

pub fn ge25519_p3_0(h: &mut Ge25519P3) {
    fe25519_0(&mut h.X);
    fe25519_1(&mut h.Y);
    fe25519_1(&mut h.Z);
    fe25519_0(&mut h.T);
}

pub fn ge25519_cached_0(h: &mut Ge25519Cached) {
    fe25519_1(&mut h.YplusX);
    fe25519_1(&mut h.YminusX);
    fe25519_1(&mut h.Z);
    fe25519_0(&mut h.T2d);
}

/* r = p */
pub fn ge25519_p3_to_cached(r: &mut Ge25519Cached, p: &Ge25519P3) {
    let yplusx = fe_add(&p.Y, &p.X);
    let yminusx = fe_sub(&p.Y, &p.X);
    let z = p.Z;
    let t2d = fe_mul(&p.T, &ED25519_D2);

    r.YplusX = yplusx;
    r.YminusX = yminusx;
    fe25519_copy(&mut r.Z, &z);
    r.T2d = t2d;
}

pub fn ge25519_p3_to_precomp(pi: &mut Ge25519Precomp, p: &Ge25519P3) {
    let mut recip: Fe25519 = [0; 10];
    let x: Fe25519;
    let y: Fe25519;
    let xy: Fe25519;

    fe25519_invert(&mut recip, &p.Z);
    x = fe_mul(&p.X, &recip);
    y = fe_mul(&p.Y, &recip);
    pi.yplusx = fe_add(&y, &x);
    pi.yminusx = fe_sub(&y, &x);
    xy = fe_mul(&x, &y);
    pi.xy2d = fe_mul(&xy, &ED25519_D2);
}

/* r = p */
pub fn ge25519_p3_to_p2(r: &mut Ge25519P2, p: &Ge25519P3) {
    let x = p.X;
    let y = p.Y;
    let z = p.Z;

    fe25519_copy(&mut r.X, &x);
    fe25519_copy(&mut r.Y, &y);
    fe25519_copy(&mut r.Z, &z);
}

pub fn ge25519_p3_tobytes(s: &mut [u8; 32], h: &Ge25519P3) {
    let mut recip: Fe25519 = [0; 10];
    let x: Fe25519;
    let y: Fe25519;

    fe25519_invert(&mut recip, &h.Z);
    x = fe_mul(&h.X, &recip);
    y = fe_mul(&h.Y, &recip);
    fe25519_tobytes(s, &y);
    s[31] ^= (fe25519_isnegative(&x) << 7) as u8;
}

/* r = 2 * p */
pub fn ge25519_p3_dbl(r: &mut Ge25519P1p1, p: &Ge25519P3) {
    let mut q = Ge25519P2::zeroed();
    ge25519_p3_to_p2(&mut q, p);
    ge25519_p2_dbl(r, &q);
}

pub fn ge25519_precomp_0(h: &mut Ge25519Precomp) {
    fe25519_1(&mut h.yplusx);
    fe25519_1(&mut h.yminusx);
    fe25519_0(&mut h.xy2d);
}

/* ------------------------------------------------------------------------- */
/* constant-time helpers                                                     */
/* ------------------------------------------------------------------------- */

/* HAVE_INLINE_ASM is not defined in the reference build -> portable path. */
fn equal(b: i8, c: i8) -> u8 {
    let x: u8 = (b as u8) ^ (c as u8); /* 0: yes; 1..255: no */
    let mut y: u32 = x as u32; /* 0: yes; 1..255: no */

    y = y.wrapping_sub(1);
    (((y >> 29) ^ (optblocker_u8() as u32)) >> 2) as u8 /* 1: yes; 0: no */
}

fn negative(b: i8) -> u8 {
    let x: u8 = b as u8; /* 0..127: no  128..255: yes */
    ((((x >> 5) as u32) ^ (optblocker_u8() as u32)) >> 2) as u8 /* 1: yes; 0: no */
}

fn ge25519_cmov(t: &mut Ge25519Precomp, u: &Ge25519Precomp, b: u8) {
    fe25519_cmov(&mut t.yplusx, &u.yplusx, b as u32);
    fe25519_cmov(&mut t.yminusx, &u.yminusx, b as u32);
    fe25519_cmov(&mut t.xy2d, &u.xy2d, b as u32);
}

fn ge25519_cmov_cached(t: &mut Ge25519Cached, u: &Ge25519Cached, b: u8) {
    fe25519_cmov(&mut t.YplusX, &u.YplusX, b as u32);
    fe25519_cmov(&mut t.YminusX, &u.YminusX, b as u32);
    fe25519_cmov(&mut t.Z, &u.Z, b as u32);
    fe25519_cmov(&mut t.T2d, &u.T2d, b as u32);
}

fn ge25519_cmov8(t: &mut Ge25519Precomp, precomp: &[Ge25519Precomp; 8], b: i8) {
    let mut minust = Ge25519Precomp::zeroed();
    let bnegative: u8 = negative(b);
    /* babs = b - (((-bnegative) & b) * ((signed char) 1 << 1)); */
    let babs: u8 =
        ((b as i32).wrapping_sub(((-(bnegative as i32)) & (b as i32)).wrapping_mul(1i32 << 1)))
            as u8;

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

fn ge25519_cmov8_base(t: &mut Ge25519Precomp, pos: c_int, b: i8) {
    /* base[i][j] = (j+1)*256^i*B  (fe_25_5/base.h) */
    ge25519_cmov8(t, &BASE[pos as usize], b);
}

fn ge25519_cmov8_cached(t: &mut Ge25519Cached, cached: &[Ge25519Cached; 8], b: i8) {
    let mut minust = Ge25519Cached::zeroed();
    let bnegative: u8 = negative(b);
    let babs: u8 =
        ((b as i32).wrapping_sub(((-(bnegative as i32)) & (b as i32)).wrapping_mul(1i32 << 1)))
            as u8;

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

/* ------------------------------------------------------------------------- */
/* r = p - q  (cached q)                                                     */
/* ------------------------------------------------------------------------- */

pub fn ge25519_sub_cached(r: &mut Ge25519P1p1, p: &Ge25519P3, q: &Ge25519Cached) {
    let t0: Fe25519;
    let mut rx: Fe25519;
    let mut ry: Fe25519;
    let mut rz: Fe25519;
    let mut rt: Fe25519;

    rx = fe_add(&p.Y, &p.X);
    ry = fe_sub(&p.Y, &p.X);
    rz = fe_mul(&rx, &q.YminusX);
    ry = fe_mul(&ry, &q.YplusX);
    rt = fe_mul(&q.T2d, &p.T);
    rx = fe_mul(&p.Z, &q.Z);
    t0 = fe_add(&rx, &rx);
    rx = fe_sub(&rz, &ry);
    ry = fe_add(&rz, &ry);
    rz = fe_sub(&t0, &rt);
    rt = fe_add(&t0, &rt);

    r.X = rx;
    r.Y = ry;
    r.Z = rz;
    r.T = rt;
}

/* LCOV_EXCL_START */
pub fn ge25519_tobytes(s: &mut [u8; 32], h: &Ge25519P2) {
    let mut recip: Fe25519 = [0; 10];
    let x: Fe25519;
    let y: Fe25519;

    fe25519_invert(&mut recip, &h.Z);
    x = fe_mul(&h.X, &recip);
    y = fe_mul(&h.Y, &recip);
    fe25519_tobytes(s, &y);
    s[31] ^= (fe25519_isnegative(&x) << 7) as u8;
}
/* LCOV_EXCL_STOP */

/* ------------------------------------------------------------------------- */
/* r = a * A + b * B    (signature verification only)                        */
/* ------------------------------------------------------------------------- */

pub fn ge25519_double_scalarmult_vartime(
    r: &mut Ge25519P2,
    a: &[u8; 32],
    A: &Ge25519P3,
    b: &[u8; 32],
) {
    /* Bi[8] comes from fe_25_5/base2.h */
    let mut aslide: [i8; 256] = [0; 256];
    let mut bslide: [i8; 256] = [0; 256];
    let mut Ai: [Ge25519Cached; 8] = [Ge25519Cached::zeroed(); 8]; /* A,3A,...,15A */
    let mut t = Ge25519P1p1::zeroed();
    let mut u = Ge25519P3::zeroed();
    let mut A2 = Ge25519P3::zeroed();
    let mut i: i32;

    slide_vartime(&mut aslide, a);
    slide_vartime(&mut bslide, b);

    ge25519_p3_to_cached(&mut Ai[0], A);

    ge25519_p3_dbl(&mut t, A);
    ge25519_p1p1_to_p3(&mut A2, &t);

    ge25519_add_cached(&mut t, &A2, &Ai[0]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[1], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[1]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[2], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[2]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[3], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[3]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[4], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[4]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[5], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[5]);
    ge25519_p1p1_to_p3(&mut u, &t);
    ge25519_p3_to_cached(&mut Ai[6], &u);

    ge25519_add_cached(&mut t, &A2, &Ai[6]);
    ge25519_p1p1_to_p3(&mut u, &t);
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
        let idx = i as usize;

        ge25519_p2_dbl(&mut t, r);

        if aslide[idx] > 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            let c = Ai[(aslide[idx] / 2) as usize];
            ge25519_add_cached(&mut t, &u, &c);
        } else if aslide[idx] < 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            let c = Ai[(aslide[idx].wrapping_neg() / 2) as usize];
            ge25519_sub_cached(&mut t, &u, &c);
        }

        if bslide[idx] > 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_add_precomp(&mut t, &u, &BI[(bslide[idx] / 2) as usize]);
        } else if bslide[idx] < 0 {
            ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_sub_precomp(&mut t, &u, &BI[(bslide[idx].wrapping_neg() / 2) as usize]);
        }

        ge25519_p1p1_to_p2(r, &t);
        i -= 1;
    }
}

/* ------------------------------------------------------------------------- */
/* h = a * p                                                                 */
/* ------------------------------------------------------------------------- */

pub fn ge25519_scalarmult(h: &mut Ge25519P3, a: &[u8; 32], p: &Ge25519P3) {
    let mut e: [i8; 64] = [0; 64];
    let mut carry: i8;
    let mut r = Ge25519P1p1::zeroed();
    let mut s = Ge25519P2::zeroed();
    let mut t2 = Ge25519P1p1::zeroed();
    let mut t3 = Ge25519P1p1::zeroed();
    let mut t4 = Ge25519P1p1::zeroed();
    let mut t5 = Ge25519P1p1::zeroed();
    let mut t6 = Ge25519P1p1::zeroed();
    let mut t7 = Ge25519P1p1::zeroed();
    let mut t8 = Ge25519P1p1::zeroed();
    let mut p2 = Ge25519P3::zeroed();
    let mut p3 = Ge25519P3::zeroed();
    let mut p4 = Ge25519P3::zeroed();
    let mut p5 = Ge25519P3::zeroed();
    let mut p6 = Ge25519P3::zeroed();
    let mut p7 = Ge25519P3::zeroed();
    let mut p8 = Ge25519P3::zeroed();
    let mut pi: [Ge25519Cached; 8] = [Ge25519Cached::zeroed(); 8];
    let mut t = Ge25519Cached::zeroed();
    let mut i: i32;

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

    i = 0;
    while i < 32 {
        let k = i as usize;
        e[2 * k + 0] = (((a[k] as i32) >> 0) & 15) as i8;
        e[2 * k + 1] = (((a[k] as i32) >> 4) & 15) as i8;
        i += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    i = 0;
    while i < 63 {
        let k = i as usize;
        e[k] = ((e[k] as i32) + (carry as i32)) as i8;
        carry = ((e[k] as i32) + 8) as i8;
        carry = ((carry as i32) >> 4) as i8;
        e[k] = ((e[k] as i32) - (carry as i32) * (1i32 << 4)) as i8;
        i += 1;
    }
    e[63] = ((e[63] as i32) + (carry as i32)) as i8;
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    i = 63;
    while i != 0 {
        ge25519_cmov8_cached(&mut t, &pi, e[i as usize]);
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

/* ------------------------------------------------------------------------- */
/* h = a * B (with precomputation)                                           */
/* ------------------------------------------------------------------------- */

pub fn ge25519_scalarmult_base(h: &mut Ge25519P3, a: &[u8; 32]) {
    let mut e: [i8; 64] = [0; 64];
    let mut carry: i8;
    let mut r = Ge25519P1p1::zeroed();
    let mut s = Ge25519P2::zeroed();
    let mut t = Ge25519Precomp::zeroed();
    let mut i: i32;

    i = 0;
    while i < 32 {
        let k = i as usize;
        e[2 * k + 0] = (((a[k] as i32) >> 0) & 15) as i8;
        e[2 * k + 1] = (((a[k] as i32) >> 4) & 15) as i8;
        i += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    i = 0;
    while i < 63 {
        let k = i as usize;
        e[k] = ((e[k] as i32) + (carry as i32)) as i8;
        carry = ((e[k] as i32) + 8) as i8;
        carry = ((carry as i32) >> 4) as i8;
        e[k] = ((e[k] as i32) - (carry as i32) * (1i32 << 4)) as i8;
        i += 1;
    }
    e[63] = ((e[63] as i32) + (carry as i32)) as i8;
    /* each e[i] is between -8 and 8 */

    ge25519_p3_0(h);

    i = 1;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i as usize]);
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

    i = 0;
    while i < 64 {
        ge25519_cmov8_base(&mut t, i / 2, e[i as usize]);
        ge25519_add_precomp(&mut r, h, &t);
        ge25519_p1p1_to_p3(h, &r);
        i += 2;
    }
}

/* ------------------------------------------------------------------------- */
/* p3 helpers                                                                */
/* ------------------------------------------------------------------------- */

/* r = 2p */
pub fn ge25519_p3p3_dbl(r: &mut Ge25519P3, p: &Ge25519P3) {
    let mut p1p1 = Ge25519P1p1::zeroed();

    ge25519_p3_dbl(&mut p1p1, p);
    ge25519_p1p1_to_p3(r, &p1p1);
}

/* r = -p */
pub fn ge25519_p3_neg(r: &mut Ge25519P3, p: &Ge25519P3) {
    let x = fe_neg(&p.X);
    let y = p.Y;
    let z = p.Z;
    let t = fe_neg(&p.T);

    r.X = x;
    fe25519_copy(&mut r.Y, &y);
    fe25519_copy(&mut r.Z, &z);
    r.T = t;
}

/* r = p+q */
pub fn ge25519_p3_add(r: &mut Ge25519P3, p: &Ge25519P3, q: &Ge25519P3) {
    let mut q_cached = Ge25519Cached::zeroed();
    let mut p1p1 = Ge25519P1p1::zeroed();

    ge25519_p3_to_cached(&mut q_cached, q);
    ge25519_add_cached(&mut p1p1, p, &q_cached);
    ge25519_p1p1_to_p3(r, &p1p1);
}

/* r = p-q */
pub fn ge25519_p3_sub(r: &mut Ge25519P3, p: &Ge25519P3, q: &Ge25519P3) {
    let mut q_neg = Ge25519P3::zeroed();

    ge25519_p3_neg(&mut q_neg, q);
    ge25519_p3_add(r, p, &q_neg);
}

/* r = r*(2^n)+q */
pub fn ge25519_p3_dbladd(r: &mut Ge25519P3, n: c_int, q: &Ge25519P3) {
    let mut p2 = Ge25519P2::zeroed();
    let mut p1p1 = Ge25519P1p1::zeroed();
    let mut i: c_int;

    ge25519_p3_to_p2(&mut p2, r);
    i = 0;
    while i < n {
        ge25519_p2_dbl(&mut p1p1, &p2);
        ge25519_p1p1_to_p2(&mut p2, &p1p1);
        i += 1;
    }
    ge25519_p1p1_to_p3(r, &p1p1);
    let rc = *r;
    ge25519_p3_add(r, &rc, q);
}

/* multiply by the order of the main subgroup
   l = 2^252+27742317777372353535851937790883648493 */
pub fn ge25519_mul_l(r: &mut Ge25519P3, p: &Ge25519P3) {
    let mut _10 = Ge25519P3::zeroed();
    let mut _11 = Ge25519P3::zeroed();
    let mut _100 = Ge25519P3::zeroed();
    let mut _110 = Ge25519P3::zeroed();
    let mut _1000 = Ge25519P3::zeroed();
    let mut _1011 = Ge25519P3::zeroed();
    let mut _10000 = Ge25519P3::zeroed();
    let mut _100000 = Ge25519P3::zeroed();
    let mut _100110 = Ge25519P3::zeroed();
    let mut _1000000 = Ge25519P3::zeroed();
    let mut _1010000 = Ge25519P3::zeroed();
    let mut _1010011 = Ge25519P3::zeroed();
    let mut _1100011 = Ge25519P3::zeroed();
    let mut _1100111 = Ge25519P3::zeroed();
    let mut _1101011 = Ge25519P3::zeroed();
    let mut _10010011 = Ge25519P3::zeroed();
    let mut _10010111 = Ge25519P3::zeroed();
    let mut _10111101 = Ge25519P3::zeroed();
    let mut _11010011 = Ge25519P3::zeroed();
    let mut _11100111 = Ge25519P3::zeroed();
    let mut _11101101 = Ge25519P3::zeroed();
    let mut _11110101 = Ge25519P3::zeroed();

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
    {
        let rc = *r;
        ge25519_p3_add(r, &rc, &_11110101);
    }
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

/* ------------------------------------------------------------------------- */
/* predicates                                                                */
/* ------------------------------------------------------------------------- */

pub fn ge25519_is_on_curve(p: &Ge25519P3) -> c_int {
    let x2: Fe25519;
    let y2: Fe25519;
    let z2: Fe25519;
    let z4: Fe25519;
    let mut t0: Fe25519;
    let mut t1: Fe25519;

    x2 = fe_sq(&p.X);
    y2 = fe_sq(&p.Y);
    z2 = fe_sq(&p.Z);
    t0 = fe_sub(&y2, &x2);
    t0 = fe_mul(&t0, &z2);

    t1 = fe_mul(&x2, &y2);
    t1 = fe_mul(&t1, &ED25519_D);
    z4 = fe_sq(&z2);
    t1 = fe_add(&t1, &z4);
    t0 = fe_sub(&t0, &t1);

    fe25519_iszero(&t0)
}

pub fn ge25519_is_on_main_subgroup(p: &Ge25519P3) -> c_int {
    let mut pl = Ge25519P3::zeroed();
    let t: Fe25519;

    ge25519_mul_l(&mut pl, p);

    t = fe_sub(&pl.Y, &pl.Z);

    fe25519_iszero(&pl.X) & fe25519_iszero(&t)
}

pub fn ge25519_is_canonical(s: &[u8; 32]) -> c_int {
    let mut c: u8;
    let d: u8;
    let mut i: u32;

    c = (s[31] & 0x7f) ^ 0x7f;
    i = 30;
    while i > 0 {
        c |= s[i as usize] ^ 0xff;
        i -= 1;
    }
    c = (((c as u32).wrapping_sub(1u32)) >> 8) as u8;
    d = ((0xedu32.wrapping_sub(1u32).wrapping_sub(s[0] as u32)) >> 8) as u8;

    1 - ((c & d & 1) as c_int)
}

pub fn ge25519_has_small_order(p: &Ge25519P3) -> c_int {
    let y_sqrtm1: Fe25519;
    let mut c: Fe25519;
    let mut ret: c_int = 0;

    ret |= fe25519_iszero(&p.X);
    ret |= fe25519_iszero(&p.Y);
    ret |= fe25519_iszero(&p.Z);
    y_sqrtm1 = fe_mul(&p.Y, &FE25519_SQRTM1);
    c = fe_sub(&y_sqrtm1, &p.X);
    ret |= fe25519_iszero(&c);
    c = fe_add(&y_sqrtm1, &p.X);
    ret |= fe25519_iszero(&c);

    ret
}

/* ========================================================================= */
/* exported C ABI wrappers                                                   */
/* ========================================================================= */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes(
    h: *mut Ge25519P3,
    s: *const u8,
) -> c_int {
    ge25519_frombytes(&mut *h, &*(s as *const [u8; 32]))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes_negate_vartime(
    h: *mut Ge25519P3,
    s: *const u8,
) -> c_int {
    ge25519_frombytes_negate_vartime(&mut *h, &*(s as *const [u8; 32]))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p2(
    r: *mut Ge25519P2,
    p: *const Ge25519P1p1,
) {
    let pv = *p;
    ge25519_p1p1_to_p2(&mut *r, &pv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p3(
    r: *mut Ge25519P3,
    p: *const Ge25519P1p1,
) {
    let pv = *p;
    ge25519_p1p1_to_p3(&mut *r, &pv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p2_to_p3(r: *mut Ge25519P3, p: *const Ge25519P2) {
    let pv = *p;
    ge25519_p2_to_p3(&mut *r, &pv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const Ge25519P3) {
    let hv = *h;
    ge25519_p3_tobytes(&mut *(s as *mut [u8; 32]), &hv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_tobytes(s: *mut u8, h: *const Ge25519P2) {
    let hv = *h;
    ge25519_tobytes(&mut *(s as *mut [u8; 32]), &hv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_add(
    r: *mut Ge25519P3,
    p: *const Ge25519P3,
    q: *const Ge25519P3,
) {
    let pv = *p;
    let qv = *q;
    ge25519_p3_add(&mut *r, &pv, &qv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_sub(
    r: *mut Ge25519P3,
    p: *const Ge25519P3,
    q: *const Ge25519P3,
) {
    let pv = *p;
    let qv = *q;
    ge25519_p3_sub(&mut *r, &pv, &qv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_double_scalarmult_vartime(
    r: *mut Ge25519P2,
    a: *const u8,
    A: *const Ge25519P3,
    b: *const u8,
) {
    let av = *A;
    ge25519_double_scalarmult_vartime(
        &mut *r,
        &*(a as *const [u8; 32]),
        &av,
        &*(b as *const [u8; 32]),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult(
    h: *mut Ge25519P3,
    a: *const u8,
    p: *const Ge25519P3,
) {
    let pv = *p;
    let av = *(a as *const [u8; 32]);
    ge25519_scalarmult(&mut *h, &av, &pv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult_base(h: *mut Ge25519P3, a: *const u8) {
    let av = *(a as *const [u8; 32]);
    ge25519_scalarmult_base(&mut *h, &av);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_curve(p: *const Ge25519P3) -> c_int {
    ge25519_is_on_curve(&*p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_main_subgroup(p: *const Ge25519P3) -> c_int {
    ge25519_is_on_main_subgroup(&*p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int {
    ge25519_is_canonical(&*(s as *const [u8; 32]))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_has_small_order(p: *const Ge25519P3) -> c_int {
    ge25519_has_small_order(&*p)
}
