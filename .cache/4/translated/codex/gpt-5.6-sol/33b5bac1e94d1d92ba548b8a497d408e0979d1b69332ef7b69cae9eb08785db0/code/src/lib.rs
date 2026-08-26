use std::ffi::{c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
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
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[inline]
fn c_abs(value: f32) -> f32 {
    if value < 0.0 { -value } else { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: f32) -> C2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: C2v) -> C2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: C2v) -> C2v {
    c2V(c_abs(a.x), c_abs(a.y))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(a: C2Ray, b: C2Circle, out: *mut C2Raycast) -> c_int {
    let p = b.p;
    let m = c2Sub(a.p, p);
    let c = c2Dot(m, m) - b.r * b.r;
    let projection = c2Dot(m, a.d);
    let disc = projection * projection - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -projection - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        unsafe {
            (*out).t = t;
        }
        let impact = c2Add(a.p, c2Mulvs(a.d, t));
        unsafe {
            (*out).n = c2Norm(c2Sub(impact, p));
        }
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
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
        1.0
    } else {
        let d = da - db;
        if d != 0.0 { da / d } else { 0.0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoAABB(a: C2Ray, b: C2Aabb, out: *mut C2Raycast) -> c_int {
    let p0 = a.p;
    let p1 = c2Add(a.p, c2Mulvs(a.d, a.t));
    let a_box = C2Aabb {
        min: c2Minv(p0, p1),
        max: c2Maxv(p0, p1),
    };
    if c2AABBtoAABB(a_box, b) == 0 {
        return 0;
    }
    let ab = c2Sub(p1, p0);
    let n = c2Skew(ab);
    let abs_n = c2Absv(n);
    let half_extents = c2Mulvs(c2Sub(b.max, b.min), 0.5);
    let center_of_b_box = c2Mulvs(c2Add(b.min, b.max), 0.5);
    let center_distance = c2Dot(n, c2Sub(p0, center_of_b_box));
    let d = c_abs(center_distance) - c2Dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, -1.0, b.min.x);
    let db0 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, -1.0, b.min.x);
    let da1 = c2_signed_dist_point_to_plane_one_dimensional(p0.x, 1.0, b.max.x);
    let db1 = c2_signed_dist_point_to_plane_one_dimensional(p1.x, 1.0, b.max.x);
    let da2 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, -1.0, b.min.y);
    let db2 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, -1.0, b.min.y);
    let da3 = c2_signed_dist_point_to_plane_one_dimensional(p0.y, 1.0, b.max.y);
    let db3 = c2_signed_dist_point_to_plane_one_dimensional(p1.y, 1.0, b.max.y);
    let mut t0 = c2_ray_to_plane_one_dimensional(da0, db0);
    let mut t1 = c2_ray_to_plane_one_dimensional(da1, db1);
    let mut t2 = c2_ray_to_plane_one_dimensional(da2, db2);
    let mut t3 = c2_ray_to_plane_one_dimensional(da3, db3);
    let hit0 = (t0 <= 1.0) as c_int;
    let hit1 = (t1 <= 1.0) as c_int;
    let hit2 = (t2 <= 1.0) as c_int;
    let hit3 = (t3 <= 1.0) as c_int;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = hit0 as f32 * t0;
        t1 = hit1 as f32 * t1;
        t2 = hit2 as f32 * t2;
        t3 = hit3 as f32 * t3;
        unsafe {
            if t0 >= t1 && t0 >= t2 && t0 >= t3 {
                (*out).t = t0 * a.t;
                (*out).n = c2V(-1.0, 0.0);
            } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
                (*out).t = t1 * a.t;
                (*out).n = c2V(1.0, 0.0);
            } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
                (*out).t = t2 * a.t;
                (*out).n = c2V(0.0, -1.0);
            } else {
                (*out).t = t3 * a.t;
                (*out).n = c2V(0.0, 1.0);
            }
        }
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: C2m, b: C2v) -> C2v {
    C2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(a: C2Aabb, b: C2v) -> c_int {
    let d0 = (b.x < a.min.x) as c_int;
    let d1 = (b.y < a.min.y) as c_int;
    let d2 = (b.x > a.max.x) as c_int;
    let d3 = (b.y > a.max.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(a: C2Circle, b: C2v) -> c_int {
    let n = c2Sub(a.p, b);
    let d2 = c2Dot(n, n);
    (d2 < a.r * a.r) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCapsule(a: C2Ray, b: C2Capsule, out: *mut C2Raycast) -> c_int {
    let mut m = C2m {
        x: c2V(0.0, 0.0),
        y: c2Norm(c2Sub(b.b, b.a)),
    };
    m.x = c2CCW90(m.y);
    let cap_n = c2Sub(b.b, b.a);
    let y_bb = c2MulmvT(m, cap_n);
    let y_ap = c2MulmvT(m, c2Sub(a.p, b.a));
    let y_ad = c2MulmvT(m, a.d);
    let y_ae = c2Add(y_ap, c2Mulvs(y_ad, a.t));
    let capsule_bb = C2Aabb {
        min: c2V(-b.r, 0.0),
        max: c2V(b.r, y_bb.y),
    };
    unsafe {
        (*out).n = c2Norm(cap_n);
        (*out).t = 0.0;
    }
    if c2AABBtoPoint(capsule_bb, y_ap) != 0 {
        return 1;
    }
    let capsule_a = C2Circle { p: b.a, r: b.r };
    let capsule_b = C2Circle { p: b.b, r: b.r };
    if c2CircleToPoint(capsule_a, a.p) != 0 {
        return 1;
    } else if c2CircleToPoint(capsule_b, a.p) != 0 {
        return 1;
    }
    let min_abs_x = if c_abs(y_ae.x) < c_abs(y_ap.x) {
        c_abs(y_ae.x)
    } else {
        c_abs(y_ap.x)
    };
    if y_ae.x * y_ap.x < 0.0 || min_abs_x < b.r {
        let ca = C2Circle { p: b.a, r: b.r };
        let cb = C2Circle { p: b.b, r: b.r };
        if c_abs(y_ap.x) < b.r {
            if y_ap.y < 0.0 {
                return unsafe { c2RaytoCircle(a, ca, out) };
            } else {
                return unsafe { c2RaytoCircle(a, cb, out) };
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return unsafe { c2RaytoCircle(a, ca, out) };
            }
            if y >= y_bb.y {
                return unsafe { c2RaytoCircle(a, cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { m.x } else { c2Skew(m.y) };
                    (*out).t = t * a.t;
                }
                return 1;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    a: C2Ray,
    b: *const c_void,
    type_b: c_int,
    out: *mut C2Raycast,
) -> c_int {
    match type_b {
        C2_TYPE_CIRCLE => unsafe { c2RaytoCircle(a, b.cast::<C2Circle>().read(), out) },
        C2_TYPE_AABB => unsafe { c2RaytoAABB(a, b.cast::<C2Aabb>().read(), out) },
        C2_TYPE_CAPSULE => unsafe { c2RaytoCapsule(a, b.cast::<C2Capsule>().read(), out) },
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gen_ray(
    cast1: *mut C2Raycast,
    cast2: *mut C2Raycast,
    cast3: *mut C2Raycast,
    mp_x: f32,
    mp_y: f32,
    r_p_x: f32,
    r_p_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    cap_a_x: f32,
    cap_a_y: f32,
    cap_b_x: f32,
    cap_b_y: f32,
    cap_r: f32,
    bb_min_x: f32,
    bb_min_y: f32,
    bb_max_x: f32,
    bb_max_y: f32,
) -> c_int {
    let mut hit = 0;
    let mp = c2V(mp_x, mp_y);
    let ray_p = c2V(r_p_x, r_p_y);
    let ray_d = c2Norm(c2Sub(mp, ray_p));
    let ray = C2Ray {
        p: ray_p,
        d: ray_d,
        t: c2Dot(mp, ray_d) - c2Dot(ray_p, ray_d),
    };
    let circle = C2Circle {
        p: c2V(c_p_x, c_p_y),
        r: c_r,
    };
    hit += unsafe {
        c2CastRay(
            ray,
            (&circle as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
            cast1,
        )
    };
    let capsule = C2Capsule {
        a: c2V(cap_a_x, cap_a_y),
        b: c2V(cap_b_x, cap_b_y),
        r: cap_r,
    };
    hit += unsafe {
        c2CastRay(
            ray,
            (&capsule as *const C2Capsule).cast(),
            C2_TYPE_CAPSULE,
            cast2,
        )
    } << 1;
    let bb = C2Aabb {
        min: c2V(bb_min_x, bb_min_y),
        max: c2V(bb_max_x, bb_max_y),
    };
    hit += unsafe { c2CastRay(ray, (&bb as *const C2Aabb).cast(), C2_TYPE_AABB, cast3) } << 2;
    hit
}
