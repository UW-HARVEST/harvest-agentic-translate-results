#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

mod tables;

pub type tflac_u32 = u32;
pub type tflac_u16 = u16;
pub type tflac_u8 = u8;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum C2_TYPE {
    CIRCLE = 0,
    AABB = 1,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[inline]
fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[inline]
fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> i32 {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    if d2 < r2 { 1 } else { 0 }
}

fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> i32 {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    if d2 < r2 { 1 } else { 0 }
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> i32 {
    let d0 = if B.max.x < A.min.x { 1 } else { 0 };
    let d1 = if A.max.x < B.min.x { 1 } else { 0 };
    let d2 = if B.max.y < A.min.y { 1 } else { 0 };
    let d3 = if A.max.y < B.min.y { 1 } else { 0 };
    let combined: i32 = d0 | d1 | d2 | d3;
    if combined == 0 { 1 } else { 0 }
}

#[derive(Copy, Clone)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
}

fn f2(a: &Shape, type_a: C2_TYPE, b: &Shape, type_b: C2_TYPE) -> i32 {
    match type_a {
        C2_TYPE::CIRCLE => match type_b {
            C2_TYPE::CIRCLE => {
                let ca = match a { Shape::Circle(c) => *c, _ => return 0 };
                let cb = match b { Shape::Circle(c) => *c, _ => return 0 };
                c2CircletoCircle(ca, cb)
            }
            C2_TYPE::AABB => {
                let ca = match a { Shape::Circle(c) => *c, _ => return 0 };
                let bb = match b { Shape::Aabb(c) => *c, _ => return 0 };
                c2CircletoAABB(ca, bb)
            }
        },
        C2_TYPE::AABB => match type_b {
            C2_TYPE::CIRCLE => {
                let cb = match b { Shape::Circle(c) => *c, _ => return 0 };
                let ba = match a { Shape::Aabb(c) => *c, _ => return 0 };
                c2CircletoAABB(cb, ba)
            }
            C2_TYPE::AABB => {
                let ba = match a { Shape::Aabb(c) => *c, _ => return 0 };
                let bb = match b { Shape::Aabb(c) => *c, _ => return 0 };
                c2AABBtoAABB(ba, bb)
            }
        },
    }
}

fn f3(v1: i32, v2: i32) -> i32 {
    if v2 == 0 {
        return 0;
    }
    let q: i32;
    let r: i32;
    let i32_min: i32 = -0x7fffffff - 1;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != i32_min {
            q = -(v1 / (-v2));
            r = v1 % (-v2);
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != i32_min {
        if v2 >= 0 {
            q = -((-v1) / v2);
            r = -((-v1) % v2);
        } else if v2 != i32_min {
            q = (-v1) / (-v2);
            r = -((-v1) % (-v2));
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        q = -((-(v1.wrapping_add(v2))) / v2) - 1;
        r = -((-(v1.wrapping_add(v2))) % v2);
    } else if v2 != i32_min {
        q = ((-(v1.wrapping_sub(v2))) / (-v2)) + 1;
        r = -((-(v1.wrapping_sub(v2))) % (-v2));
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 {
        q
    } else if v2 > 0 {
        q + (-1)
    } else {
        q + 1
    }
}

#[derive(Copy, Clone)]
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

pub fn f4(rnd: &mut cn_rnd_t) -> f64 {
    let value = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

pub fn f5(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

pub fn f7(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    let ch_ne_2: u32 = if channels != 2 { 1 } else { 0 };
    let ch_eq_2: u32 = if channels == 2 { 1 } else { 0 };
    let bd_ne_32: u32 = if bitdepth != 32 { 1 } else { 0 };
    18u32
        .wrapping_add(channels)
        .wrapping_add(
            (blocksize
                .wrapping_mul(bitdepth)
                .wrapping_mul(channels.wrapping_mul(ch_ne_2))
                .wrapping_add(blocksize.wrapping_mul(bitdepth).wrapping_mul(ch_eq_2))
                .wrapping_add(
                    blocksize
                        .wrapping_mul(bitdepth.wrapping_add(bd_ne_32))
                        .wrapping_mul(ch_eq_2),
                )
                .wrapping_add(7))
                / 8,
        )
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

#[inline]
fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(a.x - b.x, a.y - b.y)
}

#[inline]
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    a.x * b.x + a.y * b.y
}

pub fn f9(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
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

pub fn f10(h: u16) -> f32 {
    let n = (h >> 10) as usize;
    let idx = ((h & 0x3ff) as usize) + (tables::M_OFFSET[n] as usize);
    let combined = tables::M_MANTISSA[idx].wrapping_add(tables::M_EXPONENT[n]);
    f32::from_bits(combined)
}

pub fn f11(dest: &mut [f32; 3], src: &[f32; 3]) {
    let h = src[0];
    let s = src[1];
    let l = src[2];
    if s == 0.0 {
        dest[0] = l;
        dest[1] = l;
        dest[2] = l;
        return;
    }
    let c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    let m = 1.0f32 * (l - 0.5f32 * c);
    let x = c * (1.0f32 - ((h / 60.0f32) % 2.0f32 - 1.0f32).abs());
    if h >= 0.0 && h < 60.0 {
        dest[0] = c + m;
        dest[1] = x + m;
        dest[2] = m;
    } else if h >= 60.0 && h < 120.0 {
        dest[0] = x + m;
        dest[1] = c + m;
        dest[2] = m;
    } else if h < 120.0 && h < 180.0 {
        dest[0] = m;
        dest[1] = c + m;
        dest[2] = x + m;
    } else if h >= 180.0 && h < 240.0 {
        dest[0] = m;
        dest[1] = x + m;
        dest[2] = c + m;
    } else if h >= 240.0 && h < 300.0 {
        dest[0] = x + m;
        dest[1] = m;
        dest[2] = c + m;
    } else if h >= 300.0 && h < 360.0 {
        dest[0] = c + m;
        dest[1] = m;
        dest[2] = x + m;
    } else {
        dest[0] = m;
        dest[1] = m;
        dest[2] = m;
    }
}

pub fn f12(dest: &mut [f32; 3], src: &[f32; 3]) {
    let mut h = src[0];
    let s = src[1];
    let v = src[2];
    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }
    h /= 60.0f32;
    let i = h.floor() as i32;
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
    dest[0] = r;
    dest[1] = g;
    dest[2] = b;
}

pub fn f13(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r = src[0];
    let g = src[1];
    let b = src[2];
    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;
    let mut min = r;
    let mut max = r;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    let delta = max - min;
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

#[no_mangle]
pub extern "C" fn agglom(
    f2_1: f32, f2_2: f32, f2_3: f32, f2_7: f32, f2_8: f32, f2_9: f32, f2_10: f32,
    f3_1: i32, f3_2: i32,
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
    let f2_6 = C2_TYPE::CIRCLE;

    let f2_11 = c2AABB {
        min: c2v { x: f2_7, y: f2_8 },
        max: c2v { x: f2_9, y: f2_10 },
    };
    let f2_12 = C2_TYPE::AABB;

    let shape_a = Shape::Circle(f2_5);
    let shape_b = Shape::Aabb(f2_11);
    let f2_r = f2(&shape_a, f2_6, &shape_b, f2_12);
    ret += f2_r as f64;

    let f3_r = f3(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = cn_rnd_t { state: [f4_1, f4_2] };
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
