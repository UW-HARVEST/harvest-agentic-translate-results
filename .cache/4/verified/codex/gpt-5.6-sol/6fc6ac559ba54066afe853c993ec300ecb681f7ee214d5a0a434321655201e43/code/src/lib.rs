use std::ffi::{c_int, c_void};

#[link(name = "m")]
unsafe extern "C" {
    fn fabsf(value: f32) -> f32;
    fn floorf(value: f32) -> f32;
    fn fmodf(numerator: f32, denominator: f32) -> f32;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CnRnd {
    pub state: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LmVec2 {
    pub x: f32,
    pub y: f32,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2Aabb) -> c_int {
    let closest = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, closest);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    (!(d0 | d1 | d2 | d3)) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn f2(a: *const c_void, type_a: c_int, b: *const c_void, type_b: c_int) -> c_int {
    match type_a {
        0 => match type_b {
            0 => {
                // SAFETY: This has the same caller contract as the C function.
                unsafe { c2CircletoCircle(*(a.cast::<C2Circle>()), *(b.cast::<C2Circle>())) }
            }
            1 => unsafe { c2CircletoAABB(*(a.cast::<C2Circle>()), *(b.cast::<C2Aabb>())) },
            _ => 0,
        },
        1 => match type_b {
            0 => unsafe { c2CircletoAABB(*(b.cast::<C2Circle>()), *(a.cast::<C2Aabb>())) },
            1 => unsafe { c2AABBtoAABB(*(a.cast::<C2Aabb>()), *(b.cast::<C2Aabb>())) },
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

    let dividend = i64::from(v1);
    let divisor = i64::from(v2);
    let quotient = dividend / divisor;
    let remainder = dividend % divisor;
    let result = if remainder >= 0 {
        quotient
    } else {
        quotient + if v2 > 0 { -1 } else { 1 }
    };
    result as c_int
}

fn cn_rnd_next(rnd: &mut CnRnd) -> u64 {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

#[unsafe(no_mangle)]
pub extern "C" fn f4(rnd: *mut CnRnd) -> f64 {
    // SAFETY: This has the same non-null, aligned pointer contract as the C function.
    let value = cn_rnd_next(unsafe { &mut *rnd });
    let result = (1023_u64 << 52) | (value >> 12);
    f64::from_bits(result) - 1.0
}

#[unsafe(no_mangle)]
pub extern "C" fn f5(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

#[unsafe(no_mangle)]
pub extern "C" fn f7(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let stereo = u32::from(channels == 2);
    let not_stereo = u32::from(channels != 2);
    let not_32_bit = u32::from(bitdepth != 32);

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(not_stereo));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(stereo);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(not_32_bit))
        .wrapping_mul(stereo);
    let bytes = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7)
        / 8;

    18_u32.wrapping_add(channels).wrapping_add(bytes)
}

fn lm_sub2(a: LmVec2, b: LmVec2) -> LmVec2 {
    LmVec2 {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn lm_dot2(a: LmVec2, b: LmVec2) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn f9(p1: LmVec2, p2: LmVec2, p3: LmVec2, p: LmVec2) -> LmVec2 {
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
    LmVec2 { x: u, y: v }
}

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> f32 {
    let sign = (u32::from(h & 0x8000)) << 16;
    let exponent = u32::from((h >> 10) & 0x1f);
    let mantissa = u32::from(h & 0x03ff);

    let bits = match exponent {
        0 if mantissa == 0 => sign,
        0 => {
            let leading_bit = 31 - mantissa.leading_zeros();
            let float_exponent = leading_bit + 103;
            let fraction = (mantissa - (1 << leading_bit)) << (23 - leading_bit);
            sign | (float_exponent << 23) | fraction
        }
        31 => sign | 0x7f80_0000 | (mantissa << 13),
        _ => sign | ((exponent + 112) << 23) | (mantissa << 13),
    };
    f32::from_bits(bits)
}

#[unsafe(no_mangle)]
pub extern "C" fn f11(dest: *mut f32, src: *const f32) {
    // SAFETY: Both pointers have the same three-element array contract as in C.
    let (h, s, l) = unsafe { (*src, *src.add(1), *src.add(2)) };
    if s == 0.0 {
        unsafe {
            *dest = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }

    let c = unsafe { (1.0_f32 - fabsf(2.0_f32 * l - 1.0_f32)) * s };
    let m = 1.0_f32 * (l - 0.5_f32 * c);
    let x = unsafe { c * (1.0_f32 - fabsf(fmodf(h / 60.0_f32, 2.0_f32) - 1.0_f32)) };
    let rgb = if h >= 0.0 && h < 60.0 {
        [c + m, x + m, m]
    } else if h >= 60.0 && h < 120.0 {
        [x + m, c + m, m]
    } else if h < 120.0 && h < 180.0 {
        [m, c + m, x + m]
    } else if h >= 180.0 && h < 240.0 {
        [m, x + m, c + m]
    } else if h >= 240.0 && h < 300.0 {
        [x + m, m, c + m]
    } else if h >= 300.0 && h < 360.0 {
        [c + m, m, x + m]
    } else {
        [m, m, m]
    };
    unsafe {
        *dest = rgb[0];
        *dest.add(1) = rgb[1];
        *dest.add(2) = rgb[2];
    }
}

fn float_to_c_int(value: f32) -> c_int {
    if !value.is_finite() || value < c_int::MIN as f32 || value >= 2_147_483_648.0_f32 {
        c_int::MIN
    } else {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn f12(dest: *mut f32, src: *const f32) {
    // SAFETY: Both pointers have the same three-element array contract as in C.
    let (mut h, s, v) = unsafe { (*src, *src.add(1), *src.add(2)) };
    if s == 0.0 {
        unsafe {
            *dest = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
        }
        return;
    }

    h /= 60.0_f32;
    let i = float_to_c_int(unsafe { floorf(h) });
    let f = h - i as f32;
    let p = v * (1.0_f32 - s);
    let q = v * (1.0_f32 - s * f);
    let t = v * (1.0_f32 - s * (1.0_f32 - f));
    let rgb = match i {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    };
    unsafe {
        *dest = rgb[0];
        *dest.add(1) = rgb[1];
        *dest.add(2) = rgb[2];
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn f13(dest: *mut f32, src: *const f32) {
    // SAFETY: Both pointers have the same three-element array contract as in C.
    let (r, g, b) = unsafe { (*src, *src.add(1), *src.add(2)) };
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
        unsafe {
            *dest = h;
            *dest.add(1) = s;
            *dest.add(2) = v;
        }
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
    unsafe {
        *dest = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
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
    f7_1: u32,
    f7_2: u32,
    f7_3: u32,
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

    let f2_5 = C2Circle {
        p: C2v { x: f2_1, y: f2_2 },
        r: f2_3,
    };
    let f2_11 = C2Aabb {
        min: C2v { x: f2_7, y: f2_8 },
        max: C2v { x: f2_9, y: f2_10 },
    };
    ret += f64::from(f2(
        (&raw const f2_5).cast(),
        0,
        (&raw const f2_11).cast(),
        1,
    ));

    ret += f64::from(f3(f3_1, f3_2));

    let mut f4_3 = CnRnd {
        state: [f4_1, f4_2],
    };
    let f4_r = f4(&raw mut f4_3);
    if !f4_r.is_nan() {
        ret += f4_r;
    }

    ret += f64::from(f5(f5_1));
    ret += f64::from(f7(f7_1, f7_2, f7_3));

    let f9_r = f9(
        LmVec2 { x: f9_1, y: f9_2 },
        LmVec2 { x: f9_4, y: f9_5 },
        LmVec2 { x: f9_7, y: f9_8 },
        LmVec2 { x: f9_10, y: f9_11 },
    );
    if !f9_r.x.is_nan() {
        ret += f64::from(f9_r.x);
    }
    if !f9_r.y.is_nan() {
        ret += f64::from(f9_r.y);
    }

    let f10_r = f10(f10_1);
    if !f10_r.is_nan() {
        ret += f64::from(f10_r);
    }

    let mut f11_r = [0.0_f32; 3];
    let f11_5 = [f11_2, f11_3, f11_4];
    f11(f11_r.as_mut_ptr(), f11_5.as_ptr());
    for value in f11_r {
        if !value.is_nan() {
            ret += f64::from(value);
        }
    }

    let mut f12_r = [0.0_f32; 3];
    let f12_5 = [f12_2, f12_3, f12_4];
    f12(f12_r.as_mut_ptr(), f12_5.as_ptr());
    for value in f12_r {
        if !value.is_nan() {
            ret += f64::from(value);
        }
    }

    let mut f13_r = [0.0_f32; 3];
    let f13_5 = [f13_2, f13_3, f13_4];
    f13(f13_r.as_mut_ptr(), f13_5.as_ptr());
    for value in f13_r {
        if !value.is_nan() {
            ret += f64::from(value);
        }
    }

    ret
}
