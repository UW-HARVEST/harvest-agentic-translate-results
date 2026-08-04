#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod inner {
use std::ffi::c_int;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[inline]
pub fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
pub fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline]
pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
pub fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[inline]
pub fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
pub fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
pub fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[inline]
pub fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

pub fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

pub unsafe fn c2MakeProxy(shape: *const std::ffi::c_void, ty: C2_TYPE, p: &mut c2Proxy) {
    match ty {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let c = unsafe { &*(shape as *const c2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE::C2_TYPE_AABB => {
            let bb = unsafe { &*(shape as *const c2AABB) };
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts, bb);
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

#[inline]
pub fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[inline]
pub fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

pub fn c2GJKSimplexMetric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

#[inline]
pub fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
pub fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline]
pub fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

pub fn c22(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u <= 0.0 {
        s.a = s.b;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else {
        s.a.u = u;
        s.b.u = v;
        s.div = u + v;
        s.count = 2;
    }
}

pub fn c23(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
    let uAB = c2Dot(b, c2Sub(b, a));
    let vAB = c2Dot(a, c2Sub(a, b));
    let uBC = c2Dot(c, c2Sub(c, b));
    let vBC = c2Dot(b, c2Sub(b, c));
    let uCA = c2Dot(a, c2Sub(a, c));
    let vCA = c2Dot(c, c2Sub(c, a));
    let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
    let uABC = c2Det2(b, c) * area;
    let vABC = c2Det2(c, a) * area;
    let wABC = c2Det2(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.a = s.b;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.a = s.c;
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.a.u = uAB;
        s.b.u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.a = s.b;
        s.b = s.c;
        s.a.u = uBC;
        s.b.u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.b = s.a;
        s.a = s.c;
        s.a.u = uCA;
        s.b.u = vCA;
        s.div = uCA + vCA;
        s.count = 2;
    } else {
        s.a.u = uABC;
        s.b.u = vABC;
        s.c.u = wABC;
        s.div = uABC + vABC + wABC;
        s.count = 3;
    }
}

#[inline]
pub fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[inline]
pub fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[inline]
pub fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

pub fn c2D(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2Neg(s.a.p),
        2 => {
            let ab = c2Sub(s.b.p, s.a.p);
            if c2Det2(ab, c2Neg(s.a.p)) > 0.0 {
                c2Skew(ab)
            } else {
                c2CCW90(ab)
            }
        }
        _ => c2V(0.0, 0.0),
    }
}

pub fn c2Support(verts: &[c2v], count: c_int, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    let mut dmax = c2Dot(verts[0], d);
    let mut i: c_int = 1;
    while i < count {
        let dot = c2Dot(verts[i as usize], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

pub fn c2Witness(s: &c2Simplex, a_out: &mut c2v, b_out: &mut c2v) {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => {
            *a_out = s.a.sA;
            *b_out = s.a.sB;
        }
        2 => {
            *a_out = c2Add(
                c2Mulvs(s.a.sA, den * s.a.u),
                c2Mulvs(s.b.sA, den * s.b.u),
            );
            *b_out = c2Add(
                c2Mulvs(s.a.sB, den * s.a.u),
                c2Mulvs(s.b.sB, den * s.b.u),
            );
        }
        3 => {
            *a_out = c2Add(
                c2Add(
                    c2Mulvs(s.a.sA, den * s.a.u),
                    c2Mulvs(s.b.sA, den * s.b.u),
                ),
                c2Mulvs(s.c.sA, den * s.c.u),
            );
            *b_out = c2Add(
                c2Add(
                    c2Mulvs(s.a.sB, den * s.a.u),
                    c2Mulvs(s.b.sB, den * s.b.u),
                ),
                c2Mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a_out = c2V(0.0, 0.0);
            *b_out = c2V(0.0, 0.0);
        }
    }
}

#[inline]
pub fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[inline]
pub fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

pub fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(
            c2Mulvs(s.a.p, den * s.a.u),
            c2Mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[inline]
pub fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[allow(clippy::too_many_arguments)]
pub unsafe fn c2GJK(
    A: *const std::ffi::c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    B: *const std::ffi::c_void,
    typeB: C2_TYPE,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *ax_ptr }
    };
    let bx = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *bx_ptr }
    };

    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    unsafe {
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);
    }

    let mut s = c2Simplex::default();
    let mut cache_was_read = 0i32;

    if !cache.is_null() {
        let cache_ref = unsafe { &mut *cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count as usize {
                let iA = cache_ref.iA[i];
                let iB = cache_ref.iB[i];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = match i {
                    0 => &mut s.a,
                    1 => &mut s.b,
                    2 => &mut s.c,
                    _ => &mut s.d,
                };
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = 1;
            }
        }
    }

    if cache_was_read == 0 {
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, pA.verts[0]);
        s.a.sB = c2Mulxv(bx, pB.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int;
    let mut d0: f32 = 3.40282346638528859811704183484516925e+38;
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = match i {
                0 => &s.a,
                1 => &s.b,
                2 => &s.c,
                _ => &s.d,
            };
            saveA[i] = v.iA;
            saveB[i] = v.iB;
        }
        match s.count {
            1 => {}
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
        }
        if s.count == 3 {
            hit = 1;
            break;
        }
        let p = c2L(&s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&s);
        if c2Dot(d, d)
            < 1.19209289550781250000000000000000000e-7_f32
                * 1.19209289550781250000000000000000000e-7_f32
        {
            break;
        }
        let iA = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);

        // v = verts + s.count
        {
            let idx = s.count as usize;
            let v = match idx {
                0 => &mut s.a,
                1 => &mut s.b,
                2 => &mut s.c,
                _ => &mut s.d,
            };
            v.iA = iA;
            v.sA = sA;
            v.iB = iB;
            v.sB = sB;
            v.p = c2Sub(v.sB, v.sA);
        }

        let mut dup = 0i32;
        for i in 0..save_count as usize {
            if iA == saveA[i] && iB == saveB[i] {
                dup = 1;
                break;
            }
        }
        if dup != 0 {
            break;
        }
        s.count += 1;
        iter += 1;
    }

    let mut a = c2v::default();
    let mut b = c2v::default();
    c2Witness(&s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > 1.19209289550781250000000000000000000e-7_f32 {
            dist -= rA + rB;
            let n = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, rA));
            b = c2Sub(b, c2Mulvs(n, rB));
            if a.x == b.x && a.y == b.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a, b), 0.5);
            a = p;
            b = p;
            dist = 0.0;
        }
    }

    if !cache.is_null() {
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = c2GJKSimplexMetric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count as usize {
            let v = match i {
                0 => &s.a,
                1 => &s.b,
                2 => &s.c,
                _ => &s.d,
            };
            cache_ref.iA[i] = v.iA;
            cache_ref.iB[i] = v.iB;
        }
        cache_ref.div = s.div;
    }
    if !outA.is_null() {
        unsafe { *outA = a };
    }
    if !outB.is_null() {
        unsafe { *outB = b };
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter };
    }
    dist
}

pub fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    (!(d0 | d1 | d2 | d3)) & 1
}

pub fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let dist = unsafe {
        c2GJK(
            &A as *const c2AABB as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_AABB,
            std::ptr::null(),
            &B as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if dist != 0.0 {
        0
    } else {
        1
    }
}

pub fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let dist = unsafe {
        c2GJK(
            &A as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            std::ptr::null(),
            &B as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if dist != 0.0 {
        0
    } else {
        1
    }
}

pub fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

pub fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

pub fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = A.r + B.r;
    (d2 < r * r) as c_int
}

pub unsafe fn c2Collided(
    A: *const std::ffi::c_void,
    typeA: C2_TYPE,
    B: *const std::ffi::c_void,
    typeB: C2_TYPE,
) -> c_int {
    match typeA {
        C2_TYPE::C2_TYPE_CIRCLE => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => unsafe {
                c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle))
            },
            C2_TYPE::C2_TYPE_AABB => unsafe {
                c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB))
            },
            C2_TYPE::C2_TYPE_CAPSULE => unsafe {
                c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule))
            },
        },
        C2_TYPE::C2_TYPE_AABB => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => unsafe {
                c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB))
            },
            C2_TYPE::C2_TYPE_AABB => unsafe {
                c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB))
            },
            C2_TYPE::C2_TYPE_CAPSULE => unsafe {
                c2AABBtoCapsule(*(A as *const c2AABB), *(B as *const c2Capsule))
            },
        },
        C2_TYPE::C2_TYPE_CAPSULE => match typeB {
            C2_TYPE::C2_TYPE_CIRCLE => unsafe {
                c2CircletoCapsule(*(B as *const c2Circle), *(A as *const c2Capsule))
            },
            C2_TYPE::C2_TYPE_AABB => unsafe {
                c2AABBtoCapsule(*(B as *const c2AABB), *(A as *const c2Capsule))
            },
            C2_TYPE::C2_TYPE_CAPSULE => unsafe {
                c2CapsuletoCapsule(*(A as *const c2Capsule), *(B as *const c2Capsule))
            },
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let capsule_in = c2Capsule {
        a: c2V(min_x, min_y),
        b: c2V(max_x, max_y),
        r,
    };

    let circle = c2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };

    let aabb = c2AABB {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };

    let capsule_local = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    unsafe {
        result += c2Collided(
            &circle as *const c2Circle as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CIRCLE,
            &capsule_in as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
        );

        result += c2Collided(
            &aabb as *const c2AABB as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_AABB,
            &capsule_in as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
        ) << 1;

        result += c2Collided(
            &capsule_local as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            &capsule_in as *const c2Capsule as *const std::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
        ) << 2;
    }

    result
}

} // end mod inner

use std::ffi::c_int;
use inner::{c2v, c2r, c2x, c2Circle, c2AABB, c2Capsule, c2GJKCache, c2Proxy, c2Simplex, C2_TYPE};

// =====================================================================
// FFI export wrappers — these mirror the C library's exported symbols.
// =====================================================================

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    inner::c2V(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    inner::c2Mulvs(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    inner::c2Maxv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    inner::c2Minv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    inner::c2Clampv(a, lo, hi)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    inner::c2Sub(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    inner::c2Dot(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    inner::c2RotIdentity()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    inner::c2xIdentity()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *const c2AABB) {
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    let bb_ref = unsafe { &*bb };
    inner::c2BBVerts(out_slice, bb_ref);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(
    shape: *const std::ffi::c_void,
    ty: c_int,
    p: *mut c2Proxy,
) {
    let p_ref = unsafe { &mut *p };
    let ty_enum = match ty {
        0 => C2_TYPE::C2_TYPE_CIRCLE,
        1 => C2_TYPE::C2_TYPE_AABB,
        2 => C2_TYPE::C2_TYPE_CAPSULE,
        _ => return,
    };
    unsafe { inner::c2MakeProxy(shape, ty_enum, p_ref) };
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    inner::c2Len(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    inner::c2Det2(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    let s_ref = unsafe { &*s };
    inner::c2GJKSimplexMetric(s_ref)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    inner::c2Mulrv(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    inner::c2Add(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    inner::c2Mulxv(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let s_ref = unsafe { &mut *s };
    inner::c22(s_ref);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let s_ref = unsafe { &mut *s };
    inner::c23(s_ref);
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    inner::c2Neg(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    inner::c2Skew(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    inner::c2CCW90(a)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    let s_ref = unsafe { &*s };
    inner::c2D(s_ref)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let slice = unsafe { std::slice::from_raw_parts(verts, count as usize) };
    inner::c2Support(slice, count, d)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let s_ref = unsafe { &*s };
    let a_ref = unsafe { &mut *a };
    let b_ref = unsafe { &mut *b };
    inner::c2Witness(s_ref, a_ref, b_ref);
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    inner::c2Div(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    inner::c2Norm(a)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let s_ref = unsafe { &*s };
    inner::c2L(s_ref)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    inner::c2MulrvT(a, b)
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn c2GJK(
    a: *const std::ffi::c_void,
    type_a: c_int,
    ax_ptr: *const c2x,
    b: *const std::ffi::c_void,
    type_b: c_int,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ta = match type_a {
        0 => C2_TYPE::C2_TYPE_CIRCLE,
        1 => C2_TYPE::C2_TYPE_AABB,
        2 => C2_TYPE::C2_TYPE_CAPSULE,
        _ => return 0.0,
    };
    let tb = match type_b {
        0 => C2_TYPE::C2_TYPE_CIRCLE,
        1 => C2_TYPE::C2_TYPE_AABB,
        2 => C2_TYPE::C2_TYPE_CAPSULE,
        _ => return 0.0,
    };
    unsafe {
        inner::c2GJK(
            a, ta, ax_ptr, b, tb, bx_ptr, out_a, out_b, use_radius, iterations, cache,
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(a: c2AABB, b: c2AABB) -> c_int {
    inner::c2AABBtoAABB(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(a: c2AABB, b: c2Capsule) -> c_int {
    inner::c2AABBtoCapsule(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(a: c2Capsule, b: c2Capsule) -> c_int {
    inner::c2CapsuletoCapsule(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: c2Circle, b: c2Circle) -> c_int {
    inner::c2CircletoCircle(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: c2Circle, b: c2AABB) -> c_int {
    inner::c2CircletoAABB(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(a: c2Circle, b: c2Capsule) -> c_int {
    inner::c2CircletoCapsule(a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    a: *const std::ffi::c_void,
    type_a: c_int,
    b: *const std::ffi::c_void,
    type_b: c_int,
) -> c_int {
    let ta = match type_a {
        0 => C2_TYPE::C2_TYPE_CIRCLE,
        1 => C2_TYPE::C2_TYPE_AABB,
        2 => C2_TYPE::C2_TYPE_CAPSULE,
        _ => return 0,
    };
    let tb = match type_b {
        0 => C2_TYPE::C2_TYPE_CIRCLE,
        1 => C2_TYPE::C2_TYPE_AABB,
        2 => C2_TYPE::C2_TYPE_CAPSULE,
        _ => return 0,
    };
    unsafe { inner::c2Collided(a, ta, b, tb) }
}
