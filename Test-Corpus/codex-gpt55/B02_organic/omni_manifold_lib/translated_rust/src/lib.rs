#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::ffi::{c_float, c_int, c_void};
use std::ptr;

pub type C2_TYPE = c_int;

pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_POLY: C2_TYPE = 3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [c_float; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2h {
    pub n: c2v,
    pub d: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2r {
    pub c: c_float,
    pub s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2GJKCache {
    pub metric: c_float,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Proxy {
    pub radius: c_float,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: c_float,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: c_float,
    pub count: c_int,
}

#[inline]
fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
fn mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline]
fn maxv(a: c2v, b: c2v) -> c2v {
    v(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

#[inline]
fn minv(a: c2v, b: c2v) -> c2v {
    v(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

#[inline]
fn clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    maxv(lo, minv(a, hi))
}

#[inline]
fn sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
fn add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline]
fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
fn dist(h: c2h, p: c2v) -> f32 {
    dot(h.n, p) - h.d
}

#[inline]
fn plane_at(p: &c2Poly, i: usize) -> c2h {
    c2h {
        n: p.norms[i],
        d: dot(p.norms[i], p.verts[i]),
    }
}

#[inline]
fn rot_identity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[inline]
fn x_identity() -> c2x {
    c2x {
        p: v(0.0, 0.0),
        r: rot_identity(),
    }
}

#[inline]
fn len(a: c2v) -> f32 {
    dot(a, a).sqrt()
}

#[inline]
fn det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[inline]
fn mulrv(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
fn mulrvT(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[inline]
fn mulxv(a: c2x, b: c2v) -> c2v {
    add(mulrv(a.r, b), a.p)
}

#[inline]
fn mulxvT(a: c2x, b: c2v) -> c2v {
    mulrvT(a.r, sub(b, a.p))
}

#[inline]
fn intersect(a: c2v, b: c2v, da: f32, db: f32) -> c2v {
    add(a, mulvs(sub(b, a), da / (da - db)))
}

#[inline]
fn divv(a: c2v, b: f32) -> c2v {
    mulvs(a, 1.0 / b)
}

#[inline]
fn norm(a: c2v) -> c2v {
    divv(a, len(a))
}

#[inline]
fn neg(a: c2v) -> c2v {
    v(-a.x, -a.y)
}

#[inline]
fn ccw90(a: c2v) -> c2v {
    v(a.y, -a.x)
}

#[inline]
fn skew(a: c2v) -> c2v {
    v(-a.y, a.x)
}

#[inline]
fn absv(a: c2v) -> c2v {
    v(if a.x < 0.0 { -a.x } else { a.x }, if a.y < 0.0 { -a.y } else { a.y })
}

fn bb_verts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = v(bb.min.x, bb.max.y);
}

unsafe fn make_proxy(shape: *const c_void, typ: C2_TYPE, p: &mut c2Proxy) {
    match typ {
        C2_TYPE_CIRCLE => {
            let c = unsafe { &*(shape as *const c2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE_AABB => {
            let bb = unsafe { &*(shape as *const c2AABB) };
            p.radius = 0.0;
            p.count = 4;
            bb_verts(&mut p.verts, bb);
        }
        C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {}
    }
}

fn simplex_metric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => len(sub(s.b.p, s.a.p)),
        3 => det2(sub(s.b.p, s.a.p), sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn clip(seg: &mut [c2v; 2], h: c2h) -> c_int {
    let mut out = [c2v::default(); 4];
    let mut sp = 0usize;
    let d0 = dist(h, seg[0]);
    if d0 < 0.0 {
        out[sp] = seg[0];
        sp += 1;
    }
    let d1 = dist(h, seg[1]);
    if d1 < 0.0 {
        out[sp] = seg[1];
        sp += 1;
    }
    if d0 == 0.0 && d1 == 0.0 {
        out[sp] = seg[0];
        sp += 1;
        out[sp] = seg[1];
        sp += 1;
    } else if d0 * d1 <= 0.0 {
        out[sp] = intersect(seg[0], seg[1], d0, d1);
        sp += 1;
    }
    seg[0] = out[0];
    seg[1] = out[1];
    sp as c_int
}

fn side_planes(seg: &mut [c2v; 2], ra: c2v, rb: c2v, h: Option<&mut c2h>) -> c_int {
    let inn = norm(sub(rb, ra));
    let left = c2h {
        n: neg(inn),
        d: dot(neg(inn), ra),
    };
    let right = c2h {
        n: inn,
        d: dot(inn, rb),
    };
    if clip(seg, left) < 2 {
        return 0;
    }
    if clip(seg, right) < 2 {
        return 0;
    }
    if let Some(h) = h {
        h.n = ccw90(inn);
        h.d = dot(ccw90(inn), ra);
    }
    1
}

fn side_planes_from_poly(seg: &mut [c2v; 2], x: c2x, p: &c2Poly, e: c_int, h: Option<&mut c2h>) -> c_int {
    let e = e as usize;
    let next = if e + 1 == p.count as usize { 0 } else { e + 1 };
    side_planes(seg, mulxv(x, p.verts[e]), mulxv(x, p.verts[next]), h)
}

fn solve2(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let u = dot(b, sub(b, a));
    let vv = dot(a, sub(a, b));
    if vv <= 0.0 {
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
        s.b.u = vv;
        s.div = u + vv;
        s.count = 2;
    }
}

fn solve3(s: &mut c2Simplex) {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det2(sub(b, a), sub(c, a));
    let uABC = det2(b, c) * area;
    let vABC = det2(c, a) * area;
    let wABC = det2(a, b) * area;
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

fn direction(s: &c2Simplex) -> c2v {
    match s.count {
        1 => neg(s.a.p),
        2 => {
            let ab = sub(s.b.p, s.a.p);
            if det2(ab, neg(s.a.p)) > 0.0 {
                skew(ab)
            } else {
                ccw90(ab)
            }
        }
        _ => v(0.0, 0.0),
    }
}

fn support(verts: &[c2v; 8], count: c_int, d: c2v) -> c_int {
    let mut imax = 0;
    let mut dmax = dot(verts[0], d);
    for i in 1..(count as usize) {
        let dotv = dot(verts[i], d);
        if dotv > dmax {
            imax = i as c_int;
            dmax = dotv;
        }
    }
    imax
}

unsafe fn support_raw(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let mut imax = 0;
    let mut dmax = dot(unsafe { *verts }, d);
    for i in 1..count {
        let dotv = dot(unsafe { *verts.add(i as usize) }, d);
        if dotv > dmax {
            imax = i;
            dmax = dotv;
        }
    }
    imax
}

fn witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = add(mulvs(s.a.sA, den * s.a.u), mulvs(s.b.sA, den * s.b.u));
            *b = add(mulvs(s.a.sB, den * s.a.u), mulvs(s.b.sB, den * s.b.u));
        }
        3 => {
            *a = add(add(mulvs(s.a.sA, den * s.a.u), mulvs(s.b.sA, den * s.b.u)), mulvs(s.c.sA, den * s.c.u));
            *b = add(add(mulvs(s.a.sB, den * s.a.u), mulvs(s.b.sB, den * s.b.u)), mulvs(s.c.sB, den * s.c.u));
        }
        _ => {
            *a = v(0.0, 0.0);
            *b = v(0.0, 0.0);
        }
    }
}

fn simplex_l(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => add(mulvs(s.a.p, den * s.a.u), mulvs(s.b.p, den * s.b.u)),
        _ => v(0.0, 0.0),
    }
}

unsafe fn gjk(
    A: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: C2_TYPE,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() { x_identity() } else { unsafe { *ax_ptr } };
    let bx = if bx_ptr.is_null() { x_identity() } else { unsafe { *bx_ptr } };
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    unsafe {
        make_proxy(A, typeA, &mut pA);
        make_proxy(B, typeB, &mut pB);
    }
    let mut s = c2Simplex::default();
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = (cache_ref.count != 0) as c_int;
        if cache_was_good != 0 {
            for i in 0..(cache_ref.count as usize) {
                let iA = cache_ref.iA[i];
                let iB = cache_ref.iB[i];
                let sA = mulxv(ax, pA.verts[iA as usize]);
                let sB = mulxv(bx, pB.verts[iB as usize]);
                let vv = match i {
                    0 => &mut s.a,
                    1 => &mut s.b,
                    _ => &mut s.c,
                };
                vv.iA = iA;
                vv.sA = sA;
                vv.iB = iB;
                vv.sB = sB;
                vv.p = sub(vv.sB, vv.sA);
                vv.u = 0.0;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = simplex_metric(&s);
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
        s.a.sA = mulxv(ax, pA.verts[0]);
        s.a.sB = mulxv(bx, pB.verts[0]);
        s.a.p = sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }
    let mut saveA = [0; 3];
    let mut saveB = [0; 3];
    let mut save_count: c_int;
    let mut d0 = f32::MAX;
    let mut iter = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        let verts = [&s.a, &s.b, &s.c];
        for i in 0..(save_count as usize) {
            saveA[i] = verts[i].iA;
            saveB[i] = verts[i].iB;
        }
        match s.count {
            2 => solve2(&mut s),
            3 => solve3(&mut s),
            _ => {}
        }
        if s.count == 3 {
            hit = 1;
            break;
        }
        let p = simplex_l(&s);
        let d1 = dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = direction(&s);
        if dot(d, d) < f32::EPSILON * f32::EPSILON {
            break;
        }
        let iA = support(&pA.verts, pA.count, mulrvT(ax.r, neg(d)));
        let sA = mulxv(ax, pA.verts[iA as usize]);
        let iB = support(&pB.verts, pB.count, mulrvT(bx.r, d));
        let sB = mulxv(bx, pB.verts[iB as usize]);
        let vv = match s.count {
            0 => &mut s.a,
            1 => &mut s.b,
            _ => &mut s.c,
        };
        vv.iA = iA;
        vv.sA = sA;
        vv.iB = iB;
        vv.sB = sB;
        vv.p = sub(vv.sB, vv.sA);
        let mut dup = 0;
        for i in 0..(save_count as usize) {
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
    witness(&s, &mut a, &mut b);
    let mut distv = len(sub(a, b));
    if hit != 0 {
        a = b;
        distv = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if distv > rA + rB && distv > f32::EPSILON {
            distv -= rA + rB;
            let n = norm(sub(b, a));
            a = add(a, mulvs(n, rA));
            b = sub(b, mulvs(n, rB));
            if a.x == b.x && a.y == b.y {
                distv = 0.0;
            }
        } else {
            let p = mulvs(add(a, b), 0.5);
            a = p;
            b = p;
            distv = 0.0;
        }
    }
    if !cache.is_null() {
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = simplex_metric(&s);
        cache_ref.count = s.count;
        let verts = [&s.a, &s.b, &s.c];
        for i in 0..(s.count as usize) {
            cache_ref.iA[i] = verts[i].iA;
            cache_ref.iB[i] = verts[i].iB;
        }
        cache_ref.div = s.div;
    }
    unsafe {
        if !outA.is_null() {
            *outA = a;
        }
        if !outB.is_null() {
            *outB = b;
        }
        if !iterations.is_null() {
            *iterations = iter;
        }
    }
    distv
}

fn circle_circle_manifold(A: c2Circle, B: c2Circle, m: &mut c2Manifold) {
    m.count = 0;
    let d = sub(B.p, A.p);
    let d2 = dot(d, d);
    let r = A.r + B.r;
    if d2 < r * r {
        let l = d2.sqrt();
        let n = if l != 0.0 { mulvs(d, 1.0 / l) } else { v(0.0, 1.0) };
        m.count = 1;
        m.depths[0] = r - l;
        m.contact_points[0] = sub(B.p, mulvs(n, B.r));
        m.n = n;
    }
}

fn circle_aabb_manifold(A: c2Circle, B: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let L = clampv(A.p, B.min, B.max);
    let ab = sub(L, A.p);
    let d2 = dot(ab, ab);
    let r2 = A.r * A.r;
    if d2 < r2 {
        if d2 != 0.0 {
            let d = d2.sqrt();
            let n = norm(ab);
            m.count = 1;
            m.depths[0] = A.r - d;
            m.contact_points[0] = add(A.p, mulvs(n, d));
            m.n = n;
        } else {
            let mid = mulvs(add(B.min, B.max), 0.5);
            let e = mulvs(sub(B.max, B.min), 0.5);
            let d = sub(A.p, mid);
            let abs_d = absv(d);
            let x_overlap = e.x - abs_d.x;
            let y_overlap = e.y - abs_d.y;
            let (depth, n) = if x_overlap < y_overlap {
                (x_overlap, mulvs(v(1.0, 0.0), if d.x < 0.0 { 1.0 } else { -1.0 }))
            } else {
                (y_overlap, mulvs(v(0.0, 1.0), if d.y < 0.0 { 1.0 } else { -1.0 }))
            };
            m.count = 1;
            m.depths[0] = A.r + depth;
            m.contact_points[0] = sub(A.p, mulvs(n, depth));
            m.n = n;
        }
    }
}

unsafe fn circle_capsule_manifold(A: c2Circle, B: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut a = c2v::default();
    let mut b = c2v::default();
    let r = A.r + B.r;
    let d = unsafe {
        gjk(
            &A as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            ptr::null(),
            &B as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            &mut a,
            &mut b,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if d < r {
        let n = if d == 0.0 { norm(skew(sub(B.b, B.a))) } else { norm(sub(b, a)) };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = sub(b, mulvs(n, B.r));
        m.n = n;
    }
}

fn aabb_aabb_manifold(A: c2AABB, B: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let mid_a = mulvs(add(A.min, A.max), 0.5);
    let mid_b = mulvs(add(B.min, B.max), 0.5);
    let eA = absv(mulvs(sub(A.max, A.min), 0.5));
    let eB = absv(mulvs(sub(B.max, B.min), 0.5));
    let d = sub(mid_b, mid_a);
    let dx = eA.x + eB.x - if d.x < 0.0 { -d.x } else { d.x };
    if dx < 0.0 {
        return;
    }
    let dy = eA.y + eB.y - if d.y < 0.0 { -d.y } else { d.y };
    if dy < 0.0 {
        return;
    }
    let (n, depth, p) = if dx < dy {
        if d.x < 0.0 {
            (v(-1.0, 0.0), dx, sub(mid_a, v(eA.x, 0.0)))
        } else {
            (v(1.0, 0.0), dx, add(mid_a, v(eA.x, 0.0)))
        }
    } else if d.y < 0.0 {
        (v(0.0, -1.0), dy, sub(mid_a, v(0.0, eA.y)))
    } else {
        (v(0.0, 1.0), dy, add(mid_a, v(0.0, eA.y)))
    };
    m.count = 1;
    m.contact_points[0] = p;
    m.depths[0] = depth;
    m.n = n;
}

fn keep_deep(seg: &[c2v; 2], h: c2h, m: &mut c2Manifold) {
    let mut cp = 0usize;
    for p in seg.iter().take(2) {
        let d = dist(h, *p);
        if d <= 0.0 {
            m.contact_points[cp] = *p;
            m.depths[cp] = -d;
            cp += 1;
        }
    }
    m.count = cp as c_int;
    m.n = h.n;
}

fn incident(incident: &mut [c2v; 2], ip: &c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    let mut index = !0i32;
    let mut min_dot = f32::MAX;
    for i in 0..(ip.count as usize) {
        let dotv = dot(rn_in_incident_space, ip.norms[i]);
        if dotv < min_dot {
            min_dot = dotv;
            index = i as c_int;
        }
    }
    let idx = index as usize;
    incident[0] = mulxv(ix, ip.verts[idx]);
    incident[1] = mulxv(ix, ip.verts[if idx + 1 == ip.count as usize { 0 } else { idx + 1 }]);
}

unsafe fn capsule_poly_manifold(A: c2Capsule, B: *const c2Poly, bx_ptr: *const c2x, m: &mut c2Manifold) {
    m.count = 0;
    let B_ref = unsafe { &*B };
    let mut a = c2v::default();
    let mut b = c2v::default();
    let d = unsafe {
        gjk(
            &A as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            B as *const c_void,
            C2_TYPE_POLY,
            bx_ptr,
            &mut a,
            &mut b,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if d < 1.0e-6 {
        let bx = if bx_ptr.is_null() { x_identity() } else { unsafe { *bx_ptr } };
        let A_in_B = c2Capsule {
            a: mulxvT(bx, A.a),
            b: mulxvT(bx, A.b),
            r: 0.0,
        };
        let ab = norm(sub(A_in_B.a, A_in_B.b));
        let ab_h0 = c2h {
            n: ccw90(ab),
            d: dot(A_in_B.a, ccw90(ab)),
        };
        let v0 = support(&B_ref.verts, B_ref.count, neg(ab_h0.n));
        let s0 = dist(ab_h0, B_ref.verts[v0 as usize]);
        let ab_h1 = c2h {
            n: skew(ab),
            d: dot(A_in_B.a, skew(ab)),
        };
        let v1 = support(&B_ref.verts, B_ref.count, neg(ab_h1.n));
        let s1 = dist(ab_h1, B_ref.verts[v1 as usize]);
        let mut index = !0i32;
        let mut sep = -f32::MAX;
        let mut code = 0;
        for i in 0..(B_ref.count as usize) {
            let h = plane_at(B_ref, i);
            let da = dot(A_in_B.a, neg(h.n));
            let db = dot(A_in_B.b, neg(h.n));
            let dd = if da > db { dist(h, A_in_B.a) } else { dist(h, A_in_B.b) };
            if dd > sep {
                sep = dd;
                index = i as c_int;
            }
        }
        if s0 > sep {
            index = v0;
            code = 1;
        }
        if s1 > sep {
            index = v1;
            code = 2;
        }
        match code {
            0 => {
                let mut seg = [A.a, A.b];
                let mut h = c2h::default();
                if side_planes_from_poly(&mut seg, bx, B_ref, index, Some(&mut h)) == 0 {
                    return;
                }
                keep_deep(&seg, h, m);
                m.n = neg(m.n);
            }
            1 => {
                let mut inc = [c2v::default(); 2];
                incident(&mut inc, B_ref, bx, ab_h0.n);
                let mut h = c2h::default();
                if side_planes(&mut inc, A_in_B.b, A_in_B.a, Some(&mut h)) == 0 {
                    return;
                }
                keep_deep(&inc, h, m);
            }
            2 => {
                let mut inc = [c2v::default(); 2];
                incident(&mut inc, B_ref, bx, ab_h1.n);
                let mut h = c2h::default();
                if side_planes(&mut inc, A_in_B.a, A_in_B.b, Some(&mut h)) == 0 {
                    return;
                }
                keep_deep(&inc, h, m);
            }
            _ => return,
        }
        for i in 0..(m.count as usize) {
            m.depths[i] += A.r;
        }
    } else if d < A.r {
        m.count = 1;
        m.n = norm(sub(b, a));
        m.contact_points[0] = add(a, mulvs(m.n, A.r));
        m.depths[0] = A.r - d;
    }
}

fn norms(verts: &[c2v; 8], norms: &mut [c2v; 8], count: c_int) {
    for i in 0..(count as usize) {
        let b = if i + 1 < count as usize { i + 1 } else { 0 };
        norms[i] = norm(ccw90(sub(verts[b], verts[i])));
    }
}

unsafe fn aabb_capsule_manifold(A: c2AABB, B: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut p = c2Poly { count: 4, ..Default::default() };
    bb_verts(&mut p.verts, &A);
    let verts = p.verts;
    norms(&verts, &mut p.norms, 4);
    unsafe { capsule_poly_manifold(B, &p, ptr::null(), m) };
    m.n = neg(m.n);
}

unsafe fn capsule_capsule_manifold(A: c2Capsule, B: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut a = c2v::default();
    let mut b = c2v::default();
    let r = A.r + B.r;
    let d = unsafe {
        gjk(
            &A as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            &B as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            ptr::null(),
            &mut a,
            &mut b,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if d < r {
        let n = if d == 0.0 { norm(skew(sub(A.b, A.a))) } else { norm(sub(b, a)) };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = sub(b, mulvs(n, B.r));
        m.n = n;
    }
}

unsafe fn collide(A: *const c_void, typeA: C2_TYPE, B: *const c_void, typeB: C2_TYPE, m: &mut c2Manifold) {
    m.count = 0;
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => circle_circle_manifold(unsafe { *(A as *const c2Circle) }, unsafe { *(B as *const c2Circle) }, m),
            C2_TYPE_AABB => circle_aabb_manifold(unsafe { *(A as *const c2Circle) }, unsafe { *(B as *const c2AABB) }, m),
            C2_TYPE_CAPSULE => unsafe { circle_capsule_manifold(*(A as *const c2Circle), *(B as *const c2Capsule), m) },
            _ => {}
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => {
                circle_aabb_manifold(unsafe { *(B as *const c2Circle) }, unsafe { *(A as *const c2AABB) }, m);
                m.n = neg(m.n);
            }
            C2_TYPE_AABB => aabb_aabb_manifold(unsafe { *(A as *const c2AABB) }, unsafe { *(B as *const c2AABB) }, m),
            C2_TYPE_CAPSULE => unsafe { aabb_capsule_manifold(*(A as *const c2AABB), *(B as *const c2Capsule), m) },
            _ => {}
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => {
                unsafe { circle_capsule_manifold(*(B as *const c2Circle), *(A as *const c2Capsule), m) };
                m.n = neg(m.n);
            }
            C2_TYPE_AABB => {
                unsafe { aabb_capsule_manifold(*(B as *const c2AABB), *(A as *const c2Capsule), m) };
                m.n = neg(m.n);
            }
            C2_TYPE_CAPSULE => unsafe { capsule_capsule_manifold(*(A as *const c2Capsule), *(B as *const c2Capsule), m) },
            _ => {}
        },
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> c2v { v(x, y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: c_float) -> c2v { mulvs(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v { maxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v { minv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v { clampv(a, lo, hi) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v { sub(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> c_float { dot(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: c2h, p: c2v) -> c_float { dist(h, p) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2PlaneAt(p: *const c2Poly, i: c_int) -> c2h { plane_at(unsafe { &*p }, i as usize) }

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r { rot_identity() }

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x { x_identity() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe { bb_verts(std::slice::from_raw_parts_mut(out, 4), &*bb) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, typ: C2_TYPE, p: *mut c2Proxy) {
    unsafe { make_proxy(shape, typ, &mut *p) };
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> c_float { len(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> c_float { det2(a, b) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> c_float { simplex_metric(unsafe { &*s }) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v { mulrv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v { mulrvT(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v { add(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v { mulxv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v { mulxvT(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: c2v, b: c2v, da: c_float, db: c_float) -> c2v { intersect(a, b, da, db) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: c_float) -> c2v { divv(a, b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v { norm(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v { neg(a) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v { ccw90(a) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) { solve2(unsafe { &mut *s }) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) { solve3(unsafe { &mut *s }) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v { skew(a) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v { direction(unsafe { &*s }) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe { support_raw(verts, count, d) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe { witness(&*s, &mut *a, &mut *b) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v { simplex_l(unsafe { &*s }) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: C2_TYPE,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> c_float {
    unsafe { gjk(A, typeA, ax_ptr, B, typeB, bx_ptr, outA, outB, use_radius, iterations, cache) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v { absv(a) }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCircleManifold(A: c2Circle, B: c2Circle, m: *mut c2Manifold) {
    circle_circle_manifold(A, B, unsafe { &mut *m });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoAABBManifold(A: c2Circle, B: c2AABB, m: *mut c2Manifold) {
    circle_aabb_manifold(A, B, unsafe { &mut *m });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(A: c2Circle, B: c2Capsule, m: *mut c2Manifold) {
    unsafe { circle_capsule_manifold(A, B, &mut *m) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoAABBManifold(A: c2AABB, B: c2AABB, m: *mut c2Manifold) {
    aabb_aabb_manifold(A, B, unsafe { &mut *m });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoPolyManifold(A: c2Capsule, B: *const c2Poly, bx_ptr: *const c2x, m: *mut c2Manifold) {
    unsafe { capsule_poly_manifold(A, B, bx_ptr, &mut *m) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *mut c2v, norms_ptr: *mut c2v, count: c_int) {
    for i in 0..count {
        let a = i as usize;
        let b = if i + 1 < count { (i + 1) as usize } else { 0 };
        let e = unsafe { sub(*verts.add(b), *verts.add(a)) };
        unsafe {
            *norms_ptr.add(a) = norm(ccw90(e));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(A: c2AABB, B: c2Capsule, m: *mut c2Manifold) {
    unsafe { aabb_capsule_manifold(A, B, &mut *m) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(A: c2Capsule, B: c2Capsule, m: *mut c2Manifold) {
    unsafe { capsule_capsule_manifold(A, B, &mut *m) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(A: *const c_void, typeA: C2_TYPE, B: *const c_void, typeB: C2_TYPE, m: *mut c2Manifold) {
    unsafe { collide(A, typeA, B, typeB, &mut *m) };
}

#[unsafe(no_mangle)]
pub extern "C" fn ptr_from_parts(typ: C2_TYPE, a: c_float, b: c_float, c: c_float, d: c_float, e: c_float) -> *mut c_void {
    match typ {
        C2_TYPE_CIRCLE => Box::into_raw(Box::new(c2Circle { p: v(a, b), r: c })) as *mut c_void,
        C2_TYPE_AABB => Box::into_raw(Box::new(c2AABB { min: v(a, b), max: v(c, d) })) as *mut c_void,
        C2_TYPE_CAPSULE => Box::into_raw(Box::new(c2Capsule { a: v(a, b), b: v(c, d), r: e })) as *mut c_void,
        _ => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_manifold(
    m: *mut c2Manifold,
    type_a: C2_TYPE,
    a1: c_float,
    a2: c_float,
    a3: c_float,
    a4: c_float,
    a5: c_float,
    type_b: C2_TYPE,
    b1: c_float,
    b2: c_float,
    b3: c_float,
    b4: c_float,
    b5: c_float,
) {
    let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    unsafe { collide(A, type_a, B, type_b, &mut *m) };
}
