// Translation of c_src/src/lib.c to Rust.
//
// The original C source is a library (no main()), so this executable produces
// no output, matching what a C program with an empty/no-op main would produce.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

#[derive(Copy, Clone, Default, Debug)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[derive(Copy, Clone)]
enum C2Type {
    Circle,
    Aabb,
    Capsule,
    Poly,
}

#[derive(Copy, Clone, Default, Debug)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Copy, Clone, Default, Debug)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Copy, Clone, Default, Debug)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Copy, Clone, Default, Debug)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Copy, Clone, Default, Debug)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Copy, Clone, Debug)]
struct C2Poly {
    count: i32,
    verts: [C2v; 8],
    norms: [C2v; 8],
}

impl Default for C2Poly {
    fn default() -> Self {
        Self {
            count: 0,
            verts: [C2v::default(); 8],
            norms: [C2v::default(); 8],
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
struct C2Ray {
    p: C2v,
    d: C2v,
    t: f32,
}

#[derive(Copy, Clone, Default, Debug)]
struct C2m {
    x: C2v,
    y: C2v,
}

fn c2v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c2_dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_len(a: C2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2_sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2_mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2_div(a: C2v, b: f32) -> C2v {
    c2_mulvs(a, 1.0 / b)
}

fn c2_norm(a: C2v) -> C2v {
    c2_div(a, c2_len(a))
}

fn c2_minv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2_maxv(a: C2v, b: C2v) -> C2v {
    c2v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2_skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

fn c2_absv(a: C2v) -> C2v {
    c2v(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

fn c2_ray_to_circle(a: C2Ray, b: C2Circle, out: &mut C2Raycast) -> i32 {
    let p = b.p;
    let m = c2_sub(a.p, p);
    let c = c2_dot(m, m) - b.r * b.r;
    let bb = c2_dot(m, a.d);
    let disc = bb * bb - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -bb - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        out.t = t;
        let impact = c2_add(a.p, c2_mulvs(a.d, t));
        out.n = c2_norm(c2_sub(impact, p));
        return 1;
    }
    0
}

fn c2_aabb_to_aabb(a: C2AABB, b: C2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    if (d0 | d1 | d2 | d3) != 0 { 0 } else { 1 }
}

#[inline]
fn c2_signed_dist_point_to_plane_one_dim(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

#[inline]
fn c2_ray_to_plane_one_dim(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0
    } else {
        let d = da - db;
        if d != 0.0 {
            da / d
        } else {
            0.0
        }
    }
}

fn c2_ray_to_aabb(a: C2Ray, b: C2AABB, out: &mut C2Raycast) -> i32 {
    let p0 = a.p;
    let p1 = c2_add(a.p, c2_mulvs(a.d, a.t));
    let a_box = C2AABB {
        min: c2_minv(p0, p1),
        max: c2_maxv(p0, p1),
    };
    if c2_aabb_to_aabb(a_box, b) == 0 {
        return 0;
    }
    let ab = c2_sub(p1, p0);
    let n = c2_skew(ab);
    let abs_n = c2_absv(n);
    let half_extents = c2_mulvs(c2_sub(b.max, b.min), 0.5);
    let center_of_b_box = c2_mulvs(c2_add(b.min, b.max), 0.5);
    let dot_val = c2_dot(n, c2_sub(p0, center_of_b_box));
    let abs_dot = if dot_val < 0.0 { -dot_val } else { dot_val };
    let d = abs_dot - c2_dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2_signed_dist_point_to_plane_one_dim(p0.x, -1.0, b.min.x);
    let db0 = c2_signed_dist_point_to_plane_one_dim(p1.x, -1.0, b.min.x);
    let da1 = c2_signed_dist_point_to_plane_one_dim(p0.x, 1.0, b.max.x);
    let db1 = c2_signed_dist_point_to_plane_one_dim(p1.x, 1.0, b.max.x);
    let da2 = c2_signed_dist_point_to_plane_one_dim(p0.y, -1.0, b.min.y);
    let db2 = c2_signed_dist_point_to_plane_one_dim(p1.y, -1.0, b.min.y);
    let da3 = c2_signed_dist_point_to_plane_one_dim(p0.y, 1.0, b.max.y);
    let db3 = c2_signed_dist_point_to_plane_one_dim(p1.y, 1.0, b.max.y);
    let mut t0 = c2_ray_to_plane_one_dim(da0, db0);
    let mut t1 = c2_ray_to_plane_one_dim(da1, db1);
    let mut t2 = c2_ray_to_plane_one_dim(da2, db2);
    let mut t3 = c2_ray_to_plane_one_dim(da3, db3);
    let hit0 = (t0 <= 1.0) as i32;
    let hit1 = (t1 <= 1.0) as i32;
    let hit2 = (t2 <= 1.0) as i32;
    let hit3 = (t3 <= 1.0) as i32;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = (hit0 as f32) * t0;
        t1 = (hit1 as f32) * t1;
        t2 = (hit2 as f32) * t2;
        t3 = (hit3 as f32) * t3;
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * a.t;
            out.n = c2v(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * a.t;
            out.n = c2v(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * a.t;
            out.n = c2v(0.0, -1.0);
        } else {
            out.t = t3 * a.t;
            out.n = c2v(0.0, 1.0);
        }
        1
    } else {
        0
    }
}

fn c2_ccw90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

fn c2_mulmv_t(a: C2m, b: C2v) -> C2v {
    C2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

fn c2_aabb_to_point(a: C2AABB, b: C2v) -> i32 {
    let d0 = (b.x < a.min.x) as i32;
    let d1 = (b.y < a.min.y) as i32;
    let d2 = (b.x > a.max.x) as i32;
    let d3 = (b.y > a.max.y) as i32;
    if (d0 | d1 | d2 | d3) != 0 { 0 } else { 1 }
}

fn c2_circle_to_point(a: C2Circle, b: C2v) -> i32 {
    let n = c2_sub(a.p, b);
    let d2 = c2_dot(n, n);
    if d2 < a.r * a.r { 1 } else { 0 }
}

fn c2_ray_to_capsule(a: C2Ray, b: C2Capsule, out: &mut C2Raycast) -> i32 {
    let mut m = C2m::default();
    m.y = c2_norm(c2_sub(b.b, b.a));
    m.x = c2_ccw90(m.y);
    let cap_n = c2_sub(b.b, b.a);
    let y_bb = c2_mulmv_t(m, cap_n);
    let y_ap = c2_mulmv_t(m, c2_sub(a.p, b.a));
    let y_ad = c2_mulmv_t(m, a.d);
    let y_ae = c2_add(y_ap, c2_mulvs(y_ad, a.t));
    let capsule_bb = C2AABB {
        min: c2v(-b.r, 0.0),
        max: c2v(b.r, y_bb.y),
    };
    out.n = c2_norm(cap_n);
    out.t = 0.0;
    if c2_aabb_to_point(capsule_bb, y_ap) != 0 {
        return 1;
    } else {
        let capsule_a = C2Circle { p: b.a, r: b.r };
        let capsule_b = C2Circle { p: b.b, r: b.r };
        if c2_circle_to_point(capsule_a, a.p) != 0 {
            return 1;
        } else if c2_circle_to_point(capsule_b, a.p) != 0 {
            return 1;
        }
    }
    let abs_yae_x = if y_ae.x < 0.0 { -y_ae.x } else { y_ae.x };
    let abs_yap_x = if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x };
    let min_abs = if abs_yae_x < abs_yap_x { abs_yae_x } else { abs_yap_x };
    if y_ae.x * y_ap.x < 0.0 || min_abs < b.r {
        let ca = C2Circle { p: b.a, r: b.r };
        let cb = C2Circle { p: b.b, r: b.r };
        let abs_yap_x_again = if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x };
        if abs_yap_x_again < b.r {
            if y_ap.y < 0.0 {
                return c2_ray_to_circle(a, ca, out);
            } else {
                return c2_ray_to_circle(a, cb, out);
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return c2_ray_to_circle(a, ca, out);
            }
            if y >= y_bb.y {
                return c2_ray_to_circle(a, cb, out);
            } else {
                out.n = if c > 0.0 { m.x } else { c2_skew(m.y) };
                out.t = t * a.t;
                return 1;
            }
        }
    }
    0
}

fn c2_rot_identity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

fn c2x_identity() -> C2x {
    C2x {
        p: c2v(0.0, 0.0),
        r: c2_rot_identity(),
    }
}

fn c2_mulrv(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_mulrv_t(a: C2r, b: C2v) -> C2v {
    c2v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2_mulxv_t(a: C2x, b: C2v) -> C2v {
    c2_mulrv_t(a.r, c2_sub(b, a.p))
}

fn c2_ray_to_poly(a: C2Ray, b: &C2Poly, bx_ptr: Option<&C2x>, out: &mut C2Raycast) -> i32 {
    let bx = match bx_ptr {
        Some(x) => *x,
        None => c2x_identity(),
    };
    let p = c2_mulxv_t(bx, a.p);
    let d = c2_mulrv_t(bx.r, a.d);
    let mut lo: f32 = 0.0;
    let mut hi: f32 = a.t;
    let mut index: i32 = !0;
    for i in 0..b.count {
        let i_us = i as usize;
        let num = c2_dot(b.norms[i_us], c2_sub(b.verts[i_us], p));
        let den = c2_dot(b.norms[i_us], d);
        if den == 0.0 && num < 0.0 {
            return 0;
        } else {
            if den < 0.0 && num < lo * den {
                lo = num / den;
                index = i;
            } else if den > 0.0 && num < hi * den {
                hi = num / den;
            }
        }
        if hi < lo {
            return 0;
        }
    }
    if index != !0 {
        out.t = lo;
        out.n = c2_mulrv(bx.r, b.norms[index as usize]);
        return 1;
    }
    0
}

enum BodyRef<'a> {
    Circle(&'a C2Circle),
    Aabb(&'a C2AABB),
    Capsule(&'a C2Capsule),
    Poly(&'a C2Poly),
}

fn c2_cast_ray(
    a: C2Ray,
    b: BodyRef,
    bx: Option<&C2x>,
    type_b: C2Type,
    out: &mut C2Raycast,
) -> i32 {
    match type_b {
        C2Type::Circle => {
            if let BodyRef::Circle(c) = b {
                return c2_ray_to_circle(a, *c, out);
            }
        }
        C2Type::Aabb => {
            if let BodyRef::Aabb(c) = b {
                return c2_ray_to_aabb(a, *c, out);
            }
        }
        C2Type::Capsule => {
            if let BodyRef::Capsule(c) = b {
                return c2_ray_to_capsule(a, *c, out);
            }
        }
        C2Type::Poly => {
            if let BodyRef::Poly(c) = b {
                return c2_ray_to_poly(a, c, bx, out);
            }
        }
    }
    0
}

pub fn poly_ray(cast1: &mut C2Raycast, cast2: &mut C2Raycast) -> i32 {
    let mut hit: i32 = 0;

    let mut p = C2Poly::default();
    p.verts[0] = c2v(0.875, -11.5);
    p.verts[1] = c2v(0.875, 11.5);
    p.verts[2] = c2v(-0.875, 11.5);
    p.verts[3] = c2v(-0.875, -11.5);
    p.norms[0] = c2v(1.0, 0.0);
    p.norms[1] = c2v(0.0, 1.0);
    p.norms[2] = c2v(-1.0, 0.0);
    p.norms[3] = c2v(0.0, -1.0);
    p.count = 4;

    let ray0 = C2Ray {
        p: c2v(-3.869416, 13.0693407),
        d: c2v(1.0, 0.0),
        t: 4.0,
    };
    let ray1 = C2Ray {
        p: c2v(-3.869416, 13.0693407),
        d: c2v(0.0, -1.0),
        t: 4.0,
    };

    hit += c2_cast_ray(ray0, BodyRef::Poly(&p), None, C2Type::Poly, cast1);
    hit += c2_cast_ray(ray1, BodyRef::Poly(&p), None, C2Type::Poly, cast2) << 1;

    hit
}

fn main() {
    // The original C source (c_src/src/lib.c) is a library and contains no
    // main(). It reads no input and produces no output. This executable
    // matches that behavior by producing no output.
}
