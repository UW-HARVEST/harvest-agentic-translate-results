#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_float, c_int};

extern "C" {
    fn sqrtf(x: c_float) -> c_float;
    fn malloc(size: usize) -> *mut c_void;
}

// C2_TYPE enum (must be `c_int` width to match the C enum's default storage)
pub type C2_TYPE = c_int;
pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_POLY: C2_TYPE = 3;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [c_float; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2h {
    n: c2v,
    d: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
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

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2GJKCache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct c2Proxy {
    radius: c_float,
    count: c_int,
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

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: c_float,
    count: c_int,
}

#[inline]
fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

#[inline]
fn c2Mulvs(mut a: c2v, b: c_float) -> c2v {
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
fn c2Dot(a: c2v, b: c2v) -> c_float {
    a.x * b.x + a.y * b.y
}

#[inline]
fn c2Dist(h: c2h, p: c2v) -> c_float {
    c2Dot(h.n, p) - h.d
}

#[inline]
fn c2PlaneAt(p: &c2Poly, i: c_int) -> c2h {
    let idx = i as usize;
    c2h {
        n: p.norms[idx],
        d: c2Dot(p.norms[idx], p.verts[idx]),
    }
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

fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

unsafe fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: &mut c2Proxy) {
    match type_ {
        x if x == C2_TYPE_CIRCLE => {
            let c = &*(shape as *const c2Circle);
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        x if x == C2_TYPE_AABB => {
            let bb = &*(shape as *const c2AABB);
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts, bb);
        }
        x if x == C2_TYPE_CAPSULE => {
            let c = &*(shape as *const c2Capsule);
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {
            // No case for C2_TYPE_POLY in original C; fall through (uninitialized in C).
            // We leave p unchanged (caller may have provided default).
        }
    }
}

#[inline]
fn c2Len(a: c2v) -> c_float {
    unsafe { sqrtf(c2Dot(a, a)) }
}

#[inline]
fn c2Det2(a: c2v, b: c2v) -> c_float {
    a.x * b.y - a.y * b.x
}

fn c2GJKSimplexMetric(s: &c2Simplex) -> c_float {
    match s.count {
        2 => c2Len(c2Sub(s.b.p, s.a.p)),
        3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
        // default and case 1
        _ => 0.0,
    }
}

#[inline]
fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[inline]
fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
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

#[inline]
fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[inline]
fn c2Intersect(a: c2v, b: c2v, da: c_float, db: c_float) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

fn c2Clip(seg: &mut [c2v; 2], h: c2h) -> c_int {
    let mut out: [c2v; 2] = [c2v::default(); 2];
    let mut sp: usize = 0;
    let d0 = c2Dist(h, seg[0]);
    if d0 < 0.0 {
        out[sp] = seg[0];
        sp += 1;
    }
    let d1 = c2Dist(h, seg[1]);
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
        out[sp] = c2Intersect(seg[0], seg[1], d0, d1);
        sp += 1;
    }
    seg[0] = out[0];
    seg[1] = out[1];
    sp as c_int
}

#[inline]
fn c2Div(a: c2v, b: c_float) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[inline]
fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[inline]
fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[inline]
fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

fn c2SidePlanes(seg: &mut [c2v; 2], ra: c2v, rb: c2v, h: Option<&mut c2h>) -> c_int {
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
    if let Some(hh) = h {
        hh.n = c2CCW90(in_);
        hh.d = c2Dot(c2CCW90(in_), ra);
    }
    1
}

fn c2SidePlanesFromPoly(
    seg: &mut [c2v; 2],
    x: c2x,
    p: &c2Poly,
    e: c_int,
    h: Option<&mut c2h>,
) -> c_int {
    let ra = c2Mulxv(x, p.verts[e as usize]);
    let next_idx = if e + 1 == p.count { 0 } else { e + 1 };
    let rb = c2Mulxv(x, p.verts[next_idx as usize]);
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

#[inline]
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
        // case 3 and default
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

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(c2Mulvs(s.a.p, den * s.a.u), c2Mulvs(s.b.p, den * s.b.u)),
        _ => c2V(0.0, 0.0),
    }
}

const FLT_MAX: c_float = 3.402_823_466_385_288_6e+38;
const FLT_EPSILON: c_float = 1.192_092_895_507_812_5e-7;

#[allow(clippy::too_many_arguments)]
unsafe fn c2GJK(
    a_shape: *const c_void,
    type_a: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const c_void,
    type_b: C2_TYPE,
    bx_ptr: *const c2x,
    out_a: *mut c2v,
    out_b: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> c_float {
    let ax: c2x = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        *ax_ptr
    };
    let bx: c2x = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        *bx_ptr
    };

    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    c2MakeProxy(a_shape, type_a, &mut pA);
    c2MakeProxy(b_shape, type_b, &mut pB);

    let mut s = c2Simplex::default();

    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_was_good = (*cache).count != 0;
        if cache_was_good {
            for i in 0..(*cache).count as usize {
                let iA = (*cache).iA[i];
                let iB = (*cache).iB[i];
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
            s.count = (*cache).count;
            s.div = (*cache).div;
            let metric_old = (*cache).metric;
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
    let mut save_count: c_int;
    let mut d0: c_float = FLT_MAX;
    let mut d1: c_float;
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
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }
        let iA = c2Support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = c2Support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);

        let mut v = c2sv::default();
        v.iA = iA;
        v.sA = sA;
        v.iB = iB;
        v.sB = sB;
        v.p = c2Sub(v.sB, v.sA);

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
        let new_idx = s.count as usize;
        simplex_set(&mut s, new_idx, v);
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
    if !cache.is_null() {
        (*cache).metric = c2GJKSimplexMetric(&s);
        (*cache).count = s.count;
        for i in 0..s.count as usize {
            let v = simplex_get(&s, i);
            (*cache).iA[i] = v.iA;
            (*cache).iB[i] = v.iB;
        }
        (*cache).div = s.div;
    }
    if !out_a.is_null() {
        *out_a = a;
    }
    if !out_b.is_null() {
        *out_b = b;
    }
    if !iterations.is_null() {
        *iterations = iter;
    }
    dist
}

#[inline]
fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

fn c2CircletoCircleManifold(a: c2Circle, b: c2Circle, m: &mut c2Manifold) {
    m.count = 0;
    let d = c2Sub(b.p, a.p);
    let d2 = c2Dot(d, d);
    let r = a.r + b.r;
    if d2 < r * r {
        let l = unsafe { sqrtf(d2) };
        let n = if l != 0.0 {
            c2Mulvs(d, 1.0 / l)
        } else {
            c2V(0.0, 1.0)
        };
        m.count = 1;
        m.depths[0] = r - l;
        m.contact_points[0] = c2Sub(b.p, c2Mulvs(n, b.r));
        m.n = n;
    }
}

fn c2CircletoAABBManifold(a: c2Circle, b: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(l, a.p);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    if d2 < r2 {
        if d2 != 0.0 {
            let d = unsafe { sqrtf(d2) };
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

fn c2CircletoCapsuleManifold(a: c2Circle, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut av = c2v::default();
    let mut bv = c2v::default();
    let r = a.r + b.r;
    let d = unsafe {
        c2GJK(
            &a as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut av,
            &mut bv,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if d < r {
        let n;
        if d == 0.0 {
            n = c2Norm(c2Skew(c2Sub(b.b, b.a)));
        } else {
            n = c2Norm(c2Sub(bv, av));
        }
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(bv, c2Mulvs(n, b.r));
        m.n = n;
    }
}

fn c2AABBtoAABBManifold(a: c2AABB, b: c2AABB, m: &mut c2Manifold) {
    m.count = 0;
    let mid_a = c2Mulvs(c2Add(a.min, a.max), 0.5);
    let mid_b = c2Mulvs(c2Add(b.min, b.max), 0.5);
    let eA = c2Absv(c2Mulvs(c2Sub(a.max, a.min), 0.5));
    let eB = c2Absv(c2Mulvs(c2Sub(b.max, b.min), 0.5));
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
    let mut cp: usize = 0;
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
    // index = ~0 in C is -1 (for int)
    let mut index: c_int = !0;
    let mut min_dot = FLT_MAX;
    for i in 0..ip.count {
        let dot = c2Dot(rn_in_incident_space, ip.norms[i as usize]);
        if dot < min_dot {
            min_dot = dot;
            index = i;
        }
    }
    incident[0] = c2Mulxv(ix, ip.verts[index as usize]);
    let next_idx = if index + 1 == ip.count { 0 } else { index + 1 };
    incident[1] = c2Mulxv(ix, ip.verts[next_idx as usize]);
}

fn c2CapsuletoPolyManifold(a: c2Capsule, b: &c2Poly, bx_ptr: *const c2x, m: &mut c2Manifold) {
    m.count = 0;
    let mut av = c2v::default();
    let mut bv = c2v::default();
    let d = unsafe {
        c2GJK(
            &a as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            b as *const c2Poly as *const c_void,
            C2_TYPE_POLY,
            bx_ptr,
            &mut av,
            &mut bv,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if d < 1.0e-6 {
        let bx: c2x = if bx_ptr.is_null() {
            c2xIdentity()
        } else {
            unsafe { *bx_ptr }
        };
        let mut a_in_b = c2Capsule::default();
        a_in_b.a = c2MulxvT(bx, a.a);
        a_in_b.b = c2MulxvT(bx, a.b);
        let ab = c2Norm(c2Sub(a_in_b.a, a_in_b.b));
        let mut ab_h0 = c2h::default();
        ab_h0.n = c2CCW90(ab);
        ab_h0.d = c2Dot(a_in_b.a, ab_h0.n);
        let v0 = c2Support(&b.verts, b.count, c2Neg(ab_h0.n));
        let s0 = c2Dist(ab_h0, b.verts[v0 as usize]);
        let mut ab_h1 = c2h::default();
        ab_h1.n = c2Skew(ab);
        ab_h1.d = c2Dot(a_in_b.a, ab_h1.n);
        let v1 = c2Support(&b.verts, b.count, c2Neg(ab_h1.n));
        let s1 = c2Dist(ab_h1, b.verts[v1 as usize]);
        let mut index: c_int = !0;
        let mut sep: c_float = -FLT_MAX;
        let mut code: c_int = 0;
        for i in 0..b.count {
            let h = c2PlaneAt(b, i);
            let da = c2Dot(a_in_b.a, c2Neg(h.n));
            let db = c2Dot(a_in_b.b, c2Neg(h.n));
            let d_;
            if da > db {
                d_ = c2Dist(h, a_in_b.a);
            } else {
                d_ = c2Dist(h, a_in_b.b);
            }
            if d_ > sep {
                sep = d_;
                index = i;
            }
        }
        if s0 > sep {
            sep = s0;
            index = v0;
            code = 1;
        }
        if s1 > sep {
            sep = s1;
            let _ = sep; // suppress unused warning; matches C structure
            index = v1;
            code = 2;
        }
        match code {
            0 => {
                let mut seg: [c2v; 2] = [a.a, a.b];
                let mut h = c2h::default();
                if c2SidePlanesFromPoly(&mut seg, bx, b, index, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&seg, h, m);
                m.n = c2Neg(m.n);
            }
            1 => {
                let mut incident: [c2v; 2] = [c2v::default(); 2];
                c2Incident(&mut incident, b, bx, ab_h0.n);
                let mut h = c2h::default();
                if c2SidePlanes(&mut incident, a_in_b.b, a_in_b.a, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&incident, h, m);
            }
            2 => {
                let mut incident: [c2v; 2] = [c2v::default(); 2];
                c2Incident(&mut incident, b, bx, ab_h1.n);
                let mut h = c2h::default();
                if c2SidePlanes(&mut incident, a_in_b.a, a_in_b.b, Some(&mut h)) == 0 {
                    return;
                }
                c2KeepDeep(&incident, h, m);
            }
            _ => {
                return;
            }
        }
        for i in 0..m.count as usize {
            m.depths[i] += a.r;
        }
    } else if d < a.r {
        m.count = 1;
        m.n = c2Norm(c2Sub(bv, av));
        m.contact_points[0] = c2Add(av, c2Mulvs(m.n, a.r));
        m.depths[0] = a.r - d;
    }
}

fn c2Norms(verts: &[c2v], norms: &mut [c2v], count: c_int) {
    for i in 0..count {
        let aa = i;
        let bb = if i + 1 < count { i + 1 } else { 0 };
        let e = c2Sub(verts[bb as usize], verts[aa as usize]);
        norms[i as usize] = c2Norm(c2CCW90(e));
    }
}

fn c2AABBtoCapsuleManifold(a: c2AABB, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut p = c2Poly::default();
    {
        let (verts_slice, _) = p.verts.split_at_mut(4);
        c2BBVerts(verts_slice, &a);
    }
    p.count = 4;
    // Build norms from verts
    let verts_copy = p.verts;
    c2Norms(&verts_copy, &mut p.norms, 4);
    c2CapsuletoPolyManifold(b, &p, std::ptr::null(), m);
    m.n = c2Neg(m.n);
}

fn c2CapsuletoCapsuleManifold(a: c2Capsule, b: c2Capsule, m: &mut c2Manifold) {
    m.count = 0;
    let mut av = c2v::default();
    let mut bv = c2v::default();
    let r = a.r + b.r;
    let d = unsafe {
        c2GJK(
            &a as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut av,
            &mut bv,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if d < r {
        let n;
        if d == 0.0 {
            n = c2Norm(c2Skew(c2Sub(a.b, a.a)));
        } else {
            n = c2Norm(c2Sub(bv, av));
        }
        m.count = 1;
        m.depths[0] = r - d;
        m.contact_points[0] = c2Sub(bv, c2Mulvs(n, b.r));
        m.n = n;
    }
}

unsafe fn c2Collide(
    a: *const c_void,
    type_a: C2_TYPE,
    b: *const c_void,
    type_b: C2_TYPE,
    m: &mut c2Manifold,
) {
    m.count = 0;
    match type_a {
        x if x == C2_TYPE_CIRCLE => match type_b {
            y if y == C2_TYPE_CIRCLE => {
                c2CircletoCircleManifold(*(a as *const c2Circle), *(b as *const c2Circle), m);
            }
            y if y == C2_TYPE_AABB => {
                c2CircletoAABBManifold(*(a as *const c2Circle), *(b as *const c2AABB), m);
            }
            y if y == C2_TYPE_CAPSULE => {
                c2CircletoCapsuleManifold(*(a as *const c2Circle), *(b as *const c2Capsule), m);
            }
            _ => {}
        },
        x if x == C2_TYPE_AABB => match type_b {
            y if y == C2_TYPE_CIRCLE => {
                c2CircletoAABBManifold(*(b as *const c2Circle), *(a as *const c2AABB), m);
                m.n = c2Neg(m.n);
            }
            y if y == C2_TYPE_AABB => {
                c2AABBtoAABBManifold(*(a as *const c2AABB), *(b as *const c2AABB), m);
            }
            y if y == C2_TYPE_CAPSULE => {
                c2AABBtoCapsuleManifold(*(a as *const c2AABB), *(b as *const c2Capsule), m);
            }
            _ => {}
        },
        x if x == C2_TYPE_CAPSULE => match type_b {
            y if y == C2_TYPE_CIRCLE => {
                c2CircletoCapsuleManifold(*(b as *const c2Circle), *(a as *const c2Capsule), m);
                m.n = c2Neg(m.n);
            }
            y if y == C2_TYPE_AABB => {
                c2AABBtoCapsuleManifold(*(b as *const c2AABB), *(a as *const c2Capsule), m);
                m.n = c2Neg(m.n);
            }
            y if y == C2_TYPE_CAPSULE => {
                c2CapsuletoCapsuleManifold(
                    *(a as *const c2Capsule),
                    *(b as *const c2Capsule),
                    m,
                );
            }
            _ => {}
        },
        _ => {}
    }
}

unsafe fn ptr_from_parts(
    typ: C2_TYPE,
    a: c_float,
    b: c_float,
    c: c_float,
    d: c_float,
    e: c_float,
) -> *mut c_void {
    match typ {
        x if x == C2_TYPE_CIRCLE => {
            let circle = malloc(std::mem::size_of::<c2Circle>()) as *mut c2Circle;
            (*circle).p = c2V(a, b);
            (*circle).r = c;
            circle as *mut c_void
        }
        x if x == C2_TYPE_AABB => {
            let aabb = malloc(std::mem::size_of::<c2AABB>()) as *mut c2AABB;
            (*aabb).min = c2V(a, b);
            (*aabb).max = c2V(c, d);
            aabb as *mut c_void
        }
        x if x == C2_TYPE_CAPSULE => {
            let capsule = malloc(std::mem::size_of::<c2Capsule>()) as *mut c2Capsule;
            (*capsule).a = c2V(a, b);
            (*capsule).b = c2V(c, d);
            (*capsule).r = e;
            capsule as *mut c_void
        }
        _ => {
            // Original C falls through with no return -> undefined behavior.
            // Return null pointer to avoid Rust UB; matches typical behavior in
            // unreached cases.
            std::ptr::null_mut()
        }
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
    let a_ptr = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
    let b_ptr = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
    let m_ref = &mut *m;
    c2Collide(a_ptr, type_a, b_ptr, type_b, m_ref);
}
