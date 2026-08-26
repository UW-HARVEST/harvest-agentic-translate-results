use std::os::raw::c_int;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum C2_TYPE {
    C2_TYPE_CAPSULE = 0,
    C2_TYPE_CIRCLE = 1,
    C2_TYPE_AABB = 2,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y })
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y })
}

fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

fn c2xIdentity() -> c2x {
    c2x { p: c2V(0.0, 0.0), r: c2RotIdentity() }
}

#[derive(Copy, Clone)]
struct c2Proxy {
    radius: f32,
    count: i32,
    verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        Self {
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

enum C2Shape<'a> {
    Circle(&'a c2Circle),
    AABB(&'a c2AABB),
    Capsule(&'a c2Capsule),
}

fn c2MakeProxy(shape: C2Shape, p: &mut c2Proxy) {
    match shape {
        C2Shape::Circle(c) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2Shape::AABB(bb) => {
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts, bb);
        }
        C2Shape::Capsule(c) => {
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
    v: [c2sv; 4],
    div: f32,
    count: i32,
}

fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c2GJKSimplexMetric(s: &c2Simplex) -> f32 {
    match s.count {
        1 => 0.0,
        2 => c2Len(c2Sub(s.v[1].p, s.v[0].p)),
        3 => c2Det2(c2Sub(s.v[1].p, s.v[0].p), c2Sub(s.v[2].p, s.v[0].p)),
        _ => 0.0,
    }
}

fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

fn c22(s: &mut c2Simplex) {
    let a = s.v[0].p;
    let b = s.v[1].p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        s.v[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u <= 0.0 {
        s.v[0] = s.v[1];
        s.v[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else {
        s.v[0].u = u;
        s.v[1].u = v;
        s.div = u + v;
        s.count = 2;
    }
}

fn c23(s: &mut c2Simplex) {
    let a = s.v[0].p;
    let b = s.v[1].p;
    let c = s.v[2].p;
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
        s.v[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.v[0] = s.v[1];
        s.v[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.v[0] = s.v[2];
        s.v[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.v[0].u = uAB;
        s.v[1].u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.v[0] = s.v[1];
        s.v[1] = s.v[2];
        s.v[0].u = uBC;
        s.v[1].u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.v[1] = s.v[0];
        s.v[0] = s.v[2];
        s.v[0].u = uCA;
        s.v[1].u = vCA;
        s.div = uCA + vCA;
        s.count = 2;
    } else {
        s.v[0].u = uABC;
        s.v[1].u = vABC;
        s.v[2].u = wABC;
        s.div = uABC + vABC + wABC;
        s.count = 3;
    }
}

fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

fn c2Skew(a: c2v) -> c2v {
    c2V(-a.y, a.x)
}

fn c2CCW90(a: c2v) -> c2v {
    c2V(a.y, -a.x)
}

fn c2D(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2Neg(s.v[0].p),
        2 => {
            let ab = c2Sub(s.v[1].p, s.v[0].p);
            if c2Det2(ab, c2Neg(s.v[0].p)) > 0.0 {
                c2Skew(ab)
            } else {
                c2CCW90(ab)
            }
        }
        _ => c2V(0.0, 0.0),
    }
}

fn c2Support(verts: &[c2v], count: i32, d: c2v) -> i32 {
    let mut imax = 0;
    let mut dmax = c2Dot(verts[0], d);
    for i in 1..count as usize {
        let dot = c2Dot(verts[i], d);
        if dot > dmax {
            imax = i as i32;
            dmax = dot;
        }
    }
    imax
}

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.v[0].sA;
            *b = s.v[0].sB;
        }
        2 => {
            *a = c2Add(c2Mulvs(s.v[0].sA, den * s.v[0].u), c2Mulvs(s.v[1].sA, den * s.v[1].u));
            *b = c2Add(c2Mulvs(s.v[0].sB, den * s.v[0].u), c2Mulvs(s.v[1].sB, den * s.v[1].u));
        }
        3 => {
            *a = c2Add(c2Add(c2Mulvs(s.v[0].sA, den * s.v[0].u), c2Mulvs(s.v[1].sA, den * s.v[1].u)), c2Mulvs(s.v[2].sA, den * s.v[2].u));
            *b = c2Add(c2Add(c2Mulvs(s.v[0].sB, den * s.v[0].u), c2Mulvs(s.v[1].sB, den * s.v[1].u)), c2Mulvs(s.v[2].sB, den * s.v[2].u));
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.v[0].p,
        2 => c2Add(c2Mulvs(s.v[0].p, den * s.v[0].u), c2Mulvs(s.v[1].p, den * s.v[1].u)),
        _ => c2V(0.0, 0.0),
    }
}

fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2GJK(
    shapeA: C2Shape,
    ax_ptr: Option<&c2x>,
    shapeB: C2Shape,
    bx_ptr: Option<&c2x>,
    outA: Option<&mut c2v>,
    outB: Option<&mut c2v>,
    use_radius: i32,
    iterations: Option<&mut i32>,
    mut cache: Option<&mut c2GJKCache>,
) -> f32 {
    let ax = ax_ptr.copied().unwrap_or_else(c2xIdentity);
    let bx = bx_ptr.copied().unwrap_or_else(c2xIdentity);
    
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    c2MakeProxy(shapeA, &mut pA);
    c2MakeProxy(shapeB, &mut pB);
    
    let mut s = c2Simplex::default();
    let mut cache_was_read = false;
    
    if let Some(c) = cache.as_deref_mut() {
        let cache_was_good = c.count != 0;
        if cache_was_good {
            for i in 0..c.count as usize {
                let iA = c.iA[i];
                let iB = c.iB[i];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = &mut s.v[i];
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
                cache_was_read = true;
            }
        }
    }
    
    if !cache_was_read {
        s.v[0].iA = 0;
        s.v[0].iB = 0;
        s.v[0].sA = c2Mulxv(ax, pA.verts[0]);
        s.v[0].sB = c2Mulxv(bx, pB.verts[0]);
        s.v[0].p = c2Sub(s.v[0].sB, s.v[0].sA);
        s.v[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }
    
    let mut saveA = [0; 3];
    let mut saveB = [0; 3];
    let mut save_count = 0;
    let mut d0 = std::f32::MAX;
    let mut d1 = std::f32::MAX;
    let mut iter = 0;
    let mut hit = false;
    
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            saveA[i] = s.v[i].iA;
            saveB[i] = s.v[i].iB;
        }
        
        match s.count {
            1 => {}
            2 => c22(&mut s),
            3 => c23(&mut s),
            _ => {}
        }
        
        if s.count == 3 {
            hit = true;
            break;
        }
        
        let p = c2L(&s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;
        
        let d = c2D(&s);
        if c2Dot(d, d) < std::f32::EPSILON * std::f32::EPSILON {
            break;
        }
        
        let iA = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        
        let v = &mut s.v[s.count as usize];
        v.iA = iA;
        v.sA = sA;
        v.iB = iB;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
        
        let mut dup = false;
        for i in 0..save_count as usize {
            if iA == saveA[i] && iB == saveB[i] {
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
    
    let mut a = c2V(0.0, 0.0);
    let mut b = c2V(0.0, 0.0);
    c2Witness(&s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    
    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius != 0 {
        let rA = pA.radius;
        let rB = pB.radius;
        if dist > rA + rB && dist > std::f32::EPSILON {
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
            c.iA[i] = s.v[i].iA;
            c.iB[i] = s.v[i].iB;
        }
        c.div = s.div;
    }
    
    if let Some(outA) = outA {
        *outA = a;
    }
    if let Some(outB) = outB {
        *outB = b;
    }
    if let Some(iterations) = iterations {
        *iterations = iter;
    }
    
    dist
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> i32 {
    let d0 = B.max.x < A.min.x;
    let d1 = A.max.x < B.min.x;
    let d2 = B.max.y < A.min.y;
    let d3 = A.max.y < B.min.y;
    if !(d0 || d1 || d2 || d3) { 1 } else { 0 }
}

fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> i32 {
    if c2GJK(C2Shape::AABB(&A), None, C2Shape::Capsule(&B), None, None, None, 1, None, None) != 0.0 {
        0
    } else {
        1
    }
}

fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> i32 {
    if c2GJK(C2Shape::Capsule(&A), None, C2Shape::Capsule(&B), None, None, None, 1, None, None) != 0.0 {
        0
    } else {
        1
    }
}

fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> i32 {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    if d2 < r2 { 1 } else { 0 }
}

fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> i32 {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    if d2 < r2 { 1 } else { 0 }
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
    if d2 < r * r { 1 } else { 0 }
}

fn c2Collided(shapeA: C2Shape, shapeB: C2Shape) -> i32 {
    match (shapeA, shapeB) {
        (C2Shape::Circle(a), C2Shape::Circle(b)) => c2CircletoCircle(*a, *b),
        (C2Shape::Circle(a), C2Shape::AABB(b)) => c2CircletoAABB(*a, *b),
        (C2Shape::Circle(a), C2Shape::Capsule(b)) => c2CircletoCapsule(*a, *b),
        
        (C2Shape::AABB(a), C2Shape::Circle(b)) => c2CircletoAABB(*b, *a),
        (C2Shape::AABB(a), C2Shape::AABB(b)) => c2AABBtoAABB(*a, *b),
        (C2Shape::AABB(a), C2Shape::Capsule(b)) => c2AABBtoCapsule(*a, *b),
        
        (C2Shape::Capsule(a), C2Shape::Circle(b)) => c2CircletoCapsule(*b, *a),
        (C2Shape::Capsule(a), C2Shape::AABB(b)) => c2AABBtoCapsule(*b, *a),
        (C2Shape::Capsule(a), C2Shape::Capsule(b)) => c2CapsuletoCapsule(*a, *b),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn omni_collide(
    type_a: C2_TYPE, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
    type_b: C2_TYPE, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32,
) -> c_int {
    let circle_a = c2Circle { p: c2V(a1, a2), r: a3 };
    let aabb_a = c2AABB { min: c2V(a1, a2), max: c2V(a3, a4) };
    let capsule_a = c2Capsule { a: c2V(a1, a2), b: c2V(a3, a4), r: a5 };
    
    let shape_a = match type_a {
        C2_TYPE::C2_TYPE_CIRCLE => C2Shape::Circle(&circle_a),
        C2_TYPE::C2_TYPE_AABB => C2Shape::AABB(&aabb_a),
        C2_TYPE::C2_TYPE_CAPSULE => C2Shape::Capsule(&capsule_a),
    };

    let circle_b = c2Circle { p: c2V(b1, b2), r: b3 };
    let aabb_b = c2AABB { min: c2V(b1, b2), max: c2V(b3, b4) };
    let capsule_b = c2Capsule { a: c2V(b1, b2), b: c2V(b3, b4), r: b5 };
    
    let shape_b = match type_b {
        C2_TYPE::C2_TYPE_CIRCLE => C2Shape::Circle(&circle_b),
        C2_TYPE::C2_TYPE_AABB => C2Shape::AABB(&aabb_b),
        C2_TYPE::C2_TYPE_CAPSULE => C2Shape::Capsule(&capsule_b),
    };

    c2Collided(shape_a, shape_b)
}
