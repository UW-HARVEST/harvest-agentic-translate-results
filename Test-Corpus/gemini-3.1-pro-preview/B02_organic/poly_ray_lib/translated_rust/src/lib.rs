use std::os::raw::{c_float, c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Raycast {
    pub t: c_float,
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
    C2_TYPE_POLY = 3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2r {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

pub fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

pub fn c2Dot(a: c2v, b: c2v) -> c_float {
    a.x * b.x + a.y * b.y
}

pub fn c2Len(a: c2v) -> c_float {
    c2Dot(a, a).sqrt()
}

pub fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

pub fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

pub fn c2Mulvs(mut a: c2v, b: c_float) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

pub fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

pub fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

pub fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

pub fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

pub fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: &mut c2Raycast) -> c_int {
    let p = B.p;
    let m = c2Sub(A.p, p);
    let c = c2Dot(m, m) - B.r * B.r;
    let b = c2Dot(m, A.d);
    let disc = b * b - c;
    if disc < 0.0 {
        return 0;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 && t <= A.t {
        out.t = t;
        let impact = c2Add(A.p, c2Mulvs(A.d, t));
        out.n = c2Norm(c2Sub(impact, p));
        return 1;
    }
    0
}

pub fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = B.max.x < A.min.x;
    let d1 = A.max.x < B.min.x;
    let d2 = B.max.y < A.min.y;
    let d3 = A.max.y < B.min.y;
    if d0 || d1 || d2 || d3 { 0 } else { 1 }
}

#[inline]
pub fn c2SignedDistPointToPlane_OneDimensional(p: c_float, n: c_float, d: c_float) -> c_float {
    p * n - d * n
}

#[inline]
pub fn c2RayToPlane_OneDimensional(da: c_float, db: c_float) -> c_float {
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

pub fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: &mut c2Raycast) -> c_int {
    let p0 = A.p;
    let p1 = c2Add(A.p, c2Mulvs(A.d, A.t));
    let mut a_box = c2AABB::default();
    a_box.min = c2Minv(p0, p1);
    a_box.max = c2Maxv(p0, p1);
    if c2AABBtoAABB(a_box, B) == 0 {
        return 0;
    }
    let ab = c2Sub(p1, p0);
    let n = c2Skew(ab);
    let abs_n = c2Absv(n);
    let half_extents = c2Mulvs(c2Sub(B.max, B.min), 0.5);
    let center_of_b_box = c2Mulvs(c2Add(B.min, B.max), 0.5);

    let dot_val = c2Dot(n, c2Sub(p0, center_of_b_box));
    let d = (if dot_val < 0.0 { -dot_val } else { dot_val }) - c2Dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0 = c2SignedDistPointToPlane_OneDimensional(p0.x, -1.0, B.min.x);
    let db0 = c2SignedDistPointToPlane_OneDimensional(p1.x, -1.0, B.min.x);
    let da1 = c2SignedDistPointToPlane_OneDimensional(p0.x, 1.0, B.max.x);
    let db1 = c2SignedDistPointToPlane_OneDimensional(p1.x, 1.0, B.max.x);
    let da2 = c2SignedDistPointToPlane_OneDimensional(p0.y, -1.0, B.min.y);
    let db2 = c2SignedDistPointToPlane_OneDimensional(p1.y, -1.0, B.min.y);
    let da3 = c2SignedDistPointToPlane_OneDimensional(p0.y, 1.0, B.max.y);
    let db3 = c2SignedDistPointToPlane_OneDimensional(p1.y, 1.0, B.max.y);

    let mut t0 = c2RayToPlane_OneDimensional(da0, db0);
    let mut t1 = c2RayToPlane_OneDimensional(da1, db1);
    let mut t2 = c2RayToPlane_OneDimensional(da2, db2);
    let mut t3 = c2RayToPlane_OneDimensional(da3, db3);

    let hit0 = t0 <= 1.0;
    let hit1 = t1 <= 1.0;
    let hit2 = t2 <= 1.0;
    let hit3 = t3 <= 1.0;
    let hit = hit0 || hit1 || hit2 || hit3;

    if hit {
        t0 = if hit0 { t0 } else { 0.0 };
        t1 = if hit1 { t1 } else { 0.0 };
        t2 = if hit2 { t2 } else { 0.0 };
        t3 = if hit3 { t3 } else { 0.0 };

        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            out.t = t0 * A.t;
            out.n = c2V(-1.0, 0.0);
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            out.t = t1 * A.t;
            out.n = c2V(1.0, 0.0);
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            out.t = t2 * A.t;
            out.n = c2V(0.0, -1.0);
        } else {
            out.t = t3 * A.t;
            out.n = c2V(0.0, 1.0);
        }
        1
    } else {
        0
    }
}

pub fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

pub fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    c2v {
        x: a.x.x * b.x + a.x.y * b.y,
        y: a.y.x * b.x + a.y.y * b.y,
    }
}

pub fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    let d0 = B.x < A.min.x;
    let d1 = B.y < A.min.y;
    let d2 = B.x > A.max.x;
    let d3 = B.y > A.max.y;
    if d0 || d1 || d2 || d3 { 0 } else { 1 }
}

pub fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    let n = c2Sub(A.p, B);
    let d2 = c2Dot(n, n);
    if d2 < A.r * A.r { 1 } else { 0 }
}

pub fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: &mut c2Raycast) -> c_int {
    let mut M = c2m::default();
    M.y = c2Norm(c2Sub(B.b, B.a));
    M.x = c2CCW90(M.y);
    let cap_n = c2Sub(B.b, B.a);
    let yBb = c2MulmvT(M, cap_n);
    let yAp = c2MulmvT(M, c2Sub(A.p, B.a));
    let yAd = c2MulmvT(M, A.d);
    let yAe = c2Add(yAp, c2Mulvs(yAd, A.t));
    let mut capsule_bb = c2AABB::default();
    capsule_bb.min = c2V(-B.r, 0.0);
    capsule_bb.max = c2V(B.r, yBb.y);
    out.n = c2Norm(cap_n);
    out.t = 0.0;
    if c2AABBtoPoint(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let mut capsule_a = c2Circle::default();
        let mut capsule_b = c2Circle::default();
        capsule_a.p = B.a;
        capsule_a.r = B.r;
        capsule_b.p = B.b;
        capsule_b.r = B.r;
        if c2CircleToPoint(capsule_a, A.p) != 0 {
            return 1;
        } else if c2CircleToPoint(capsule_b, A.p) != 0 {
            return 1;
        }
    }

    let abs_yAe_x = if yAe.x < 0.0 { -yAe.x } else { yAe.x };
    let abs_yAp_x = if yAp.x < 0.0 { -yAp.x } else { yAp.x };
    let min_abs = if abs_yAe_x < abs_yAp_x { abs_yAe_x } else { abs_yAp_x };

    if yAe.x * yAp.x < 0.0 || min_abs < B.r {
        let mut Ca = c2Circle::default();
        let mut Cb = c2Circle::default();
        Ca.p = B.a;
        Ca.r = B.r;
        Cb.p = B.b;
        Cb.r = B.r;
        if abs_yAp_x < B.r {
            if yAp.y < 0.0 {
                return c2RaytoCircle(A, Ca, out);
            } else {
                return c2RaytoCircle(A, Cb, out);
            }
        } else {
            let c = if yAp.x > 0.0 { B.r } else { -B.r };
            let d = yAe.x - yAp.x;
            let t = (c - yAp.x) / d;
            let y = yAp.y + (yAe.y - yAp.y) * t;
            if y <= 0.0 {
                return c2RaytoCircle(A, Ca, out);
            }
            if y >= yBb.y {
                return c2RaytoCircle(A, Cb, out);
            } else {
                out.n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                out.t = t * A.t;
                return 1;
            }
        }
    }
    0
}

pub fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

pub fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

pub fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

pub fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

pub fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

pub fn c2RaytoPoly(A: c2Ray, B: &c2Poly, bx_ptr: *const c2x, out: &mut c2Raycast) -> c_int {
    let bx = if !bx_ptr.is_null() { unsafe { *bx_ptr } } else { c2xIdentity() };
    let p = c2MulxvT(bx, A.p);
    let d = c2MulrvT(bx.r, A.d);
    let mut lo = 0.0;
    let mut hi = A.t;
    let mut index: usize = usize::MAX;
    for i in 0..B.count as usize {
        let num = c2Dot(B.norms[i], c2Sub(B.verts[i], p));
        let den = c2Dot(B.norms[i], d);
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
    if index != usize::MAX {
        out.t = lo;
        out.n = c2Mulrv(bx.r, B.norms[index]);
        return 1;
    }
    0
}

pub fn c2CastRay(A: c2Ray, B: *const c_void, bx: *const c2x, typeB: C2_TYPE, out: &mut c2Raycast) -> c_int {
    unsafe {
        match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => c2RaytoCircle(A, *(B as *const c2Circle), out),
            C2_TYPE::C2_TYPE_AABB => c2RaytoAABB(A, *(B as *const c2AABB), out),
            C2_TYPE::C2_TYPE_CAPSULE => c2RaytoCapsule(A, *(B as *const c2Capsule), out),
            C2_TYPE::C2_TYPE_POLY => c2RaytoPoly(A, &*(B as *const c2Poly), bx, out),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn poly_ray(cast1: *mut c2Raycast, cast2: *mut c2Raycast) -> c_int {
    let mut hit = 0;
    let mut p = c2Poly::default();
    p.verts[0] = c2V(0.875, -11.5);
    p.verts[1] = c2V(0.875, 11.5);
    p.verts[2] = c2V(-0.875, 11.5);
    p.verts[3] = c2V(-0.875, -11.5);
    p.norms[0] = c2V(1.0, 0.0);
    p.norms[1] = c2V(0.0, 1.0);
    p.norms[2] = c2V(-1.0, 0.0);
    p.norms[3] = c2V(0.0, -1.0);
    p.count = 4;

    let ray0 = c2Ray {
        p: c2V(-3.869416, 13.0693407),
        d: c2V(1.0, 0.0),
        t: 4.0,
    };
    let ray1 = c2Ray {
        p: c2V(-3.869416, 13.0693407),
        d: c2V(0.0, -1.0),
        t: 4.0,
    };

    unsafe {
        hit += c2CastRay(ray0, &p as *const _ as *const c_void, std::ptr::null(), C2_TYPE::C2_TYPE_POLY, &mut *cast1);
        hit += c2CastRay(ray1, &p as *const _ as *const c_void, std::ptr::null(), C2_TYPE::C2_TYPE_POLY, &mut *cast2) << 1;
    }

    hit
}
