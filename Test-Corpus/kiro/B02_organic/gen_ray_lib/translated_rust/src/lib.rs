#![allow(non_camel_case_types)]
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
pub struct c2Raycast {
    t: f32,
    n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Ray {
    p: c2v,
    d: c2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2m {
    x: c2v,
    y: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub enum C2Type {
    Circle,
    Aabb,
    Capsule,
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

fn c2_signed_dist_point_to_plane_1d(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

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
    let a_box = c2AABB {
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
    let d = (if dot_val < 0.0 { -dot_val } else { dot_val }) - c2_dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2_signed_dist_point_to_plane_1d(p0.x, -1.0, b.min.x);
    let db0 = c2_signed_dist_point_to_plane_1d(p1.x, -1.0, b.min.x);
    let da1 = c2_signed_dist_point_to_plane_1d(p0.x, 1.0, b.max.x);
    let db1 = c2_signed_dist_point_to_plane_1d(p1.x, 1.0, b.max.x);
    let da2 = c2_signed_dist_point_to_plane_1d(p0.y, -1.0, b.min.y);
    let db2 = c2_signed_dist_point_to_plane_1d(p1.y, -1.0, b.min.y);
    let da3 = c2_signed_dist_point_to_plane_1d(p0.y, 1.0, b.max.y);
    let db3 = c2_signed_dist_point_to_plane_1d(p1.y, 1.0, b.max.y);
    let t0 = c2_ray_to_plane_1d(da0, db0);
    let t1 = c2_ray_to_plane_1d(da1, db1);
    let t2 = c2_ray_to_plane_1d(da2, db2);
    let t3 = c2_ray_to_plane_1d(da3, db3);
    let hit0 = (t0 <= 1.0) as c_int;
    let hit1 = (t1 <= 1.0) as c_int;
    let hit2 = (t2 <= 1.0) as c_int;
    let hit3 = (t3 <= 1.0) as c_int;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        let t0 = hit0 as f32 * t0;
        let t1 = hit1 as f32 * t1;
        let t2 = hit2 as f32 * t2;
        let t3 = hit3 as f32 * t3;
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

fn c2_ray_to_capsule(a: c2Ray, b: c2Capsule, out: &mut c2Raycast) -> c_int {
    let m = c2m {
        x: c2_ccw90(c2_norm(c2_sub(b.b, b.a))),
        y: c2_norm(c2_sub(b.b, b.a)),
    };
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

fn c2_cast_ray(a: c2Ray, b: *const u8, type_b: C2Type, out: &mut c2Raycast) -> c_int {
    unsafe {
        match type_b {
            C2Type::Circle => c2_ray_to_circle(a, *(b as *const c2Circle), out),
            C2Type::Aabb => c2_ray_to_aabb(a, *(b as *const c2AABB), out),
            C2Type::Capsule => c2_ray_to_capsule(a, *(b as *const c2Capsule), out),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gen_ray(
    cast1: *mut c2Raycast,
    cast2: *mut c2Raycast,
    cast3: *mut c2Raycast,
    mp_x: f32, mp_y: f32,
    r_p_x: f32, r_p_y: f32,
    c_p_x: f32, c_p_y: f32, c_r: f32,
    cap_a_x: f32, cap_a_y: f32, cap_b_x: f32, cap_b_y: f32, cap_r: f32,
    bb_min_x: f32, bb_min_y: f32, bb_max_x: f32, bb_max_y: f32,
) -> c_int {
    let mut hit: c_int = 0;

    let mp = c2v_new(mp_x, mp_y);

    let mut ray = c2Ray {
        p: c2v_new(r_p_x, r_p_y),
        d: c2v_new(0.0, 0.0),
        t: 0.0,
    };
    ray.d = c2_norm(c2_sub(mp, ray.p));
    ray.t = c2_dot(mp, ray.d) - c2_dot(ray.p, ray.d);

    let c = c2Circle {
        p: c2v_new(c_p_x, c_p_y),
        r: c_r,
    };

    unsafe {
        hit += c2_cast_ray(ray, &c as *const c2Circle as *const u8, C2Type::Circle, &mut *cast1);

        let cap = c2Capsule {
            a: c2v_new(cap_a_x, cap_a_y),
            b: c2v_new(cap_b_x, cap_b_y),
            r: cap_r,
        };

        hit += c2_cast_ray(ray, &cap as *const c2Capsule as *const u8, C2Type::Capsule, &mut *cast2) << 1;

        let bb = c2AABB {
            min: c2v_new(bb_min_x, bb_min_y),
            max: c2v_new(bb_max_x, bb_max_y),
        };

        hit += c2_cast_ray(ray, &bb as *const c2AABB as *const u8, C2Type::Aabb, &mut *cast3) << 2;
    }

    hit
}

// FFI export wrappers matching C symbol names
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v { c2v_new(x, y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 { c2_dot(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 { c2_len(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v { c2_add(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v { c2_sub(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v { c2_mulvs(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v { c2_div(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v { c2_norm(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v { c2_minv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v { c2_maxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v { c2_skew(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v { c2_absv(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v { c2_ccw90(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v { c2_mulmv_t(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(a: c2AABB, b: c2v) -> c_int { c2_aabb_to_point(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(a: c2Circle, b: c2v) -> c_int { c2_circle_to_point(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RaytoCircle(a: c2Ray, b: c2Circle, out: &mut c2Raycast) -> c_int { c2_ray_to_circle(a, b, out) }

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int { c2_aabb_to_aabb(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RaytoAABB(a: c2Ray, b: c2AABB, out: &mut c2Raycast) -> c_int { c2_ray_to_aabb(a, b, out) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RaytoCapsule(a: c2Ray, b: c2Capsule, out: &mut c2Raycast) -> c_int { c2_ray_to_capsule(a, b, out) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CastRay(a: c2Ray, b: *const u8, type_b: C2Type, out: &mut c2Raycast) -> c_int { c2_cast_ray(a, b, type_b, out) }
