use std::os::raw::{c_double, c_float, c_int, c_uint, c_ulonglong, c_ushort};

pub type TflacU32 = u32;
type TflacU8 = u8;
type TflacU16 = u16;

type Uint64T = c_ulonglong;
type Uint32T = c_uint;
type Uint16T = c_ushort;

#[repr(C)]
#[derive(Copy, Clone)]
enum C2Type {
    Circle,
    Aabb,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

fn c2_v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2_v(a.x.max(b.x), a.y.max(b.y))
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2_v(a.x.min(b.x), a.y.min(b.y))
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    (d2 < r2) as c_int
}

fn c2_circle_to_aabb(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

fn c2_aabb_to_aabb(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn f2(a: *const core::ffi::c_void, type_a: C2Type, b: *const core::ffi::c_void, type_b: C2Type) -> c_int {
    match type_a {
        C2Type::Circle => match type_b {
            C2Type::Circle => {
                let aa = unsafe { *(a as *const C2Circle) };
                let bb = unsafe { *(b as *const C2Circle) };
                c2_circle_to_circle(aa, bb)
            }
            C2Type::Aabb => {
                let aa = unsafe { *(a as *const C2Circle) };
                let bb = unsafe { *(b as *const C2Aabb) };
                c2_circle_to_aabb(aa, bb)
            }
        },
        C2Type::Aabb => match type_b {
            C2Type::Circle => {
                let bb = unsafe { *(b as *const C2Circle) };
                let aa = unsafe { *(a as *const C2Aabb) };
                c2_circle_to_aabb(bb, aa)
            }
            C2Type::Aabb => {
                let aa = unsafe { *(a as *const C2Aabb) };
                let bb = unsafe { *(b as *const C2Aabb) };
                c2_aabb_to_aabb(aa, bb)
            }
        },
    }
}

fn f3(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }
    let min = i32::MIN;
    let mut q: i32;
    let r: i32;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != min {
            q = -(v1 / -v2);
            let rr = v1 % -v2;
            return if rr >= 0 { q } else { q + 1 };
        } else {
            q = 0;
            let rr = v1;
            return if rr >= 0 { q } else { q + 1 };
        }
    } else if v1 != min {
        if v2 >= 0 {
            q = -((-v1) / v2);
            let rr = -((-v1) % v2);
            return if rr >= 0 { q } else { q - 1 };
        } else if v2 != min {
            q = (-v1) / (-v2);
            let rr = -((-v1) % (-v2));
            return if rr >= 0 { q } else { q + 1 };
        } else {
            q = 1;
            let rr = v1 - q * v2;
            return if rr >= 0 { q } else { q + 1 };
        }
    } else if v2 >= 0 {
        q = (-(-(v1 + v2) / v2)) - 1;
        let rr = -((-(v1 + v2)) % v2);
        return if rr >= 0 { q } else { q - 1 };
    } else if v2 != min {
        q = ((-(v1 - v2)) / (-v2)) + 1;
        let rr = -((-(v1 - v2)) % (-v2));
        return if rr >= 0 { q } else { q + 1 };
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 { q } else { q + if v2 > 0 { -1 } else { 1 } }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CnRndT {
    state: [u64; 2],
}

fn cn_rnd_next(rnd: &mut CnRndT) -> u64 {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

fn f4(rnd: &mut CnRndT) -> f64 {
    let value = cn_rnd_next(rnd);
    let exponent = 1023u64;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

fn f5(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

fn f7(blocksize: TflacU32, channels: TflacU32, bitdepth: TflacU32) -> TflacU32 {
    18u32 + channels
        + ((blocksize * bitdepth * (channels * (channels != 2) as u32)
            + blocksize * bitdepth * (channels == 2) as u32
            + blocksize * (bitdepth + (bitdepth != 32) as u32) * (channels == 2) as u32
            + 7)
            / 8)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct LmVec2 {
    x: f32,
    y: f32,
}

fn lm_v2(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

fn lm_sub2(a: LmVec2, b: LmVec2) -> LmVec2 {
    lm_v2(a.x - b.x, a.y - b.y)
}

fn lm_dot2(a: LmVec2, b: LmVec2) -> f32 {
    a.x * b.x + a.y * b.y
}

fn f9(p1: LmVec2, p2: LmVec2, p3: LmVec2, p: LmVec2) -> LmVec2 {
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

fn f10(h: u16) -> f32 {
    let n = (h >> 10) as u32;
    let s = (h & 0x1f) as u32;
    let e = ((h >> 5) & 0x1f) as u32;
    let bits = if e == 0 {
        if s == 0 {
            n << 31
        } else {
            let mut mant = s;
            let mut exp = -14i32;
            while (mant & 0x20) == 0 {
                mant <<= 1;
                exp -= 1;
            }
            mant &= 0x1f;
            (n << 31) | (((exp + 127) as u32) << 23) | (mant << 18)
        }
    } else if e == 31 {
        (n << 31) | 0x7f800000 | (s << 13)
    } else {
        (n << 31) | ((e + 112) << 23) | (s << 13)
    };
    f32::from_bits(bits)
}

fn f11(dest: &mut [f32; 3], src: &[f32; 3]) {
    let h = src[0];
    let s = src[1];
    let l = src[2];
    if s == 0.0 {
        dest[0] = l;
        dest[1] = l;
        dest[2] = l;
        return;
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let m = l - 0.5 * c;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
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
    let h = src[0];
    let s = src[1];
    let v = src[2];
    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }
    let hh = h / 60.0;
    let i = hh.floor() as i32;
    let f = hh - i as f32;
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

fn f13(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r = src[0];
    let g = src[1];
    let b = src[2];
    let mut h = 0.0f32;
    let mut s = 0.0f32;
    let v;
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
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

#[unsafe(no_mangle)]
pub extern "C" fn agglom(
    f2_1: c_float,
    f2_2: c_float,
    f2_3: c_float,
    f2_7: c_float,
    f2_8: c_float,
    f2_9: c_float,
    f2_10: c_float,
    f3_1: c_int,
    f3_2: c_int,
    f4_1: Uint64T,
    f4_2: Uint64T,
    f5_1: Uint32T,
    f7_1: TflacU32,
    f7_2: TflacU32,
    f7_3: TflacU32,
    f9_1: c_float,
    f9_2: c_float,
    f9_4: c_float,
    f9_5: c_float,
    f9_7: c_float,
    f9_8: c_float,
    f9_10: c_float,
    f9_11: c_float,
    f10_1: Uint16T,
    f11_2: c_float,
    f11_3: c_float,
    f11_4: c_float,
    f12_2: c_float,
    f12_3: c_float,
    f12_4: c_float,
    f13_2: c_float,
    f13_3: c_float,
    f13_4: c_float,
) -> c_double {
    let mut ret = 0.0f64;

    let f2_5 = C2Circle {
        p: C2v { x: f2_1, y: f2_2 },
        r: f2_3,
    };
    let f2_6 = C2Type::Circle;
    let f2_11 = C2Aabb {
        min: C2v { x: f2_7, y: f2_8 },
        max: C2v { x: f2_9, y: f2_10 },
    };
    let f2_12 = C2Type::Aabb;
    let f2_r = f2(
        (&f2_5 as *const C2Circle).cast(),
        f2_6,
        (&f2_11 as *const C2Aabb).cast(),
        f2_12,
    );
    ret += f2_r as f64;

    let f3_r = f3(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = CnRndT { state: [f4_1, f4_2] };
    let f4_r = f4(&mut f4_3);
    if !f4_r.is_nan() {
        ret += f4_r;
    }

    let f5_r = f5(f5_1);
    ret += f5_r as f64;

    let f7_r = f7(f7_1, f7_2, f7_3);
    ret += f7_r as f64;

    let f9_3 = LmVec2 { x: f9_1, y: f9_2 };
    let f9_6 = LmVec2 { x: f9_4, y: f9_5 };
    let f9_9 = LmVec2 { x: f9_7, y: f9_8 };
    let f9_12 = LmVec2 { x: f9_10, y: f9_11 };
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

    let mut f11_r = [0.0f32, 0.0, 0.0];
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

    let mut f12_r = [0.0f32, 0.0, 0.0];
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

    let mut f13_r = [0.0f32, 0.0, 0.0];
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
