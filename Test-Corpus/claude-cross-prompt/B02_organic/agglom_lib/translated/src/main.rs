// Translation of c_src/src/lib.c to Rust
// Public function: agglom() - aggregates results from sub-functions f2..f13
// Executable reads 33 whitespace-separated values from stdin (scanf-style)
// and prints the result with printf("%f\n", ret) format.

use std::io::Read;

mod tables;

// ---- c2 (collision) types ----

#[derive(Clone, Copy)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Clone, Copy)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum C2Type {
    Circle,
    Aabb,
}

fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2_maxv(lo, c2_minv(a, hi))
}

fn c2_sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_circle_to_circle(a: C2Circle, b: C2Circle) -> i32 {
    let c = c2_sub(b.p, a.p);
    let d2 = c2_dot(c, c);
    let mut r2 = a.r + b.r;
    r2 = r2 * r2;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_circle_to_aabb(a: C2Circle, b: C2AABB) -> i32 {
    let l = c2_clampv(a.p, b.min, b.max);
    let ab = c2_sub(a.p, l);
    let d2 = c2_dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 { 1 } else { 0 }
}

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    if (d0 | d1 | d2 | d3) == 0 { 1 } else { 0 }
}

enum C2Shape {
    Circle(C2Circle),
    Aabb(C2AABB),
}

fn f2(a: &C2Shape, type_a: C2Type, b: &C2Shape, type_b: C2Type) -> i32 {
    match type_a {
        C2Type::Circle => match type_b {
            C2Type::Circle => {
                if let (C2Shape::Circle(ca), C2Shape::Circle(cb)) = (a, b) {
                    c2_circle_to_circle(*ca, *cb)
                } else {
                    0
                }
            }
            C2Type::Aabb => {
                if let (C2Shape::Circle(ca), C2Shape::Aabb(ab)) = (a, b) {
                    c2_circle_to_aabb(*ca, *ab)
                } else {
                    0
                }
            }
        },
        C2Type::Aabb => match type_b {
            C2Type::Circle => {
                if let (C2Shape::Aabb(aa), C2Shape::Circle(cb)) = (a, b) {
                    c2_circle_to_aabb(*cb, *aa)
                } else {
                    0
                }
            }
            C2Type::Aabb => {
                if let (C2Shape::Aabb(aa), C2Shape::Aabb(bb)) = (a, b) {
                    c2_aabb_to_aabb(*aa, *bb)
                } else {
                    0
                }
            }
        },
    }
}

// ---- f3: signed division/round-toward-negative-infinity (matches C source) ----

fn f3(v1: i32, v2: i32) -> i32 {
    if v2 == 0 {
        return 0;
    }
    // The C code is intentionally tricky with chained if/else
    // Recreate the exact control flow.
    let q: i32;
    let r: i32;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != i32::MIN {
            q = -(v1 / -v2);
            r = v1 % -v2;
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != i32::MIN {
        if v2 >= 0 {
            q = -((-v1) / v2);
            r = -((-v1) % v2);
        } else if v2 != i32::MIN {
            q = (-v1) / -v2;
            r = -((-v1) % -v2);
        } else {
            q = 1;
            // r = v1 - q * v2
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        // v1 == INT_MIN, v2 >= 0
        // q = -((-(v1 + v2)) / v2) - 1, r = -((-(v1 + v2)) % v2)
        let s = v1.wrapping_add(v2);
        let neg = s.wrapping_neg();
        q = -(neg / v2) - 1;
        r = -(neg % v2);
    } else if v2 != i32::MIN {
        // v1 == INT_MIN, v2 < 0 and != INT_MIN
        // q = ((-(v1 - v2)) / (-v2)) + 1, r = -((-(v1 - v2)) % (-v2))
        let s = v1.wrapping_sub(v2);
        let neg = s.wrapping_neg();
        q = (neg / -v2) + 1;
        r = -(neg % -v2);
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 {
        q
    } else {
        q + (if v2 > 0 { -1 } else { 1 })
    }
}

// ---- f4: PRNG-based double in [0, 1) ----

struct CnRnd {
    state: [u64; 2],
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

fn f4(rnd: &mut CnRnd) -> f64 {
    let value = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

// ---- f5: bit reverse low 16 bits ----

fn f5(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

// ---- f7: tflac frame size estimate ----

fn f7(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let chan_ne2 = if channels != 2 { 1u32 } else { 0u32 };
    let chan_eq2 = if channels == 2 { 1u32 } else { 0u32 };
    let bd_ne32 = if bitdepth != 32 { 1u32 } else { 0u32 };

    let part1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(chan_ne2));
    let part2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(chan_eq2);
    let part3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(bd_ne32))
        .wrapping_mul(chan_eq2);

    18u32
        .wrapping_add(channels)
        .wrapping_add(
            part1
                .wrapping_add(part2)
                .wrapping_add(part3)
                .wrapping_add(7)
                / 8,
        )
}

// ---- f9: barycentric coordinates ----

#[derive(Clone, Copy)]
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

// ---- f10: half-float to float ----

fn f10(h: u16) -> f32 {
    let n = (h >> 10) as usize;
    let bits = tables::M_MANTISSA[(h as usize & 0x3ff) + tables::M_OFFSET[n] as usize]
        .wrapping_add(tables::M_EXPONENT[n]);
    f32::from_bits(bits)
}

// ---- f11: HSL -> RGB ----

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
    let c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    let m = 1.0f32 * (l - 0.5f32 * c);
    // fmodf in C
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
        // Original C bug preserved: dead branch that requires h < 120.0
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

// ---- f12: HSV -> RGB ----

fn f12(dest: &mut [f32; 3], src: &[f32; 3]) {
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

// ---- f13: RGB -> HSV ----

fn f13(dest: &mut [f32; 3], src: &[f32; 3]) {
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

// ---- agglom ----

#[allow(clippy::too_many_arguments)]
fn agglom(
    f2_1: f32, f2_2: f32, f2_3: f32, f2_7: f32, f2_8: f32, f2_9: f32, f2_10: f32,
    f3_1: i32, f3_2: i32,
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

    let f2_5 = C2Shape::Circle(C2Circle {
        p: C2v { x: f2_1, y: f2_2 },
        r: f2_3,
    });
    let f2_6 = C2Type::Circle;

    let f2_11 = C2Shape::Aabb(C2AABB {
        min: C2v { x: f2_7, y: f2_8 },
        max: C2v { x: f2_9, y: f2_10 },
    });
    let f2_12 = C2Type::Aabb;

    let f2_r = f2(&f2_5, f2_6, &f2_11, f2_12);
    ret += f2_r as f64;

    let f3_r = f3(f3_1, f3_2);
    ret += f3_r as f64;

    let mut f4_3 = CnRnd { state: [f4_1, f4_2] };
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

    let mut f11_r = [0.0f32; 3];
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

    let mut f12_r = [0.0f32; 3];
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

    let mut f13_r = [0.0f32; 3];
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

// ---- printf %f format implementation ----
// C's printf("%f\n", x) prints with 6 decimal digits after the decimal point.
// Special cases:
//   NaN -> "nan", -NaN -> "-nan"
//   +inf -> "inf", -inf -> "-inf"
fn format_f64_percent_f(x: f64) -> String {
    if x.is_nan() {
        // Detect sign bit
        if x.is_sign_negative() {
            return "-nan".to_string();
        } else {
            return "nan".to_string();
        }
    }
    if x.is_infinite() {
        if x.is_sign_negative() {
            return "-inf".to_string();
        } else {
            return "inf".to_string();
        }
    }
    // Use Rust's formatting which matches printf("%f") for finite values.
    // %f default precision in C is 6.
    format!("{:.6}", x)
}

fn read_all_stdin() -> String {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap_or(0);
    s
}

fn next_token<'a>(iter: &mut std::str::SplitAsciiWhitespace<'a>) -> Option<&'a str> {
    iter.next()
}

fn parse_f32(iter: &mut std::str::SplitAsciiWhitespace<'_>) -> f32 {
    next_token(iter).and_then(|t| t.parse::<f32>().ok()).unwrap_or(0.0)
}

fn parse_i32(iter: &mut std::str::SplitAsciiWhitespace<'_>) -> i32 {
    next_token(iter).and_then(|t| t.parse::<i32>().ok()).unwrap_or(0)
}

fn parse_u64(iter: &mut std::str::SplitAsciiWhitespace<'_>) -> u64 {
    next_token(iter).and_then(|t| t.parse::<u64>().ok()).unwrap_or(0)
}

fn parse_u32(iter: &mut std::str::SplitAsciiWhitespace<'_>) -> u32 {
    next_token(iter).and_then(|t| t.parse::<u32>().ok()).unwrap_or(0)
}

fn parse_u16(iter: &mut std::str::SplitAsciiWhitespace<'_>) -> u16 {
    next_token(iter).and_then(|t| t.parse::<u16>().ok()).unwrap_or(0)
}

fn main() {
    let input = read_all_stdin();
    let mut iter = input.split_ascii_whitespace();

    let f2_1 = parse_f32(&mut iter);
    let f2_2 = parse_f32(&mut iter);
    let f2_3 = parse_f32(&mut iter);
    let f2_7 = parse_f32(&mut iter);
    let f2_8 = parse_f32(&mut iter);
    let f2_9 = parse_f32(&mut iter);
    let f2_10 = parse_f32(&mut iter);

    let f3_1 = parse_i32(&mut iter);
    let f3_2 = parse_i32(&mut iter);

    let f4_1 = parse_u64(&mut iter);
    let f4_2 = parse_u64(&mut iter);

    let f5_1 = parse_u32(&mut iter);
    let f7_1 = parse_u32(&mut iter);
    let f7_2 = parse_u32(&mut iter);
    let f7_3 = parse_u32(&mut iter);

    let f9_1 = parse_f32(&mut iter);
    let f9_2 = parse_f32(&mut iter);
    let f9_4 = parse_f32(&mut iter);
    let f9_5 = parse_f32(&mut iter);
    let f9_7 = parse_f32(&mut iter);
    let f9_8 = parse_f32(&mut iter);
    let f9_10 = parse_f32(&mut iter);
    let f9_11 = parse_f32(&mut iter);

    let f10_1 = parse_u16(&mut iter);

    let f11_2 = parse_f32(&mut iter);
    let f11_3 = parse_f32(&mut iter);
    let f11_4 = parse_f32(&mut iter);
    let f12_2 = parse_f32(&mut iter);
    let f12_3 = parse_f32(&mut iter);
    let f12_4 = parse_f32(&mut iter);
    let f13_2 = parse_f32(&mut iter);
    let f13_3 = parse_f32(&mut iter);
    let f13_4 = parse_f32(&mut iter);

    let result = agglom(
        f2_1, f2_2, f2_3, f2_7, f2_8, f2_9, f2_10,
        f3_1, f3_2,
        f4_1, f4_2,
        f5_1,
        f7_1, f7_2, f7_3,
        f9_1, f9_2, f9_4, f9_5, f9_7, f9_8, f9_10, f9_11,
        f10_1,
        f11_2, f11_3, f11_4,
        f12_2, f12_3, f12_4,
        f13_2, f13_3, f13_4,
    );

    println!("{}", format_f64_percent_f(result));
}
