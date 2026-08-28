use std::ffi::{c_int, c_void};

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

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

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
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
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
pub unsafe extern "C" fn f2(
    a: *const c_void,
    type_a: c_int,
    b: *const c_void,
    type_b: c_int,
) -> c_int {
    match type_a {
        C2_TYPE_CIRCLE => match type_b {
            C2_TYPE_CIRCLE => {
                // SAFETY: The C ABI requires pointers matching their type tags.
                unsafe { c2CircletoCircle(*a.cast::<C2Circle>(), *b.cast::<C2Circle>()) }
            }
            C2_TYPE_AABB => {
                // SAFETY: The C ABI requires pointers matching their type tags.
                unsafe { c2CircletoAABB(*a.cast::<C2Circle>(), *b.cast::<C2Aabb>()) }
            }
            _ => 0,
        },
        C2_TYPE_AABB => match type_b {
            C2_TYPE_CIRCLE => {
                // SAFETY: The C ABI requires pointers matching their type tags.
                unsafe { c2CircletoAABB(*b.cast::<C2Circle>(), *a.cast::<C2Aabb>()) }
            }
            C2_TYPE_AABB => {
                // SAFETY: The C ABI requires pointers matching their type tags.
                unsafe { c2AABBtoAABB(*a.cast::<C2Aabb>(), *b.cast::<C2Aabb>()) }
            }
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

    let (q, r);
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
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        q = (-((-(v1.wrapping_add(v2))) / v2)).wrapping_sub(1);
        r = -((-(v1.wrapping_add(v2))) % v2);
    } else if v2 != c_int::MIN {
        q = ((-(v1.wrapping_sub(v2))) / (-v2)).wrapping_add(1);
        r = -((-(v1.wrapping_sub(v2))) % (-v2));
    } else {
        q = 1;
        r = 0;
    }

    if r >= 0 {
        q
    } else {
        q.wrapping_add(if v2 > 0 { -1 } else { 1 })
    }
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
pub unsafe extern "C" fn f4(rnd: *mut CnRnd) -> f64 {
    // SAFETY: The C ABI requires a valid mutable cn_rnd_t pointer.
    let value = cn_rnd_next(unsafe { &mut *rnd });
    let exponent = 1023_u64;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
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
    let non_stereo = u32::from(channels != 2);
    let bits = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(non_stereo))
        .wrapping_add(blocksize.wrapping_mul(bitdepth).wrapping_mul(stereo))
        .wrapping_add(
            blocksize
                .wrapping_mul(bitdepth.wrapping_add(u32::from(bitdepth != 32)))
                .wrapping_mul(stereo),
        )
        .wrapping_add(7);
    18_u32.wrapping_add(channels).wrapping_add(bits / 8)
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
    lm_v2(u, v)
}

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> f32 {
    let sign = (u32::from(h & 0x8000)) << 16;
    let exponent = u32::from((h >> 10) & 0x1f);
    let mut mantissa = u32::from(h & 0x03ff);

    let bits = if exponent == 0 {
        if mantissa == 0 {
            sign
        } else {
            let mut adjusted_exponent = 113_u32;
            while mantissa & 0x0400 == 0 {
                mantissa <<= 1;
                adjusted_exponent -= 1;
            }
            mantissa &= 0x03ff;
            sign | (adjusted_exponent << 23) | (mantissa << 13)
        }
    } else if exponent == 31 {
        sign | 0x7f80_0000 | (mantissa << 13)
    } else {
        sign | ((exponent + 112) << 23) | (mantissa << 13)
    };

    f32::from_bits(bits)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f11(dest: *mut f32, src: *const f32) {
    // SAFETY: The C ABI requires src and dest to each reference three floats.
    let (h, s, l) = unsafe { (*src, *src.add(1), *src.add(2)) };
    if s == 0.0 {
        unsafe {
            *dest = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }

    let c = (1.0_f32 - (2.0_f32 * l - 1.0_f32).abs()) * s;
    let m = 1.0_f32 * (l - 0.5_f32 * c);
    let x = c * (1.0_f32 - ((h / 60.0_f32) % 2.0_f32 - 1.0_f32).abs());
    let out = if (0.0..60.0).contains(&h) {
        [c + m, x + m, m]
    } else if (60.0..120.0).contains(&h) {
        [x + m, c + m, m]
    } else if h < 120.0 && h < 180.0 {
        [m, c + m, x + m]
    } else if (180.0..240.0).contains(&h) {
        [m, x + m, c + m]
    } else if (240.0..300.0).contains(&h) {
        [x + m, m, c + m]
    } else if (300.0..360.0).contains(&h) {
        [c + m, m, x + m]
    } else {
        [m, m, m]
    };
    unsafe {
        *dest = out[0];
        *dest.add(1) = out[1];
        *dest.add(2) = out[2];
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn f12(dest: *mut f32, src: *const f32) {
    // SAFETY: The C ABI requires src and dest to each reference three floats.
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
    let i = h.floor() as c_int;
    let f = h - i as f32;
    let p = v * (1.0_f32 - s);
    let q = v * (1.0_f32 - s * f);
    let t = v * (1.0_f32 - s * (1.0_f32 - f));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    unsafe {
        *dest = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn f13(dest: *mut f32, src: *const f32) {
    // SAFETY: The C ABI requires src and dest to each reference three floats.
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
    // SAFETY: Both pointers reference values matching their type tags.
    ret += unsafe {
        f2(
            (&f2_5 as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
            (&f2_11 as *const C2Aabb).cast(),
            C2_TYPE_AABB,
        )
    } as f64;

    ret += f3(f3_1, f3_2) as f64;

    let mut f4_3 = CnRnd {
        state: [f4_1, f4_2],
    };
    // SAFETY: f4_3 is a valid mutable state value.
    let f4_r = unsafe { f4(&mut f4_3) };
    if !f4_r.is_nan() {
        ret += f4_r;
    }

    ret += f5(f5_1) as f64;
    ret += f7(f7_1, f7_2, f7_3) as f64;

    let f9_r = f9(
        LmVec2 { x: f9_1, y: f9_2 },
        LmVec2 { x: f9_4, y: f9_5 },
        LmVec2 { x: f9_7, y: f9_8 },
        LmVec2 { x: f9_10, y: f9_11 },
    );
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

    let mut out = [0.0_f32; 3];
    let f11_5 = [f11_2, f11_3, f11_4];
    // SAFETY: Both arrays contain three floats.
    unsafe { f11(out.as_mut_ptr(), f11_5.as_ptr()) };
    for value in out {
        if !value.is_nan() {
            ret += value as f64;
        }
    }

    out = [0.0_f32; 3];
    let f12_5 = [f12_2, f12_3, f12_4];
    // SAFETY: Both arrays contain three floats.
    unsafe { f12(out.as_mut_ptr(), f12_5.as_ptr()) };
    for value in out {
        if !value.is_nan() {
            ret += value as f64;
        }
    }

    out = [0.0_f32; 3];
    let f13_5 = [f13_2, f13_3, f13_4];
    // SAFETY: Both arrays contain three floats.
    unsafe { f13(out.as_mut_ptr(), f13_5.as_ptr()) };
    for value in out {
        if !value.is_nan() {
            ret += value as f64;
        }
    }

    ret
}
