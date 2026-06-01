#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]

use std::ffi::c_int;

// =====================================================================
// c2 collision types
// =====================================================================

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

// C2_TYPE is an enum: C2_TYPE_CIRCLE=0, C2_TYPE_AABB=1
pub type C2_TYPE = c_int;
const C2_TYPE_CIRCLE: C2_TYPE = 0;
const C2_TYPE_AABB: C2_TYPE = 1;

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    if d2 < r2 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    if d2 < r2 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0: c_int = if B.max.x < A.min.x { 1 } else { 0 };
    let d1: c_int = if A.max.x < B.min.x { 1 } else { 0 };
    let d2: c_int = if B.max.y < A.min.y { 1 } else { 0 };
    let d3: c_int = if A.max.y < B.min.y { 1 } else { 0 };
    if (d0 | d1 | d2 | d3) == 0 { 1 } else { 0 }
}

/// # Safety
/// `A` and `B` must point to valid objects of the corresponding type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f2(
    A: *const core::ffi::c_void,
    typeA: C2_TYPE,
    B: *const core::ffi::c_void,
    typeB: C2_TYPE,
) -> c_int {
    match typeA {
        x if x == C2_TYPE_CIRCLE => match typeB {
            y if y == C2_TYPE_CIRCLE => unsafe {
                c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle))
            },
            y if y == C2_TYPE_AABB => unsafe {
                c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB))
            },
            _ => 0,
        },
        x if x == C2_TYPE_AABB => match typeB {
            y if y == C2_TYPE_CIRCLE => unsafe {
                c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB))
            },
            y if y == C2_TYPE_AABB => unsafe {
                c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB))
            },
            _ => 0,
        },
        _ => 0,
    }
}

// =====================================================================
// f3: signed integer floor division
// =====================================================================

#[unsafe(no_mangle)]
pub extern "C" fn f3(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }
    let imin: i32 = i32::MIN;
    let q: i32;
    let r: i32;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != imin {
            // -v2 in [1, IMAX]; safe
            let nv2 = (-v2) as i32;
            q = -(v1 / nv2);
            r = v1 % nv2;
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != imin {
        let nv1 = -v1; // safe
        if v2 >= 0 {
            q = -(nv1 / v2);
            r = -(nv1 % v2);
        } else if v2 != imin {
            let nv2 = -v2;
            q = nv1 / nv2;
            r = -(nv1 % nv2);
        } else {
            // v1 != IMIN, v2 == IMIN
            q = 1;
            // q*v2 may overflow if q!=1 — here q==1 so q*v2 == IMIN, then v1 - IMIN
            // for v1 in [IMIN+1, -1] is in [1, IMAX] (no overflow).
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        // v1 == IMIN
        // v1 + v2 = IMIN + v2. If v2>=0 and v2<=IMAX, sum is in [IMIN, -1], no overflow.
        let s = v1.wrapping_add(v2);
        // -(v1+v2): s in [IMIN, -1], so -s in [1, IMAX] (since s != IMIN here unless v2 == 0;
        // if v2 == 0 we'd have returned earlier).
        let ns = s.wrapping_neg();
        q = -(ns / v2) - 1;
        r = -(ns % v2);
    } else if v2 != imin {
        // v1 == IMIN, v2 < 0 and != IMIN
        // v1 - v2 = IMIN - v2 = IMIN + |v2|. |v2| in [1, IMAX]. Result in [IMIN+1, -1]. No overflow.
        let s = v1.wrapping_sub(v2);
        let ns = s.wrapping_neg();
        let nv2 = -v2;
        q = (ns / nv2) + 1;
        r = -(ns % nv2);
    } else {
        // v1 == IMIN and v2 == IMIN
        q = 1;
        r = 0;
    }
    if r >= 0 {
        q
    } else if v2 > 0 {
        q + -1
    } else {
        q + 1
    }
}

// =====================================================================
// f4: 64-bit RNG -> [0,1) double
// =====================================================================

#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

fn cn_rnd_next(rnd: &mut cn_rnd_t) -> u64 {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

/// # Safety
/// `rnd` must be a valid mutable pointer to a `cn_rnd_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f4(rnd: *mut cn_rnd_t) -> f64 {
    let rnd_ref = unsafe { &mut *rnd };
    let value = cn_rnd_next(rnd_ref);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

// =====================================================================
// f5: bit-reverse 16-bit value
// =====================================================================

#[unsafe(no_mangle)]
pub extern "C" fn f5(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

// =====================================================================
// f7: tflac frame size formula
// =====================================================================

pub type tflac_u32 = u32;

#[unsafe(no_mangle)]
pub extern "C" fn f7(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    // Use wrapping arithmetic to mimic C unsigned overflow semantics
    let ch_ne_2: u32 = if channels != 2 { 1 } else { 0 };
    let ch_eq_2: u32 = if channels == 2 { 1 } else { 0 };
    let bd_ne_32: u32 = if bitdepth != 32 { 1 } else { 0 };

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(ch_ne_2));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(ch_eq_2);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bd_ne_32))
        .wrapping_mul(ch_eq_2);

    let inner = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(inner / 8)
}

// =====================================================================
// f9: barycentric coordinates
// =====================================================================

#[repr(C)]
#[derive(Copy, Clone)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(a.x - b.x, a.y - b.y)
}

fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn f9(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);
    let inv_denom = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_v2(u, v)
}

// =====================================================================
// f10: half-float (uint16_t) to float (per a known algorithm)
// =====================================================================

include!("tables.rs");

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> f32 {
    let n = (h >> 10) as usize;
    let idx = ((h & 0x3ff) as usize).wrapping_add(M_OFFSET[n] as usize);
    let bits = M_MANTISSA[idx].wrapping_add(M_EXPONENT[n]);
    f32::from_bits(bits)
}

// =====================================================================
// f11: HSL -> RGB
// =====================================================================

/// # Safety
/// `dest` must point to at least 3 writable f32 elements; `src` to at least 3 readable f32 elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f11(dest: *mut f32, src: *const f32) {
    let h = unsafe { *src.add(0) };
    let s = unsafe { *src.add(1) };
    let l = unsafe { *src.add(2) };
    let c: f32;
    let m: f32;
    let x: f32;
    if s == 0.0 {
        unsafe {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }
    c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    m = 1.0f32 * (l - 0.5f32 * c);
    x = c * (1.0f32 - ((h / 60.0f32) % 2.0f32 - 1.0f32).abs());
    if (0.0f32..60.0f32).contains(&h) {
        unsafe {
            *dest.add(0) = c + m;
            *dest.add(1) = x + m;
            *dest.add(2) = m;
        }
    } else if (60.0f32..120.0f32).contains(&h) {
        unsafe {
            *dest.add(0) = x + m;
            *dest.add(1) = c + m;
            *dest.add(2) = m;
        }
    } else if h < 120.0f32 && h < 180.0f32 {
        // BUG-PRESERVED: original C has the same condition twice
        unsafe {
            *dest.add(0) = m;
            *dest.add(1) = c + m;
            *dest.add(2) = x + m;
        }
    } else if (180.0f32..240.0f32).contains(&h) {
        unsafe {
            *dest.add(0) = m;
            *dest.add(1) = x + m;
            *dest.add(2) = c + m;
        }
    } else if (240.0f32..300.0f32).contains(&h) {
        unsafe {
            *dest.add(0) = x + m;
            *dest.add(1) = m;
            *dest.add(2) = c + m;
        }
    } else if (300.0f32..360.0f32).contains(&h) {
        unsafe {
            *dest.add(0) = c + m;
            *dest.add(1) = m;
            *dest.add(2) = x + m;
        }
    } else {
        unsafe {
            *dest.add(0) = m;
            *dest.add(1) = m;
            *dest.add(2) = m;
        }
    }
}

// =====================================================================
// f12: HSV -> RGB
// =====================================================================

/// # Safety
/// `dest` must point to at least 3 writable f32 elements; `src` to at least 3 readable f32 elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f12(dest: *mut f32, src: *const f32) {
    let mut h = unsafe { *src.add(0) };
    let s = unsafe { *src.add(1) };
    let v = unsafe { *src.add(2) };
    if s == 0.0 {
        unsafe {
            *dest.add(0) = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
        }
        return;
    }
    h /= 60.0f32;
    let i: i32 = h.floor() as i32;
    let f = h - (i as f32);
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    unsafe {
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}

// =====================================================================
// f13: RGB -> HSV
// =====================================================================

/// # Safety
/// `dest` must point to at least 3 writable f32 elements; `src` to at least 3 readable f32 elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f13(dest: *mut f32, src: *const f32) {
    let r = unsafe { *src.add(0) };
    let g = unsafe { *src.add(1) };
    let b = unsafe { *src.add(2) };
    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;
    let mut min = r;
    let mut max = r;
    let delta: f32;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    delta = max - min;
    v = max;
    if delta == 0.0 || max == 0.0 {
        unsafe {
            *dest.add(0) = h;
            *dest.add(1) = s;
            *dest.add(2) = v;
        }
        return;
    }
    s = delta / max;
    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;
    } else {
        h = 4.0 + (r - g) / delta;
    }
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    unsafe {
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}

// =====================================================================
// agglom: top-level aggregator
// =====================================================================

#[unsafe(no_mangle)]
pub extern "C" fn agglom(
    f2_1: f32, f2_2: f32, f2_3: f32, f2_7: f32, f2_8: f32, f2_9: f32, f2_10: f32,
    f3_1: c_int, f3_2: c_int,
    f4_1: u64, f4_2: u64,
    f5_1: u32,
    f7_1: tflac_u32, f7_2: tflac_u32, f7_3: tflac_u32,
    f9_1: f32, f9_2: f32, f9_4: f32, f9_5: f32, f9_7: f32, f9_8: f32, f9_10: f32, f9_11: f32,
    f10_1: u16,
    f11_2: f32, f11_3: f32, f11_4: f32,
    f12_2: f32, f12_3: f32, f12_4: f32,
    f13_2: f32, f13_3: f32, f13_4: f32,
) -> f64 {
    let mut ret: f64 = 0.0;

    let f2_5 = c2Circle { p: c2v { x: f2_1, y: f2_2 }, r: f2_3 };
    let f2_6: C2_TYPE = C2_TYPE_CIRCLE;

    let f2_11 = c2AABB {
        min: c2v { x: f2_7, y: f2_8 },
        max: c2v { x: f2_9, y: f2_10 },
    };
    let f2_12: C2_TYPE = C2_TYPE_AABB;

    let f2_r = unsafe {
        f2(
            &f2_5 as *const c2Circle as *const core::ffi::c_void,
            f2_6,
            &f2_11 as *const c2AABB as *const core::ffi::c_void,
            f2_12,
        )
    };
    ret += f2_r as f64;

    let f3_r = f3(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = cn_rnd_t { state: [f4_1, f4_2] };
    let f4_r = unsafe { f4(&mut f4_3 as *mut cn_rnd_t) };
    if !f4_r.is_nan() {
        ret += f4_r;
    }

    let f5_r = f5(f5_1);
    ret += f5_r as f64;

    let f7_r = f7(f7_1, f7_2, f7_3);
    ret += f7_r as f64;

    let f9_3 = lm_vec2 { x: f9_1, y: f9_2 };
    let f9_6 = lm_vec2 { x: f9_4, y: f9_5 };
    let f9_9 = lm_vec2 { x: f9_7, y: f9_8 };
    let f9_12 = lm_vec2 { x: f9_10, y: f9_11 };
    let f9_r = f9(f9_3, f9_6, f9_9, f9_12);
    if !f9_r.x.is_nan() {
        ret += f9_r.x as f64;
    }
    if !f9_r.y.is_nan() {
        ret += f9_r.y as f64;
    }

    let f10_r = f10(f10_1);
    if !f10_r.is_nan() {
        ret += f10_r as f64;
    }

    let mut f11_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f11_5: [f32; 3] = [f11_2, f11_3, f11_4];
    unsafe { f11(f11_r.as_mut_ptr(), f11_5.as_ptr()); }
    if !f11_r[0].is_nan() { ret += f11_r[0] as f64; }
    if !f11_r[1].is_nan() { ret += f11_r[1] as f64; }
    if !f11_r[2].is_nan() { ret += f11_r[2] as f64; }

    let mut f12_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f12_5: [f32; 3] = [f12_2, f12_3, f12_4];
    unsafe { f12(f12_r.as_mut_ptr(), f12_5.as_ptr()); }
    if !f12_r[0].is_nan() { ret += f12_r[0] as f64; }
    if !f12_r[1].is_nan() { ret += f12_r[1] as f64; }
    if !f12_r[2].is_nan() { ret += f12_r[2] as f64; }

    let mut f13_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f13_5: [f32; 3] = [f13_2, f13_3, f13_4];
    unsafe { f13(f13_r.as_mut_ptr(), f13_5.as_ptr()); }
    if !f13_r[0].is_nan() { ret += f13_r[0] as f64; }
    if !f13_r[1].is_nan() { ret += f13_r[1] as f64; }
    if !f13_r[2].is_nan() { ret += f13_r[2] as f64; }

    ret
}
