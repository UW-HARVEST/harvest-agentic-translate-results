#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::c_int;
use std::os::raw::c_void;

// C2_TYPE enum
pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;

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

fn c2RotIdentity() -> c2r {
    c2r { c: 1.0f32, s: 0.0 }
}

fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[derive(Copy, Clone)]
struct c2Proxy {
    radius: f32,
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

fn c2BBVerts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

unsafe fn c2MakeProxy(shape: *const c_void, type_: c_int, p: &mut c2Proxy) {
    match type_ {
        x if x == C2_TYPE_CIRCLE => {
            let c = unsafe { &*(shape as *const c2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        x if x == C2_TYPE_AABB => {
            let bb = unsafe { &*(shape as *const c2AABB) };
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(&mut p.verts, bb);
        }
        x if x == C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {}
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

fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

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

fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

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

fn c2Witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0f32 / s.div;
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

fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn c2L(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a.p,
        2 => c2Add(
            c2Mulvs(s.a.p, den * s.a.u),
            c2Mulvs(s.b.p, den * s.b.u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

// Helper: get sv at index from simplex (verts = &s.a)
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

#[allow(clippy::too_many_arguments)]
unsafe fn c2GJK(
    a_shape: *const c_void,
    typeA: c_int,
    ax_ptr: *const c2x,
    b_shape: *const c_void,
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    let ax: c2x = if ax_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *ax_ptr }
    };
    let bx: c2x = if bx_ptr.is_null() {
        c2xIdentity()
    } else {
        unsafe { *bx_ptr }
    };

    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    unsafe {
        c2MakeProxy(a_shape, typeA, &mut pA);
        c2MakeProxy(b_shape, typeB, &mut pB);
    }

    let mut s = c2Simplex::default();
    let mut cache_was_read = 0;

    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            for i in 0..cache_ref.count as usize {
                let iA = cache_ref.iA[i];
                let iB = cache_ref.iB[i];
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
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
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
    let mut d0: f32 = 3.40282346638528859811704183484516925e+38f32;
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
        if c2Dot(d, d)
            < 1.19209289550781250000000000000000000e-7f32
                * 1.19209289550781250000000000000000000e-7f32
        {
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
        let idx = s.count as usize;
        simplex_set(&mut s, idx, v);
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
        if dist > rA + rB && dist > 1.19209289550781250000000000000000000e-7f32 {
            dist -= rA + rB;
            let n = c2Norm(c2Sub(b, a));
            a = c2Add(a, c2Mulvs(n, rA));
            b = c2Sub(b, c2Mulvs(n, rB));
            if a.x == b.x && a.y == b.y {
                dist = 0.0;
            }
        } else {
            let p = c2Mulvs(c2Add(a, b), 0.5f32);
            a = p;
            b = p;
            dist = 0.0;
        }
    }

    if !cache.is_null() {
        let cache_mut = unsafe { &mut *cache };
        cache_mut.metric = c2GJKSimplexMetric(&s);
        cache_mut.count = s.count;
        for i in 0..s.count as usize {
            let v = simplex_get(&s, i);
            cache_mut.iA[i] = v.iA;
            cache_mut.iB[i] = v.iB;
        }
        cache_mut.div = s.div;
    }
    if !outA.is_null() {
        unsafe { *outA = a };
    }
    if !outB.is_null() {
        unsafe { *outB = b };
    }
    if !iterations.is_null() {
        unsafe { *iterations = iter };
    }
    dist
}

fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    (!((d0 | d1 | d2 | d3) != 0)) as c_int
}

fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let dist = unsafe {
        c2GJK(
            &A as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if dist != 0.0 {
        0
    } else {
        1
    }
}

fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let dist = unsafe {
        c2GJK(
            &A as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
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

unsafe fn c2Collided(
    a_shape: *const c_void,
    typeA: c_int,
    b_shape: *const c_void,
    typeB: c_int,
) -> c_int {
    match typeA {
        x if x == C2_TYPE_CIRCLE => match typeB {
            y if y == C2_TYPE_CIRCLE => unsafe {
                c2CircletoCircle(
                    *(a_shape as *const c2Circle),
                    *(b_shape as *const c2Circle),
                )
            },
            y if y == C2_TYPE_AABB => unsafe {
                c2CircletoAABB(
                    *(a_shape as *const c2Circle),
                    *(b_shape as *const c2AABB),
                )
            },
            y if y == C2_TYPE_CAPSULE => unsafe {
                c2CircletoCapsule(
                    *(a_shape as *const c2Circle),
                    *(b_shape as *const c2Capsule),
                )
            },
            _ => 0,
        },
        x if x == C2_TYPE_AABB => match typeB {
            y if y == C2_TYPE_CIRCLE => unsafe {
                c2CircletoAABB(
                    *(b_shape as *const c2Circle),
                    *(a_shape as *const c2AABB),
                )
            },
            y if y == C2_TYPE_AABB => unsafe {
                c2AABBtoAABB(*(a_shape as *const c2AABB), *(b_shape as *const c2AABB))
            },
            y if y == C2_TYPE_CAPSULE => unsafe {
                c2AABBtoCapsule(
                    *(a_shape as *const c2AABB),
                    *(b_shape as *const c2Capsule),
                )
            },
            _ => 0,
        },
        x if x == C2_TYPE_CAPSULE => match typeB {
            y if y == C2_TYPE_CIRCLE => unsafe {
                c2CircletoCapsule(
                    *(b_shape as *const c2Circle),
                    *(a_shape as *const c2Capsule),
                )
            },
            y if y == C2_TYPE_AABB => unsafe {
                c2AABBtoCapsule(
                    *(b_shape as *const c2AABB),
                    *(a_shape as *const c2Capsule),
                )
            },
            y if y == C2_TYPE_CAPSULE => unsafe {
                c2CapsuletoCapsule(
                    *(a_shape as *const c2Capsule),
                    *(b_shape as *const c2Capsule),
                )
            },
            _ => 0,
        },
        _ => 0,
    }
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

unsafe fn ptr_from_parts(typ: c_int, a: f32, b: f32, c: f32, d: f32, e: f32) -> *mut c_void {
    match typ {
        x if x == C2_TYPE_CIRCLE => {
            let circle = unsafe { malloc(std::mem::size_of::<c2Circle>()) as *mut c2Circle };
            unsafe {
                (*circle).p = c2V(a, b);
                (*circle).r = c;
            }
            circle as *mut c_void
        }
        x if x == C2_TYPE_AABB => {
            let aabb = unsafe { malloc(std::mem::size_of::<c2AABB>()) as *mut c2AABB };
            unsafe {
                (*aabb).min = c2V(a, b);
                (*aabb).max = c2V(c, d);
            }
            aabb as *mut c_void
        }
        x if x == C2_TYPE_CAPSULE => {
            let capsule = unsafe { malloc(std::mem::size_of::<c2Capsule>()) as *mut c2Capsule };
            unsafe {
                (*capsule).a = c2V(a, b);
                (*capsule).b = c2V(c, d);
                (*capsule).r = e;
            }
            capsule as *mut c_void
        }
        _ => {
            // Original C has UB here - returns garbage. We return null, but
            // since the enum only has 3 values this branch is never reached.
            std::ptr::null_mut()
        }
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
    unsafe {
        let a_ptr = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
        let b_ptr = ptr_from_parts(type_b, b1, b2, b3, b4, b5);
        c2Collided(a_ptr, type_a, b_ptr, type_b)
    }
}
