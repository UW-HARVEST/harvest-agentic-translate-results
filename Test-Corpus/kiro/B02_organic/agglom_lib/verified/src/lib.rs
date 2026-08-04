use std::os::raw::c_int;

// --- Types ---

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Circle { p: C2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2AABB { min: C2v, max: C2v }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LmVec2 { x: f32, y: f32 }

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

// --- Collision helpers (exported with C names) ---

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v { C2v { x, y } }

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v { c2Maxv(lo, c2Minv(a, hi)) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v { a.x -= b.x; a.y -= b.y; a }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 { a.x * b.x + a.y * b.y }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2AABB) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2AABB, b: C2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn f2(a: *const u8, type_a: c_int, b: *const u8, type_b: c_int) -> c_int {
    unsafe {
        match type_a {
            C2_TYPE_CIRCLE => match type_b {
                C2_TYPE_CIRCLE => c2CircletoCircle(*(a as *const C2Circle), *(b as *const C2Circle)),
                C2_TYPE_AABB => c2CircletoAABB(*(a as *const C2Circle), *(b as *const C2AABB)),
                _ => 0,
            },
            C2_TYPE_AABB => match type_b {
                C2_TYPE_CIRCLE => c2CircletoAABB(*(b as *const C2Circle), *(a as *const C2AABB)),
                C2_TYPE_AABB => c2AABBtoAABB(*(a as *const C2AABB), *(b as *const C2AABB)),
                _ => 0,
            },
            _ => 0,
        }
    }
}

// --- Integer division (f3) ---

const INT_MIN: c_int = -0x7fffffff - 1;

#[unsafe(no_mangle)]
pub extern "C" fn f3(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 { return 0; }
    let (q, r);
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != INT_MIN {
            q = -(v1 / (-v2));
            r = v1 % (-v2);
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != INT_MIN {
        if v2 >= 0 {
            q = -((-v1) / v2);
            r = -((-v1) % v2);
        } else if v2 != INT_MIN {
            q = (-v1) / (-v2);
            r = -((-v1) % (-v2));
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        q = -((-(v1.wrapping_add(v2))) / v2) - 1;
        r = -((-(v1.wrapping_add(v2))) % v2);
    } else if v2 != INT_MIN {
        q = ((-(v1.wrapping_sub(v2))) / (-v2)).wrapping_add(1);
        r = -((-(v1.wrapping_sub(v2))) % (-v2));
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 { q } else { q + if v2 > 0 { -1 } else { 1 } }
}

// --- RNG (f4) ---

#[repr(C)]
pub struct CnRnd { state: [u64; 2] }

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
    let rnd = unsafe { &mut *rnd };
    let value = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

// --- Bit reverse (f5) ---

#[unsafe(no_mangle)]
pub extern "C" fn f5(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

// --- CRC16 tables (used nowhere in public API but present in C) ---
// The tables are static data; we include them for completeness but they aren't
// called by agglom. Omitting to keep the translation minimal.

// --- TFLAC buffer size (f7) ---

#[unsafe(no_mangle)]
pub extern "C" fn f7(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    18u32.wrapping_add(channels)
        .wrapping_add(
            ((blocksize.wrapping_mul(bitdepth).wrapping_mul(channels.wrapping_mul((channels != 2) as u32)))
                .wrapping_add(blocksize.wrapping_mul(bitdepth).wrapping_mul((channels == 2) as u32))
                .wrapping_add(blocksize.wrapping_mul(bitdepth.wrapping_add((bitdepth != 32) as u32)).wrapping_mul((channels == 2) as u32))
                .wrapping_add(7))
                / 8,
        )
}

// --- Barycentric coords (f9) ---

fn lm_v2(x: f32, y: f32) -> LmVec2 { LmVec2 { x, y } }
fn lm_sub2(a: LmVec2, b: LmVec2) -> LmVec2 { lm_v2(a.x - b.x, a.y - b.y) }
fn lm_dot2(a: LmVec2, b: LmVec2) -> f32 { a.x * b.x + a.y * b.y }

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
    let inv_denom = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_v2(u, v)
}

// --- Half-float to float (f10) ---

include!("tables.rs");

#[unsafe(no_mangle)]
pub extern "C" fn f10(h: u16) -> f32 {
    let n = (h >> 10) as usize;
    let idx = ((h & 0x3ff) as usize) + (M_OFFSET[n] as usize);
    let num = M_MANTISSA_TABLE[idx].wrapping_add(M_EXPONENT[n]);
    f32::from_bits(num)
}

// --- HSL to RGB (f11) ---

#[unsafe(no_mangle)]
pub extern "C" fn f11(dest: *mut f32, src: *const f32) {
    let (dest, src) = unsafe {
        (&mut *(dest as *mut [f32; 3]), &*(src as *const [f32; 3]))
    };
    f11_inner(dest, src);
}

fn f11_inner(dest: &mut [f32; 3], src: &[f32; 3]) {
    let h = src[0];
    let s = src[1];
    let l = src[2];
    if s == 0.0 {
        dest[0] = l; dest[1] = l; dest[2] = l;
        return;
    }
    let c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    let m = 1.0f32 * (l - 0.5f32 * c);
    let x = c * (1.0f32 - ((h / 60.0f32) % 2.0 - 1.0f32).abs());
    if h >= 0.0 && h < 60.0 {
        dest[0] = c + m; dest[1] = x + m; dest[2] = m;
    } else if h >= 60.0 && h < 120.0 {
        dest[0] = x + m; dest[1] = c + m; dest[2] = m;
    } else if h < 120.0 && h < 180.0 {
        // Note: C code has `h < 120.0f && h < 180.0f` (bug preserved)
        dest[0] = m; dest[1] = c + m; dest[2] = x + m;
    } else if h >= 180.0 && h < 240.0 {
        dest[0] = m; dest[1] = x + m; dest[2] = c + m;
    } else if h >= 240.0 && h < 300.0 {
        dest[0] = x + m; dest[1] = m; dest[2] = c + m;
    } else if h >= 300.0 && h < 360.0 {
        dest[0] = c + m; dest[1] = m; dest[2] = x + m;
    } else {
        dest[0] = m; dest[1] = m; dest[2] = m;
    }
}

// --- HSV to RGB (f12) ---

#[unsafe(no_mangle)]
pub extern "C" fn f12(dest: *mut f32, src: *const f32) {
    let (dest, src) = unsafe {
        (&mut *(dest as *mut [f32; 3]), &*(src as *const [f32; 3]))
    };
    f12_inner(dest, src);
}

fn f12_inner(dest: &mut [f32; 3], src: &[f32; 3]) {
    let mut h = src[0];
    let s = src[1];
    let v = src[2];
    if s == 0.0 {
        dest[0] = v; dest[1] = v; dest[2] = v;
        return;
    }
    h /= 60.0f32;
    let i = h.floor() as c_int;
    let f = h - i as f32;
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
    dest[0] = r; dest[1] = g; dest[2] = b;
}

// --- RGB to HSV (f13) ---

#[unsafe(no_mangle)]
pub extern "C" fn f13(dest: *mut f32, src: *const f32) {
    let (dest, src) = unsafe {
        (&mut *(dest as *mut [f32; 3]), &*(src as *const [f32; 3]))
    };
    f13_inner(dest, src);
}

fn f13_inner(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r = src[0];
    let g = src[1];
    let b = src[2];
    let mut h: f32 = 0.0;
    let s: f32;
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
        dest[0] = h; dest[1] = 0.0; dest[2] = v;
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
    if h < 0.0 { h += 360.0; }
    dest[0] = h; dest[1] = s; dest[2] = v;
}

// --- Public agglom function ---

#[unsafe(no_mangle)]
pub extern "C" fn agglom(
    f2_1: f32, f2_2: f32, f2_3: f32, f2_7: f32, f2_8: f32, f2_9: f32, f2_10: f32,
    f3_1: c_int, f3_2: c_int,
    f4_1: u64, f4_2: u64,
    f5_1: u32,
    f7_1: u32, f7_2: u32, f7_3: u32,
    f9_1: f32, f9_2: f32, f9_4: f32, f9_5: f32, f9_7: f32, f9_8: f32, f9_10: f32, f9_11: f32,
    f10_1: u16,
    f11_2: f32, f11_3: f32, f11_4: f32,
    f12_2: f32, f12_3: f32, f12_4: f32,
    f13_2: f32, f13_3: f32, f13_4: f32,
) -> f64 {
    let mut ret: f64 = 0.0;

    let f2_5 = C2Circle { p: C2v { x: f2_1, y: f2_2 }, r: f2_3 };
    let f2_6 = C2_TYPE_CIRCLE;
    let f2_11 = C2AABB { min: C2v { x: f2_7, y: f2_8 }, max: C2v { x: f2_9, y: f2_10 } };
    let f2_12 = C2_TYPE_AABB;
    let f2_r = f2(
        &f2_5 as *const C2Circle as *const u8,
        f2_6,
        &f2_11 as *const C2AABB as *const u8,
        f2_12,
    );
    ret += f2_r as f64;

    let f3_r = f3(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = CnRnd { state: [f4_1, f4_2] };
    let f4_r = f4(&mut f4_3 as *mut CnRnd);
    if !f4_r.is_nan() { ret += f4_r; }

    let f5_r = f5(f5_1);
    ret += f5_r as f64;

    let f7_r = f7(f7_1, f7_2, f7_3);
    ret += f7_r as f64;

    let f9_3 = LmVec2 { x: f9_1, y: f9_2 };
    let f9_6 = LmVec2 { x: f9_4, y: f9_5 };
    let f9_9 = LmVec2 { x: f9_7, y: f9_8 };
    let f9_12 = LmVec2 { x: f9_10, y: f9_11 };
    let f9_r = f9(f9_3, f9_6, f9_9, f9_12);
    if !f9_r.x.is_nan() { ret += f9_r.x as f64; }
    if !f9_r.y.is_nan() { ret += f9_r.y as f64; }

    let f10_r = f10(f10_1);
    if !f10_r.is_nan() { ret += f10_r as f64; }

    let mut f11_r = [0.0f32; 3];
    let f11_5 = [f11_2, f11_3, f11_4];
    f11(f11_r.as_mut_ptr(), f11_5.as_ptr());
    if !f11_r[0].is_nan() { ret += f11_r[0] as f64; }
    if !f11_r[1].is_nan() { ret += f11_r[1] as f64; }
    if !f11_r[2].is_nan() { ret += f11_r[2] as f64; }

    let mut f12_r = [0.0f32; 3];
    let f12_5 = [f12_2, f12_3, f12_4];
    f12(f12_r.as_mut_ptr(), f12_5.as_ptr());
    if !f12_r[0].is_nan() { ret += f12_r[0] as f64; }
    if !f12_r[1].is_nan() { ret += f12_r[1] as f64; }
    if !f12_r[2].is_nan() { ret += f12_r[2] as f64; }

    let mut f13_r = [0.0f32; 3];
    let f13_5 = [f13_2, f13_3, f13_4];
    f13(f13_r.as_mut_ptr(), f13_5.as_ptr());
    if !f13_r[0].is_nan() { ret += f13_r[0] as f64; }
    if !f13_r[1].is_nan() { ret += f13_r[1] as f64; }
    if !f13_r[2].is_nan() { ret += f13_r[2] as f64; }

    ret
}
