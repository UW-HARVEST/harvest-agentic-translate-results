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
    count: i32,
    iA: [i32; 3],
    iB: [i32; 3],
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

#[derive(Copy, Clone)]
struct c2Proxy {
    radius: f32,
    count: i32,
    verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

#[derive(Clone, Copy)]
enum ShapeRef<'a> {
    Circle(&'a c2Circle),
    AABB(&'a c2AABB),
    Capsule(&'a c2Capsule),
}

fn c2MakeProxy(shape: ShapeRef, p: &mut c2Proxy) {
    match shape {
        ShapeRef::Circle(c) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        ShapeRef::AABB(bb) => {
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts, bb);
        }
        ShapeRef::Capsule(c) => {
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
    }
}

#[derive(Copy, Clone, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: i32,
    iB: i32,
}

#[derive(Copy, Clone, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: i32,
}

impl c2Simplex {
    #[inline]
    fn vert(&self, i: usize) -> c2sv {
        match i {
            0 => self.a,
            1 => self.b,
            2 => self.c,
            3 => self.d,
            _ => unreachable!(),
        }
    }
    #[inline]
    fn set_vert(&mut self, i: usize, v: c2sv) {
        match i {
            0 => self.a = v,
            1 => self.b = v,
            2 => self.c = v,
            3 => self.d = v,
            _ => unreachable!(),
        }
    }
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

fn c2Support(verts: &[c2v], count: i32, d: c2v) -> i32 {
    let mut imax: i32 = 0;
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
    a_shape: ShapeRef,
    ax_ptr: Option<&c2x>,
    b_shape: ShapeRef,
    bx_ptr: Option<&c2x>,
    out_a: Option<&mut c2v>,
    out_b: Option<&mut c2v>,
    use_radius: i32,
    iterations: Option<&mut i32>,
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
    c2MakeProxy(a_shape, &mut pA);
    c2MakeProxy(b_shape, &mut pB);
    let mut s = c2Simplex::default();

    let mut cache_was_read = 0;
    if let Some(ref c) = cache {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let iA = c.iA[i];
                let iB = c.iB[i];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let mut v = s.vert(i);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
                s.set_vert(i, v);
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
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, pA.verts[0]);
        s.a.sB = c2Mulxv(bx, pB.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA: [i32; 3] = [0; 3];
    let mut saveB: [i32; 3] = [0; 3];
    let mut save_count = 0i32;
    let mut d0: f32 = 3.402_823_466_385_288_6e+38;
    let mut d1: f32;
    let mut iter = 0i32;
    let mut hit = 0i32;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = s.vert(i);
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
        if c2Dot(d, d) < 1.192_092_9e-7_f32 * 1.192_092_9e-7_f32 {
            break;
        }
        let iA_idx = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA_idx as usize]);
        let iB_idx = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB_idx as usize]);
        let pos = s.count as usize;
        let mut v = s.vert(pos);
        v.iA = iA_idx;
        v.sA = sA;
        v.iB = iB_idx;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
        s.set_vert(pos, v);
        let mut dup = 0i32;
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
        if dist > rA + rB && dist > 1.192_092_9e-7_f32 {
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
    if let Some(c) = cache {
        c.metric = c2GJKSimplexMetric(&s);
        c.count = s.count;
        for i in 0..s.count as usize {
            let v = s.vert(i);
            c.iA[i] = v.iA;
            c.iB[i] = v.iB;
        }
        c.div = s.div;
    }
    if let Some(o) = out_a {
        *o = a;
    }
    if let Some(o) = out_b {
        *o = b;
    }
    if let Some(it) = iterations {
        *it = iter;
    }
    dist
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> i32 {
    let d0 = (B.max.x < A.min.x) as i32;
    let d1 = (A.max.x < B.min.x) as i32;
    let d2 = (B.max.y < A.min.y) as i32;
    let d3 = (A.max.y < B.min.y) as i32;
    (!((d0 | d1 | d2 | d3) != 0)) as i32
}

fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> i32 {
    if c2GJK(
        ShapeRef::AABB(&A),
        None,
        ShapeRef::Capsule(&B),
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

fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> i32 {
    if c2GJK(
        ShapeRef::Capsule(&A),
        None,
        ShapeRef::Capsule(&B),
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

fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> i32 {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as i32
}

fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> i32 {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as i32
}

fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> i32 {
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
    (d2 < r * r) as i32
}

fn c2Collided(a_shape: ShapeRef, b_shape: ShapeRef) -> i32 {
    match a_shape {
        ShapeRef::Circle(a) => match b_shape {
            ShapeRef::Circle(b) => c2CircletoCircle(*a, *b),
            ShapeRef::AABB(b) => c2CircletoAABB(*a, *b),
            ShapeRef::Capsule(b) => c2CircletoCapsule(*a, *b),
        },
        ShapeRef::AABB(a) => match b_shape {
            ShapeRef::Circle(b) => c2CircletoAABB(*b, *a),
            ShapeRef::AABB(b) => c2AABBtoAABB(*a, *b),
            ShapeRef::Capsule(b) => c2AABBtoCapsule(*a, *b),
        },
        ShapeRef::Capsule(a) => match b_shape {
            ShapeRef::Circle(b) => c2CircletoCapsule(*b, *a),
            ShapeRef::AABB(b) => c2AABBtoCapsule(*b, *a),
            ShapeRef::Capsule(b) => c2CapsuletoCapsule(*a, *b),
        },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reverse_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: i32 = 0;

    let circle_in = c2Circle {
        p: c2V(x, y),
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

    let capsule = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    result += c2Collided(ShapeRef::Circle(&circle), ShapeRef::Circle(&circle_in));

    result += c2Collided(ShapeRef::AABB(&aabb), ShapeRef::Circle(&circle_in)) << 1;

    result += c2Collided(ShapeRef::Capsule(&capsule), ShapeRef::Circle(&circle_in)) << 2;

    result as c_int
}
