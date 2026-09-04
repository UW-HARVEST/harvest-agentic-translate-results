//! Translation of the `ge25519_*` / `ristretto255_*` group-element portion of
//! `crypto_core/ed25519/ref10/ed25519_ref10.c` (C lines 263..1201 and
//! 2597..2992).
//!
//! `HAVE_TI_MODE` is undefined, so `fe25519` is `int32_t[10]` (fe_25_5) and the
//! portable `#else` paths are taken. `HAVE_INLINE_ASM` is undefined, so `equal`
//! and `negative` use their portable implementations.

use super::base::{
    base, ed25519_A, ed25519_A_32, ed25519_d, ed25519_d2, ed25519_invsqrtamd, ed25519_onemsqd,
    ed25519_sqdmone, ed25519_sqrtadm1, ed25519_sqrtam2, fe25519_sqrtm1, Bi,
};
use super::{
    ge25519_cached, ge25519_p1p1, ge25519_p2, ge25519_p3, ge25519_precomp, load_3, load_4,
};
use crate::crypto_core::ed25519_ref10::fe::*;

/*
r = p + q
*/

pub unsafe fn ge25519_add_cached(
    r: *mut ge25519_p1p1,
    p: *const ge25519_p3,
    q: *const ge25519_cached,
) {
    let mut t0: [i32; 10] = [0; 10];

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

pub unsafe fn slide_vartime(r: *mut i8, a: *const u8) {
    let mut i: i32;
    let mut b: i32;
    let mut k: i32;
    let mut ribs: i32;
    let mut cmp: i32;

    i = 0;
    while i < 256 {
        *r.add(i as usize) = (1 & (*a.add((i >> 3) as usize) as i32 >> (i & 7))) as i8;
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
            ribs = (*r.add((i + b) as usize) as i32) << b;
            cmp = *r.add(i as usize) as i32 + ribs;
            if cmp <= 15 {
                *r.add(i as usize) = cmp as i8;
                *r.add((i + b) as usize) = 0;
            } else {
                cmp = *r.add(i as usize) as i32 - ribs;
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

pub static mut optblocker_u8: u8 = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> i32 {
    let mut u: [i32; 10] = [0; 10];
    let mut v: [i32; 10] = [0; 10];
    let mut vxx: [i32; 10] = [0; 10];
    let mut m_root_check: [i32; 10] = [0; 10];
    let mut p_root_check: [i32; 10] = [0; 10];
    let mut negx: [i32; 10] = [0; 10];
    let mut x_sqrtm1: [i32; 10] = [0; 10];
    let has_m_root: i32;
    let has_p_root: i32;

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
    fe25519_mul(
        x_sqrtm1.as_mut_ptr(),
        (*h).X.as_ptr(),
        fe25519_sqrtm1.as_ptr(),
    ); /* x*sqrt(-1) */
    fe25519_cmov(
        (*h).X.as_mut_ptr(),
        x_sqrtm1.as_ptr(),
        (1 - has_m_root) as u32,
    );

    fe25519_neg(negx.as_mut_ptr(), (*h).X.as_ptr());
    fe25519_cmov(
        (*h).X.as_mut_ptr(),
        negx.as_ptr(),
        (fe25519_isnegative((*h).X.as_ptr())
            ^ (((*s.add(31) as i32 >> 5) ^ optblocker_u8 as i32) >> 2)) as u32,
    );
    fe25519_mul((*h).T.as_mut_ptr(), (*h).X.as_ptr(), (*h).Y.as_ptr());

    (has_m_root | has_p_root) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes_negate_vartime(
    h: *mut ge25519_p3,
    s: *const u8,
) -> i32 {
    let mut u: [i32; 10] = [0; 10];
    let mut v: [i32; 10] = [0; 10];
    let mut v3: [i32; 10] = [0; 10];
    let mut vxx: [i32; 10] = [0; 10];
    let mut m_root_check: [i32; 10] = [0; 10];
    let mut p_root_check: [i32; 10] = [0; 10];

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
        fe25519_mul(
            (*h).X.as_mut_ptr(),
            (*h).X.as_ptr(),
            fe25519_sqrtm1.as_ptr(),
        );
    }

    if fe25519_isnegative((*h).X.as_ptr()) == (*s.add(31) as i32 >> 7) {
        /* vartime function - compiler optimization is fine */
        fe25519_neg((*h).X.as_mut_ptr(), (*h).X.as_ptr());
    }
    fe25519_mul((*h).T.as_mut_ptr(), (*h).X.as_ptr(), (*h).Y.as_ptr());

    0
}

/*
r = p + q
*/

pub unsafe fn ge25519_add_precomp(
    r: *mut ge25519_p1p1,
    p: *const ge25519_p3,
    q: *const ge25519_precomp,
) {
    let mut t0: [i32; 10] = [0; 10];

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

pub unsafe fn ge25519_sub_precomp(
    r: *mut ge25519_p1p1,
    p: *const ge25519_p3,
    q: *const ge25519_precomp,
) {
    let mut t0: [i32; 10] = [0; 10];

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

pub unsafe fn ge25519_p2_0(h: *mut ge25519_p2) {
    fe25519_0((*h).X.as_mut_ptr());
    fe25519_1((*h).Y.as_mut_ptr());
    fe25519_1((*h).Z.as_mut_ptr());
}

/*
r = 2 * p
*/

pub unsafe fn ge25519_p2_dbl(r: *mut ge25519_p1p1, p: *const ge25519_p2) {
    let mut t0: [i32; 10] = [0; 10];

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

pub unsafe fn ge25519_p3_0(h: *mut ge25519_p3) {
    fe25519_0((*h).X.as_mut_ptr());
    fe25519_1((*h).Y.as_mut_ptr());
    fe25519_1((*h).Z.as_mut_ptr());
    fe25519_0((*h).T.as_mut_ptr());
}

pub unsafe fn ge25519_cached_0(h: *mut ge25519_cached) {
    fe25519_1((*h).YplusX.as_mut_ptr());
    fe25519_1((*h).YminusX.as_mut_ptr());
    fe25519_1((*h).Z.as_mut_ptr());
    fe25519_0((*h).T2d.as_mut_ptr());
}

/*
r = p
*/

pub unsafe fn ge25519_p3_to_cached(r: *mut ge25519_cached, p: *const ge25519_p3) {
    fe25519_add((*r).YplusX.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_sub((*r).YminusX.as_mut_ptr(), (*p).Y.as_ptr(), (*p).X.as_ptr());
    fe25519_copy((*r).Z.as_mut_ptr(), (*p).Z.as_ptr());
    fe25519_mul((*r).T2d.as_mut_ptr(), (*p).T.as_ptr(), ed25519_d2.as_ptr());
}

pub unsafe fn ge25519_p3_to_precomp(pi: *mut ge25519_precomp, p: *const ge25519_p3) {
    let mut recip: [i32; 10] = [0; 10];
    let mut x: [i32; 10] = [0; 10];
    let mut y: [i32; 10] = [0; 10];
    let mut xy: [i32; 10] = [0; 10];

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

pub unsafe fn ge25519_p3_to_p2(r: *mut ge25519_p2, p: *const ge25519_p3) {
    fe25519_copy((*r).X.as_mut_ptr(), (*p).X.as_ptr());
    fe25519_copy((*r).Y.as_mut_ptr(), (*p).Y.as_ptr());
    fe25519_copy((*r).Z.as_mut_ptr(), (*p).Z.as_ptr());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3) {
    let mut recip: [i32; 10] = [0; 10];
    let mut x: [i32; 10] = [0; 10];
    let mut y: [i32; 10] = [0; 10];

    _sodium_fe25519_invert(recip.as_mut_ptr(), (*h).Z.as_ptr());
    fe25519_mul(x.as_mut_ptr(), (*h).X.as_ptr(), recip.as_ptr());
    fe25519_mul(y.as_mut_ptr(), (*h).Y.as_ptr(), recip.as_ptr());
    _sodium_fe25519_tobytes(s, y.as_ptr());
    *s.add(31) ^= (fe25519_isnegative(x.as_ptr()) << 7) as u8;
}

/*
r = 2 * p
*/

pub unsafe fn ge25519_p3_dbl(r: *mut ge25519_p1p1, p: *const ge25519_p3) {
    let mut q: ge25519_p2 = core::mem::zeroed();
    ge25519_p3_to_p2(&mut q, p);
    ge25519_p2_dbl(r, &q);
}

pub unsafe fn ge25519_precomp_0(h: *mut ge25519_precomp) {
    fe25519_1((*h).yplusx.as_mut_ptr());
    fe25519_1((*h).yminusx.as_mut_ptr());
    fe25519_0((*h).xy2d.as_mut_ptr());
}

pub unsafe fn equal(b: i8, c: i8) -> u8 {
    let x: u8 = (b as u8) ^ (c as u8); /* 0: yes; 1..255: no */
    let mut y: u32 = x as u32; /* 0: yes; 1..255: no */

    y = y.wrapping_sub(1);
    (((y >> 29) ^ optblocker_u8 as u32) >> 2) as u8 /* 1: yes; 0: no */
}

pub unsafe fn negative(b: i8) -> u8 {
    let x: u8 = b as u8; /* 0..127: no 128..255: yes */
    (((x as u32 >> 5) ^ optblocker_u8 as u32) >> 2) as u8 /* 1: yes; 0: no */
}

pub unsafe fn ge25519_cmov(t: *mut ge25519_precomp, u: *const ge25519_precomp, b: u8) {
    fe25519_cmov((*t).yplusx.as_mut_ptr(), (*u).yplusx.as_ptr(), b as u32);
    fe25519_cmov((*t).yminusx.as_mut_ptr(), (*u).yminusx.as_ptr(), b as u32);
    fe25519_cmov((*t).xy2d.as_mut_ptr(), (*u).xy2d.as_ptr(), b as u32);
}

pub unsafe fn ge25519_cmov_cached(t: *mut ge25519_cached, u: *const ge25519_cached, b: u8) {
    fe25519_cmov((*t).YplusX.as_mut_ptr(), (*u).YplusX.as_ptr(), b as u32);
    fe25519_cmov((*t).YminusX.as_mut_ptr(), (*u).YminusX.as_ptr(), b as u32);
    fe25519_cmov((*t).Z.as_mut_ptr(), (*u).Z.as_ptr(), b as u32);
    fe25519_cmov((*t).T2d.as_mut_ptr(), (*u).T2d.as_ptr(), b as u32);
}

pub unsafe fn ge25519_cmov8(t: *mut ge25519_precomp, precomp: *const ge25519_precomp, b: i8) {
    let mut minust: ge25519_precomp = core::mem::zeroed();
    let bnegative: u8 = negative(b);
    let babs: u8 = (b as i32
        - (((bnegative as i8).wrapping_neg() as i32 & b as i32) * ((1i8 << 1) as i32)))
        as u8;

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

pub unsafe fn ge25519_cmov8_base(t: *mut ge25519_precomp, pos: i32, b: i8) {
    ge25519_cmov8(t, base[pos as usize].as_ptr(), b);
}

pub unsafe fn ge25519_cmov8_cached(t: *mut ge25519_cached, cached: *const ge25519_cached, b: i8) {
    let mut minust: ge25519_cached = core::mem::zeroed();
    let bnegative: u8 = negative(b);
    let babs: u8 = (b as i32
        - (((bnegative as i8).wrapping_neg() as i32 & b as i32) * ((1i8 << 1) as i32)))
        as u8;

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

pub unsafe fn ge25519_sub_cached(
    r: *mut ge25519_p1p1,
    p: *const ge25519_p3,
    q: *const ge25519_cached,
) {
    let mut t0: [i32; 10] = [0; 10];

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
    let mut recip: [i32; 10] = [0; 10];
    let mut x: [i32; 10] = [0; 10];
    let mut y: [i32; 10] = [0; 10];

    _sodium_fe25519_invert(recip.as_mut_ptr(), (*h).Z.as_ptr());
    fe25519_mul(x.as_mut_ptr(), (*h).X.as_ptr(), recip.as_ptr());
    fe25519_mul(y.as_mut_ptr(), (*h).Y.as_ptr(), recip.as_ptr());
    _sodium_fe25519_tobytes(s, y.as_ptr());
    *s.add(31) ^= (fe25519_isnegative(x.as_ptr()) << 7) as u8;
}
/* LCOV_EXCL_STOP */

/*
r = a * A + b * B
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_double_scalarmult_vartime(
    r: *mut ge25519_p2,
    a: *const u8,
    A: *const ge25519_p3,
    b: *const u8,
) {
    let mut aslide: [i8; 256] = [0; 256];
    let mut bslide: [i8; 256] = [0; 256];
    let mut Ai: [ge25519_cached; 8] = core::mem::zeroed(); /* A,3A,5A,7A,9A,11A,13A,15A */
    let mut t: ge25519_p1p1 = core::mem::zeroed();
    let mut u: ge25519_p3 = core::mem::zeroed();
    let mut A2: ge25519_p3 = core::mem::zeroed();
    let mut i: i32;

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
            ge25519_add_precomp(&mut t, &u, &Bi[(bslide[i as usize] / 2) as usize]);
        } else if bslide[i as usize] < 0 {
            _sodium_ge25519_p1p1_to_p3(&mut u, &t);
            ge25519_sub_precomp(&mut t, &u, &Bi[((-bslide[i as usize]) / 2) as usize]);
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
    let mut r: ge25519_p1p1 = core::mem::zeroed();
    let mut s: ge25519_p2 = core::mem::zeroed();
    let mut t2: ge25519_p1p1 = core::mem::zeroed();
    let mut t3: ge25519_p1p1 = core::mem::zeroed();
    let mut t4: ge25519_p1p1 = core::mem::zeroed();
    let mut t5: ge25519_p1p1 = core::mem::zeroed();
    let mut t6: ge25519_p1p1 = core::mem::zeroed();
    let mut t7: ge25519_p1p1 = core::mem::zeroed();
    let mut t8: ge25519_p1p1 = core::mem::zeroed();
    let mut p2: ge25519_p3 = core::mem::zeroed();
    let mut p3: ge25519_p3 = core::mem::zeroed();
    let mut p4: ge25519_p3 = core::mem::zeroed();
    let mut p5: ge25519_p3 = core::mem::zeroed();
    let mut p6: ge25519_p3 = core::mem::zeroed();
    let mut p7: ge25519_p3 = core::mem::zeroed();
    let mut p8: ge25519_p3 = core::mem::zeroed();
    let mut pi: [ge25519_cached; 8] = core::mem::zeroed();
    let mut t: ge25519_cached = core::mem::zeroed();
    let mut i: i32;

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
        e[(2 * i + 0) as usize] = ((*a.add(i as usize) as i32 >> 0) & 15) as i8;
        e[(2 * i + 1) as usize] = ((*a.add(i as usize) as i32 >> 4) & 15) as i8;
        i += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    i = 0;
    while i < 63 {
        e[i as usize] = (e[i as usize] as i32 + carry as i32) as i8;
        carry = (e[i as usize] as i32 + 8) as i8;
        carry = (carry as i32 >> 4) as i8;
        e[i as usize] = (e[i as usize] as i32 - carry as i32 * ((1i8 << 4) as i32)) as i8;
        i += 1;
    }
    e[63] = (e[63] as i32 + carry as i32) as i8;
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
    let mut r: ge25519_p1p1 = core::mem::zeroed();
    let mut s: ge25519_p2 = core::mem::zeroed();
    let mut t: ge25519_precomp = core::mem::zeroed();
    let mut i: i32;

    i = 0;
    while i < 32 {
        e[(2 * i + 0) as usize] = ((*a.add(i as usize) as i32 >> 0) & 15) as i8;
        e[(2 * i + 1) as usize] = ((*a.add(i as usize) as i32 >> 4) & 15) as i8;
        i += 1;
    }
    /* each e[i] is between 0 and 15 */
    /* e[63] is between 0 and 7 */

    carry = 0;
    i = 0;
    while i < 63 {
        e[i as usize] = (e[i as usize] as i32 + carry as i32) as i8;
        carry = (e[i as usize] as i32 + 8) as i8;
        carry = (carry as i32 >> 4) as i8;
        e[i as usize] = (e[i as usize] as i32 - carry as i32 * ((1i8 << 4) as i32)) as i8;
        i += 1;
    }
    e[63] = (e[63] as i32 + carry as i32) as i8;
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
pub unsafe fn ge25519_p3p3_dbl(r: *mut ge25519_p3, p: *const ge25519_p3) {
    let mut p1p1: ge25519_p1p1 = core::mem::zeroed();

    ge25519_p3_dbl(&mut p1p1, p);
    _sodium_ge25519_p1p1_to_p3(r, &p1p1);
}

/* r = -p */
pub unsafe fn ge25519_p3_neg(r: *mut ge25519_p3, p: *const ge25519_p3) {
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
    let mut q_cached: ge25519_cached = core::mem::zeroed();
    let mut p1p1: ge25519_p1p1 = core::mem::zeroed();

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
    let mut q_neg: ge25519_p3 = core::mem::zeroed();

    ge25519_p3_neg(&mut q_neg, q);
    _sodium_ge25519_p3_add(r, p, &q_neg);
}

/* r = r*(2^n)+q */
pub unsafe fn ge25519_p3_dbladd(r: *mut ge25519_p3, n: i32, q: *const ge25519_p3) {
    let mut p2: ge25519_p2 = core::mem::zeroed();
    let mut p1p1: ge25519_p1p1 = core::mem::zeroed();
    let mut i: i32;

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
pub unsafe fn ge25519_mul_l(r: *mut ge25519_p3, p: *const ge25519_p3) {
    let mut _10: ge25519_p3 = core::mem::zeroed();
    let mut _11: ge25519_p3 = core::mem::zeroed();
    let mut _100: ge25519_p3 = core::mem::zeroed();
    let mut _110: ge25519_p3 = core::mem::zeroed();
    let mut _1000: ge25519_p3 = core::mem::zeroed();
    let mut _1011: ge25519_p3 = core::mem::zeroed();
    let mut _10000: ge25519_p3 = core::mem::zeroed();
    let mut _100000: ge25519_p3 = core::mem::zeroed();
    let mut _100110: ge25519_p3 = core::mem::zeroed();
    let mut _1000000: ge25519_p3 = core::mem::zeroed();
    let mut _1010000: ge25519_p3 = core::mem::zeroed();
    let mut _1010011: ge25519_p3 = core::mem::zeroed();
    let mut _1100011: ge25519_p3 = core::mem::zeroed();
    let mut _1100111: ge25519_p3 = core::mem::zeroed();
    let mut _1101011: ge25519_p3 = core::mem::zeroed();
    let mut _10010011: ge25519_p3 = core::mem::zeroed();
    let mut _10010111: ge25519_p3 = core::mem::zeroed();
    let mut _10111101: ge25519_p3 = core::mem::zeroed();
    let mut _11010011: ge25519_p3 = core::mem::zeroed();
    let mut _11100111: ge25519_p3 = core::mem::zeroed();
    let mut _11101101: ge25519_p3 = core::mem::zeroed();
    let mut _11110101: ge25519_p3 = core::mem::zeroed();

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
pub unsafe extern "C" fn _sodium_ge25519_is_on_curve(p: *const ge25519_p3) -> i32 {
    let mut x2: [i32; 10] = [0; 10];
    let mut y2: [i32; 10] = [0; 10];
    let mut z2: [i32; 10] = [0; 10];
    let mut z4: [i32; 10] = [0; 10];
    let mut t0: [i32; 10] = [0; 10];
    let mut t1: [i32; 10] = [0; 10];

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
pub unsafe extern "C" fn _sodium_ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> i32 {
    let mut pl: ge25519_p3 = core::mem::zeroed();
    let mut t: [i32; 10] = [0; 10];

    ge25519_mul_l(&mut pl, p);

    fe25519_sub(t.as_mut_ptr(), pl.Y.as_ptr(), pl.Z.as_ptr());

    fe25519_iszero(pl.X.as_ptr()) & fe25519_iszero(t.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_canonical(s: *const u8) -> i32 {
    let mut c: u8;
    let d: u8;
    let mut i: u32;

    c = (*s.add(31) & 0x7f) ^ 0x7f;
    i = 30;
    while i > 0 {
        c |= *s.add(i as usize) ^ 0xff;
        i -= 1;
    }
    c = (((c as u32).wrapping_sub(1u32)) >> 8) as u8;
    d = ((0xedu32.wrapping_sub(1u32).wrapping_sub(*s.add(0) as u32)) >> 8) as u8;

    1 - (c as i32 & d as i32 & 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> i32 {
    let mut y_sqrtm1: [i32; 10] = [0; 10];
    let mut c: [i32; 10] = [0; 10];
    let mut ret: i32 = 0;

    ret |= fe25519_iszero((*p).X.as_ptr());
    ret |= fe25519_iszero((*p).Y.as_ptr());
    ret |= fe25519_iszero((*p).Z.as_ptr());
    fe25519_mul(
        y_sqrtm1.as_mut_ptr(),
        (*p).Y.as_ptr(),
        fe25519_sqrtm1.as_ptr(),
    );
    fe25519_sub(c.as_mut_ptr(), y_sqrtm1.as_ptr(), (*p).X.as_ptr());
    ret |= fe25519_iszero(c.as_ptr());
    fe25519_add(c.as_mut_ptr(), y_sqrtm1.as_ptr(), (*p).X.as_ptr());
    ret |= fe25519_iszero(c.as_ptr());

    ret
}

/* ---------------------------------------------------------------------- */
/* C lines 2597..2992                                                     */
/* ---------------------------------------------------------------------- */

/* montgomery to edwards */
pub unsafe fn ge25519_mont_to_ed(xed: *mut i32, yed: *mut i32, x: *const i32, y: *const i32) {
    let mut one: [i32; 10] = [0; 10];
    let mut x_plus_one: [i32; 10] = [0; 10];
    let mut x_minus_one: [i32; 10] = [0; 10];
    let mut x_plus_one_y_inv: [i32; 10] = [0; 10];

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
        fe25519_iszero(x_plus_one_y_inv.as_ptr()) as u32,
    );
}

/* montgomery -- recover y = sqrt(x^3 + A*x^2 + x) */
pub unsafe fn ge25519_xmont_to_ymont(y: *mut i32, x: *const i32) -> i32 {
    let mut x2: [i32; 10] = [0; 10];
    let mut x3: [i32; 10] = [0; 10];

    fe25519_sq(x2.as_mut_ptr(), x);
    fe25519_mul(x3.as_mut_ptr(), x, x2.as_ptr());
    fe25519_mul32(x2.as_mut_ptr(), x2.as_ptr(), ed25519_A_32 as u32);
    fe25519_add(y, x3.as_ptr(), x);
    fe25519_add(y, y, x2.as_ptr());

    fe25519_sqrt(y, y)
}

/* multiply by the cofactor */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_clear_cofactor(p3: *mut ge25519_p3) {
    let mut p1: ge25519_p1p1 = core::mem::zeroed();
    let mut p2: ge25519_p2 = core::mem::zeroed();

    ge25519_p3_dbl(&mut p1, p3);
    _sodium_ge25519_p1p1_to_p2(&mut p2, &p1);
    ge25519_p2_dbl(&mut p1, &p2);
    _sodium_ge25519_p1p1_to_p2(&mut p2, &p1);
    ge25519_p2_dbl(&mut p1, &p2);
    _sodium_ge25519_p1p1_to_p3(p3, &p1);
}

pub unsafe fn ge25519_elligator2(x: *mut i32, y: *mut i32, r: *const i32, notsquare_p: *mut i32) {
    let mut gx1: [i32; 10] = [0; 10];
    let mut rr2: [i32; 10] = [0; 10];
    let mut x2: [i32; 10] = [0; 10];
    let mut x3: [i32; 10] = [0; 10];
    let mut negx: [i32; 10] = [0; 10];
    let notsquare: i32;

    fe25519_sq2(rr2.as_mut_ptr(), r);
    rr2[0] += 1;
    _sodium_fe25519_invert(rr2.as_mut_ptr(), rr2.as_ptr());
    fe25519_mul32(x, rr2.as_ptr(), ed25519_A_32 as u32);
    fe25519_neg(x, x); /* x=x1 */

    fe25519_sq(x2.as_mut_ptr(), x);
    fe25519_mul(x3.as_mut_ptr(), x, x2.as_ptr());
    fe25519_mul32(x2.as_mut_ptr(), x2.as_ptr(), ed25519_A_32 as u32); /* x2 = A*x1^2 */
    fe25519_add(gx1.as_mut_ptr(), x3.as_ptr(), x);
    fe25519_add(gx1.as_mut_ptr(), gx1.as_ptr(), x2.as_ptr()); /* gx1 = x1^3 + A*x1^2 + x1 */

    notsquare = fe25519_notsquare(gx1.as_ptr());

    /* gx1 not a square  => x = -x1-A */
    fe25519_neg(negx.as_mut_ptr(), x);
    fe25519_cmov(x, negx.as_ptr(), notsquare as u32);
    fe25519_0(x2.as_mut_ptr());
    fe25519_cmov(x2.as_mut_ptr(), ed25519_A.as_ptr(), notsquare as u32);
    fe25519_sub(x, x, x2.as_ptr());

    /* y = sqrt(gx1) or sqrt(gx2) with gx2 = gx1 * (A+x1) / -x1 */
    /* but it is about as fast to just recompute from the curve equation. */
    if ge25519_xmont_to_ymont(y, x) != 0 {
        crate::abort(); /* LCOV_EXCL_LINE */
    }
    *notsquare_p = notsquare;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8) {
    let mut p3: ge25519_p3 = core::mem::zeroed();
    let mut x: [i32; 10] = [0; 10];
    let mut y: [i32; 10] = [0; 10];
    let mut negxed: [i32; 10] = [0; 10];
    let mut r_fe: [i32; 10] = [0; 10];
    let mut notsquare: i32 = 0;
    let x_sign: u8;

    crate::common::memcpy(s, r, 32);
    x_sign = (((*s.add(31) as i32 >> 5) ^ optblocker_u8 as i32) >> 2) as u8;
    *s.add(31) &= 0x7f;
    _sodium_fe25519_frombytes(r_fe.as_mut_ptr(), s);

    ge25519_elligator2(
        x.as_mut_ptr(),
        y.as_mut_ptr(),
        r_fe.as_ptr(),
        &mut notsquare,
    );

    ge25519_mont_to_ed(p3.X.as_mut_ptr(), p3.Y.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    fe25519_neg(negxed.as_mut_ptr(), p3.X.as_ptr());
    fe25519_cmov(
        p3.X.as_mut_ptr(),
        negxed.as_ptr(),
        (fe25519_isnegative(p3.X.as_ptr()) ^ x_sign as i32) as u32,
    );

    fe25519_1(p3.Z.as_mut_ptr());
    fe25519_mul(p3.T.as_mut_ptr(), p3.X.as_ptr(), p3.Y.as_ptr());
    _sodium_ge25519_clear_cofactor(&mut p3);
    _sodium_ge25519_p3_tobytes(s, &p3);
}

pub unsafe fn fe25519_reduce64(fe_f: *mut i32, h: *const u8) {
    let mut fl: [u8; 32] = [0; 32];
    let mut gl: [u8; 32] = [0; 32];
    let mut fe_g: [i32; 10] = [0; 10];
    let mut i: usize;

    crate::common::memcpy(fl.as_mut_ptr(), h, 32);
    crate::common::memcpy(gl.as_mut_ptr(), h.add(32), 32);
    fl[31] &= 0x7f;
    gl[31] &= 0x7f;
    _sodium_fe25519_frombytes(fe_f, fl.as_ptr());
    _sodium_fe25519_frombytes(fe_g.as_mut_ptr(), gl.as_ptr());
    *fe_f.add(0) += (((*h.add(31) as i32 >> 5) ^ optblocker_u8 as i32) >> 2) * 19
        + (((*h.add(63) as i32 >> 5) ^ optblocker_u8 as i32) >> 2) * 722;
    i = 0;
    while i < core::mem::size_of::<[i32; 10]>() / core::mem::size_of::<i32>() {
        *fe_f.add(i) += 38 * fe_g[i];
        i += 1;
    }
    fe25519_reduce(fe_f, fe_f);
}

/* LCOV_EXCL_START */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8) {
    let mut p3: ge25519_p3 = core::mem::zeroed();
    let mut fe_f: [i32; 10] = [0; 10];
    let mut x: [i32; 10] = [0; 10];
    let mut y: [i32; 10] = [0; 10];
    let mut negy: [i32; 10] = [0; 10];
    let mut notsquare: i32 = 0;
    let y_sign: u8;

    fe25519_reduce64(fe_f.as_mut_ptr(), h);
    ge25519_elligator2(
        x.as_mut_ptr(),
        y.as_mut_ptr(),
        fe_f.as_ptr(),
        &mut notsquare,
    );

    y_sign = (notsquare ^ 1) as u8;
    fe25519_neg(negy.as_mut_ptr(), y.as_ptr());
    fe25519_cmov(
        y.as_mut_ptr(),
        negy.as_ptr(),
        (fe25519_isnegative(y.as_ptr()) ^ y_sign as i32) as u32,
    );

    ge25519_mont_to_ed(p3.X.as_mut_ptr(), p3.Y.as_mut_ptr(), x.as_ptr(), y.as_ptr());

    fe25519_1(p3.Z.as_mut_ptr());
    fe25519_mul(p3.T.as_mut_ptr(), p3.X.as_ptr(), p3.Y.as_ptr());
    _sodium_ge25519_clear_cofactor(&mut p3);
    _sodium_ge25519_p3_tobytes(s, &p3);
}
/* LCOV_EXCL_STOP */

/* Ristretto group */

pub unsafe fn ristretto255_sqrt_ratio_m1(x: *mut i32, u: *const i32, v: *const i32) -> i32 {
    let mut v3: [i32; 10] = [0; 10];
    let mut vxx: [i32; 10] = [0; 10];
    let mut m_root_check: [i32; 10] = [0; 10];
    let mut p_root_check: [i32; 10] = [0; 10];
    let mut f_root_check: [i32; 10] = [0; 10];
    let mut x_sqrtm1: [i32; 10] = [0; 10];
    let has_m_root: i32;
    let has_p_root: i32;
    let has_f_root: i32;

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
    fe25519_add(
        f_root_check.as_mut_ptr(),
        vxx.as_ptr(),
        f_root_check.as_ptr(),
    ); /* vx^2+u*sqrt(-1) */
    has_m_root = fe25519_iszero(m_root_check.as_ptr());
    has_p_root = fe25519_iszero(p_root_check.as_ptr());
    has_f_root = fe25519_iszero(f_root_check.as_ptr());
    fe25519_mul(x_sqrtm1.as_mut_ptr(), x, fe25519_sqrtm1.as_ptr()); /* x*sqrt(-1) */

    fe25519_cmov(x, x_sqrtm1.as_ptr(), (has_p_root | has_f_root) as u32);
    fe25519_abs(x);

    has_m_root | has_p_root
}

pub unsafe fn ristretto255_is_canonical(s: *const u8) -> i32 {
    let mut c: u8;
    let d: u8;
    let e: u8;
    let mut i: u32;

    c = (*s.add(31) & 0x7f) ^ 0x7f;
    i = 30;
    while i > 0 {
        c |= *s.add(i as usize) ^ 0xff;
        i -= 1;
    }
    c = (((c as u32).wrapping_sub(1u32)) >> 8) as u8;
    d = ((0xedu32.wrapping_sub(1u32).wrapping_sub(*s.add(0) as u32)) >> 8) as u8;
    e = (((*s.add(31) as i32 >> 5) ^ optblocker_u8 as i32) >> 2) as u8;

    1 - (((c as i32 & d as i32) | e as i32 | *s.add(0) as i32) & 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_frombytes(h: *mut ge25519_p3, s: *const u8) -> i32 {
    let mut inv_sqrt: [i32; 10] = [0; 10];
    let mut one: [i32; 10] = [0; 10];
    let mut s_: [i32; 10] = [0; 10];
    let mut ss: [i32; 10] = [0; 10];
    let mut u1: [i32; 10] = [0; 10];
    let mut u2: [i32; 10] = [0; 10];
    let mut u1u1: [i32; 10] = [0; 10];
    let mut u2u2: [i32; 10] = [0; 10];
    let mut v: [i32; 10] = [0; 10];
    let mut v_u2u2: [i32; 10] = [0; 10];
    let notsquare: i32;

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
    let mut den1: [i32; 10] = [0; 10];
    let mut den2: [i32; 10] = [0; 10];
    let mut den_inv: [i32; 10] = [0; 10];
    let mut eden: [i32; 10] = [0; 10];
    let mut inv_sqrt: [i32; 10] = [0; 10];
    let mut ix: [i32; 10] = [0; 10];
    let mut iy: [i32; 10] = [0; 10];
    let mut one: [i32; 10] = [0; 10];
    let mut s_: [i32; 10] = [0; 10];
    let mut t_z_inv: [i32; 10] = [0; 10];
    let mut u1: [i32; 10] = [0; 10];
    let mut u2: [i32; 10] = [0; 10];
    let mut u1_u2u2: [i32; 10] = [0; 10];
    let mut x_: [i32; 10] = [0; 10];
    let mut y_: [i32; 10] = [0; 10];
    let mut x_z_inv: [i32; 10] = [0; 10];
    let mut z_inv: [i32; 10] = [0; 10];
    let mut zmy: [i32; 10] = [0; 10];
    let rotate: i32;

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
    fe25519_mul(
        eden.as_mut_ptr(),
        den1.as_ptr(),
        ed25519_invsqrtamd.as_ptr(),
    ); /* eden = den1/sqrt(a-d) */

    fe25519_mul(t_z_inv.as_mut_ptr(), (*h).T.as_ptr(), z_inv.as_ptr()); /* t_z_inv = T*z_inv */
    rotate = fe25519_isnegative(t_z_inv.as_ptr());

    fe25519_copy(x_.as_mut_ptr(), (*h).X.as_ptr());
    fe25519_copy(y_.as_mut_ptr(), (*h).Y.as_ptr());
    fe25519_copy(den_inv.as_mut_ptr(), den2.as_ptr());

    fe25519_cmov(x_.as_mut_ptr(), iy.as_ptr(), rotate as u32);
    fe25519_cmov(y_.as_mut_ptr(), ix.as_ptr(), rotate as u32);
    fe25519_cmov(den_inv.as_mut_ptr(), eden.as_ptr(), rotate as u32);

    fe25519_mul(x_z_inv.as_mut_ptr(), x_.as_ptr(), z_inv.as_ptr());
    fe25519_cneg(y_.as_mut_ptr(), fe25519_isnegative(x_z_inv.as_ptr()) as u32);

    fe25519_sub(s_.as_mut_ptr(), (*h).Z.as_ptr(), y_.as_ptr());
    fe25519_mul(s_.as_mut_ptr(), den_inv.as_ptr(), s_.as_ptr());
    fe25519_abs(s_.as_mut_ptr());
    _sodium_fe25519_tobytes(s, s_.as_ptr());
}

pub unsafe fn ristretto255_elligator(p: *mut ge25519_p3, t: *const i32) {
    let mut c: [i32; 10] = [0; 10];
    let mut n: [i32; 10] = [0; 10];
    let mut one: [i32; 10] = [0; 10];
    let mut r: [i32; 10] = [0; 10];
    let mut rpd: [i32; 10] = [0; 10];
    let mut s: [i32; 10] = [0; 10];
    let mut s_prime: [i32; 10] = [0; 10];
    let mut ss: [i32; 10] = [0; 10];
    let mut u: [i32; 10] = [0; 10];
    let mut v: [i32; 10] = [0; 10];
    let mut w0: [i32; 10] = [0; 10];
    let mut w1: [i32; 10] = [0; 10];
    let mut w2: [i32; 10] = [0; 10];
    let mut w3: [i32; 10] = [0; 10];
    let wasnt_square: i32;

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
    fe25519_cmov(s.as_mut_ptr(), s_prime.as_ptr(), wasnt_square as u32);
    fe25519_cmov(c.as_mut_ptr(), r.as_ptr(), wasnt_square as u32);

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
    let mut r0: [i32; 10] = [0; 10];
    let mut r1: [i32; 10] = [0; 10];
    let mut p0: ge25519_p3 = core::mem::zeroed();
    let mut p1: ge25519_p3 = core::mem::zeroed();
    let mut p: ge25519_p3 = core::mem::zeroed();

    _sodium_fe25519_frombytes(r0.as_mut_ptr(), h);
    _sodium_fe25519_frombytes(r1.as_mut_ptr(), h.add(32));
    ristretto255_elligator(&mut p0, r0.as_ptr());
    ristretto255_elligator(&mut p1, r1.as_ptr());
    _sodium_ge25519_p3_add(&mut p, &p0, &p1);
    _sodium_ristretto255_p3_tobytes(s, &p);
}
