// Translation of c_src/src/lib.c to Rust.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[derive(Copy, Clone, Debug, Default)]
pub enum C2_TYPE {
    #[default]
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
    C2_TYPE_CAPSULE,
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

pub fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

pub fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

pub fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

pub fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

pub fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

pub fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

pub fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
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

#[derive(Copy, Clone, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [c2v; 8],
}

pub fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

pub enum Shape {
    Circle(c2Circle),
    AABB(c2AABB),
    Capsule(c2Capsule),
}

pub fn c2MakeProxy(shape: &Shape, _type: C2_TYPE, p: &mut c2Proxy) {
    match shape {
        Shape::Circle(c) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        Shape::AABB(bb) => {
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

#[derive(Copy, Clone, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: i32,
}

impl c2Simplex {
    fn get(&self, i: usize) -> c2sv {
        match i {
            0 => self.a,
            1 => self.b,
            2 => self.c,
            3 => self.d,
            _ => panic!("simplex index out of bounds"),
        }
    }
    fn set(&mut self, i: usize, v: c2sv) {
        match i {
            0 => self.a = v,
            1 => self.b = v,
            2 => self.c = v,
            3 => self.d = v,
            _ => panic!("simplex index out of bounds"),
        }
    }
}

pub fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

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

pub fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

pub fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

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

pub fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

pub fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

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

pub fn c2Support(verts: &[c2v], count: i32, d: c2v) -> i32 {
    let mut imax: i32 = 0;
    let mut dmax = c2Dot(verts[0], d);
    let mut i: i32 = 1;
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
    let den = 1.0_f32 / s.div;
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

pub fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

pub fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

pub fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0_f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(
            c2Mulvs(s.a.p, den * s.a.u),
            c2Mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

pub fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[allow(clippy::too_many_arguments)]
pub fn c2GJK(
    A: &Shape,
    typeA: C2_TYPE,
    ax_ptr: Option<&c2x>,
    B: &Shape,
    typeB: C2_TYPE,
    bx_ptr: Option<&c2x>,
    outA: Option<&mut c2v>,
    outB: Option<&mut c2v>,
    use_radius: i32,
    iterations_out: Option<&mut i32>,
    cache: Option<&mut c2GJKCache>,
) -> f32 {
    let ax: c2x = match ax_ptr {
        None => c2xIdentity(),
        Some(p) => *p,
    };
    let bx: c2x = match bx_ptr {
        None => c2xIdentity(),
        Some(p) => *p,
    };
    let mut pA: c2Proxy = c2Proxy::default();
    let mut pB: c2Proxy = c2Proxy::default();
    c2MakeProxy(A, typeA, &mut pA);
    c2MakeProxy(B, typeB, &mut pB);
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
                let mut v = c2sv::default();
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
                s.set(i, v);
            }
            s.count = c.count;
            s.div = c.div;
            let metric_old = c.metric;
            let metric = c2GJKSimplexMetric(&s);
            let min_metric = if metric < metric_old {
                metric
            } else {
                metric_old
            };
            let max_metric = if metric > metric_old {
                metric
            } else {
                metric_old
            };
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
    let mut save_count: i32;
    let mut d0: f32 = 3.402_823_466_385_288_6e+38_f32;
    let mut d1: f32;
    let mut iter: i32 = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count as usize {
            let v = s.get(i);
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
        let iA = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);
        let idx = s.count as usize;
        let mut v = c2sv::default();
        v.iA = iA;
        v.sA = sA;
        v.iB = iB;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);
        s.set(idx, v);
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
            let v = s.get(i);
            c.iA[i] = v.iA;
            c.iB[i] = v.iB;
        }
        c.div = s.div;
    }
    if let Some(o) = outA {
        *o = a;
    }
    if let Some(o) = outB {
        *o = b;
    }
    if let Some(o) = iterations_out {
        *o = iter;
    }
    dist
}

#[allow(clippy::too_many_arguments)]
pub fn gjk_cache(
    reverse: i8,
    _a9: &mut c2v,
    _b9: &mut c2v,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    let mut cache = c2GJKCache::default();
    cache.count = 0;

    let A = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 15.0,
    };

    let B = c2Capsule {
        a: c2v { x: 100.0, y: -25.0 },
        b: c2v { x: 75.0, y: 100.0 },
        r: 10.0,
    };

    let mut a0 = c2v::default();
    let mut b0 = c2v::default();
    let mut a = c2v::default();
    let mut b = c2v::default();

    let mut iterations: i32 = -1;
    let mut cached_iterations: i32 = -1;

    let shape_a = Shape::Circle(A);
    let shape_b = Shape::Capsule(B);

    let _d0 = c2GJK(
        &shape_a,
        C2_TYPE::C2_TYPE_CIRCLE,
        None,
        &shape_b,
        C2_TYPE::C2_TYPE_CAPSULE,
        None,
        Some(&mut a0),
        Some(&mut b0),
        1,
        Some(&mut iterations),
        Some(&mut cache),
    );
    let _d1 = c2GJK(
        &shape_a,
        C2_TYPE::C2_TYPE_CIRCLE,
        None,
        &shape_b,
        C2_TYPE::C2_TYPE_CAPSULE,
        None,
        Some(&mut a),
        Some(&mut b),
        1,
        Some(&mut cached_iterations),
        Some(&mut cache),
    );

    let mut bb = c2AABB::default();
    bb.min = c2V(a1, a2);
    bb.max = c2V(a3, a4);

    let mut cap = c2Capsule::default();
    cap.a = c2V(b1, b2);
    cap.b = c2V(b3, b4);
    cap.r = b5;

    let cap_shape = Shape::Capsule(cap);
    let bb_shape = Shape::AABB(bb);

    if reverse != 0 {
        c2GJK(
            &cap_shape,
            C2_TYPE::C2_TYPE_CAPSULE,
            None,
            &bb_shape,
            C2_TYPE::C2_TYPE_AABB,
            None,
            Some(&mut a),
            Some(&mut b),
            1,
            None,
            None,
        );
    } else {
        c2GJK(
            &bb_shape,
            C2_TYPE::C2_TYPE_AABB,
            None,
            &cap_shape,
            C2_TYPE::C2_TYPE_CAPSULE,
            None,
            Some(&mut a),
            Some(&mut b),
            1,
            None,
            None,
        );
    }
}
