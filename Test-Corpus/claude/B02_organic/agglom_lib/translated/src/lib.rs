#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(clippy::all)]

use std::ffi::c_int;

type tflac_u32 = u32;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct c2v {
    x: f32,
    y: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    (!((d0 | d1 | d2 | d3) != 0)) as c_int
}

unsafe fn f2(A: *const u8, typeA: C2_TYPE, B: *const u8, typeB: C2_TYPE) -> c_int {
    unsafe {
        match typeA {
            C2_TYPE::C2_TYPE_CIRCLE => match typeB {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle))
                }
                C2_TYPE::C2_TYPE_AABB => {
                    c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB))
                }
            },
            C2_TYPE::C2_TYPE_AABB => match typeB {
                C2_TYPE::C2_TYPE_CIRCLE => {
                    c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB))
                }
                C2_TYPE::C2_TYPE_AABB => {
                    c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB))
                }
            },
        }
    }
}

fn f3(v1: i32, v2: i32) -> c_int {
    if v2 == 0 {
        return 0;
    }
    let q: i32;
    let r: i32;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != -0x7fffffff - 1 {
            q = (v1 / (-v2)).wrapping_neg();
            r = v1 % (-v2);
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != -0x7fffffff - 1 {
        if v2 >= 0 {
            q = ((-v1) / v2).wrapping_neg();
            r = ((-v1) % v2).wrapping_neg();
        } else if v2 != -0x7fffffff - 1 {
            q = (-v1) / (-v2);
            r = ((-v1) % (-v2)).wrapping_neg();
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        q = ((v1.wrapping_add(v2)).wrapping_neg() / v2).wrapping_neg().wrapping_sub(1);
        r = ((v1.wrapping_add(v2)).wrapping_neg() % v2).wrapping_neg();
    } else if v2 != -0x7fffffff - 1 {
        q = ((v1.wrapping_sub(v2)).wrapping_neg() / (-v2)).wrapping_add(1);
        r = ((v1.wrapping_sub(v2)).wrapping_neg() % (-v2)).wrapping_neg();
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 {
        q
    } else {
        q + if v2 > 0 { -1 } else { 1 }
    }
}

#[derive(Copy, Clone)]
struct cn_rnd_t {
    state: [u64; 2],
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

fn f4(rnd: &mut cn_rnd_t) -> f64 {
    let value = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

fn f5(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

fn f7(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    // Preserve C unsigned integer semantics with wrapping arithmetic.
    let part1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul((channels != 2) as u32));
    let part2 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul((channels == 2) as u32);
    let part3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add((bitdepth != 32) as u32))
        .wrapping_mul((channels == 2) as u32);
    let inner = part1.wrapping_add(part2).wrapping_add(part3).wrapping_add(7);
    18u32
        .wrapping_add(channels)
        .wrapping_add(inner / 8)
}

#[derive(Copy, Clone)]
struct lm_vec2 {
    x: f32,
    y: f32,
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

fn f9(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);
    let invDenom = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * invDenom;
    let v = (dot00 * dot12 - dot01 * dot02) * invDenom;
    lm_v2(u, v)
}

include!("tables.rs");

fn f10(h: u16) -> f32 {
    let n: usize = (h >> 10) as usize;
    let idx = ((h & 0x3ff) as usize).wrapping_add(M_OFFSET[n] as usize);
    let num = M_MANTISSA[idx].wrapping_add(M_EXPONENT[n]);
    f32::from_bits(num)
}

fn fabsf(x: f32) -> f32 {
    x.abs()
}

fn fmodf(x: f32, y: f32) -> f32 {
    // libm fmodf semantics: x - truncf(x/y) * y, preserving sign of x
    // Rust's f32::rem_euclid is different; use libm-equivalent computation.
    // The C standard fmodf returns x - n*y where n = trunc(x/y).
    // f32::%/rem_assign in Rust matches IEEE 754 remainder with truncation toward 0,
    // which is the same as C fmodf.
    x % y
}

fn floorf(x: f32) -> f32 {
    x.floor()
}

fn f11(dest: &mut [f32; 3], src: &[f32; 3]) {
    let h = src[0];
    let s = src[1];
    let l = src[2];
    let c: f32;
    let m: f32;
    let x: f32;
    if s == 0.0 {
        dest[0] = l;
        dest[1] = l;
        dest[2] = l;
        return;
    }
    c = (1.0f32 - fabsf(2.0f32 * l - 1.0f32)) * s;
    m = 1.0f32 * (l - 0.5f32 * c);
    x = c * (1.0f32 - fabsf(fmodf(h / 60.0f32, 2.0) - 1.0f32));
    if h >= 0.0f32 && h < 60.0f32 {
        dest[0] = c + m;
        dest[1] = x + m;
        dest[2] = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        dest[0] = x + m;
        dest[1] = c + m;
        dest[2] = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        // Note: this is a bug in the original C, preserved exactly.
        dest[0] = m;
        dest[1] = c + m;
        dest[2] = x + m;
    } else if h >= 180.0f32 && h < 240.0f32 {
        dest[0] = m;
        dest[1] = x + m;
        dest[2] = c + m;
    } else if h >= 240.0f32 && h < 300.0f32 {
        dest[0] = x + m;
        dest[1] = m;
        dest[2] = c + m;
    } else if h >= 300.0f32 && h < 360.0f32 {
        dest[0] = c + m;
        dest[1] = m;
        dest[2] = x + m;
    } else {
        dest[0] = m;
        dest[1] = m;
        dest[2] = m;
    }
}

fn f12(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r: f32;
    let g: f32;
    let b: f32;
    let f: f32;
    let p: f32;
    let q: f32;
    let t: f32;
    let mut h = src[0];
    let s = src[1];
    let v = src[2];
    let i: i32;
    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }
    h /= 60.0f32;
    i = floorf(h) as i32;
    f = h - (i as f32);
    p = v * (1.0 - s);
    q = v * (1.0 - s * f);
    t = v * (1.0 - s * (1.0 - f));
    match i {
        0 => {
            r = v;
            g = t;
            b = p;
        }
        1 => {
            r = q;
            g = v;
            b = p;
        }
        2 => {
            r = p;
            g = v;
            b = t;
        }
        3 => {
            r = p;
            g = q;
            b = v;
        }
        4 => {
            r = t;
            g = p;
            b = v;
        }
        _ => {
            r = v;
            g = p;
            b = q;
        }
    }
    dest[0] = r;
    dest[1] = g;
    dest[2] = b;
}

fn f13(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r = src[0];
    let g = src[1];
    let b = src[2];
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
        dest[0] = h;
        dest[1] = s;
        dest[2] = v;
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
    dest[0] = h;
    dest[1] = s;
    dest[2] = v;
}

#[unsafe(no_mangle)]
pub extern "C" fn agglom(
    f2_1: f32,
    f2_2: f32,
    f2_3: f32,
    f2_7: f32,
    f2_8: f32,
    f2_9: f32,
    f2_10: f32,
    f3_1: c_int,
    f3_2: c_int,
    f4_1: u64,
    f4_2: u64,
    f5_1: u32,
    f7_1: tflac_u32,
    f7_2: tflac_u32,
    f7_3: tflac_u32,
    f9_1: f32,
    f9_2: f32,
    f9_4: f32,
    f9_5: f32,
    f9_7: f32,
    f9_8: f32,
    f9_10: f32,
    f9_11: f32,
    f10_1: u16,
    f11_2: f32,
    f11_3: f32,
    f11_4: f32,
    f12_2: f32,
    f12_3: f32,
    f12_4: f32,
    f13_2: f32,
    f13_3: f32,
    f13_4: f32,
) -> f64 {
    let mut ret: f64 = 0.0;

    let f2_5 = c2Circle {
        p: c2v { x: f2_1, y: f2_2 },
        r: f2_3,
    };
    let f2_6 = C2_TYPE::C2_TYPE_CIRCLE;

    let f2_11 = c2AABB {
        min: c2v { x: f2_7, y: f2_8 },
        max: c2v { x: f2_9, y: f2_10 },
    };
    let f2_12 = C2_TYPE::C2_TYPE_AABB;

    let f2_r = unsafe {
        f2(
            &f2_5 as *const c2Circle as *const u8,
            f2_6,
            &f2_11 as *const c2AABB as *const u8,
            f2_12,
        )
    };
    ret += f2_r as f64;

    let f3_r = f3(f3_1 as i32, f3_2 as i32);
    ret += f3_r as f64;

    let mut f4_3 = cn_rnd_t {
        state: [f4_1, f4_2],
    };
    let f4_r = f4(&mut f4_3);
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
    f11(&mut f11_r, &f11_5);
    if !f11_r[0].is_nan() {
        ret += f11_r[0] as f64;
    }
    if !f11_r[1].is_nan() {
        ret += f11_r[1] as f64;
    }
    if !f11_r[2].is_nan() {
        ret += f11_r[2] as f64;
    }

    let mut f12_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f12_5: [f32; 3] = [f12_2, f12_3, f12_4];
    f12(&mut f12_r, &f12_5);
    if !f12_r[0].is_nan() {
        ret += f12_r[0] as f64;
    }
    if !f12_r[1].is_nan() {
        ret += f12_r[1] as f64;
    }
    if !f12_r[2].is_nan() {
        ret += f12_r[2] as f64;
    }

    let mut f13_r: [f32; 3] = [0.0, 0.0, 0.0];
    let f13_5: [f32; 3] = [f13_2, f13_3, f13_4];
    f13(&mut f13_r, &f13_5);
    if !f13_r[0].is_nan() {
        ret += f13_r[0] as f64;
    }
    if !f13_r[1].is_nan() {
        ret += f13_r[1] as f64;
    }
    if !f13_r[2].is_nan() {
        ret += f13_r[2] as f64;
    }

    ret
}
