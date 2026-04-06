#![allow(non_camel_case_types, non_snake_case, unused_assignments, private_interfaces)]
use std::ptr;

// ── Types matching C layout ──

#[repr(C)]
#[derive(Clone, Copy)]
pub enum C2_TYPE {
    C2_TYPE_CAPSULE = 0,
    C2_TYPE_CIRCLE = 1,
    C2_TYPE_AABB = 2,
    C2_TYPE_POLY = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
pub struct c2Manifold {
    pub count: i32,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2h {
    n: c2v,
    d: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2x {
    p: c2v,
    r: c2r,
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
struct c2Poly {
    count: i32,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2GJKCache {
    metric: f32,
    count: i32,
    iA: [i32; 3],
    iB: [i32; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: i32,
    iB: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: i32,
}

#[derive(Clone, Copy)]
struct c2Proxy {
    radius: f32,
    count: i32,
    verts: [c2v; 8],
}

// ── Helper functions ──

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v { c2v { x, y } }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v { a.x *= b; a.y *= b; a }

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(if a.x > b.x { a.x } else { b.x }, if a.y > b.y { a.y } else { b.y })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(if a.x < b.x { a.x } else { b.x }, if a.y < b.y { a.y } else { b.y })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v { c2Maxv(lo, c2Minv(a, hi)) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v { a.x -= b.x; a.y -= b.y; a }

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v { a.x += b.x; a.y += b.y; a }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 { a.x * b.x + a.y * b.y }

#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: c2h, p: c2v) -> f32 { c2Dot(h.n, p) - h.d }

fn c2Dist_h(h: c2h, p: c2v) -> f32 { c2Dist(h, p) }

#[unsafe(no_mangle)]
pub extern "C" fn c2PlaneAt(p: *const c2Poly, i: i32) -> c2h {
    let p = unsafe { &*p };
    let i = i as usize;
    c2h { n: p.norms[i], d: c2Dot(p.norms[i], p.verts[i]) }
}

fn c2PlaneAt_ref(p: &c2Poly, i: i32) -> c2h {
    let i = i as usize;
    c2h { n: p.norms[i], d: c2Dot(p.norms[i], p.verts[i]) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r { c2r { c: 1.0, s: 0.0 } }

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x { c2x { p: c2V(0.0, 0.0), r: c2RotIdentity() } }

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 { c2Dot(a, a).sqrt() }

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 { a.x * b.y - a.y * b.x }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v { c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v { c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v { c2Add(c2Mulrv(a.r, b), a.p) }

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v { c2MulrvT(a.r, c2Sub(b, a.p)) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v { c2Mulvs(a, 1.0 / b) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v { c2Div(a, c2Len(a)) }

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v { c2V(-a.x, -a.y) }

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v { c2v { x: a.y, y: -a.x } }

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v { c2v { x: -a.y, y: a.x } }

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(if a.x < 0.0 { -a.x } else { a.x }, if a.y < 0.0 { -a.y } else { a.y })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: c2v, b: c2v, da: f32, db: f32) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    let bb = &*bb;
    *out.add(0) = bb.min;
    *out.add(1) = c2V(bb.max.x, bb.min.y);
    *out.add(2) = bb.max;
    *out.add(3) = c2V(bb.min.x, bb.max.y);
}

fn c2BBVerts_slice(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const u8, typ: C2_TYPE, p: *mut c2Proxy) {
    let p = &mut *p;
    match typ {
        C2_TYPE::C2_TYPE_CIRCLE => {
            let c = &*(shape as *const c2Circle);
            p.radius = c.r; p.count = 1; p.verts[0] = c.p;
        },
        C2_TYPE::C2_TYPE_AABB => {
            let bb = &*(shape as *const c2AABB);
            p.radius = 0.0; p.count = 4; c2BBVerts_slice(&mut p.verts, bb);
        },
        C2_TYPE::C2_TYPE_CAPSULE => {
            let c = &*(shape as *const c2Capsule);
            p.radius = c.r; p.count = 2; p.verts[0] = c.a; p.verts[1] = c.b;
        },
        _ => {}
    }
}

fn c2MakeProxy_internal(shape: *const u8, typ: C2_TYPE, p: &mut c2Proxy) {
    unsafe { c2MakeProxy(shape, typ, p as *mut c2Proxy) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    let s = &*s;
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2GJKSimplexMetric_ref(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

fn c2Clip(seg: &mut [c2v; 2], h: c2h) -> i32 {
    let mut out = [c2v::default(); 2];
    let mut sp = 0usize;
    let d0 = c2Dist_h(h, seg[0]);
    let d1 = c2Dist_h(h, seg[1]);
    if d0 < 0.0 { out[sp] = seg[0]; sp += 1; }
    if d1 < 0.0 { out[sp] = seg[1]; sp += 1; }
    if d0 == 0.0 && d1 == 0.0 {
        out[sp] = seg[0]; sp += 1;
        out[sp] = seg[1]; sp += 1;
    } else if d0 * d1 <= 0.0 {
        out[sp] = c2Intersect(seg[0], seg[1], d0, d1); sp += 1;
    }
    seg[0] = out[0];
    seg[1] = out[1];
    sp as i32
}

fn c2SidePlanes(seg: &mut [c2v; 2], ra: c2v, rb: c2v, h: Option<&mut c2h>) -> i32 {
    let inv = c2Norm(c2Sub(rb, ra));
    let left = c2h { n: c2Neg(inv), d: c2Dot(c2Neg(inv), ra) };
    let right = c2h { n: inv, d: c2Dot(inv, rb) };
    if c2Clip(seg, left) < 2 { return 0; }
    if c2Clip(seg, right) < 2 { return 0; }
    if let Some(h) = h {
        h.n = c2CCW90(inv);
        h.d = c2Dot(c2CCW90(inv), ra);
    }
    1
}

fn c2SidePlanesFromPoly(seg: &mut [c2v; 2], x: c2x, p: &c2Poly, e: i32, h: Option<&mut c2h>) -> i32 {
    let ra = c2Mulxv(x, p.verts[e as usize]);
    let next = if e + 1 == p.count { 0 } else { (e + 1) as usize };
    let rb = c2Mulxv(x, p.verts[next]);
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
        s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if u <= 0.0 {
        s.a = s.b; s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else {
        s.a.u = u; s.b.u = v; s.div = u + v; s.count = 2;
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
        s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.a = s.b; s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.a = s.c; s.a.u = 1.0; s.div = 1.0; s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.a.u = uAB; s.b.u = vAB; s.div = uAB + vAB; s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.a = s.b; s.b = s.c; s.a.u = uBC; s.b.u = vBC; s.div = uBC + vBC; s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.b = s.a; s.a = s.c; s.a.u = uCA; s.b.u = vCA; s.div = uCA + vCA; s.count = 2;
    } else {
        s.a.u = uABC; s.b.u = vABC; s.c.u = wABC; s.div = uABC + vABC + wABC; s.count = 3;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    let s = &*s;
    match s.count {
        1 => c2Neg(s.a.p),
        2 => {
            let ab = c2Sub(s.b.p, s.a.p);
            if c2Det2(ab, c2Neg(s.a.p)) > 0.0 { c2Skew(ab) } else { c2CCW90(ab) }
        }
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: i32, d: c2v) -> i32 {
    let mut imax = 0i32;
    if count <= 0 { return 0; }
    let mut dmax = c2Dot(*verts.add(0), d);
    for i in 1..count {
        let dot = c2Dot(*verts.add(i as usize), d);
        if dot > dmax { imax = i; dmax = dot; }
    }
    imax
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let s = &*s;
    let a = &mut *a;
    let b = &mut *b;
    let den = 1.0 / s.div;
    match s.count {
        1 => { *a = s.a.sA; *b = s.a.sB; }
        2 => {
            *a = c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u));
            *b = c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u));
        }
        3 => {
            *a = c2Add(c2Add(c2Mulvs(s.a.sA, den * s.a.u), c2Mulvs(s.b.sA, den * s.b.u)), c2Mulvs(s.c.sA, den * s.c.u));
            *b = c2Add(c2Add(c2Mulvs(s.a.sB, den * s.a.u), c2Mulvs(s.b.sB, den * s.b.u)), c2Mulvs(s.c.sB, den * s.c.u));
        }
        _ => { *a = c2V(0.0, 0.0); *b = c2V(0.0, 0.0); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let s = &*s;
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;
const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    a_shape: *const u8, typeA: C2_TYPE, ax_ptr: *const c2x,
    b_shape: *const u8, typeB: C2_TYPE, bx_ptr: *const c2x,
    outA: *mut c2v, outB: *mut c2v,
    use_radius: i32, iterations: *mut i32, cache: *mut c2GJKCache,
) -> f32 {
    let ax = if ax_ptr.is_null() { c2xIdentity() } else { unsafe { *ax_ptr } };
    let bx = if bx_ptr.is_null() { c2xIdentity() } else { unsafe { *bx_ptr } };
    let mut pA = c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] };
    let mut pB = c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] };
    c2MakeProxy_internal(a_shape, typeA, &mut pA);
    c2MakeProxy_internal(b_shape, typeB, &mut pB);

    let mut s = c2Simplex {
        a: c2sv::default(), b: c2sv::default(), c: c2sv::default(), d: c2sv::default(),
        div: 0.0, count: 0,
    };
    let mut cache_was_read = false;
    if !cache.is_null() {
        let ca = unsafe { &*cache };
        let cache_was_good = ca.count != 0;
        if cache_was_good {
            for i in 0..ca.count as usize {
                let iA = ca.iA[i];
                let iB = ca.iB[i];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = match i { 0 => &mut s.a, 1 => &mut s.b, 2 => &mut s.c, _ => &mut s.d };
                v.iA = iA; v.sA = sA; v.iB = iB; v.sB = sB;
                v.p = c2Sub(v.sB, v.sA); v.u = 0.0;
            }
            s.count = ca.count;
            s.div = ca.div;
            let metric_old = ca.metric;
            let metric = c2GJKSimplexMetric_ref(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = true;
            }
        }
    }
    if !cache_was_read {
        s.a.iA = 0; s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, pA.verts[0]);
        s.a.sB = c2Mulxv(bx, pB.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0; s.div = 1.0; s.count = 1;
    }

    let mut saveA = [0i32; 3];
    let mut saveB = [0i32; 3];
    let mut save_count: i32;
    let mut d0: f32 = FLT_MAX;
    let mut d1: f32 = FLT_MAX;
    let mut iter = 0i32;
    let mut hit = false;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = match i { 0 => &s.a, 1 => &s.b, 2 => &s.c, _ => &s.d };
            saveA[i] = v.iA; saveB[i] = v.iB;
        }
        match s.count {
            1 => {}
            2 => c22(&mut s as *mut c2Simplex),
            3 => c23(&mut s as *mut c2Simplex),
            _ => {}
        }
        if s.count == 3 { hit = true; break; }
        let p = c2L(&mut s as *mut c2Simplex);
        d1 = c2Dot(p, p);
        if d1 > d0 { break; }
        d0 = d1;
        let d = c2D(&mut s as *mut c2Simplex);
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON { break; }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let v = match s.count { 0 => &mut s.a, 1 => &mut s.b, 2 => &mut s.c, _ => &mut s.d };
        v.iA = iA; v.sA = sA; v.iB = iB; v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
        let mut dup = false;
        for i in 0..save_count as usize {
            if iA == saveA[i] && iB == saveB[i] { dup = true; break; }
        }
        if dup { break; }
        s.count += 1;
        iter += 1;
    }

    let mut a = c2v::default();
    let mut b = c2v::default();
    c2Witness(&mut s as *mut c2Simplex, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > FLT_EPSILON {
            dist -= rA + rB;
            let n = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, rA));
            b = c2Sub(b, c2Mulvs(n, rB));
            if a.x == b.x && a.y == b.y { dist = 0.0; }
        } else {
            let p = c2Mulvs(c2Add(a, b), 0.5);
            a = p; b = p; dist = 0.0;
        }
    }

    if !cache.is_null() {
        let ca = unsafe { &mut *cache };
        ca.metric = c2GJKSimplexMetric_ref(&s);
        ca.count = s.count;
        for i in 0..s.count as usize {
            let v = match i { 0 => &s.a, 1 => &s.b, 2 => &s.c, _ => &s.d };
            ca.iA[i] = v.iA; ca.iB[i] = v.iB;
        }
        ca.div = s.div;
    }
    if !outA.is_null() { unsafe { *outA = a; } }
    if !outB.is_null() { unsafe { *outB = b; } }
    if !iterations.is_null() { unsafe { *iterations = iter; } }
    dist
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircleManifold(a: c2Circle, b: c2Circle, m: &mut c2Manifold) {
    m.count = 0;
    let d = c2Sub(b.p, a.p);
    let d2 = c2Dot(d, d);
    let r = a.r + b.r;
    if d2 < r * r {
        let l = d2.sqrt();
        let n = if l != 0.0 { c2Mulvs(d, 1.0 / l) } else { c2V(0.0, 1.0) };
        m.count = 1;
        m.depths[0] = r - l;
        m.contact_points[0] = c2Sub(b.p, c2Mulvs(n, b.r));
        m.n = n;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABBManifold(a: c2Circle, b: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(l, a.p);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 {
        if d2 != 0.0 {
            let d = d2.sqrt();
            let n = c2Norm(ab);
            m.count = 1;
            m.depths[0] = a.r - d;
            m.contact_points[0] = c2Add(a.p, c2Mulvs(n, d));
            m.n = n;
        } else {
            let mid = c2Mulvs(c2Add(b.min, b.max), 0.5);
            let e = c2Mulvs(c2Sub(b.max, b.min), 0.5);
            let d = c2Sub(a.p, mid);
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
            m.depths[0] = a.r + depth;
            m.contact_points[0] = c2Sub(a.p, c2Mulvs(n, depth));
            m.n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsuleManifold(a: c2Circle, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut oa = c2v::default();
    let mut ob = c2v::default();
    let r = a.r + b.r;
    let d = unsafe { c2GJK(
        &a as *const c2Circle as *const u8, C2_TYPE::C2_TYPE_CIRCLE, ptr::null(),
        &b as *const c2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, ptr::null(),
        &mut oa, &mut ob, 0, ptr::null_mut(), ptr::null_mut(),
    ) };
    if d < r {
        let n = if d == 0.0 { c2Norm(c2Skew(c2Sub(b.b, b.a))) } else { c2Norm(c2Sub(ob, oa)) };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(ob, c2Mulvs(n, b.r));
        m.n = n;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABBManifold(a: c2AABB, b: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let mid_a = c2Mulvs(c2Add(a.min, a.max), 0.5);
    let mid_b = c2Mulvs(c2Add(b.min, b.max), 0.5);
    let eA = c2Absv(c2Mulvs(c2Sub(a.max, a.min), 0.5));
    let eB = c2Absv(c2Mulvs(c2Sub(b.max, b.min), 0.5));
    let d = c2Sub(mid_b, mid_a);
    let dx = eA.x + eB.x - (if d.x < 0.0 { -d.x } else { d.x });
    if dx < 0.0 { return; }
    let dy = eA.y + eB.y - (if d.y < 0.0 { -d.y } else { d.y });
    if dy < 0.0 { return; }
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

fn c2KeepDeep(seg: &[c2v; 2], h: c2h, m: &mut c2Manifold) {
    let mut cp = 0;
    for i in 0..2 {
        let p = seg[i];
        let d = c2Dist_h(h, p);
        if d <= 0.0 {
            m.contact_points[cp] = p;
            m.depths[cp] = -d;
            cp += 1;
        }
    }
    m.count = cp as i32;
    m.n = h.n;
}

fn c2Incident(incident: &mut [c2v; 2], ip: &c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    let mut index = 0usize;
    let mut min_dot = FLT_MAX;
    for i in 0..ip.count as usize {
        let dot = c2Dot(rn_in_incident_space, ip.norms[i]);
        if dot < min_dot { min_dot = dot; index = i; }
    }
    incident[0] = c2Mulxv(ix, ip.verts[index]);
    let next = if index + 1 == ip.count as usize { 0 } else { index + 1 };
    incident[1] = c2Mulxv(ix, ip.verts[next]);
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoPolyManifold(a: c2Capsule, b: &c2Poly, bx_ptr: *const c2x, m: &mut c2Manifold) {
    m.count = 0;
    let mut oa = c2v::default();
    let mut ob = c2v::default();
    let d = unsafe { c2GJK(
        &a as *const c2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, ptr::null(),
        b as *const c2Poly as *const u8, C2_TYPE::C2_TYPE_POLY, bx_ptr,
        &mut oa, &mut ob, 0, ptr::null_mut(), ptr::null_mut(),
    ) };
    if d < 1.0e-6 {
        let bx = if bx_ptr.is_null() { c2xIdentity() } else { unsafe { *bx_ptr } };
        let a_in_b = c2Capsule { a: c2MulxvT(bx, a.a), b: c2MulxvT(bx, a.b), r: a.r };
        let ab = c2Norm(c2Sub(a_in_b.a, a_in_b.b));
        let ab_h0 = c2h { n: c2CCW90(ab), d: c2Dot(a_in_b.a, c2CCW90(ab)) };
        let v0 = unsafe { c2Support(b.verts.as_ptr(), b.count, c2Neg(ab_h0.n)) };
        let s0 = c2Dist_h(ab_h0, b.verts[v0 as usize]);
        let ab_h1 = c2h { n: c2Skew(ab), d: c2Dot(a_in_b.a, c2Skew(ab)) };
        let v1 = unsafe { c2Support(b.verts.as_ptr(), b.count, c2Neg(ab_h1.n)) };
        let s1 = c2Dist_h(ab_h1, b.verts[v1 as usize]);
        let mut index = 0i32;
        let mut sep = -FLT_MAX;
        let mut code = 0i32;
        for i in 0..b.count {
            let h = c2PlaneAt_ref(b, i);
            let da = c2Dot(a_in_b.a, c2Neg(h.n));
            let db = c2Dot(a_in_b.b, c2Neg(h.n));
            let d = if da > db { c2Dist_h(h, a_in_b.a) } else { c2Dist_h(h, a_in_b.b) };
            if d > sep { sep = d; index = i; }
        }
        if s0 > sep { sep = s0; index = v0; code = 1; }
        if s1 > sep { sep = s1; index = v1; code = 2; }
        match code {
            0 => {
                let mut seg = [a.a, a.b];
                let mut h = c2h { n: c2v::default(), d: 0.0 };
                if c2SidePlanesFromPoly(&mut seg, bx, b, index, Some(&mut h)) == 0 { return; }
                let seg2 = [seg[0], seg[1]];
                c2KeepDeep(&seg2, h, m);
                m.n = c2Neg(m.n);
            }
            1 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(&mut incident, b, bx, ab_h0.n);
                let mut h = c2h { n: c2v::default(), d: 0.0 };
                if c2SidePlanes(&mut incident, a_in_b.b, a_in_b.a, Some(&mut h)) == 0 { return; }
                let inc2 = [incident[0], incident[1]];
                c2KeepDeep(&inc2, h, m);
            }
            2 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(&mut incident, b, bx, ab_h1.n);
                let mut h = c2h { n: c2v::default(), d: 0.0 };
                if c2SidePlanes(&mut incident, a_in_b.a, a_in_b.b, Some(&mut h)) == 0 { return; }
                let inc2 = [incident[0], incident[1]];
                c2KeepDeep(&inc2, h, m);
            }
            _ => return,
        }
        for i in 0..m.count as usize {
            m.depths[i] += a.r;
        }
    } else if d < a.r {
        m.count = 1;
        m.n = c2Norm(c2Sub(ob, oa));
        m.contact_points[0] = c2Add(oa, c2Mulvs(m.n, a.r));
        m.depths[0] = a.r - d;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *const c2v, norms: *mut c2v, count: i32) {
    for i in 0..count as usize {
        let b = if i + 1 < count as usize { i + 1 } else { 0 };
        let e = c2Sub(*verts.add(b), *verts.add(i));
        *norms.add(i) = c2Norm(c2CCW90(e));
    }
}

fn c2Norms_internal(verts: &[c2v], norms: &mut [c2v], count: i32) {
    for i in 0..count as usize {
        let b = if i + 1 < count as usize { i + 1 } else { 0 };
        let e = c2Sub(verts[b], verts[i]);
        norms[i] = c2Norm(c2CCW90(e));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsuleManifold(a: c2AABB, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut p = c2Poly { count: 4, verts: [c2v::default(); 8], norms: [c2v::default(); 8] };
    c2BBVerts_slice(&mut p.verts, &a);
    c2Norms_internal(&p.verts, &mut p.norms, 4);
    c2CapsuletoPolyManifold(b, &p, ptr::null(), m);
    m.n = c2Neg(m.n);
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsuleManifold(a: c2Capsule, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut oa = c2v::default();
    let mut ob = c2v::default();
    let r = a.r + b.r;
    let d = unsafe { c2GJK(
        &a as *const c2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, ptr::null(),
        &b as *const c2Capsule as *const u8, C2_TYPE::C2_TYPE_CAPSULE, ptr::null(),
        &mut oa, &mut ob, 0, ptr::null_mut(), ptr::null_mut(),
    ) };
    if d < r {
        let n = if d == 0.0 { c2Norm(c2Skew(c2Sub(a.b, a.a))) } else { c2Norm(c2Sub(ob, oa)) };
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(ob, c2Mulvs(n, b.r));
        m.n = n;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(a: *const u8, typeA: C2_TYPE, b: *const u8, typeB: C2_TYPE, m: *mut c2Manifold) {
    let m = &mut *m;
    m.count = 0;
    use C2_TYPE::*;
    match (typeA, typeB) {
        (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE) => unsafe {
            c2CircletoCircleManifold(*(a as *const c2Circle), *(b as *const c2Circle), m);
        },
        (C2_TYPE_CIRCLE, C2_TYPE_AABB) => unsafe {
            c2CircletoAABBManifold(*(a as *const c2Circle), *(b as *const c2AABB), m);
        },
        (C2_TYPE_CIRCLE, C2_TYPE_CAPSULE) => unsafe {
            c2CircletoCapsuleManifold(*(a as *const c2Circle), *(b as *const c2Capsule), m);
        },
        (C2_TYPE_AABB, C2_TYPE_CIRCLE) => unsafe {
            c2CircletoAABBManifold(*(b as *const c2Circle), *(a as *const c2AABB), m);
            m.n = c2Neg(m.n);
        },
        (C2_TYPE_AABB, C2_TYPE_AABB) => unsafe {
            c2AABBtoAABBManifold(*(a as *const c2AABB), *(b as *const c2AABB), m);
        },
        (C2_TYPE_AABB, C2_TYPE_CAPSULE) => unsafe {
            c2AABBtoCapsuleManifold(*(a as *const c2AABB), *(b as *const c2Capsule), m);
        },
        (C2_TYPE_CAPSULE, C2_TYPE_CIRCLE) => unsafe {
            c2CircletoCapsuleManifold(*(b as *const c2Circle), *(a as *const c2Capsule), m);
            m.n = c2Neg(m.n);
        },
        (C2_TYPE_CAPSULE, C2_TYPE_AABB) => unsafe {
            c2AABBtoCapsuleManifold(*(b as *const c2AABB), *(a as *const c2Capsule), m);
            m.n = c2Neg(m.n);
        },
        (C2_TYPE_CAPSULE, C2_TYPE_CAPSULE) => unsafe {
            c2CapsuletoCapsuleManifold(*(a as *const c2Capsule), *(b as *const c2Capsule), m);
        },
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ptr_from_parts(typ: C2_TYPE, a: f32, b: f32, c: f32, d: f32, e: f32) -> *mut u8 {
    use C2_TYPE::*;
    match typ {
        C2_TYPE_CIRCLE => {
            let p = Box::into_raw(Box::new(c2Circle { p: c2V(a, b), r: c }));
            p as *mut u8
        }
        C2_TYPE_AABB => {
            let p = Box::into_raw(Box::new(c2AABB { min: c2V(a, b), max: c2V(c, d) }));
            p as *mut u8
        }
        C2_TYPE_CAPSULE => {
            let p = Box::into_raw(Box::new(c2Capsule { a: c2V(a, b), b: c2V(c, d), r: e }));
            p as *mut u8
        }
        _ => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn omni_manifold(
    m: *mut c2Manifold,
    type_a: C2_TYPE, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
    type_b: C2_TYPE, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32,
) {
    let a = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let b = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    unsafe { c2Collide(a, type_a, b, type_b, m) };
    // Note: C code leaks these allocations; we match that behavior
}
