// Rust translation of c_src/src/lib.c
//
// Preserves the C ABI for `omni_manifold` so the resulting cdylib can be used
// as a drop-in replacement for the original C library.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::too_many_arguments)]

use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Public types (matching include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum C2_TYPE {
    C2_TYPE_CAPSULE = 0,
    C2_TYPE_CIRCLE = 1,
    C2_TYPE_AABB = 2,
    C2_TYPE_POLY = 3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
struct c2h {
    n: c2v,
    d: f32,
}

#[derive(Copy, Clone, Debug)]
struct c2r {
    c: f32,
    s: f32,
}

#[derive(Copy, Clone, Debug)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[derive(Copy, Clone, Debug)]
struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

#[derive(Copy, Clone, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[derive(Copy, Clone, Debug)]
struct c2Proxy {
    radius: f32,
    count: c_int,
    verts: [c2v; 8],
}

#[derive(Copy, Clone, Debug)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

#[derive(Copy, Clone, Debug)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    #[allow(dead_code)]
    d: c2sv,
    div: f32,
    count: c_int,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FLT_MAX: f32 = 3.402_823_466_385_288_6e+38;
const FLT_EPSILON: f32 = 1.192_092_9e-7;

// ---------------------------------------------------------------------------
// Default impls for zero-init usage
// ---------------------------------------------------------------------------

impl Default for c2v {
    fn default() -> Self {
        c2v { x: 0.0, y: 0.0 }
    }
}

impl Default for c2h {
    fn default() -> Self {
        c2h {
            n: c2v::default(),
            d: 0.0,
        }
    }
}

impl Default for c2r {
    fn default() -> Self {
        c2r { c: 1.0, s: 0.0 }
    }
}

impl Default for c2x {
    fn default() -> Self {
        c2x {
            p: c2v::default(),
            r: c2r::default(),
        }
    }
}

impl Default for c2Manifold {
    fn default() -> Self {
        c2Manifold {
            count: 0,
            depths: [0.0; 2],
            contact_points: [c2v::default(); 2],
            n: c2v::default(),
        }
    }
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

impl Default for c2sv {
    fn default() -> Self {
        c2sv {
            sA: c2v::default(),
            sB: c2v::default(),
            p: c2v::default(),
            u: 0.0,
            iA: 0,
            iB: 0,
        }
    }
}

impl Default for c2Simplex {
    fn default() -> Self {
        c2Simplex {
            a: c2sv::default(),
            b: c2sv::default(),
            c: c2sv::default(),
            d: c2sv::default(),
            div: 0.0,
            count: 0,
        }
    }
}

impl Default for c2Poly {
    fn default() -> Self {
        c2Poly {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
}

// ---------------------------------------------------------------------------
// Vector / scalar helpers
// ---------------------------------------------------------------------------

fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
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

fn c2Dist(h: c2h, p: c2v) -> f32 {
    c2Dot(h.n, p) - h.d
}

fn c2PlaneAt(p: &c2Poly, i: usize) -> c2h {
    c2h {
        n: p.norms[i],
        d: c2Dot(p.norms[i], p.verts[i]),
    }
}

fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
enum ShapeRef<'a> {
    Circle(&'a c2Circle),
    Aabb(&'a c2AABB),
    Capsule(&'a c2Capsule),
    Poly(&'a c2Poly),
}

fn c2MakeProxy(shape: ShapeRef<'_>) -> c2Proxy {
    let mut p = c2Proxy::default();
    match shape {
        ShapeRef::Circle(c) => {
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        ShapeRef::Aabb(bb) => {
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts[..4], bb);
        }
        ShapeRef::Capsule(c) => {
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        ShapeRef::Poly(poly) => {
            p.radius = 0.0;
            p.count = poly.count;
            let n = poly.count as usize;
            p.verts[..n].copy_from_slice(&poly.verts[..n]);
        }
    }
    p
}

fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

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

fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

fn c2Intersect(a: c2v, b: c2v, da: f32, db: f32) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

fn c2Clip(seg: &mut [c2v; 2], h: c2h) -> c_int {
    let mut out = [c2v::default(); 2];
    let mut sp: usize = 0;
    let d0 = c2Dist(h, seg[0]);
    let d1 = c2Dist(h, seg[1]);
    if d0 < 0.0 {
        out[sp] = seg[0];
        sp += 1;
    }
    if d1 < 0.0 {
        out[sp] = seg[1];
        sp += 1;
    }
    if d0 == 0.0 && d1 == 0.0 {
        // The C version writes to out[sp++] possibly past the bound — keep
        // behaviour safe by clamping.
        if sp < 2 {
            out[sp] = seg[0];
            sp += 1;
        }
        if sp < 2 {
            out[sp] = seg[1];
            sp += 1;
        }
    } else if d0 * d1 <= 0.0 {
        if sp < 2 {
            out[sp] = c2Intersect(seg[0], seg[1], d0, d1);
            sp += 1;
        }
    }
    seg[0] = out[0];
    seg[1] = out[1];
    sp as c_int
}

fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2SidePlanes(seg: &mut [c2v; 2], ra: c2v, rb: c2v, h: Option<&mut c2h>) -> c_int {
    let inn = c2Norm(c2Sub(rb, ra));
    let left = c2h {
        n: c2Neg(inn),
        d: c2Dot(c2Neg(inn), ra),
    };
    let right = c2h {
        n: inn,
        d: c2Dot(inn, rb),
    };
    if c2Clip(seg, left) < 2 {
        return 0;
    }
    if c2Clip(seg, right) < 2 {
        return 0;
    }
    if let Some(hh) = h {
        hh.n = c2CCW90(inn);
        hh.d = c2Dot(c2CCW90(inn), ra);
    }
    1
}

fn c2SidePlanesFromPoly(
    seg: &mut [c2v; 2],
    x: c2x,
    p: &c2Poly,
    e: usize,
    h: Option<&mut c2h>,
) -> c_int {
    let ra = c2Mulxv(x, p.verts[e]);
    let next = if e + 1 == p.count as usize { 0 } else { e + 1 };
    let rb = c2Mulxv(x, p.verts[next]);
    c2SidePlanes(seg, ra, rb, h)
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

fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
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

fn c2Support(verts: &[c2v], count: usize, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    let mut dmax = c2Dot(verts[0], d);
    for i in 1..count {
        let dot = c2Dot(verts[i], d);
        if dot > dmax {
            imax = i as c_int;
            dmax = dot;
        }
    }
    imax
}

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
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

fn c2L(s: &c2Simplex) -> c2v {
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

// Indexing helper for the simplex's "verts array" (a, b, c, d).
fn simplex_get(s: &c2Simplex, i: usize) -> c2sv {
    match i {
        0 => s.a,
        1 => s.b,
        2 => s.c,
        _ => s.d,
    }
}

fn simplex_set(s: &mut c2Simplex, i: usize, v: c2sv) {
    match i {
        0 => s.a = v,
        1 => s.b = v,
        2 => s.c = v,
        _ => s.d = v,
    }
}

#[allow(clippy::too_many_arguments)]
fn c2GJK(
    A: ShapeRef<'_>,
    ax_ptr: Option<&c2x>,
    B: ShapeRef<'_>,
    bx_ptr: Option<&c2x>,
    outA: Option<&mut c2v>,
    outB: Option<&mut c2v>,
    use_radius: bool,
    iterations: Option<&mut c_int>,
    cache: Option<&mut c2GJKCache>,
) -> f32 {
    let ax = ax_ptr.copied().unwrap_or_else(c2xIdentity);
    let bx = bx_ptr.copied().unwrap_or_else(c2xIdentity);
    let pA = c2MakeProxy(A);
    let pB = c2MakeProxy(B);
    let mut s = c2Simplex::default();
    let mut cache_was_read = false;

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
                simplex_set(&mut s, i, v);
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
        s.a.iA = 0;
        s.a.iB = 0;
        s.a.sA = c2Mulxv(ax, pA.verts[0]);
        s.a.sB = c2Mulxv(bx, pB.verts[0]);
        s.a.p = c2Sub(s.a.sB, s.a.sA);
        s.a.u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA = [0i32; 3];
    let mut saveB = [0i32; 3];
    let mut save_count: usize;
    let mut d0 = FLT_MAX;
    let mut d1;
    let mut iter: c_int = 0;
    let mut hit = false;

    while iter < 20 {
        save_count = s.count as usize;
        for i in 0..save_count {
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
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }
        let iA = c2Support(&pA.verts, pA.count as usize, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count as usize, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);

        let mut v = c2sv::default();
        v.iA = iA;
        v.sA = sA;
        v.iB = iB;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);

        let mut dup = false;
        for i in 0..save_count {
            if iA == saveA[i] && iB == saveB[i] {
                dup = true;
                break;
            }
        }
        if dup {
            break;
        }
        let new_idx = s.count as usize;
        simplex_set(&mut s, new_idx, v);
        s.count += 1;
        iter += 1;
    }

    let mut a = c2v::default();
    let mut b = c2v::default();
    c2Witness(&s, &mut a, &mut b);
    let mut dist = c2Len(c2Sub(a, b));
    if hit {
        a = b;
        dist = 0.0;
    } else if use_radius {
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

    if let Some(c) = cache {
        c.metric = c2GJKSimplexMetric(&s);
        c.count = s.count;
        for i in 0..s.count as usize {
            let v = simplex_get(&s, i);
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
    if let Some(it) = iterations {
        *it = iter;
    }
    dist
}

fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

fn c2CircletoCircleManifold(A: c2Circle, B: c2Circle, m: &mut c2Manifold) {
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

fn c2CircletoAABBManifold(A: c2Circle, B: c2AABB, m: &mut c2Manifold) {
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
            let d_vec = c2Sub(A.p, mid);
            let abs_d = c2Absv(d_vec);
            let x_overlap = e.x - abs_d.x;
            let y_overlap = e.y - abs_d.y;
            let depth;
            let mut n;
            if x_overlap < y_overlap {
                depth = x_overlap;
                n = c2V(1.0, 0.0);
                n = c2Mulvs(n, if d_vec.x < 0.0 { 1.0 } else { -1.0 });
            } else {
                depth = y_overlap;
                n = c2V(0.0, 1.0);
                n = c2Mulvs(n, if d_vec.y < 0.0 { 1.0 } else { -1.0 });
            }
            m.count = 1;
            m.depths[0] = A.r + depth;
            m.contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
            m.n = n;
        }
    }
}

fn c2CircletoCapsuleManifold(A: c2Circle, B: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut a = c2v::default();
    let mut b = c2v::default();
    let r = A.r + B.r;
    let d = c2GJK(
        ShapeRef::Circle(&A),
        None,
        ShapeRef::Capsule(&B),
        None,
        Some(&mut a),
        Some(&mut b),
        false,
        None,
        None,
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

fn c2AABBtoAABBManifold(A: c2AABB, B: c2AABB, m: &mut c2Manifold) {
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

fn c2KeepDeep(seg: &[c2v; 2], h: c2h, m: &mut c2Manifold) {
    let mut cp = 0usize;
    for i in 0..2 {
        let p = seg[i];
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

fn c2Incident(incident: &mut [c2v; 2], ip: &c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    let mut index: usize = 0;
    let mut min_dot = FLT_MAX;
    for i in 0..ip.count as usize {
        let dot = c2Dot(rn_in_incident_space, ip.norms[i]);
        if dot < min_dot {
            min_dot = dot;
            index = i;
        }
    }
    incident[0] = c2Mulxv(ix, ip.verts[index]);
    let next = if index + 1 == ip.count as usize {
        0
    } else {
        index + 1
    };
    incident[1] = c2Mulxv(ix, ip.verts[next]);
}

fn c2CapsuletoPolyManifold(A: c2Capsule, B: &c2Poly, bx_ptr: Option<&c2x>, m: &mut c2Manifold) {
    m.count = 0;
    let mut a = c2v::default();
    let mut b = c2v::default();
    let d = c2GJK(
        ShapeRef::Capsule(&A),
        None,
        ShapeRef::Poly(B),
        bx_ptr,
        Some(&mut a),
        Some(&mut b),
        false,
        None,
        None,
    );
    if d < 1.0e-6 {
        let bx = bx_ptr.copied().unwrap_or_else(c2xIdentity);
        let mut A_in_B = c2Capsule {
            a: c2v::default(),
            b: c2v::default(),
            r: A.r,
        };
        A_in_B.a = c2MulxvT(bx, A.a);
        A_in_B.b = c2MulxvT(bx, A.b);
        let ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));
        let mut ab_h0 = c2h::default();
        ab_h0.n = c2CCW90(ab);
        ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
        let v0 = c2Support(&B.verts, B.count as usize, c2Neg(ab_h0.n));
        let s0 = c2Dist(ab_h0, B.verts[v0 as usize]);
        let mut ab_h1 = c2h::default();
        ab_h1.n = c2Skew(ab);
        ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
        let v1 = c2Support(&B.verts, B.count as usize, c2Neg(ab_h1.n));
        let s1 = c2Dist(ab_h1, B.verts[v1 as usize]);
        let mut index: c_int = -1;
        let mut sep = -FLT_MAX;
        let mut code: i32 = 0;
        for i in 0..B.count as usize {
            let h = c2PlaneAt(B, i);
            let da = c2Dot(A_in_B.a, c2Neg(h.n));
            let db = c2Dot(A_in_B.b, c2Neg(h.n));
            let dd = if da > db {
                c2Dist(h, A_in_B.a)
            } else {
                c2Dist(h, A_in_B.b)
            };
            if dd > sep {
                sep = dd;
                index = i as c_int;
            }
        }
        if s0 > sep {
            sep = s0;
            index = v0;
            code = 1;
        }
        if s1 > sep {
            // sep assignment is dead in original C; mirror anyway
            let _ = sep;
            index = v1;
            code = 2;
        }
        match code {
            0 => {
                let mut seg = [A.a, A.b];
                let mut h = c2h::default();
                if c2SidePlanesFromPoly(&mut seg, bx, B, index as usize, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&seg, h, m);
                m.n = c2Neg(m.n);
            }
            1 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(&mut incident, B, bx, ab_h0.n);
                let mut h = c2h::default();
                if c2SidePlanes(&mut incident, A_in_B.b, A_in_B.a, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&incident, h, m);
            }
            2 => {
                let mut incident = [c2v::default(); 2];
                c2Incident(&mut incident, B, bx, ab_h1.n);
                let mut h = c2h::default();
                if c2SidePlanes(&mut incident, A_in_B.a, A_in_B.b, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&incident, h, m);
            }
            _ => return,
        }
        for i in 0..m.count as usize {
            m.depths[i] += A.r;
        }
    } else if d < A.r {
        m.count = 1;
        m.n = c2Norm(c2Sub(b, a));
        m.contact_points[0] = c2Add(a, c2Mulvs(m.n, A.r));
        m.depths[0] = A.r - d;
    }
}

fn c2Norms(verts: &[c2v], norms: &mut [c2v], count: usize) {
    for i in 0..count {
        let a = i;
        let b = if i + 1 < count { i + 1 } else { 0 };
        let e = c2Sub(verts[b], verts[a]);
        norms[i] = c2Norm(c2CCW90(e));
    }
}

fn c2AABBtoCapsuleManifold(A: c2AABB, B: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut p = c2Poly::default();
    {
        let (verts_part, _) = p.verts.split_at_mut(4);
        c2BBVerts(verts_part, &A);
    }
    p.count = 4;
    // Borrow split: copy verts first, then compute norms.
    let verts_copy = p.verts;
    c2Norms(&verts_copy, &mut p.norms, 4);
    c2CapsuletoPolyManifold(B, &p, None, m);
    m.n = c2Neg(m.n);
}

fn c2CapsuletoCapsuleManifold(A: c2Capsule, B: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut a = c2v::default();
    let mut b = c2v::default();
    let r = A.r + B.r;
    let d = c2GJK(
        ShapeRef::Capsule(&A),
        None,
        ShapeRef::Capsule(&B),
        None,
        Some(&mut a),
        Some(&mut b),
        false,
        None,
        None,
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

#[derive(Copy, Clone, Debug)]
enum AnyShape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

fn shape_from_parts(typ: C2_TYPE, a: f32, b: f32, c: f32, d: f32, e: f32) -> Option<AnyShape> {
    match typ {
        C2_TYPE::C2_TYPE_CIRCLE => Some(AnyShape::Circle(c2Circle {
            p: c2V(a, b),
            r: c,
        })),
        C2_TYPE::C2_TYPE_AABB => Some(AnyShape::Aabb(c2AABB {
            min: c2V(a, b),
            max: c2V(c, d),
        })),
        C2_TYPE::C2_TYPE_CAPSULE => Some(AnyShape::Capsule(c2Capsule {
            a: c2V(a, b),
            b: c2V(c, d),
            r: e,
        })),
        C2_TYPE::C2_TYPE_POLY => None,
    }
}

fn c2Collide(A: &AnyShape, type_a: C2_TYPE, B: &AnyShape, type_b: C2_TYPE, m: &mut c2Manifold) {
    m.count = 0;
    match (type_a, A, type_b, B) {
        (C2_TYPE::C2_TYPE_CIRCLE, AnyShape::Circle(ca), C2_TYPE::C2_TYPE_CIRCLE, AnyShape::Circle(cb)) => {
            c2CircletoCircleManifold(*ca, *cb, m);
        }
        (C2_TYPE::C2_TYPE_CIRCLE, AnyShape::Circle(ca), C2_TYPE::C2_TYPE_AABB, AnyShape::Aabb(bb)) => {
            c2CircletoAABBManifold(*ca, *bb, m);
        }
        (C2_TYPE::C2_TYPE_CIRCLE, AnyShape::Circle(ca), C2_TYPE::C2_TYPE_CAPSULE, AnyShape::Capsule(cap)) => {
            c2CircletoCapsuleManifold(*ca, *cap, m);
        }
        (C2_TYPE::C2_TYPE_AABB, AnyShape::Aabb(aa), C2_TYPE::C2_TYPE_CIRCLE, AnyShape::Circle(cb)) => {
            c2CircletoAABBManifold(*cb, *aa, m);
            m.n = c2Neg(m.n);
        }
        (C2_TYPE::C2_TYPE_AABB, AnyShape::Aabb(aa), C2_TYPE::C2_TYPE_AABB, AnyShape::Aabb(bb)) => {
            c2AABBtoAABBManifold(*aa, *bb, m);
        }
        (C2_TYPE::C2_TYPE_AABB, AnyShape::Aabb(aa), C2_TYPE::C2_TYPE_CAPSULE, AnyShape::Capsule(cap)) => {
            c2AABBtoCapsuleManifold(*aa, *cap, m);
        }
        (C2_TYPE::C2_TYPE_CAPSULE, AnyShape::Capsule(cap), C2_TYPE::C2_TYPE_CIRCLE, AnyShape::Circle(cb)) => {
            c2CircletoCapsuleManifold(*cb, *cap, m);
            m.n = c2Neg(m.n);
        }
        (C2_TYPE::C2_TYPE_CAPSULE, AnyShape::Capsule(cap), C2_TYPE::C2_TYPE_AABB, AnyShape::Aabb(bb)) => {
            c2AABBtoCapsuleManifold(*bb, *cap, m);
            m.n = c2Neg(m.n);
        }
        (C2_TYPE::C2_TYPE_CAPSULE, AnyShape::Capsule(ca), C2_TYPE::C2_TYPE_CAPSULE, AnyShape::Capsule(cb)) => {
            c2CapsuletoCapsuleManifold(*ca, *cb, m);
        }
        _ => {}
    }
}

/// FFI entry point matching the C signature in `include/lib.h`.
///
/// # Safety
///
/// `m` must be a valid, non-null pointer to a `c2Manifold` that the caller
/// owns for the duration of the call.
#[no_mangle]
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
    let manifold = &mut *m;
    let a_shape = match shape_from_parts(type_a, a1, a2, a3, a4, a5) {
        Some(s) => s,
        None => return,
    };
    let b_shape = match shape_from_parts(type_b, b1, b2, b3, b4, b5) {
        Some(s) => s,
        None => return,
    };
    c2Collide(&a_shape, type_a, &b_shape, type_b, manifold);
}
