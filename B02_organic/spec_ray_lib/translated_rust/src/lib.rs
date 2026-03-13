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
}

fn c2v_new(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2len(a: c2v) -> f32 {
    c2dot(a, a).sqrt()
}

fn c2add(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x + b.x, y: a.y + b.y }
}

fn c2sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}

fn c2mulvs(a: c2v, b: f32) -> c2v {
    c2v { x: a.x * b, y: a.y * b }
}

fn c2div(a: c2v, b: f32) -> c2v {
    c2mulvs(a, 1.0f32 / b)
}

fn c2norm(a: c2v) -> c2v {
    c2div(a, c2len(a))
}

fn c2minv(a: c2v, b: c2v) -> c2v {
    c2v_new(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2maxv(a: c2v, b: c2v) -> c2v {
    c2v_new(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

fn c2absv(a: c2v) -> c2v {
    c2v_new(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

fn c2ray_to_circle(a: c2Ray, b: c2Circle, out: &mut c2Raycast) -> i32 {
    let p = b.p;
    let m = c2sub(a.p, p);
    let c = c2dot(m, m) - b.r * b.r;
    let bv = c2dot(m, a.d);
    let disc = bv * bv - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -bv - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        out.t = t;
        let impact = c2add(a.p, c2mulvs(a.d, t));
        out.n = c2norm(c2sub(impact, p));
        return 1;
    }
    0
}

fn c2aabb_to_aabb(a: c2AABB, b: c2AABB) -> i32 {
    let d0 = (b.max.x < a.min.x) as i32;
    let d1 = (a.max.x < b.min.x) as i32;
    let d2 = (b.max.y < a.min.y) as i32;
    let d3 = (a.max.y < b.min.y) as i32;
    ((d0 | d1 | d2 | d3) == 0) as i32
}

#[inline]
fn c2_signed_dist_point_to_plane_one_dimensional(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

#[inline]
fn c2_ray_to_plane_one_dimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0f32
    } else {
        let d = da - db;
        if d != 0.0 {
            da / d
        } else {
            0.0
        }
    }
}

fn c2ray_to_aabb(a: c2Ray, b: c2AABB, out: &mut c2Raycast) -> i32 {
    let p0 = a.p;
    let p1 = c2add(a.p, c2mulvs(a.d, a.t));
    let a_box = c2AABB {
        min: c2minv(p0, p1),
        max: c2maxv(p0, p1),
    };
    if c2aabb_to_aabb(a_box, b) == 0 {
        return 0;
    }
    let ab = c2sub(p1, p0);
    let n = c2skew(ab);
    let abs_n = c2absv(n);
    let half_extents = c2mulvs(c2sub(b.max, b.min), 0.5f32);
    let center_of_b_box = c2mulvs(c2add(b.min, b.max), 0.5f32);
    let dot_val = c2dot(n, c2sub(p0, center_of_b_box));
    let d = (if dot_val < 0.0 { -dot_val } else { dot_val }) - c2dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, -1.0f32, b.min.x);
    let db0 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, -1.0f32, b.min.x);
    let da1 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, 1.0f32, b.max.x);
    let db1 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, 1.0f32, b.max.x);
    let da2 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, -1.0f32, b.min.y);
    let db2 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, -1.0f32, b.min.y);
    let da3 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, 1.0f32, b.max.y);
    let db3 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, 1.0f32, b.max.y);
    let t0 = c2_ray_to_plane_one_dimensional(da0, db0);
    let t1 = c2_ray_to_plane_one_dimensional(da1, db1);
    let t2 = c2_ray_to_plane_one_dimensional(da2, db2);
    let t3 = c2_ray_to_plane_one_dimensional(da3, db3);
    let hit0 = (t0 <= 1.0f32) as i32;
    let hit1 = (t1 <= 1.0f32) as i32;
    let hit2 = (t2 <= 1.0f32) as i32;
    let hit3 = (t3 <= 1.0f32) as i32;
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

fn c2ccw90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2mulmv_t(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

fn c2aabb_to_point(a: c2AABB, b: c2v) -> i32 {
    let d0 = (b.x < a.min.x) as i32;
    let d1 = (b.y < a.min.y) as i32;
    let d2 = (b.x > a.max.x) as i32;
    let d3 = (b.y > a.max.y) as i32;
    ((d0 | d1 | d2 | d3) == 0) as i32
}

fn c2circle_to_point(a: c2Circle, b: c2v) -> i32 {
    let n = c2sub(a.p, b);
    let d2 = c2dot(n, n);
    (d2 < a.r * a.r) as i32
}

fn c2ray_to_capsule(a: c2Ray, b: c2Capsule, out: &mut c2Raycast) -> i32 {
    let m = c2m {
        y: c2norm(c2sub(b.b, b.a)),
        x: c2ccw90(c2norm(c2sub(b.b, b.a))),
    };
    let cap_n = c2sub(b.b, b.a);
    let y_bb = c2mulmv_t(m, cap_n);
    let y_ap = c2mulmv_t(m, c2sub(a.p, b.a));
    let y_ad = c2mulmv_t(m, a.d);
    let y_ae = c2add(y_ap, c2mulvs(y_ad, a.t));
    let capsule_bb = c2AABB {
        min: c2v_new(-b.r, 0.0),
        max: c2v_new(b.r, y_bb.y),
    };
    out.n = c2norm(cap_n);
    out.t = 0.0;
    if c2aabb_to_point(capsule_bb, y_ap) != 0 {
        return 1;
    } else {
        let capsule_a = c2Circle { p: b.a, r: b.r };
        let capsule_b = c2Circle { p: b.b, r: b.r };
        if c2circle_to_point(capsule_a, a.p) != 0 {
            return 1;
        } else if c2circle_to_point(capsule_b, a.p) != 0 {
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
                return c2ray_to_circle(a, ca, out);
            } else {
                return c2ray_to_circle(a, cb, out);
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return c2ray_to_circle(a, ca, out);
            }
            if y >= y_bb.y {
                return c2ray_to_circle(a, cb, out);
            } else {
                out.n = if c > 0.0 { m.x } else { c2skew(m.y) };
                out.t = t * a.t;
                return 1;
            }
        }
    }
    0
}

fn c2cast_ray(a: c2Ray, b: *const core::ffi::c_void, type_b: C2_TYPE, out: &mut c2Raycast) -> i32 {
    match type_b {
        C2_TYPE::C2_TYPE_CIRCLE => {
            c2ray_to_circle(a, unsafe { *(b as *const c2Circle) }, out)
        }
        C2_TYPE::C2_TYPE_AABB => {
            c2ray_to_aabb(a, unsafe { *(b as *const c2AABB) }, out)
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            c2ray_to_capsule(a, unsafe { *(b as *const c2Capsule) }, out)
            // Note: original C has unreachable `return 0;` after this case
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn spec_ray(
    cast: *mut c2Raycast,
    mp_x: f32,
    mp_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    r_p_x: f32,
    r_p_y: f32,
) -> i32 {
    let mp = c2v_new(mp_x, mp_y);
    let c = c2Circle {
        p: c2v_new(c_p_x, c_p_y),
        r: c_r,
    };
    let mut ray = c2Ray {
        p: c2v_new(r_p_x, r_p_y),
        d: c2v_new(0.0, 0.0),
        t: 0.0,
    };
    ray.d = c2norm(c2sub(mp, ray.p));
    ray.t = c2dot(mp, ray.d) - c2dot(ray.p, ray.d);

    let out = unsafe { &mut *cast };
    c2cast_ray(ray, &c as *const c2Circle as *const core::ffi::c_void, C2_TYPE::C2_TYPE_CIRCLE, out)
}
