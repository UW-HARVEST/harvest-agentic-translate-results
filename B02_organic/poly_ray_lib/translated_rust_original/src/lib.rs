use std::ffi::c_int;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2x {
    p: c2v,
    r: c2r,
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

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Ray {
    p: c2v,
    d: c2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2m {
    x: c2v,
    y: c2v,
}

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
enum C2_TYPE {
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
    C2_TYPE_CAPSULE,
    C2_TYPE_POLY,
}

fn c2v_new(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2_dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2_len(a: c2v) -> f32 {
    c2_dot(a, a).sqrt()
}

fn c2_add(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x + b.x, y: a.y + b.y }
}

fn c2_sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}

fn c2_mulvs(a: c2v, b: f32) -> c2v {
    c2v { x: a.x * b, y: a.y * b }
}

fn c2_div(a: c2v, b: f32) -> c2v {
    c2_mulvs(a, 1.0f32 / b)
}

fn c2_norm(a: c2v) -> c2v {
    c2_div(a, c2_len(a))
}

fn c2_minv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x < b.x { a.x } else { b.x },
        y: if a.y < b.y { a.y } else { b.y },
    }
}

fn c2_maxv(a: c2v, b: c2v) -> c2v {
    c2v {
        x: if a.x > b.x { a.x } else { b.x },
        y: if a.y > b.y { a.y } else { b.y },
    }
}

fn c2_skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

fn c2_absv(a: c2v) -> c2v {
    c2v {
        x: if a.x < 0.0 { -a.x } else { a.x },
        y: if a.y < 0.0 { -a.y } else { a.y },
    }
}

fn c2_ray_to_circle(a: c2Ray, b: c2Circle, out: &mut c2Raycast) -> c_int {
    let p = b.p;
    let m = c2_sub(a.p, p);
    let c = c2_dot(m, m) - b.r * b.r;
    let bv = c2_dot(m, a.d);
    let disc = bv * bv - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -bv - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        out.t = t;
        let impact = c2_add(a.p, c2_mulvs(a.d, t));
        out.n = c2_norm(c2_sub(impact, p));
        return 1;
    }
    0
}

fn c2_aabb_to_aabb(a: c2AABB, b: c2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[inline]
fn c2_signed_dist_point_to_plane_1d(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

#[inline]
fn c2_ray_to_plane_1d(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0f32
    } else {
        let d = da - db;
        if d != 0.0 { da / d } else { 0.0 }
    }
}

fn c2_ray_to_aabb(a: c2Ray, b: c2AABB, out: &mut c2Raycast) -> c_int {
    let p0 = a.p;
    let p1 = c2_add(a.p, c2_mulvs(a.d, a.t));
    let a_box = c2AABB { min: c2_minv(p0, p1), max: c2_maxv(p0, p1) };
    if c2_aabb_to_aabb(a_box, b) == 0 {
        return 0;
    }
    let ab = c2_sub(p1, p0);
    let n = c2_skew(ab);
    let abs_n = c2_absv(n);
    let half_extents = c2_mulvs(c2_sub(b.max, b.min), 0.5f32);
    let center_of_b_box = c2_mulvs(c2_add(b.min, b.max), 0.5f32);
    let dot_val = c2_dot(n, c2_sub(p0, center_of_b_box));
    let d = (if dot_val < 0.0 { -dot_val } else { dot_val }) - c2_dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2_signed_dist_point_to_plane_1d(p0.x, -1.0f32, b.min.x);
    let db0 = c2_signed_dist_point_to_plane_1d(p1.x, -1.0f32, b.min.x);
    let da1 = c2_signed_dist_point_to_plane_1d(p0.x, 1.0f32, b.max.x);
    let db1 = c2_signed_dist_point_to_plane_1d(p1.x, 1.0f32, b.max.x);
    let da2 = c2_signed_dist_point_to_plane_1d(p0.y, -1.0f32, b.min.y);
    let db2 = c2_signed_dist_point_to_plane_1d(p1.y, -1.0f32, b.min.y);
    let da3 = c2_signed_dist_point_to_plane_1d(p0.y, 1.0f32, b.max.y);
    let db3 = c2_signed_dist_point_to_plane_1d(p1.y, 1.0f32, b.max.y);
    let t0 = c2_ray_to_plane_1d(da0, db0);
    let t1 = c2_ray_to_plane_1d(da1, db1);
    let t2 = c2_ray_to_plane_1d(da2, db2);
    let t3 = c2_ray_to_plane_1d(da3, db3);
    let hit0 = (t0 <= 1.0f32) as c_int;
    let hit1 = (t1 <= 1.0f32) as c_int;
    let hit2 = (t2 <= 1.0f32) as c_int;
    let hit3 = (t3 <= 1.0f32) as c_int;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        let t0 = (hit0 as f32) * t0;
        let t1 = (hit1 as f32) * t1;
        let t2 = (hit2 as f32) * t2;
        let t3 = (hit3 as f32) * t3;
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * a.t;
            out.n = c2v_new(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * a.t;
            out.n = c2v_new(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * a.t;
            out.n = c2v_new(0.0, -1.0);
        } else {
            out.t = t3 * a.t;
            out.n = c2v_new(0.0, 1.0);
        }
        1
    } else {
        0
    }
}

fn c2_ccw90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2_mulmv_t(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

fn c2_aabb_to_point(a: c2AABB, b: c2v) -> c_int {
    let d0 = (b.x < a.min.x) as c_int;
    let d1 = (b.y < a.min.y) as c_int;
    let d2 = (b.x > a.max.x) as c_int;
    let d3 = (b.y > a.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn c2_circle_to_point(a: c2Circle, b: c2v) -> c_int {
    let n = c2_sub(a.p, b);
    let d2 = c2_dot(n, n);
    (d2 < a.r * a.r) as c_int
}

fn c2_ray_to_capsule(a: c2Ray, b: c2Capsule, out: &mut c2Raycast) -> c_int {
    let mut m = c2m { x: c2v { x: 0.0, y: 0.0 }, y: c2v { x: 0.0, y: 0.0 } };
    m.y = c2_norm(c2_sub(b.b, b.a));
    m.x = c2_ccw90(m.y);
    let cap_n = c2_sub(b.b, b.a);
    let y_bb = c2_mulmv_t(m, cap_n);
    let y_ap = c2_mulmv_t(m, c2_sub(a.p, b.a));
    let y_ad = c2_mulmv_t(m, a.d);
    let y_ae = c2_add(y_ap, c2_mulvs(y_ad, a.t));
    let capsule_bb = c2AABB {
        min: c2v_new(-b.r, 0.0),
        max: c2v_new(b.r, y_bb.y),
    };
    out.n = c2_norm(cap_n);
    out.t = 0.0;
    if c2_aabb_to_point(capsule_bb, y_ap) != 0 {
        return 1;
    } else {
        let capsule_a = c2Circle { p: b.a, r: b.r };
        let capsule_b = c2Circle { p: b.b, r: b.r };
        if c2_circle_to_point(capsule_a, a.p) != 0 {
            return 1;
        } else if c2_circle_to_point(capsule_b, a.p) != 0 {
            return 1;
        }
    }
    let abs_ae_x = if y_ae.x < 0.0 { -y_ae.x } else { y_ae.x };
    let abs_ap_x = if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x };
    let min_abs = if abs_ae_x < abs_ap_x { abs_ae_x } else { abs_ap_x };
    if y_ae.x * y_ap.x < 0.0 || min_abs < b.r {
        let ca = c2Circle { p: b.a, r: b.r };
        let cb = c2Circle { p: b.b, r: b.r };
        if abs_ap_x < b.r {
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

fn c2_rot_identity() -> c2r {
    c2r { c: 1.0f32, s: 0.0 }
}

fn c2x_identity() -> c2x {
    c2x { p: c2v_new(0.0, 0.0), r: c2_rot_identity() }
}

fn c2_mulrv(a: c2r, b: c2v) -> c2v {
    c2v_new(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2_mulrv_t(a: c2r, b: c2v) -> c2v {
    c2v_new(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2_mulxv_t(a: c2x, b: c2v) -> c2v {
    c2_mulrv_t(a.r, c2_sub(b, a.p))
}

fn c2_ray_to_poly(a: c2Ray, b: &c2Poly, bx_ptr: *const c2x, out: &mut c2Raycast) -> c_int {
    let bx = if bx_ptr.is_null() {
        c2x_identity()
    } else {
        unsafe { *bx_ptr }
    };
    let p = c2_mulxv_t(bx, a.p);
    let d = c2_mulrv_t(bx.r, a.d);
    let mut lo: f32 = 0.0;
    let mut hi: f32 = a.t;
    let mut index: c_int = !0;
    for i in 0..b.count {
        let num = c2_dot(b.norms[i as usize], c2_sub(b.verts[i as usize], p));
        let den = c2_dot(b.norms[i as usize], d);
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

fn c2_cast_ray(a: c2Ray, b: *const u8, bx: *const c2x, type_b: C2_TYPE, out: &mut c2Raycast) -> c_int {
    match type_b {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let circle = unsafe { &*(b as *const c2Circle) };
            c2_ray_to_circle(a, *circle, out)
        }
        C2_TYPE::C2_TYPE_AABB => {
            let aabb = unsafe { &*(b as *const c2AABB) };
            c2_ray_to_aabb(a, *aabb, out)
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let capsule = unsafe { &*(b as *const c2Capsule) };
            c2_ray_to_capsule(a, *capsule, out)
        }
        C2_TYPE::C2_TYPE_POLY => {
            let poly = unsafe { &*(b as *const c2Poly) };
            c2_ray_to_poly(a, poly, bx, out)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn poly_ray(cast1: *mut c2Raycast, cast2: *mut c2Raycast) -> c_int {
    let mut hit: c_int = 0;

    let zero = c2v { x: 0.0, y: 0.0 };
    let mut p = c2Poly {
        count: 4,
        verts: [zero; 8],
        norms: [zero; 8],
    };
    p.verts[0] = c2v_new(0.875f32, -11.5f32);
    p.verts[1] = c2v_new(0.875f32, 11.5f32);
    p.verts[2] = c2v_new(-0.875f32, 11.5f32);
    p.verts[3] = c2v_new(-0.875f32, -11.5f32);
    p.norms[0] = c2v_new(1.0, 0.0);
    p.norms[1] = c2v_new(0.0, 1.0);
    p.norms[2] = c2v_new(-1.0, 0.0);
    p.norms[3] = c2v_new(0.0, -1.0);

    let ray0 = c2Ray { p: c2v_new(-3.869416f32, 13.0693407f32), d: c2v_new(1.0, 0.0), t: 4.0 };
    let ray1 = c2Ray { p: c2v_new(-3.869416f32, 13.0693407f32), d: c2v_new(0.0, -1.0), t: 4.0 };

    let out1 = unsafe { &mut *cast1 };
    let out2 = unsafe { &mut *cast2 };

    hit += c2_cast_ray(ray0, &p as *const c2Poly as *const u8, ptr::null(), C2_TYPE::C2_TYPE_POLY, out1);
    hit += c2_cast_ray(ray1, &p as *const c2Poly as *const u8, ptr::null(), C2_TYPE::C2_TYPE_POLY, out2) << 1;

    hit
}
