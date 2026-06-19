#![allow(non_camel_case_types)]
#![allow(clippy::too_many_arguments)]

use std::ffi::c_int;

#[link(name = "m")]
unsafe extern "C" {
    fn fabsf(x: f32) -> f32;
    fn fmodf(x: f32, y: f32) -> f32;
    fn floorf(x: f32) -> f32;
}

type tflac_u32 = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[inline]
fn c2_v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
fn c2_maxv(a: c2v, b: c2v) -> c2v {
    c2_v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2_minv(a: c2v, b: c2v) -> c2v {
    c2_v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2_clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2_maxv(lo, c2_minv(a, hi))
}

#[inline]
fn c2_sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
fn c2_dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_aabb_to_aabb(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

enum C2Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
}

fn f2(a: C2Shape, b: C2Shape) -> c_int {
    match a {
        C2Shape::Circle(circle_a) => match b {
            C2Shape::Circle(circle_b) => c2_circle_to_circle(circle_a, circle_b),
            C2Shape::Aabb(aabb_b) => c2_circle_to_aabb(circle_a, aabb_b),
        },
        C2Shape::Aabb(aabb_a) => match b {
            C2Shape::Circle(circle_b) => c2_circle_to_aabb(circle_b, aabb_a),
            C2Shape::Aabb(aabb_b) => c2_aabb_to_aabb(aabb_a, aabb_b),
        },
    }
}

fn f3(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }

    let q: c_int;
    let r: c_int;

    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != c_int::MIN {
            q = -(v1 / -v2);
            r = v1 % -v2;
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != c_int::MIN {
        if v2 >= 0 {
            q = -((-v1) / v2);
            r = -((-v1) % v2);
        } else if v2 != c_int::MIN {
            q = (-v1) / (-v2);
            r = -((-v1) % (-v2));
        } else {
            q = 1;
            r = v1 - q * v2;
        }
    } else if v2 >= 0 {
        q = -((-(v1 + v2)) / v2) - 1;
        r = -((-(v1 + v2)) % v2);
    } else if v2 != c_int::MIN {
        q = ((-(v1 - v2)) / (-v2)) + 1;
        r = -((-(v1 - v2)) % (-v2));
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

#[repr(C)]
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
    let exponent = 1023_u64;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

fn f5(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

fn f7(blocksize: tflac_u32, channels: tflac_u32, bitdepth: tflac_u32) -> tflac_u32 {
    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(u32::from(channels != 2)));
    let term2 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(u32::from(channels == 2));
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(u32::from(bitdepth != 32)))
        .wrapping_mul(u32::from(channels == 2));
    18_u32
        .wrapping_add(channels)
        .wrapping_add(term1.wrapping_add(term2).wrapping_add(term3).wrapping_add(7) / 8)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct lm_vec2 {
    x: f32,
    y: f32,
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

fn f9(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);
    let inv_denom = 1.0_f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_v2(u, v)
}

fn f10(h: u16) -> f32 {
    let sign = ((h & 0x8000) as u32) << 16;
    let exp = ((h >> 10) & 0x1f) as u32;
    let mant = (h & 0x03ff) as u32;

    let bits = if exp == 0 {
        if mant == 0 {
            sign
        } else {
            let mut mantissa = mant;
            let mut shift = -1_i32;
            while (mantissa & 0x400) == 0 {
                shift += 1;
                mantissa <<= 1;
            }
            mantissa &= 0x03ff;
            sign | (((127 - 15 - shift) as u32) << 23) | (mantissa << 13)
        }
    } else if exp == 0x1f {
        sign | 0x7f80_0000 | (mant << 13)
    } else {
        sign | ((exp + 112) << 23) | (mant << 13)
    };

    f32::from_bits(bits)
}

fn f11(dest: &mut [f32; 3], src: &[f32; 3]) {
    let h = src[0];
    let s = src[1];
    let l = src[2];
    let c;
    let m;
    let x;

    if s == 0.0 {
        dest[0] = l;
        dest[1] = l;
        dest[2] = l;
        return;
    }

    unsafe {
        c = (1.0_f32 - fabsf(2.0_f32 * l - 1.0_f32)) * s;
        m = 1.0_f32 * (l - 0.5_f32 * c);
        x = c * (1.0_f32 - fabsf(fmodf(h / 60.0_f32, 2.0_f32) - 1.0_f32));
    }

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

fn f12(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r;
    let g;
    let b;
    let h = src[0];
    let s = src[1];
    let v = src[2];
    let f;
    let p;
    let q;
    let t;
    let i;

    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }

    let mut h = h;
    h /= 60.0_f32;
    unsafe {
        i = floorf(h) as c_int;
    }
    f = h - i as f32;
    p = v * (1.0_f32 - s);
    q = v * (1.0_f32 - s * f);
    t = v * (1.0_f32 - s * (1.0_f32 - f));

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
    let mut h = 0.0_f32;
    let mut s = 0.0_f32;
    let mut min = r;
    let mut max = r;

    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };

    let delta = max - min;
    let v = max;

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
        h = 2.0_f32 + (b - r) / delta;
    } else {
        h = 4.0_f32 + (r - g) / delta;
    }
    h *= 60.0_f32;
    if h < 0.0 {
        h += 360.0_f32;
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
    let mut ret = 0.0_f64;

    let f2_5 = c2Circle {
        p: c2v { x: f2_1, y: f2_2 },
        r: f2_3,
    };
    let f2_11 = c2AABB {
        min: c2v { x: f2_7, y: f2_8 },
        max: c2v { x: f2_9, y: f2_10 },
    };

    let f2_r = f2(C2Shape::Circle(f2_5), C2Shape::Aabb(f2_11));
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
    let f9_12 = lm_vec2 {
        x: f9_10,
        y: f9_11,
    };

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

    let mut f11_r = [0.0_f32, 0.0_f32, 0.0_f32];
    let f11_5 = [f11_2, f11_3, f11_4];
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

    let mut f12_r = [0.0_f32, 0.0_f32, 0.0_f32];
    let f12_5 = [f12_2, f12_3, f12_4];
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

    let mut f13_r = [0.0_f32, 0.0_f32, 0.0_f32];
    let f13_5 = [f13_2, f13_3, f13_4];
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
