//! Rust translation of `c_src/src/lib.c` (a trimmed-down `cute_c2` style 2D
//! collision library).
//!
//! The translation is intentionally literal: operation order, float precision
//! (`f32` throughout) and branch order are preserved so results are
//! bit-identical to the C original. Original bugs / quirks are kept.

#![allow(non_snake_case)]

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Public C enum (from include/lib.h)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;

// ---------------------------------------------------------------------------
// Basic types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Default)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Clone, Copy, Default)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Clone, Copy, Default)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Clone, Copy, Default)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[derive(Clone, Copy, Default)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Clone, Copy, Default)]
struct C2GjkCache {
    metric: f32,
    count: i32,
    iA: [i32; 3],
    iB: [i32; 3],
    div: f32,
}

/// Tagged union standing in for the `const void *shape` + `C2_TYPE` pair used
/// by the C code. In the C original the type tag always matches the allocated
/// struct, so a tagged enum is behaviourally equivalent.
#[derive(Clone, Copy)]
enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
}

// ---------------------------------------------------------------------------
// Vector math
// ---------------------------------------------------------------------------

#[inline]
fn c2V(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[inline]
fn c2Mulvs(mut a: C2v, b: f32) -> C2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline]
fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline]
fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[inline]
fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline]
fn c2Dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline]
fn c2RotIdentity() -> C2r {
    C2r { c: 1.0, s: 0.0 }
}

#[inline]
fn c2xIdentity() -> C2x {
    C2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[inline]
fn c2Len(a: C2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[inline]
fn c2Det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[inline]
fn c2Mulrv(a: C2r, b: C2v) -> C2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
fn c2Add(mut a: C2v, b: C2v) -> C2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline]
fn c2Mulxv(a: C2x, b: C2v) -> C2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[inline]
fn c2Neg(a: C2v) -> C2v {
    c2V(-a.x, -a.y)
}

#[inline]
fn c2Skew(a: C2v) -> C2v {
    C2v { x: -a.y, y: a.x }
}

#[inline]
fn c2CCW90(a: C2v) -> C2v {
    C2v { x: a.y, y: -a.x }
}

#[inline]
fn c2Div(a: C2v, b: f32) -> C2v {
    c2Mulvs(a, 1.0 / b)
}

#[inline]
fn c2Norm(a: C2v) -> C2v {
    c2Div(a, c2Len(a))
}

#[inline]
fn c2MulrvT(a: C2r, b: C2v) -> C2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

// ---------------------------------------------------------------------------
// Proxy
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct C2Proxy {
    radius: f32,
    count: i32,
    verts: [C2v; 8],
}

impl Default for C2Proxy {
    fn default() -> Self {
        C2Proxy {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

fn c2BBVerts(out: &mut [C2v], bb: &C2Aabb) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

fn c2MakeProxy(shape: &Shape, p: &mut C2Proxy) {
    match shape {
        Shape::Circle(c) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        Shape::Aabb(bb) => {
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts, bb);
        }
        Shape::Capsule(c) => {
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

// ---------------------------------------------------------------------------
// Simplex
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct C2sv {
    sA: C2v,
    sB: C2v,
    p: C2v,
    u: f32,
    iA: i32,
    iB: i32,
}

/// In C this is `struct { c2sv a, b, c, d; float div; int count; }` and the
/// code aliases `&s.a` as an array of four `c2sv`. Modelled directly as the
/// array here; `verts[0..3]` are `a`, `b`, `c`, `d`.
#[derive(Clone, Copy, Default)]
struct C2Simplex {
    verts: [C2sv; 4],
    div: f32,
    count: i32,
}

fn c2GJKSimplexMetric(s: &C2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.verts[1].p, s.verts[0].p)),
        3 => c2Det2(
            c2Sub(s.verts[1].p, s.verts[0].p),
            c2Sub(s.verts[2].p, s.verts[0].p),
        ),
        // `default:` falls through into `case 1:` in the C source.
        _ => 0.0,
    }
}

fn c22(s: &mut C2Simplex) {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else {
        s.verts[0].u = u;
        s.verts[1].u = v;
        s.div = u + v;
        s.count = 2;
    }
}

fn c23(s: &mut C2Simplex) {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let c = s.verts[2].p;
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
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.verts[0] = s.verts[2];
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.verts[0].u = uAB;
        s.verts[1].u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[1] = s.verts[2];
        s.verts[0].u = uBC;
        s.verts[1].u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.verts[1] = s.verts[0];
        s.verts[0] = s.verts[2];
        s.verts[0].u = uCA;
        s.verts[1].u = vCA;
        s.div = uCA + vCA;
        s.count = 2;
    } else {
        s.verts[0].u = uABC;
        s.verts[1].u = vABC;
        s.verts[2].u = wABC;
        s.div = uABC + vABC + wABC;
        s.count = 3;
    }
}

fn c2D(s: &C2Simplex) -> C2v {
    match s.count {
        1 => c2Neg(s.verts[0].p),
        2 => {
            let ab = c2Sub(s.verts[1].p, s.verts[0].p);
            if c2Det2(ab, c2Neg(s.verts[0].p)) > 0.0 {
                return c2Skew(ab);
            }
            c2CCW90(ab)
        }
        _ => c2V(0.0, 0.0),
    }
}

fn c2Support(verts: &[C2v], count: i32, d: C2v) -> i32 {
    let mut imax = 0;
    let mut dmax = c2Dot(verts[0], d);
    for i in 1..count {
        let dot = c2Dot(verts[i as usize], d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
    }
    imax
}

fn c2Witness(s: &C2Simplex) -> (C2v, C2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => (s.verts[0].sA, s.verts[0].sB),
        2 => (
            c2Add(
                c2Mulvs(s.verts[0].sA, den * s.verts[0].u),
                c2Mulvs(s.verts[1].sA, den * s.verts[1].u),
            ),
            c2Add(
                c2Mulvs(s.verts[0].sB, den * s.verts[0].u),
                c2Mulvs(s.verts[1].sB, den * s.verts[1].u),
            ),
        ),
        3 => (
            c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sA, den * s.verts[0].u),
                    c2Mulvs(s.verts[1].sA, den * s.verts[1].u),
                ),
                c2Mulvs(s.verts[2].sA, den * s.verts[2].u),
            ),
            c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sB, den * s.verts[0].u),
                    c2Mulvs(s.verts[1].sB, den * s.verts[1].u),
                ),
                c2Mulvs(s.verts[2].sB, den * s.verts[2].u),
            ),
        ),
        _ => (c2V(0.0, 0.0), c2V(0.0, 0.0)),
    }
}

fn c2L(s: &C2Simplex) -> C2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.verts[0].p,
        2 => c2Add(
            c2Mulvs(s.verts[0].p, den * s.verts[0].u),
            c2Mulvs(s.verts[1].p, den * s.verts[1].u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

const FLT_MAX: f32 = 3.40282346638528859811704183484516925e+38;
const FLT_EPSILON: f32 = 1.19209289550781250000000000000000000e-7;

#[allow(clippy::too_many_arguments)]
fn c2GJK(
    A: &Shape,
    ax_ptr: Option<&C2x>,
    B: &Shape,
    bx_ptr: Option<&C2x>,
    outA: Option<&mut C2v>,
    outB: Option<&mut C2v>,
    use_radius: i32,
    iterations: Option<&mut i32>,
    mut cache: Option<&mut C2GjkCache>,
) -> f32 {
    let ax = match ax_ptr {
        None => c2xIdentity(),
        Some(x) => *x,
    };
    let bx = match bx_ptr {
        None => c2xIdentity(),
        Some(x) => *x,
    };
    let mut pA = C2Proxy::default();
    let mut pB = C2Proxy::default();
    c2MakeProxy(A, &mut pA);
    c2MakeProxy(B, &mut pB);
    let mut s = C2Simplex::default();
    let mut cache_was_read = 0;
    if let Some(c) = cache.as_deref() {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let iA = c.iA[i];
                let iB = c.iB[i];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = &mut s.verts[i];
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = c.count;
            s.div = c.div;
            let metric_old = c.metric;
            let metric = c2GJKSimplexMetric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = 1;
            }
        }
    }
    if cache_was_read == 0 {
        s.verts[0].iA = 0;
        s.verts[0].iB = 0;
        s.verts[0].sA = c2Mulxv(ax, pA.verts[0]);
        s.verts[0].sB = c2Mulxv(bx, pB.verts[0]);
        s.verts[0].p = c2Sub(s.verts[0].sB, s.verts[0].sA);
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }
    let mut saveA = [0i32; 3];
    let mut saveB = [0i32; 3];
    let mut save_count;
    let mut d0 = FLT_MAX;
    let mut _d1 = FLT_MAX;
    let mut iter = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            saveA[i] = s.verts[i].iA;
            saveB[i] = s.verts[i].iB;
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
        let d1 = c2Dot(p, p);
        _d1 = d1;
        if d1 > d0 {
            break;
        }
        d0 = d1;
        let d = c2D(&s);
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }
        let iA = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        {
            let v = &mut s.verts[s.count as usize];
            v.iA = iA;
            v.sA = sA;
            v.iB = iB;
            v.sB = sB;
            v.p = c2Sub(v.sB, v.sA);
        }
        let mut dup = 0;
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
    let (mut a, mut b) = c2Witness(&s);
    let mut dist = c2Len(c2Sub(a, b));
    if hit != 0 {
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
    if let Some(c) = cache.as_deref_mut() {
        c.metric = c2GJKSimplexMetric(&s);
        c.count = s.count;
        for i in 0..s.count as usize {
            c.iA[i] = s.verts[i].iA;
            c.iB[i] = s.verts[i].iB;
        }
        c.div = s.div;
    }
    if let Some(o) = outA {
        *o = a;
    }
    if let Some(o) = outB {
        *o = b;
    }
    if let Some(it) = iterations {
        *it = iter;
    }
    dist
}

// ---------------------------------------------------------------------------
// Shape vs shape tests
// ---------------------------------------------------------------------------

fn c2AABBtoAABB(A: C2Aabb, B: C2Aabb) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

fn c2AABBtoCapsule(A: C2Aabb, B: C2Capsule) -> c_int {
    if c2GJK(
        &Shape::Aabb(A),
        None,
        &Shape::Capsule(B),
        None,
        None,
        None,
        1,
        None,
        None,
    ) != 0.0
    {
        return 0;
    }
    1
}

fn c2CapsuletoCapsule(A: C2Capsule, B: C2Capsule) -> c_int {
    if c2GJK(
        &Shape::Capsule(A),
        None,
        &Shape::Capsule(B),
        None,
        None,
        None,
        1,
        None,
        None,
    ) != 0.0
    {
        return 0;
    }
    1
}

fn c2CircletoCircle(A: C2Circle, B: C2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

fn c2CircletoAABB(A: C2Circle, B: C2Aabb) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

fn c2CircletoCapsule(A: C2Circle, B: C2Capsule) -> c_int {
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

fn c2Collided(A: &Shape, typeA: c_int, B: &Shape, typeB: c_int) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(as_circle(A), as_circle(B)),
            C2_TYPE_AABB => c2CircletoAABB(as_circle(A), as_aabb(B)),
            C2_TYPE_CAPSULE => c2CircletoCapsule(as_circle(A), as_capsule(B)),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(as_circle(B), as_aabb(A)),
            C2_TYPE_AABB => c2AABBtoAABB(as_aabb(A), as_aabb(B)),
            C2_TYPE_CAPSULE => c2AABBtoCapsule(as_aabb(A), as_capsule(B)),
            _ => 0,
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCapsule(as_circle(B), as_capsule(A)),
            C2_TYPE_AABB => c2AABBtoCapsule(as_aabb(B), as_capsule(A)),
            C2_TYPE_CAPSULE => c2CapsuletoCapsule(as_capsule(A), as_capsule(B)),
            _ => 0,
        },
        _ => 0,
    }
}

// The C code reaches these casts only when the tag matches the stored shape,
// so the `unreachable` arms mirror casts that never happen.
fn as_circle(s: &Shape) -> C2Circle {
    match s {
        Shape::Circle(c) => *c,
        _ => C2Circle::default(),
    }
}

fn as_aabb(s: &Shape) -> C2Aabb {
    match s {
        Shape::Aabb(bb) => *bb,
        _ => C2Aabb::default(),
    }
}

fn as_capsule(s: &Shape) -> C2Capsule {
    match s {
        Shape::Capsule(c) => *c,
        _ => C2Capsule::default(),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Mirrors `ptr_from_parts`. The C version has no `default` case and falls off
/// the end of the function for out-of-range tags; here that yields `None`,
/// which the caller turns into a `0` result (matching `c2Collided`'s default).
fn ptr_from_parts(typ: c_int, a: f32, b: f32, c: f32, d: f32, e: f32) -> Option<Shape> {
    match typ {
        C2_TYPE_CIRCLE => Some(Shape::Circle(C2Circle {
            p: c2V(a, b),
            r: c,
        })),
        C2_TYPE_AABB => Some(Shape::Aabb(C2Aabb {
            min: c2V(a, b),
            max: c2V(c, d),
        })),
        C2_TYPE_CAPSULE => Some(Shape::Capsule(C2Capsule {
            a: c2V(a, b),
            b: c2V(c, d),
            r: e,
        })),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn omni_collide(
    type_a: c_int,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: c_int,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) -> c_int {
    let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);

    match (A, B) {
        (Some(a), Some(b)) => c2Collided(&a, type_a, &b, type_b),
        _ => 0,
    }
}
