#![allow(non_snake_case)]

use std::ffi::{c_int, c_void};

type C2Type = c_int;

const C2_TYPE_CAPSULE: C2Type = 0;
const C2_TYPE_CIRCLE: C2Type = 1;
const C2_TYPE_AABB: C2Type = 2;
const C2_TYPE_POLY: C2Type = 3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2h {
    pub n: C2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Poly {
    pub count: c_int,
    pub verts: [C2v; 8],
    pub norms: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2GjkCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [C2v; 2],
    pub n: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2sv {
    pub sA: C2v,
    pub sB: C2v,
    pub p: C2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct C2Simplex {
    pub a: C2sv,
    pub b: C2sv,
    pub c: C2sv,
    pub d: C2sv,
    pub div: f32,
    pub count: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: C2h, p: C2v) -> f32 {
    c2Dot(h.n, p) - h.d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2PlaneAt(p: *const C2Poly, i: c_int) -> C2h {
    let p = &*p;
    let i = i as usize;
    C2h {
        n: p.norms[i],
        d: c2Dot(p.norms[i], p.verts[i]),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> C2x {
    C2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut C2v, bb: *mut C2Aabb) {
    let bb = &*bb;
    *out.add(0) = bb.min;
    *out.add(1) = c2V(bb.max.x, bb.min.y);
    *out.add(2) = bb.max;
    *out.add(3) = c2V(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, typ: C2Type, p: *mut C2Proxy) {
    let p = &mut *p;
    match typ {
        C2_TYPE_CIRCLE => {
            let c = &*(shape as *const C2Circle);
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE_AABB => {
            let bb = shape as *mut C2Aabb;
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(p.verts.as_mut_ptr(), bb);
        }
        C2_TYPE_CAPSULE => {
            let c = &*(shape as *const C2Capsule);
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: C2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut C2Simplex) -> f32 {
    let s = &*s;
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
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
pub extern "C" fn c2Add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: C2x, b: C2v) -> C2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: C2x, b: C2v) -> C2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: C2v, b: C2v, da: f32, db: f32) -> C2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

unsafe fn c2_clip(seg: *mut C2v, h: C2h) -> c_int {
    let input0 = *seg.add(0);
    let input1 = *seg.add(1);
    let mut out = [C2v::default(); 2];
    let mut sp = 0usize;
    let d0 = c2Dist(h, input0);
    let d1 = c2Dist(h, input1);
    if d0 < 0.0 {
        out[sp] = input0;
        sp += 1;
    }
    if d1 < 0.0 {
        out[sp] = input1;
        sp += 1;
    }
    if d0 == 0.0 && d1 == 0.0 {
        out[sp] = input0;
        sp += 1;
        out[sp] = input1;
        sp += 1;
    } else if d0 * d1 <= 0.0 {
        out[sp] = c2Intersect(input0, input1, d0, d1);
        sp += 1;
    }
    *seg.add(0) = out[0];
    *seg.add(1) = out[1];
    sp as c_int
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
pub extern "C" fn c2Neg(a: C2v) -> C2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

unsafe fn c2_side_planes(seg: *mut C2v, ra: C2v, rb: C2v, h: *mut C2h) -> c_int {
    let in_v = c2Norm(c2Sub(rb, ra));
    let left = C2h {
        n: c2Neg(in_v),
        d: c2Dot(c2Neg(in_v), ra),
    };
    let right = C2h {
        n: in_v,
        d: c2Dot(in_v, rb),
    };
    if c2_clip(seg, left) < 2 {
        return 0;
    }
    if c2_clip(seg, right) < 2 {
        return 0;
    }
    if !h.is_null() {
        (*h).n = c2CCW90(in_v);
        (*h).d = c2Dot(c2CCW90(in_v), ra);
    }
    1
}

unsafe fn c2_side_planes_from_poly(
    seg: *mut C2v,
    x: C2x,
    p: *const C2Poly,
    e: c_int,
    h: *mut C2h,
) -> c_int {
    let p_ref = &*p;
    let e = e as usize;
    let ra = c2Mulxv(x, p_ref.verts[e]);
    let next = if e + 1 == p_ref.count as usize { 0 } else { e + 1 };
    let rb = c2Mulxv(x, p_ref.verts[next]);
    c2_side_planes(seg, ra, rb, h)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut C2Simplex) {
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
pub unsafe extern "C" fn c23(s: *mut C2Simplex) {
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
pub extern "C" fn c2Skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut C2Simplex) -> C2v {
    let s = &*s;
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
pub unsafe extern "C" fn c2Support(verts: *const C2v, count: c_int, d: C2v) -> c_int {
    let mut imax = 0;
    let mut dmax = c2Dot(*verts, d);
    for i in 1..count {
        let dot = c2Dot(*verts.add(i as usize), d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut C2Simplex, a: *mut C2v, b: *mut C2v) {
    let s = &*s;
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.a.sA;
            *b = s.a.sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs(s.a.sA, den * s.a.u),
                c2Mulvs(s.b.sA, den * s.b.u),
            );
            *b = c2Add(
                c2Mulvs(s.a.sB, den * s.a.u),
                c2Mulvs(s.b.sB, den * s.b.u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs(s.a.sA, den * s.a.u),
                    c2Mulvs(s.b.sA, den * s.b.u),
                ),
                c2Mulvs(s.c.sA, den * s.c.u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs(s.a.sB, den * s.a.u),
                    c2Mulvs(s.b.sB, den * s.b.u),
                ),
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
pub unsafe extern "C" fn c2L(s: *mut C2Simplex) -> C2v {
    let s = &*s;
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(
            c2Mulvs(s.a.p, den * s.a.u),
            c2Mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: C2Type,
    ax_ptr: *const C2x,
    B: *const c_void,
    typeB: C2Type,
    bx_ptr: *const C2x,
    outA: *mut C2v,
    outB: *mut C2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut C2GjkCache,
) -> f32 {
    let ax = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        *ax_ptr
    };
    let bx = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        *bx_ptr
    };
    let mut pA = C2Proxy::default();
    let mut pB = C2Proxy::default();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
    let mut s = C2Simplex::default();
    let verts = &mut s.a as *mut C2sv;
    let mut cache_was_read = false;
    if !cache.is_null() {
        let cache_ref = &*cache;
        if cache_ref.count != 0 {
            for i in 0..cache_ref.count {
                let index = i as usize;
                let iA = cache_ref.iA[index];
                let iB = cache_ref.iB[index];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = &mut *verts.add(index);
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
            let metric = c2GJKSimplexMetric(&mut s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = true;
            }
        }
    }
    if !cache_was_read {
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
    let mut d0 = f32::MAX;
    let mut iter = 0;
    let mut hit = false;
    while iter < 20 {
        let save_count = s.count;
        for i in 0..save_count {
            saveA[i as usize] = (*verts.add(i as usize)).iA;
            saveB[i as usize] = (*verts.add(i as usize)).iB;
        }
        match s.count {
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
        }
        if s.count == 3 {
            hit = true;
            break;
        }
        let p = c2L(&mut s);
        let d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&mut s);
        if c2Dot(d, d) < f32::EPSILON * f32::EPSILON {
            break;
        }
        let iA = c2Support(
            pA.verts.as_ptr(),
            pA.count,
            c2MulrvT(ax.r, c2Neg(d)),
        );
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let v = &mut *verts.add(s.count as usize);
        v.iA = iA;
        v.sA = sA;
        v.iB = iB;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
        let mut dup = false;
        for i in 0..save_count {
            if iA == saveA[i as usize] && iB == saveB[i as usize] {
                dup = true;
                break;
            }
        }
        if dup {
            break;
        }
        s.count += 1;
        iter += 1;
    }
    let mut a = C2v::default();
    let mut b = C2v::default();
    c2Witness(&mut s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > f32::EPSILON {
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
        for i in 0..s.count {
            let v = &*verts.add(i as usize);
            cache_ref.iA[i as usize] = v.iA;
            cache_ref.iB[i as usize] = v.iB;
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
pub extern "C" fn c2Absv(a: C2v) -> C2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCircleManifold(
    A: C2Circle,
    B: C2Circle,
    m: *mut C2Manifold,
) {
    let m = &mut *m;
    m.count = 0;
    let d = c2Sub(B.p, A.p);
    let d2 = c2Dot(d, d);
    let r = A.r + B.r;
    if d2 < r * r {
        let l = d2.sqrt();
        let n = if l != 0.0 {
            c2Mulvs(d, 1.0 / l)
        } else {
            c2V(0.0, 1.0)
        };
        m.count = 1;
        m.depths[0] = r - l;
        m.contact_points[0] = c2Sub(B.p, c2Mulvs(n, B.r));
        m.n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoAABBManifold(
    A: C2Circle,
    B: C2Aabb,
    m: *mut C2Manifold,
) {
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
            let (depth, n) = if x_overlap < y_overlap {
                (
                    x_overlap,
                    c2Mulvs(c2V(1.0, 0.0), if d.x < 0.0 { 1.0 } else { -1.0 }),
                )
            } else {
                (
                    y_overlap,
                    c2Mulvs(c2V(0.0, 1.0), if d.y < 0.0 { 1.0 } else { -1.0 }),
                )
            };
            m.count = 1;
            m.depths[0] = A.r + depth;
            m.contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
            m.n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(
    A: C2Circle,
    B: C2Capsule,
    m: *mut C2Manifold,
) {
    let m = &mut *m;
    m.count = 0;
    let mut a = C2v::default();
    let mut b = C2v::default();
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
pub unsafe extern "C" fn c2AABBtoAABBManifold(
    A: C2Aabb,
    B: C2Aabb,
    m: *mut C2Manifold,
) {
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
    let (depth, n, p) = if dx < dy {
        if d.x < 0.0 {
            (dx, c2V(-1.0, 0.0), c2Sub(mid_a, c2V(eA.x, 0.0)))
        } else {
            (dx, c2V(1.0, 0.0), c2Add(mid_a, c2V(eA.x, 0.0)))
        }
    } else if d.y < 0.0 {
        (dy, c2V(0.0, -1.0), c2Sub(mid_a, c2V(0.0, eA.y)))
    } else {
        (dy, c2V(0.0, 1.0), c2Add(mid_a, c2V(0.0, eA.y)))
    };
    m.count = 1;
    m.contact_points[0] = p;
    m.depths[0] = depth;
    m.n = n;
}

unsafe fn c2_keep_deep(seg: *mut C2v, h: C2h, m: *mut C2Manifold) {
    let m = &mut *m;
    let mut cp = 0usize;
    for i in 0..2 {
        let p = *seg.add(i);
        let d = c2Dist(h, p);
        if d <= 0.0 {
            m.contact_points[cp] = p;
            m.depths[cp] = -d;
            cp += 1;
        }
    }
    m.count = cp as c_int;
    m.n = h.n;
}

unsafe fn c2_incident(
    incident: *mut C2v,
    ip: *const C2Poly,
    ix: C2x,
    rn_in_incident_space: C2v,
) {
    let ip = &*ip;
    let mut index = !0usize;
    let mut min_dot = f32::MAX;
    for i in 0..ip.count as usize {
        let dot = c2Dot(rn_in_incident_space, ip.norms[i]);
        if dot < min_dot {
            min_dot = dot;
            index = i;
        }
    }
    *incident.add(0) = c2Mulxv(ix, ip.verts[index]);
    let next = if index + 1 == ip.count as usize {
        0
    } else {
        index + 1
    };
    *incident.add(1) = c2Mulxv(ix, ip.verts[next]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoPolyManifold(
    A: C2Capsule,
    B: *const C2Poly,
    bx_ptr: *const C2x,
    m: *mut C2Manifold,
) {
    (*m).count = 0;
    let mut a = C2v::default();
    let mut b = C2v::default();
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
    if d < 1.0e-6 {
        let bx = if bx_ptr.is_null() {
            c2xIdentity()
        } else {
            *bx_ptr
        };
        let A_in_B = C2Capsule {
            a: c2MulxvT(bx, A.a),
            b: c2MulxvT(bx, A.b),
            r: A.r,
        };
        let ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));
        let ab_h0 = C2h {
            n: c2CCW90(ab),
            d: c2Dot(A_in_B.a, c2CCW90(ab)),
        };
        let v0 = c2Support((*B).verts.as_ptr(), (*B).count, c2Neg(ab_h0.n));
        let s0 = c2Dist(ab_h0, (*B).verts[v0 as usize]);
        let ab_h1 = C2h {
            n: c2Skew(ab),
            d: c2Dot(A_in_B.a, c2Skew(ab)),
        };
        let v1 = c2Support((*B).verts.as_ptr(), (*B).count, c2Neg(ab_h1.n));
        let s1 = c2Dist(ab_h1, (*B).verts[v1 as usize]);
        let mut index = !0;
        let mut sep = -f32::MAX;
        let mut code = 0;
        for i in 0..(*B).count {
            let h = c2PlaneAt(B, i);
            let da = c2Dot(A_in_B.a, c2Neg(h.n));
            let db = c2Dot(A_in_B.b, c2Neg(h.n));
            let distance = if da > db {
                c2Dist(h, A_in_B.a)
            } else {
                c2Dist(h, A_in_B.b)
            };
            if distance > sep {
                sep = distance;
                index = i;
            }
        }
        if s0 > sep {
            sep = s0;
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
                let mut h = C2h::default();
                if c2_side_planes_from_poly(seg.as_mut_ptr(), bx, B, index, &mut h) == 0 {
                    return;
                }
                c2_keep_deep(seg.as_mut_ptr(), h, m);
                (*m).n = c2Neg((*m).n);
            }
            1 => {
                let mut incident = [C2v::default(); 2];
                c2_incident(incident.as_mut_ptr(), B, bx, ab_h0.n);
                let mut h = C2h::default();
                if c2_side_planes(incident.as_mut_ptr(), A_in_B.b, A_in_B.a, &mut h) == 0 {
                    return;
                }
                c2_keep_deep(incident.as_mut_ptr(), h, m);
            }
            2 => {
                let mut incident = [C2v::default(); 2];
                c2_incident(incident.as_mut_ptr(), B, bx, ab_h1.n);
                let mut h = C2h::default();
                if c2_side_planes(incident.as_mut_ptr(), A_in_B.a, A_in_B.b, &mut h) == 0 {
                    return;
                }
                c2_keep_deep(incident.as_mut_ptr(), h, m);
            }
            _ => return,
        }
        for i in 0..(*m).count as usize {
            (*m).depths[i] += A.r;
        }
    } else if d < A.r {
        (*m).count = 1;
        (*m).n = c2Norm(c2Sub(b, a));
        (*m).contact_points[0] = c2Add(a, c2Mulvs((*m).n, A.r));
        (*m).depths[0] = A.r - d;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *mut C2v, norms: *mut C2v, count: c_int) {
    for i in 0..count {
        let a = i as usize;
        let b = if i + 1 < count { (i + 1) as usize } else { 0 };
        let e = c2Sub(*verts.add(b), *verts.add(a));
        *norms.add(a) = c2Norm(c2CCW90(e));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(
    A: C2Aabb,
    B: C2Capsule,
    m: *mut C2Manifold,
) {
    (*m).count = 0;
    let mut p = C2Poly::default();
    c2BBVerts(p.verts.as_mut_ptr(), &A as *const _ as *mut C2Aabb);
    p.count = 4;
    c2Norms(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), 4);
    c2CapsuletoPolyManifold(B, &p, std::ptr::null(), m);
    (*m).n = c2Neg((*m).n);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(
    A: C2Capsule,
    B: C2Capsule,
    m: *mut C2Manifold,
) {
    (*m).count = 0;
    let mut a = C2v::default();
    let mut b = C2v::default();
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
        (*m).count = 1;
        (*m).depths[0] = r - d;
        (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
        (*m).n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(
    A: *const c_void,
    typeA: C2Type,
    B: *const c_void,
    typeB: C2Type,
    m: *mut C2Manifold,
) {
    (*m).count = 0;
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoCircleManifold(*(A as *const C2Circle), *(B as *const C2Circle), m)
            }
            C2_TYPE_AABB => {
                c2CircletoAABBManifold(*(A as *const C2Circle), *(B as *const C2Aabb), m)
            }
            C2_TYPE_CAPSULE => {
                c2CircletoCapsuleManifold(*(A as *const C2Circle), *(B as *const C2Capsule), m)
            }
            _ => {}
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoAABBManifold(*(B as *const C2Circle), *(A as *const C2Aabb), m);
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_AABB => {
                c2AABBtoAABBManifold(*(A as *const C2Aabb), *(B as *const C2Aabb), m)
            }
            C2_TYPE_CAPSULE => {
                c2AABBtoCapsuleManifold(*(A as *const C2Aabb), *(B as *const C2Capsule), m)
            }
            _ => {}
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => {
                c2CircletoCapsuleManifold(*(B as *const C2Circle), *(A as *const C2Capsule), m);
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_AABB => {
                c2AABBtoCapsuleManifold(*(B as *const C2Aabb), *(A as *const C2Capsule), m);
                (*m).n = c2Neg((*m).n);
            }
            C2_TYPE_CAPSULE => {
                c2CapsuletoCapsuleManifold(*(A as *const C2Capsule), *(B as *const C2Capsule), m)
            }
            _ => {}
        },
        _ => {}
    }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: C2Type,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
) -> *mut c_void {
    match typ {
        C2_TYPE_CIRCLE => {
            let circle = malloc(std::mem::size_of::<C2Circle>()) as *mut C2Circle;
            (*circle).p = c2V(a, b);
            (*circle).r = c;
            circle as *mut c_void
        }
        C2_TYPE_AABB => {
            let aabb = malloc(std::mem::size_of::<C2Aabb>()) as *mut C2Aabb;
            (*aabb).min = c2V(a, b);
            (*aabb).max = c2V(c, d);
            aabb as *mut c_void
        }
        C2_TYPE_CAPSULE => {
            let capsule = malloc(std::mem::size_of::<C2Capsule>()) as *mut C2Capsule;
            (*capsule).a = c2V(a, b);
            (*capsule).b = c2V(c, d);
            (*capsule).r = e;
            capsule as *mut c_void
        }
        _ => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omni_manifold(
    m: *mut C2Manifold,
    type_a: C2Type,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: C2Type,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    c2Collide(A, type_a, B, type_b, m);
}
