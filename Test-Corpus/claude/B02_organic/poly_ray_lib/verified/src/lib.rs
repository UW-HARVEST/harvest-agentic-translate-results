use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2Poly {
    pub count: c_int,
    pub verts: [C2v; 8],
    pub norms: [C2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
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
    c2Mulvs(a, 1.0f32 / b)
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
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RaytoCircle(a: C2Ray, b: C2Circle, out: *mut C2Raycast) -> c_int {
    let p = b.p;
    let m = c2Sub(a.p, p);
    let c = c2Dot(m, m) - b.r * b.r;
    let bb = c2Dot(m, a.d);
    let disc = bb * bb - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -bb - disc.sqrt();
    if t >= 0.0 && t <= a.t {
        let out = unsafe { &mut *out };
        out.t = t;
        let impact = c2Add(a.p, c2Mulvs(a.d, t));
        out.n = c2Norm(c2Sub(impact, p));
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: C2AABB, b: C2AABB) -> c_int {
    let d0 = (b.max.x < a.min.x) as c_int;
    let d1 = (a.max.x < b.min.x) as c_int;
    let d2 = (b.max.y < a.min.y) as c_int;
    let d3 = (a.max.y < b.min.y) as c_int;
    (!(d0 | d1 | d2 | d3)) & 1
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

#[unsafe(no_mangle)]
pub extern "C" fn c2RaytoAABB(a: C2Ray, b: C2AABB, out: *mut C2Raycast) -> c_int {
    let p0 = a.p;
    let p1 = c2Add(a.p, c2Mulvs(a.d, a.t));
    let a_box = C2AABB {
        min: c2Minv(p0, p1),
        max: c2Maxv(p0, p1),
    };
    if c2AABBtoAABB(a_box, b) == 0 {
        return 0;
    }
    let ab = c2Sub(p1, p0);
    let n = c2Skew(ab);
    let abs_n = c2Absv(n);
    let half_extents = c2Mulvs(c2Sub(b.max, b.min), 0.5f32);
    let center_of_b_box = c2Mulvs(c2Add(b.min, b.max), 0.5f32);
    let dotn = c2Dot(n, c2Sub(p0, center_of_b_box));
    let d = (if dotn < 0.0 { -dotn } else { dotn }) - c2Dot(abs_n, half_extents);
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
    let mut t0 = c2_ray_to_plane_one_dimensional(da0, db0);
    let mut t1 = c2_ray_to_plane_one_dimensional(da1, db1);
    let mut t2 = c2_ray_to_plane_one_dimensional(da2, db2);
    let mut t3 = c2_ray_to_plane_one_dimensional(da3, db3);
    let hit0 = (t0 <= 1.0f32) as c_int;
    let hit1 = (t1 <= 1.0f32) as c_int;
    let hit2 = (t2 <= 1.0f32) as c_int;
    let hit3 = (t3 <= 1.0f32) as c_int;
    let hit = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = (hit0 as f32) * t0;
        t1 = (hit1 as f32) * t1;
        t2 = (hit2 as f32) * t2;
        t3 = (hit3 as f32) * t3;
        let out = unsafe { &mut *out };
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * a.t;
            out.n = c2V(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * a.t;
            out.n = c2V(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * a.t;
            out.n = c2V(0.0, -1.0);
        } else {
            out.t = t3 * a.t;
            out.n = c2V(0.0, 1.0);
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
pub extern "C" fn c2AABBtoPoint(a: C2AABB, b: C2v) -> c_int {
    let d0 = (b.x < a.min.x) as c_int;
    let d1 = (b.y < a.min.y) as c_int;
    let d2 = (b.x > a.max.x) as c_int;
    let d3 = (b.y > a.max.y) as c_int;
    (!(d0 | d1 | d2 | d3)) & 1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(a: C2Circle, b: C2v) -> c_int {
    let n = c2Sub(a.p, b);
    let d2 = c2Dot(n, n);
    (d2 < a.r * a.r) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RaytoCapsule(a: C2Ray, b: C2Capsule, out: *mut C2Raycast) -> c_int {
    let m_y = c2Norm(c2Sub(b.b, b.a));
    let m_x = c2CCW90(m_y);
    let m = C2m { x: m_x, y: m_y };
    let cap_n = c2Sub(b.b, b.a);
    let y_bb = c2MulmvT(m, cap_n);
    let y_ap = c2MulmvT(m, c2Sub(a.p, b.a));
    let y_ad = c2MulmvT(m, a.d);
    let y_ae = c2Add(y_ap, c2Mulvs(y_ad, a.t));
    let capsule_bb = C2AABB {
        min: c2V(-b.r, 0.0),
        max: c2V(b.r, y_bb.y),
    };
    {
        let out_ref = unsafe { &mut *out };
        out_ref.n = c2Norm(cap_n);
        out_ref.t = 0.0;
    }
    if c2AABBtoPoint(capsule_bb, y_ap) != 0 {
        return 1;
    } else {
        let capsule_a = C2Circle { p: b.a, r: b.r };
        let capsule_b = C2Circle { p: b.b, r: b.r };
        if c2CircleToPoint(capsule_a, a.p) != 0 {
            return 1;
        } else if c2CircleToPoint(capsule_b, a.p) != 0 {
            return 1;
        }
    }
    let abs_yae_x = if y_ae.x < 0.0 { -y_ae.x } else { y_ae.x };
    let abs_yap_x = if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x };
    let min_abs = if abs_yae_x < abs_yap_x { abs_yae_x } else { abs_yap_x };
    if y_ae.x * y_ap.x < 0.0 || min_abs < b.r {
        let ca = C2Circle { p: b.a, r: b.r };
        let cb = C2Circle { p: b.b, r: b.r };
        if (if y_ap.x < 0.0 { -y_ap.x } else { y_ap.x }) < b.r {
            if y_ap.y < 0.0 {
                return c2RaytoCircle(a, ca, out);
            } else {
                return c2RaytoCircle(a, cb, out);
            }
        } else {
            let c = if y_ap.x > 0.0 { b.r } else { -b.r };
            let d = y_ae.x - y_ap.x;
            let t = (c - y_ap.x) / d;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return c2RaytoCircle(a, ca, out);
            }
            if y >= y_bb.y {
                return c2RaytoCircle(a, cb, out);
            } else {
                let out_ref = unsafe { &mut *out };
                out_ref.n = if c > 0.0 { m.x } else { c2Skew(m.y) };
                out_ref.t = t * a.t;
                return 1;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r {
    C2r { c: 1.0f32, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x {
    C2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: C2r, b: C2v) -> C2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: C2r, b: C2v) -> C2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: C2x, b: C2v) -> C2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RaytoPoly(
    a: C2Ray,
    b: *const C2Poly,
    bx_ptr: *const C2x,
    out: *mut C2Raycast,
) -> c_int {
    let bx = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *bx_ptr }
    };
    let b_ref = unsafe { &*b };
    let p = c2MulxvT(bx, a.p);
    let d = c2MulrvT(bx.r, a.d);
    let mut lo: f32 = 0.0;
    let mut hi: f32 = a.t;
    let mut index: c_int = !0;
    let count = b_ref.count;
    let mut i: c_int = 0;
    while i < count {
        let idx = i as usize;
        let num = c2Dot(b_ref.norms[idx], c2Sub(b_ref.verts[idx], p));
        let den = c2Dot(b_ref.norms[idx], d);
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
        i += 1;
    }
    if index != !0 {
        let out = unsafe { &mut *out };
        out.t = lo;
        out.n = c2Mulrv(bx.r, b_ref.norms[index as usize]);
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CastRay(
    a: C2Ray,
    b: *const std::ffi::c_void,
    bx: *const C2x,
    type_b: c_int,
    out: *mut C2Raycast,
) -> c_int {
    match type_b {
        x if x == C2_TYPE_CIRCLE => unsafe { c2RaytoCircle(a, *(b as *const C2Circle), out) },
        x if x == C2_TYPE_AABB => unsafe { c2RaytoAABB(a, *(b as *const C2AABB), out) },
        x if x == C2_TYPE_CAPSULE => unsafe { c2RaytoCapsule(a, *(b as *const C2Capsule), out) },
        x if x == C2_TYPE_POLY => c2RaytoPoly(a, b as *const C2Poly, bx, out),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn poly_ray(cast1: *mut C2Raycast, cast2: *mut C2Raycast) -> c_int {
    let mut hit: c_int = 0;

    let mut p = C2Poly {
        count: 0,
        verts: [C2v { x: 0.0, y: 0.0 }; 8],
        norms: [C2v { x: 0.0, y: 0.0 }; 8],
    };
    p.verts[0] = c2V(0.875f32, -11.5f32);
    p.verts[1] = c2V(0.875f32, 11.5f32);
    p.verts[2] = c2V(-0.875f32, 11.5f32);
    p.verts[3] = c2V(-0.875f32, -11.5f32);
    p.norms[0] = c2V(1.0, 0.0);
    p.norms[1] = c2V(0.0, 1.0);
    p.norms[2] = c2V(-1.0, 0.0);
    p.norms[3] = c2V(0.0, -1.0);
    p.count = 4;

    let ray0 = C2Ray {
        p: C2v {
            x: -3.869416f32,
            y: 13.0693407f32,
        },
        d: C2v { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    let ray1 = C2Ray {
        p: C2v {
            x: -3.869416f32,
            y: 13.0693407f32,
        },
        d: C2v { x: 0.0, y: -1.0 },
        t: 4.0,
    };

    hit += c2CastRay(
        ray0,
        &p as *const C2Poly as *const std::ffi::c_void,
        std::ptr::null(),
        C2_TYPE_POLY,
        cast1,
    );
    hit += c2CastRay(
        ray1,
        &p as *const C2Poly as *const std::ffi::c_void,
        std::ptr::null(),
        C2_TYPE_POLY,
        cast2,
    ) << 1;

    hit
}
