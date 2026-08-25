use std::ffi::{c_float, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Raycast {
    pub t: c_float,
    pub n: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Circle {
    pub p: C2v,
    pub r: c_float,
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
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: c_float,
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
fn c_abs(value: c_float) -> c_float {
    if value < 0.0 { -value } else { value }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn c_mul(mut destination: c_float, source: c_float) -> c_float {
    unsafe {
        core::arch::asm!(
            "mulss {destination}, {source}",
            destination = inout(xmm_reg) destination,
            source = in(xmm_reg) source,
            options(pure, nomem, nostack),
        );
    }
    destination
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
fn c_mul(destination: c_float, source: c_float) -> c_float {
    destination * source
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn c_add(mut destination: c_float, source: c_float) -> c_float {
    unsafe {
        core::arch::asm!(
            "addss {destination}, {source}",
            destination = inout(xmm_reg) destination,
            source = in(xmm_reg) source,
            options(pure, nomem, nostack),
        );
    }
    destination
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
fn c_add(destination: c_float, source: c_float) -> c_float {
    destination + source
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> c_float {
    c_add(c_mul(a.x, b.x), c_mul(a.y, b.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> c_float {
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
pub extern "C" fn c2Mulvs(mut a: C2v, b: c_float) -> C2v {
    a.x = c_mul(b, a.x);
    a.y = c_mul(b, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: C2v, b: c_float) -> C2v {
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
    let ray_dot = c2Dot(m, a.d);
    let disc = ray_dot * ray_dot - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -ray_dot - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        unsafe {
            (*out).t = t;
            let impact = c2Add(a.p, c2Mulvs(a.d, t));
            (*out).n = c2Norm(c2Sub(impact, p));
        }
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2Aabb, b: C2Aabb) -> c_int {
    let d0 = b.max.x < a.min.x;
    let d1 = a.max.x < b.min.x;
    let d2 = b.max.y < a.min.y;
    let d3 = a.max.y < b.min.y;
    (!(d0 | d1 | d2 | d3)) as c_int
}

#[inline]
fn c2_signed_dist_point_to_plane_one_dimensional(p: c_float, n: c_float, d: c_float) -> c_float {
    p * n - d * n
}

#[inline]
fn c2_ray_to_plane_one_dimensional(da: c_float, db: c_float) -> c_float {
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
    let hit0 = t0 <= 1.0;
    let hit1 = t1 <= 1.0;
    let hit2 = t2 <= 1.0;
    let hit3 = t3 <= 1.0;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit {
        t0 = (hit0 as c_int) as c_float * t0;
        t1 = (hit1 as c_int) as c_float * t1;
        t2 = (hit2 as c_int) as c_float * t2;
        t3 = (hit3 as c_int) as c_float * t3;
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
        x: c_add(c_mul(a.x.x, b.x), c_mul(a.x.y, b.y)),
        y: c_add(c_mul(a.y.x, b.x), c_mul(a.y.y, b.y)),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(a: C2Aabb, b: C2v) -> c_int {
    let d0 = b.x < a.min.x;
    let d1 = b.y < a.min.y;
    let d2 = b.x > a.max.x;
    let d3 = b.y > a.max.y;
    (!(d0 | d1 | d2 | d3)) as c_int
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

    let closest_x = if c_abs(y_ae.x) < c_abs(y_ap.x) {
        c_abs(y_ae.x)
    } else {
        c_abs(y_ap.x)
    };
    if y_ae.x * y_ap.x < 0.0 || closest_x < b.r {
        if c_abs(y_ap.x) < b.r {
            if y_ap.y < 0.0 {
                return unsafe { c2RaytoCircle(a, capsule_a, out) };
            } else {
                return unsafe { c2RaytoCircle(a, capsule_b, out) };
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return unsafe { c2RaytoCircle(a, capsule_a, out) };
            }
            if y >= y_bb.y {
                return unsafe { c2RaytoCircle(a, capsule_b, out) };
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
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let fallthrough_result = {
        let value: c_int;
        unsafe {
            core::arch::asm!(
                "",
                lateout("eax") value,
                options(nomem, nostack, preserves_flags),
            );
        }
        value
    };
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let fallthrough_result = 0;

    match type_b {
        C2_TYPE_CIRCLE => unsafe { c2RaytoCircle(a, b.cast::<C2Circle>().read(), out) },
        C2_TYPE_AABB => unsafe { c2RaytoAABB(a, b.cast::<C2Aabb>().read(), out) },
        C2_TYPE_CAPSULE => unsafe { c2RaytoCapsule(a, b.cast::<C2Capsule>().read(), out) },
        _ => fallthrough_result,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn spec_ray(
    cast: *mut C2Raycast,
    mp_x: c_float,
    mp_y: c_float,
    c_p_x: c_float,
    c_p_y: c_float,
    c_r: c_float,
    r_p_x: c_float,
    r_p_y: c_float,
) -> c_int {
    let mp = c2V(mp_x, mp_y);
    let circle = C2Circle {
        p: c2V(c_p_x, c_p_y),
        r: c_r,
    };
    let ray_p = c2V(r_p_x, r_p_y);
    let ray_d = c2Norm(c2Sub(mp, ray_p));
    let ray = C2Ray {
        p: ray_p,
        d: ray_d,
        t: c2Dot(mp, ray_d) - c2Dot(ray_p, ray_d),
    };
    unsafe {
        c2CastRay(
            ray,
            (&circle as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
            cast,
        )
    }
}
