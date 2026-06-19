#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_int, c_void};

pub type C2_TYPE = c_int;

pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_POLY: C2_TYPE = 3;

const C2_MAX_FLOAT: f32 = 3.40282346638528859811704183484516925e38_f32;
const C2_EPSILON: f32 = 1.19209289550781250000000000000000000e-7_f32;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2h {
    n: c2v,
    d: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Proxy {
    radius: f32,
    count: c_int,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: c_int,
}

#[inline]
fn usize_from_i32(v: c_int) -> usize {
    v as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: c2h, p: c2v) -> f32 {
    c2Dot(h.n, p) - h.d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2PlaneAt(p: *const c2Poly, i: c_int) -> c2h {
    let p = &*p;
    let idx = usize_from_i32(i);
    c2h {
        n: p.norms[idx],
        d: c2Dot(p.norms[idx], p.verts[idx]),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    let out = std::slice::from_raw_parts_mut(out, 4);
    let bb = &*bb;
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: *mut c2Proxy) {
    let p = &mut *p;
    match type_ {
        C2_TYPE_CIRCLE => {
            let c = &*(shape as *const c2Circle);
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE_AABB => {
            let bb = &*(shape as *const c2AABB);
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(p.verts.as_mut_ptr(), bb as *const c2AABB as *mut c2AABB);
        }
        C2_TYPE_CAPSULE => {
            let c = &*(shape as *const c2Capsule);
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    let s = &mut *s;
    match s.count {
        1 => 0.0,
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: c2v, b: c2v, da: f32, db: f32) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

unsafe fn c2Clip(seg: *mut c2v, h: c2h) -> c_int {
    let seg_slice = std::slice::from_raw_parts_mut(seg, 2);
    let mut out = [c2v::default(); 2];
    let mut sp = 0usize;
    let d0 = c2Dist(h, seg_slice[0]);
    if d0 < 0.0 {
        out[sp] = seg_slice[0];
        sp += 1;
    }
    let d1 = c2Dist(h, seg_slice[1]);
    if d1 < 0.0 {
        out[sp] = seg_slice[1];
        sp += 1;
    }
    if d0 == 0.0 && d1 == 0.0 {
        if sp < 2 {
            out[sp] = seg_slice[0];
            sp += 1;
        }
        if sp < 2 {
            out[sp] = seg_slice[1];
            sp += 1;
        }
    } else if d0 * d1 <= 0.0 && sp < 2 {
        out[sp] = c2Intersect(seg_slice[0], seg_slice[1], d0, d1);
        sp += 1;
    }
    seg_slice[0] = out[0];
    seg_slice[1] = out[1];
    sp as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

unsafe fn c2SidePlanes(seg: *mut c2v, ra: c2v, rb: c2v, h: *mut c2h) -> c_int {
    let in_ = c2Norm(c2Sub(rb, ra));
    let left = c2h {
        n: c2Neg(in_),
        d: c2Dot(c2Neg(in_), ra),
    };
    let right = c2h {
        n: in_,
        d: c2Dot(in_, rb),
    };
    if c2Clip(seg, left) < 2 {
        return 0;
    }
    if c2Clip(seg, right) < 2 {
        return 0;
    }
    if !h.is_null() {
        (*h).n = c2CCW90(in_);
        (*h).d = c2Dot(c2CCW90(in_), ra);
    }
    1
}

unsafe fn c2SidePlanesFromPoly(seg: *mut c2v, x: c2x, p: *const c2Poly, e: c_int, h: *mut c2h) -> c_int {
    let p = &*p;
    let ei = usize_from_i32(e);
    let ra = c2Mulxv(x, p.verts[ei]);
    let rb = c2Mulxv(x, p.verts[if ei + 1 == usize_from_i32(p.count) { 0 } else { ei + 1 }]);
    c2SidePlanes(seg, ra, rb, h)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let s = &mut *s;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let s = &mut *s;
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

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    let s = &mut *s;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let verts = std::slice::from_raw_parts(verts, usize_from_i32(count.max(1)));
    let mut imax = 0;
    let mut dmax = c2Dot(verts[0], d);
    let mut i = 1;
    while i < count {
        let dot = c2Dot(verts[usize_from_i32(i)], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let s = &mut *s;
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u));
            *b = c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u));
        }
        3 => {
            *a = c2Add(
                c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u)),
                c2Mulvs(s.c.sA, den * s.c.u),
            );
            *b = c2Add(
                c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u)),
                c2Mulvs(s.c.sB, den * s.c.u),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let s = &mut *s;
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

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
) -> f32 {
    let ax = if ax_ptr.is_null() { c2xIdentity() } else { *ax_ptr };
    let bx = if bx_ptr.is_null() { c2xIdentity() } else { *bx_ptr };
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
    let mut s = c2Simplex::default();
    let verts = &mut s.a as *mut c2sv;
    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_ref = &mut *cache;
        let cache_was_good = (cache_ref.count != 0) as c_int;
        if cache_was_good != 0 {
            let mut i = 0;
            while i < cache_ref.count {
                let iA = cache_ref.iA[usize_from_i32(i)];
                let iB = cache_ref.iB[usize_from_i32(i)];
                let sA = c2Mulxv(ax, pA.verts[usize_from_i32(iA)]);
                let sB = c2Mulxv(bx, pB.verts[usize_from_i32(iB)]);
                let v = verts.add(usize_from_i32(i));
                (*v).iA = iA;
                (*v).sA = sA;
                (*v).iB = iB;
                (*v).sB = sB;
                (*v).p = c2Sub((*v).sB, (*v).sA);
                (*v).u = 0.0;
                i += 1;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric(&mut s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8_f32) {
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
    let mut saveA = [0; 3];
    let mut saveB = [0; 3];
    let mut save_count = 0;
    let mut d0 = C2_MAX_FLOAT;
    let mut d1 = C2_MAX_FLOAT;
    let mut iter = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        let mut i = 0;
        while i < save_count {
            saveA[usize_from_i32(i)] = (*verts.add(usize_from_i32(i))).iA;
            saveB[usize_from_i32(i)] = (*verts.add(usize_from_i32(i))).iB;
            i += 1;
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
        let p = c2L(&mut s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&mut s);
        if c2Dot(d, d) < C2_EPSILON * C2_EPSILON {
            break;
        }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[usize_from_i32(iA)]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[usize_from_i32(iB)]);
        let v = verts.add(usize_from_i32(s.count));
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup = 0;
        let mut i = 0;
        while i < save_count {
            if iA == saveA[usize_from_i32(i)] && iB == saveB[usize_from_i32(i)] {
                dup = 1;
                break;
            }
            i += 1;
        }
        if dup != 0 {
            break;
        }
        s.count += 1;
        iter += 1;
    }
    let mut a = c2V(0.0, 0.0);
    let mut b = c2V(0.0, 0.0);
    c2Witness(&mut s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > C2_EPSILON {
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
        let cache_ref = &mut *cache;
        cache_ref.metric = c2GJKSimplexMetric(&mut s);
        cache_ref.count = s.count;
        let mut i = 0;
        while i < s.count {
            let v = verts.add(usize_from_i32(i));
            cache_ref.iA[usize_from_i32(i)] = (*v).iA;
            cache_ref.iB[usize_from_i32(i)] = (*v).iB;
            i += 1;
        }
        cache_ref.div = s.div;
    }
    if !outA.is_null() {
        *outA = a;
    }
    if !outB.is_null() {
        *outB = b;
    }
    if !iterations.is_null() {
        *iterations = iter;
    }
    dist
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(if a.x < 0.0 { -a.x } else { a.x }, if a.y < 0.0 { -a.y } else { a.y })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCircleManifold(A: c2Circle, B: c2Circle, m: *mut c2Manifold) {
    let m = &mut *m;
    m.count = 0;
    let d = c2Sub(B.p, A.p);
    let d2 = c2Dot(d, d);
    let r = A.r + B.r;
    if d2 < r * r {
        let l = d2.sqrt();
        let n = if l != 0.0 { c2Mulvs(d, 1.0 / l) } else { c2V(0.0, 1.0) };
        m.count = 1;
        m.depths[0] = r - l;
        m.contact_points[0] = c2Sub(B.p, c2Mulvs(n, B.r));
        m.n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoAABBManifold(A: c2Circle, B: c2AABB, m: *mut c2Manifold) {
    let m = &mut *m;
    m.count = 0;
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(L, A.p);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    if d2 < r2 {
        if d2 != 0.0 {
            let d = d2.sqrt();
            let n = c2Norm(ab);
            m.count = 1;
            m.depths[0] = A.r - d;
            m.contact_points[0] = c2Add(A.p, c2Mulvs(n, d));
            m.n = n;
        } else {
            let mid = c2Mulvs(c2Add(B.min, B.max), 0.5);
            let e = c2Mulvs(c2Sub(B.max, B.min), 0.5);
            let d = c2Sub(A.p, mid);
            let abs_d = c2Absv(d);
            let x_overlap = e.x - abs_d.x;
            let y_overlap = e.y - abs_d.y;
            let depth;
            let mut n;
            if x_overlap < y_overlap {
                depth = x_overlap;
                n = c2V(1.0, 0.0);
                n = c2Mulvs(n, if d.x < 0.0 { 1.0 } else { -1.0 });
            } else {
                depth = y_overlap;
                n = c2V(0.0, 1.0);
                n = c2Mulvs(n, if d.y < 0.0 { 1.0 } else { -1.0 });
            }
            m.count = 1;
            m.depths[0] = A.r + depth;
            m.contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
            m.n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(A: c2Circle, B: c2Capsule, m: *mut c2Manifold) {
    let m = &mut *m;
    m.count = 0;
    let mut a = c2V(0.0, 0.0);
    let mut b = c2V(0.0, 0.0);
    let r = A.r + B.r;
    let d = c2GJK(
        &A as *const _ as *const c_void,
        C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const _ as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a,
        &mut b,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < r {
        let n = if d == 0.0 {
            c2Norm(c2Skew(c2Sub(B.b, B.a)))
        } else {
            c2Norm(c2Sub(b, a))
        };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
        m.n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoAABBManifold(A: c2AABB, B: c2AABB, m: *mut c2Manifold) {
    let m = &mut *m;
    m.count = 0;
    let mid_a = c2Mulvs(c2Add(A.min, A.max), 0.5);
    let mid_b = c2Mulvs(c2Add(B.min, B.max), 0.5);
    let eA = c2Absv(c2Mulvs(c2Sub(A.max, A.min), 0.5));
    let eB = c2Absv(c2Mulvs(c2Sub(B.max, B.min), 0.5));
    let d = c2Sub(mid_b, mid_a);
    let dx = eA.x + eB.x - if d.x < 0.0 { -d.x } else { d.x };
    if dx < 0.0 {
        return;
    }
    let dy = eA.y + eB.y - if d.y < 0.0 { -d.y } else { d.y };
    if dy < 0.0 {
        return;
    }
    let n;
    let depth;
    let p;
    if dx < dy {
        depth = dx;
        if d.x < 0.0 {
            n = c2V(-1.0, 0.0);
            p = c2Sub(mid_a, c2V(eA.x, 0.0));
        } else {
            n = c2V(1.0, 0.0);
            p = c2Add(mid_a, c2V(eA.x, 0.0));
        }
    } else {
        depth = dy;
        if d.y < 0.0 {
            n = c2V(0.0, -1.0);
            p = c2Sub(mid_a, c2V(0.0, eA.y));
        } else {
            n = c2V(0.0, 1.0);
            p = c2Add(mid_a, c2V(0.0, eA.y));
        }
    }
    m.count = 1;
    m.contact_points[0] = p;
    m.depths[0] = depth;
    m.n = n;
}

unsafe fn c2KeepDeep(seg: *mut c2v, h: c2h, m: *mut c2Manifold) {
    let seg = std::slice::from_raw_parts(seg, 2);
    let m = &mut *m;
    let mut cp = 0usize;
    let mut i = 0usize;
    while i < 2 {
        let p = seg[i];
        let d = c2Dist(h, p);
        if d <= 0.0 {
            m.contact_points[cp] = p;
            m.depths[cp] = -d;
            cp += 1;
        }
        i += 1;
    }
    m.count = cp as c_int;
    m.n = h.n;
}

unsafe fn c2Incident(incident: *mut c2v, ip: *const c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    let ip = &*ip;
    let mut index = !0usize;
    let mut min_dot = C2_MAX_FLOAT;
    let mut i = 0;
    while i < ip.count {
        let dot = c2Dot(rn_in_incident_space, ip.norms[usize_from_i32(i)]);
        if dot < min_dot {
            min_dot = dot;
            index = usize_from_i32(i);
        }
        i += 1;
    }
    let incident = std::slice::from_raw_parts_mut(incident, 2);
    incident[0] = c2Mulxv(ix, ip.verts[index]);
    incident[1] = c2Mulxv(ix, ip.verts[if index + 1 == usize_from_i32(ip.count) { 0 } else { index + 1 }]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoPolyManifold(
    A: c2Capsule,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    m: *mut c2Manifold,
) {
    let m = &mut *m;
    m.count = 0;
    let mut a = c2V(0.0, 0.0);
    let mut b = c2V(0.0, 0.0);
    let d = c2GJK(
        &A as *const _ as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        B as *const c_void,
        C2_TYPE_POLY,
        bx_ptr,
        &mut a,
        &mut b,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < 1.0e-6_f32 {
        let bx = if bx_ptr.is_null() { c2xIdentity() } else { *bx_ptr };
        let mut A_in_B = c2Capsule::default();
        A_in_B.a = c2MulxvT(bx, A.a);
        A_in_B.b = c2MulxvT(bx, A.b);
        let ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));
        let mut ab_h0 = c2h::default();
        ab_h0.n = c2CCW90(ab);
        ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
        let v0 = c2Support((*B).verts.as_ptr(), (*B).count, c2Neg(ab_h0.n));
        let s0 = c2Dist(ab_h0, (*B).verts[usize_from_i32(v0)]);
        let mut ab_h1 = c2h::default();
        ab_h1.n = c2Skew(ab);
        ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
        let v1 = c2Support((*B).verts.as_ptr(), (*B).count, c2Neg(ab_h1.n));
        let s1 = c2Dist(ab_h1, (*B).verts[usize_from_i32(v1)]);
        let mut index = !0usize;
        let mut sep = -C2_MAX_FLOAT;
        let mut code = 0;
        let mut i = 0;
        while i < (*B).count {
            let h = c2PlaneAt(B, i);
            let da = c2Dot(A_in_B.a, c2Neg(h.n));
            let db = c2Dot(A_in_B.b, c2Neg(h.n));
            let d = if da > db { c2Dist(h, A_in_B.a) } else { c2Dist(h, A_in_B.b) };
            if d > sep {
                sep = d;
                index = usize_from_i32(i);
            }
            i += 1;
        }
        if s0 > sep {
            sep = s0;
            index = usize_from_i32(v0);
            code = 1;
        }
        if s1 > sep {
            index = usize_from_i32(v1);
            code = 2;
        }
        match code {
            0 => {
                let mut seg = [A.a, A.b];
                let mut h = c2h::default();
                if c2SidePlanesFromPoly(seg.as_mut_ptr(), bx, B, index as c_int, &mut h) == 0 {
                    return;
                }
                c2KeepDeep(seg.as_mut_ptr(), h, m);
                m.n = c2Neg(m.n);
            }
            1 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(incident.as_mut_ptr(), B, bx, ab_h0.n);
                let mut h = c2h::default();
                if c2SidePlanes(incident.as_mut_ptr(), A_in_B.b, A_in_B.a, &mut h) == 0 {
                    return;
                }
                c2KeepDeep(incident.as_mut_ptr(), h, m);
            }
            2 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(incident.as_mut_ptr(), B, bx, ab_h1.n);
                let mut h = c2h::default();
                if c2SidePlanes(incident.as_mut_ptr(), A_in_B.a, A_in_B.b, &mut h) == 0 {
                    return;
                }
                c2KeepDeep(incident.as_mut_ptr(), h, m);
            }
            _ => return,
        }
        let mut i = 0;
        while i < m.count {
            m.depths[usize_from_i32(i)] += A.r;
            i += 1;
        }
    } else if d < A.r {
        m.count = 1;
        m.n = c2Norm(c2Sub(b, a));
        m.contact_points[0] = c2Add(a, c2Mulvs(m.n, A.r));
        m.depths[0] = A.r - d;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *mut c2v, norms: *mut c2v, count: c_int) {
    let verts = std::slice::from_raw_parts_mut(verts, usize_from_i32(count));
    let norms = std::slice::from_raw_parts_mut(norms, usize_from_i32(count));
    let mut i = 0;
    while i < count {
        let a = usize_from_i32(i);
        let b = if i + 1 < count { usize_from_i32(i + 1) } else { 0 };
        let e = c2Sub(verts[b], verts[a]);
        norms[a] = c2Norm(c2CCW90(e));
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(A: c2AABB, B: c2Capsule, m: *mut c2Manifold) {
    let m = &mut *m;
    m.count = 0;
    let mut p = c2Poly::default();
    c2BBVerts(p.verts.as_mut_ptr(), &A as *const _ as *mut c2AABB);
    p.count = 4;
    c2Norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), 4);
    c2CapsuletoPolyManifold(B, &p, std::ptr::null(), m);
    m.n = c2Neg(m.n);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(A: c2Capsule, B: c2Capsule, m: *mut c2Manifold) {
    let m = &mut *m;
    m.count = 0;
    let mut a = c2V(0.0, 0.0);
    let mut b = c2V(0.0, 0.0);
    let r = A.r + B.r;
    let d = c2GJK(
        &A as *const _ as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &B as *const _ as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a,
        &mut b,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if d < r {
        let n = if d == 0.0 {
            c2Norm(c2Skew(c2Sub(A.b, A.a)))
        } else {
            c2Norm(c2Sub(b, a))
        };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
        m.n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(
    A: *const c_void,
    typeA: C2_TYPE,
    B: *const c_void,
    typeB: C2_TYPE,
    m: *mut c2Manifold,
) {
    let mref = &mut *m;
    mref.count = 0;
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircleManifold(*(A as *const c2Circle), *(B as *const c2Circle), m),
            C2_TYPE_AABB => c2CircletoAABBManifold(*(A as *const c2Circle), *(B as *const c2AABB), m),
            C2_TYPE_CAPSULE => c2CircletoCapsuleManifold(*(A as *const c2Circle), *(B as *const c2Capsule), m),
            _ => {}
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoAABBManifold(*(B as *const c2Circle), *(A as *const c2AABB), m);
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_AABB => c2AABBtoAABBManifold(*(A as *const c2AABB), *(B as *const c2AABB), m),
            C2_TYPE_CAPSULE => c2AABBtoCapsuleManifold(*(A as *const c2AABB), *(B as *const c2Capsule), m),
            _ => {}
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoCapsuleManifold(*(B as *const c2Circle), *(A as *const c2Capsule), m);
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_AABB => {
                c2AABBtoCapsuleManifold(*(B as *const c2AABB), *(A as *const c2Capsule), m);
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_CAPSULE => c2CapsuletoCapsuleManifold(*(A as *const c2Capsule), *(B as *const c2Capsule), m),
            _ => {}
        },
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: C2_TYPE,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
) -> *mut c_void {
    let ptr = match typ {
        C2_TYPE_CIRCLE => Box::into_raw(Box::new(c2Circle { p: c2V(a, b), r: c })) as *mut c_void,
        C2_TYPE_AABB => Box::into_raw(Box::new(c2AABB { min: c2V(a, b), max: c2V(c, d) })) as *mut c_void,
        C2_TYPE_CAPSULE => Box::into_raw(Box::new(c2Capsule { a: c2V(a, b), b: c2V(c, d), r: e })) as *mut c_void,
        _ => std::ptr::null_mut(),
    };
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_manifold(
    m: *mut c2Manifold,
    type_a: C2_TYPE,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: C2_TYPE,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    c2Collide(A as *const c_void, type_a, B as *const c_void, type_b, m);
}
