#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_int;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct c2v {
    x: f32,
    y: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct c2r {
    c: f32,
    s: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct c2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[inline]
fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[inline]
fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline]
fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[inline]
fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[inline]
fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[derive(Copy, Clone, Default)]
struct c2Proxy {
    radius: f32,
    count: c_int,
    verts: [c2v; 8],
}

fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

enum ShapeRef<'a> {
    Circle(&'a c2Circle),
    Aabb(&'a c2AABB),
    Capsule(&'a c2Capsule),
}

fn c2MakeProxy(shape: &ShapeRef, ty: C2_TYPE, p: &mut c2Proxy) {
    match ty {
        C2_TYPE::C2_TYPE_CIRCLE => {
            if let ShapeRef::Circle(c) = shape {
                p.radius = c.r;
                p.count = 1;
                p.verts[0] = c.p;
            }
        }
        C2_TYPE::C2_TYPE_AABB => {
            if let ShapeRef::Aabb(bb) = shape {
                p.radius = 0.0;
                p.count = 4;
                c2BBVerts(&mut p.verts, bb);
            }
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            if let ShapeRef::Capsule(c) = shape {
                p.radius = c.r;
                p.count = 2;
                p.verts[0] = c.a;
                p.verts[1] = c.b;
            }
        }
    }
}

#[derive(Copy, Clone, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

#[derive(Copy, Clone, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: c_int,
}

#[inline]
fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[inline]
fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2GJKSimplexMetric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        _ => 0.0,
    }
}

#[inline]
fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline]
fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

fn c22(s: &mut c2Simplex) {
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

fn c23(s: &mut c2Simplex) {
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
fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[inline]
fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[inline]
fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2D(s: &c2Simplex) -> c2v {
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

fn c2Support(verts: &[c2v], count: c_int, d: c2v) -> c_int {
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

fn simplex_get(s: &c2Simplex, i: usize) -> c2sv {
    match i {
        0 => s.a,
        1 => s.b,
        2 => s.c,
        3 => s.d,
        _ => unreachable!(),
    }
}

fn simplex_set(s: &mut c2Simplex, i: usize, v: c2sv) {
    match i {
        0 => s.a = v,
        1 => s.b = v,
        2 => s.c = v,
        3 => s.d = v,
        _ => unreachable!(),
    }
}

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0f32 / s.div;
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

#[inline]
fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[inline]
fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

#[inline]
fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2GJK(
    a_shape: &ShapeRef,
    typeA: C2_TYPE,
    ax_ptr: Option<&c2x>,
    b_shape: &ShapeRef,
    typeB: C2_TYPE,
    bx_ptr: Option<&c2x>,
    outA: Option<&mut c2v>,
    outB: Option<&mut c2v>,
    use_radius: c_int,
    iterations: Option<&mut c_int>,
    cache: Option<&mut c2GJKCache>,
) -> f32 {
    let ax = match ax_ptr {
        None => c2xIdentity(),
        Some(p) => *p,
    };
    let bx = match bx_ptr {
        None => c2xIdentity(),
        Some(p) => *p,
    };
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    c2MakeProxy(a_shape, typeA, &mut pA);
    c2MakeProxy(b_shape, typeB, &mut pB);
    let mut s = c2Simplex::default();
    let mut cache_was_read = 0;

    if let Some(ref cache_ref) = cache {
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count as usize {
                let iA = cache_ref.iA[i];
                let iB = cache_ref.iB[i];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let mut v = simplex_get(&s, i);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
                simplex_set(&mut s, i, v);
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
    let mut save_count: c_int = 0;
    let mut d0: f32 = 3.40282346638528859811704183484516925e+38;
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = simplex_get(&s, i);
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
        if c2Dot(d, d) < 1.19209289550781250000000000000000000e-7
            * 1.19209289550781250000000000000000000e-7
        {
            break;
        }
        let iA_idx = c2Support(&pA.verts[..pA.count as usize], pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA_idx as usize]);
        let iB_idx = c2Support(&pB.verts[..pB.count as usize], pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB_idx as usize]);
        let cur_count = s.count as usize;
        let mut v = simplex_get(&s, cur_count);
        v.iA = iA_idx;
        v.sA = sA;
        v.iB = iB_idx;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
        simplex_set(&mut s, cur_count, v);
        let mut dup = 0;
        for i in 0..save_count as usize {
            if iA_idx == saveA[i] && iB_idx == saveB[i] {
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
    let _ = save_count;

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
        if dist > rA + rB && dist > 1.19209289550781250000000000000000000e-7 {
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
    if let Some(cache_ref) = cache {
        cache_ref.metric = c2GJKSimplexMetric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count as usize {
            let v = simplex_get(&s, i);
            cache_ref.iA[i] = v.iA;
            cache_ref.iB[i] = v.iB;
        }
        cache_ref.div = s.div;
    }
    if let Some(out) = outA {
        *out = a;
    }
    if let Some(out) = outB {
        *out = b;
    }
    if let Some(it) = iterations {
        *it = iter;
    }
    dist
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    (!(d0 | d1 | d2 | d3) & 1) as c_int
}

fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let dist = c2GJK(
        &ShapeRef::Aabb(&A),
        C2_TYPE::C2_TYPE_AABB,
        None,
        &ShapeRef::Capsule(&B),
        C2_TYPE::C2_TYPE_CAPSULE,
        None,
        None,
        None,
        1,
        None,
        None,
    );
    if dist != 0.0 {
        0
    } else {
        1
    }
}

fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let dist = c2GJK(
        &ShapeRef::Capsule(&A),
        C2_TYPE::C2_TYPE_CAPSULE,
        None,
        &ShapeRef::Capsule(&B),
        C2_TYPE::C2_TYPE_CAPSULE,
        None,
        None,
        None,
        1,
        None,
        None,
    );
    if dist != 0.0 {
        0
    } else {
        1
    }
}

fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
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

fn c2Collided_circle_x(A: &c2Circle, b_shape: &ShapeRef, typeB: C2_TYPE) -> c_int {
    match typeB {
        C2_TYPE::C2_TYPE_CIRCLE => {
            if let ShapeRef::Circle(B) = b_shape {
                c2CircletoCircle(*A, **B)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_AABB => {
            if let ShapeRef::Aabb(B) = b_shape {
                c2CircletoAABB(*A, **B)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            if let ShapeRef::Capsule(B) = b_shape {
                c2CircletoCapsule(*A, **B)
            } else {
                0
            }
        }
    }
}

fn c2Collided_aabb_x(A: &c2AABB, b_shape: &ShapeRef, typeB: C2_TYPE) -> c_int {
    match typeB {
        C2_TYPE::C2_TYPE_CIRCLE => {
            if let ShapeRef::Circle(B) = b_shape {
                c2CircletoAABB(**B, *A)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_AABB => {
            if let ShapeRef::Aabb(B) = b_shape {
                c2AABBtoAABB(*A, **B)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            if let ShapeRef::Capsule(B) = b_shape {
                c2AABBtoCapsule(*A, **B)
            } else {
                0
            }
        }
    }
}

fn c2Collided_capsule_x(A: &c2Capsule, b_shape: &ShapeRef, typeB: C2_TYPE) -> c_int {
    match typeB {
        C2_TYPE::C2_TYPE_CIRCLE => {
            if let ShapeRef::Circle(B) = b_shape {
                c2CircletoCapsule(**B, *A)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_AABB => {
            if let ShapeRef::Aabb(B) = b_shape {
                c2AABBtoCapsule(**B, *A)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            if let ShapeRef::Capsule(B) = b_shape {
                c2CapsuletoCapsule(*A, **B)
            } else {
                0
            }
        }
    }
}

fn c2Collided(a_shape: &ShapeRef, typeA: C2_TYPE, b_shape: &ShapeRef, typeB: C2_TYPE) -> c_int {
    match typeA {
        C2_TYPE::C2_TYPE_CIRCLE => {
            if let ShapeRef::Circle(A) = a_shape {
                c2Collided_circle_x(A, b_shape, typeB)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_AABB => {
            if let ShapeRef::Aabb(A) = a_shape {
                c2Collided_aabb_x(A, b_shape, typeB)
            } else {
                0
            }
        }
        C2_TYPE::C2_TYPE_CAPSULE => {
            if let ShapeRef::Capsule(A) = a_shape {
                c2Collided_capsule_x(A, b_shape, typeB)
            } else {
                0
            }
        }
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

    let capsule_obj = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    result += c2Collided(
        &ShapeRef::Circle(&circle),
        C2_TYPE::C2_TYPE_CIRCLE,
        &ShapeRef::Capsule(&capsule_in),
        C2_TYPE::C2_TYPE_CAPSULE,
    );

    result += c2Collided(
        &ShapeRef::Aabb(&aabb),
        C2_TYPE::C2_TYPE_AABB,
        &ShapeRef::Capsule(&capsule_in),
        C2_TYPE::C2_TYPE_CAPSULE,
    ) << 1;

    result += c2Collided(
        &ShapeRef::Capsule(&capsule_obj),
        C2_TYPE::C2_TYPE_CAPSULE,
        &ShapeRef::Capsule(&capsule_in),
        C2_TYPE::C2_TYPE_CAPSULE,
    ) << 2;

    result
}
