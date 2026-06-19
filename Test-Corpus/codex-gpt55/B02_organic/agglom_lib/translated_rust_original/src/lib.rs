use std::ffi::{c_double, c_float, c_int, c_uint, c_ulonglong, c_void};

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

#[link(name = "m")]
unsafe extern "C" {
    fn fmodf(x: c_float, y: c_float) -> c_float;
    fn floorf(x: c_float) -> c_float;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> c2v {
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
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> c_float {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let r2 = (a.r + b.r) * (a.r + b.r);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f2(
    a: *const c_void,
    type_a: c_int,
    b: *const c_void,
    type_b: c_int,
) -> c_int {
    match type_a {
        C2_TYPE_CIRCLE => match type_b {
            C2_TYPE_CIRCLE => c2CircletoCircle(unsafe { *(a as *const c2Circle) }, unsafe {
                *(b as *const c2Circle)
            }),
            C2_TYPE_AABB => c2CircletoAABB(unsafe { *(a as *const c2Circle) }, unsafe {
                *(b as *const c2AABB)
            }),
            _ => 0,
        },
        C2_TYPE_AABB => match type_b {
            C2_TYPE_CIRCLE => c2CircletoAABB(unsafe { *(b as *const c2Circle) }, unsafe {
                *(a as *const c2AABB)
            }),
            C2_TYPE_AABB => c2AABBtoAABB(unsafe { *(a as *const c2AABB) }, unsafe {
                *(b as *const c2AABB)
            }),
            _ => 0,
        },
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn f3(v1: c_int, v2: c_int) -> c_int {
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
        q = (-(v1 - v2)) / (-v2) + 1;
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
#[derive(Clone, Copy)]
pub struct cn_rnd_t {
    pub state: [c_ulonglong; 2],
}

fn cn_rnd_next(rnd: &mut cn_rnd_t) -> c_ulonglong {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x.wrapping_shl(23);
    x ^= x.wrapping_shr(17);
    x ^= y ^ y.wrapping_shr(26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f4(rnd: *mut cn_rnd_t) -> c_double {
    let value = cn_rnd_next(unsafe { &mut *rnd });
    let exponent: u64 = 1023;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

#[unsafe(no_mangle)]
pub extern "C" fn f5(mut a: c_uint) -> c_uint {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn f7(blocksize: c_uint, channels: c_uint, bitdepth: c_uint) -> c_uint {
    let channels_ne_2 = (channels != 2) as c_uint;
    let channels_eq_2 = (channels == 2) as c_uint;
    let bitdepth_ne_32 = (bitdepth != 32) as c_uint;
    let num = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(channels_ne_2))
        .wrapping_add(
            blocksize
                .wrapping_mul(bitdepth)
                .wrapping_mul(channels_eq_2),
        )
        .wrapping_add(
            blocksize
                .wrapping_mul(bitdepth.wrapping_add(bitdepth_ne_32))
                .wrapping_mul(channels_eq_2),
        )
        .wrapping_add(7);
    18u32.wrapping_add(channels).wrapping_add(num / 8)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct lm_vec2 {
    pub x: c_float,
    pub y: c_float,
}

fn lm_v2(x: c_float, y: c_float) -> lm_vec2 {
    lm_vec2 { x, y }
}

fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(a.x - b.x, a.y - b.y)
}

fn lm_dot2(a: lm_vec2, b: lm_vec2) -> c_float {
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

fn half_to_float_bits(h: u16) -> u32 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exp = ((h >> 10) & 0x1f) as i32;
    let mant = (h & 0x03ff) as u32;

    if exp == 0 {
        if mant == 0 {
            sign
        } else {
            let mut m = mant;
            let mut e = -14i32;
            while (m & 0x0400) == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x03ff;
            sign | (((e + 127) as u32) << 23) | (m << 13)
        }
    } else if exp == 31 {
        sign | 0x7f80_0000 | (mant << 13)
    } else {
        sign | (((exp - 15 + 127) as u32) << 23) | (mant << 13)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> c_float {
    f32::from_bits(half_to_float_bits(h))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f11(dest: *mut c_float, src: *const c_float) {
    let h = unsafe { *src.add(0) };
    let s = unsafe { *src.add(1) };
    let l = unsafe { *src.add(2) };
    let c: c_float;
    let m: c_float;
    let x: c_float;
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
    x = c * (1.0f32 - unsafe { fmodf(h / 60.0f32, 2.0f32) - 1.0f32 }.abs());
    unsafe {
        if h >= 0.0f32 && h < 60.0f32 {
            *dest.add(0) = c + m;
            *dest.add(1) = x + m;
            *dest.add(2) = m;
        } else if h >= 60.0f32 && h < 120.0f32 {
            *dest.add(0) = x + m;
            *dest.add(1) = c + m;
            *dest.add(2) = m;
        } else if h < 120.0f32 && h < 180.0f32 {
            *dest.add(0) = m;
            *dest.add(1) = c + m;
            *dest.add(2) = x + m;
        } else if h >= 180.0f32 && h < 240.0f32 {
            *dest.add(0) = m;
            *dest.add(1) = x + m;
            *dest.add(2) = c + m;
        } else if h >= 240.0f32 && h < 300.0f32 {
            *dest.add(0) = x + m;
            *dest.add(1) = m;
            *dest.add(2) = c + m;
        } else if h >= 300.0f32 && h < 360.0f32 {
            *dest.add(0) = c + m;
            *dest.add(1) = m;
            *dest.add(2) = x + m;
        } else {
            *dest.add(0) = m;
            *dest.add(1) = m;
            *dest.add(2) = m;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f12(dest: *mut c_float, src: *const c_float) {
    let r: c_float;
    let g: c_float;
    let b: c_float;
    let f: c_float;
    let p: c_float;
    let q: c_float;
    let t: c_float;
    let mut h = unsafe { *src.add(0) };
    let s = unsafe { *src.add(1) };
    let v = unsafe { *src.add(2) };
    let i: c_int;
    if s == 0.0 {
        unsafe {
            *dest.add(0) = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
        }
        return;
    }
    h /= 60.0f32;
    i = unsafe { floorf(h) } as c_int;
    f = h - i as c_float;
    p = v * (1.0f32 - s);
    q = v * (1.0f32 - s * f);
    t = v * (1.0f32 - s * (1.0f32 - f));
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
    unsafe {
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f13(dest: *mut c_float, src: *const c_float) {
    let r = unsafe { *src.add(0) };
    let g = unsafe { *src.add(1) };
    let b = unsafe { *src.add(2) };
    let mut h = 0.0f32;
    let mut s = 0.0f32;
    let mut min = r;
    let mut max = r;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    let delta = max - min;
    let v = max;
    if delta == 0.0f32 || max == 0.0f32 {
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
        h = 2.0f32 + (b - r) / delta;
    } else {
        h = 4.0f32 + (r - g) / delta;
    }
    h *= 60.0f32;
    if h < 0.0f32 {
        h += 360.0f32;
    }
    unsafe {
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
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
    f4_1: c_ulonglong,
    f4_2: c_ulonglong,
    f5_1: c_uint,
    f7_1: c_uint,
    f7_2: c_uint,
    f7_3: c_uint,
    f9_1: c_float,
    f9_2: c_float,
    f9_4: c_float,
    f9_5: c_float,
    f9_7: c_float,
    f9_8: c_float,
    f9_10: c_float,
    f9_11: c_float,
    f10_1: u16,
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

    let f2_5 = c2Circle {
        p: c2v { x: f2_1, y: f2_2 },
        r: f2_3,
    };
    let f2_6 = C2_TYPE_CIRCLE;
    let f2_11 = c2AABB {
        min: c2v { x: f2_7, y: f2_8 },
        max: c2v { x: f2_9, y: f2_10 },
    };
    let f2_12 = C2_TYPE_AABB;
    let f2_r = unsafe {
        f2(
            (&f2_5 as *const c2Circle).cast(),
            f2_6,
            (&f2_11 as *const c2AABB).cast(),
            f2_12,
        )
    };
    ret += f2_r as f64;

    let f3_r = f3(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = cn_rnd_t {
        state: [f4_1, f4_2],
    };
    let f4_r = unsafe { f4(&mut f4_3) };
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

    let mut f11_r = [0.0f32, 0.0f32, 0.0f32];
    let f11_5 = [f11_2, f11_3, f11_4];
    unsafe { f11(f11_r.as_mut_ptr(), f11_5.as_ptr()) };
    if !f11_r[0].is_nan() {
        ret += f11_r[0] as f64;
    }
    if !f11_r[1].is_nan() {
        ret += f11_r[1] as f64;
    }
    if !f11_r[2].is_nan() {
        ret += f11_r[2] as f64;
    }

    let mut f12_r = [0.0f32, 0.0f32, 0.0f32];
    let f12_5 = [f12_2, f12_3, f12_4];
    unsafe { f12(f12_r.as_mut_ptr(), f12_5.as_ptr()) };
    if !f12_r[0].is_nan() {
        ret += f12_r[0] as f64;
    }
    if !f12_r[1].is_nan() {
        ret += f12_r[1] as f64;
    }
    if !f12_r[2].is_nan() {
        ret += f12_r[2] as f64;
    }

    let mut f13_r = [0.0f32, 0.0f32, 0.0f32];
    let f13_5 = [f13_2, f13_3, f13_4];
    unsafe { f13(f13_r.as_mut_ptr(), f13_5.as_ptr()) };
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
