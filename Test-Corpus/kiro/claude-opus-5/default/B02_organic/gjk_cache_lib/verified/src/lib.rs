//! Direct translation of c_src/src/lib.c (tinyc2 / cute_c2 GJK subset).
//!
//! Every arithmetic operation is kept in the same order and in `f32` so that
//! results are bit-identical to the C original. Comparisons that the C code
//! writes as ternaries (`a > b ? a : b`) are reproduced with explicit
//! comparisons rather than `f32::max`/`f32::min`, because those differ for NaN.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

// The float literals used verbatim by the C source.
const FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e38;
const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// types
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// Same layout as the C `c2Simplex` (`c2sv a, b, c, d; float div; int count;`).
/// The C code takes `c2sv *verts = &s.a` and indexes it, so the four vertices
/// are modelled as an array here.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// vector helpers
// ---------------------------------------------------------------------------

#[inline]
fn v_new(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    v_new(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    v_new(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: v_new(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    v_new(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    v_new(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    v_new(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

// ---------------------------------------------------------------------------
// proxies
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    let bb = &*bb;
    let out = std::slice::from_raw_parts_mut(out, 4);
    out[0] = bb.min;
    out[1] = v_new(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = v_new(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, r#type: c_int, p: *mut c2Proxy) {
    let p = &mut *p;
    match r#type {
        C2_TYPE_CIRCLE => {
            let c = &*(shape as *const c2Circle);
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE_AABB => {
            let bb = shape as *mut c2AABB;
            p.radius = 0.0;
            p.count = 4;
            c2BBVerts(p.verts.as_mut_ptr(), bb);
        }
        C2_TYPE_CAPSULE => {
            let c = &*(shape as *const c2Capsule);
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        // The C switch has no default case: nothing is written.
        _ => {}
    }
}

fn make_proxy(shape: *const c_void, r#type: c_int) -> c2Proxy {
    let mut p = c2Proxy::default();
    unsafe { c2MakeProxy(shape, r#type, &mut p) };
    p
}

// ---------------------------------------------------------------------------
// simplex
// ---------------------------------------------------------------------------

fn gjk_simplex_metric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.verts[1].p, s.verts[0].p)),
        3 => c2Det2(
            c2Sub(s.verts[1].p, s.verts[0].p),
            c2Sub(s.verts[2].p, s.verts[0].p),
        ),
        // `default` and `case 1` both return 0 in the C source.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    gjk_simplex_metric(&*s)
}

fn simplex2(s: &mut c2Simplex) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    simplex2(&mut *s);
}

fn simplex3(s: &mut c2Simplex) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    simplex3(&mut *s);
}

fn direction(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2Neg(s.verts[0].p),
        2 => {
            let ab = c2Sub(s.verts[1].p, s.verts[0].p);
            if c2Det2(ab, c2Neg(s.verts[0].p)) > 0.0 {
                c2Skew(ab)
            } else {
                c2CCW90(ab)
            }
        }
        _ => v_new(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    direction(&*s)
}

fn support(verts: &[c2v], count: c_int, d: c2v) -> c_int {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let len = if count > 1 { count as usize } else { 1 };
    support(std::slice::from_raw_parts(verts, len), count, d)
}

fn witness(s: &c2Simplex) -> (c2v, c2v) {
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
        _ => (v_new(0.0, 0.0), v_new(0.0, 0.0)),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let (wa, wb) = witness(&*s);
    *a = wa;
    *b = wb;
}

fn lambda(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.verts[0].p,
        2 => c2Add(
            c2Mulvs(s.verts[0].p, den * s.verts[0].u),
            c2Mulvs(s.verts[1].p, den * s.verts[1].u),
        ),
        _ => v_new(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    lambda(&*s)
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
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

    let pA = make_proxy(A, typeA);
    let pB = make_proxy(B, typeB);

    let mut s = c2Simplex::default();

    let mut cache_was_read = false;
    if !cache.is_null() {
        let cache_was_good = (*cache).count != 0;
        if cache_was_good {
            for i in 0..(*cache).count {
                let iA = (*cache).iA[i as usize];
                let iB = (*cache).iB[i as usize];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = &mut s.verts[i as usize];
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
            }
            s.count = (*cache).count;
            s.div = (*cache).div;
            let metric_old = (*cache).metric;
            let metric = gjk_simplex_metric(&s);
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
                cache_was_read = true;
            }
        }
    }

    if !cache_was_read {
        s.verts[0].iA = 0;
        s.verts[0].iB = 0;
        s.verts[0].sA = c2Mulxv(ax, pA.verts[0]);
        s.verts[0].sB = c2Mulxv(bx, pB.verts[0]);
        s.verts[0].p = c2Sub(s.verts[0].sB, s.verts[0].sA);
        s.verts[0].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA = [0 as c_int; 3];
    let mut saveB = [0 as c_int; 3];
    let mut save_count: c_int;
    let mut d0 = FLT_MAX;
    let mut d1;
    let mut iter: c_int = 0;
    let mut hit = false;

    while iter < 20 {
        save_count = s.count;
        for i in 0..save_count {
            saveA[i as usize] = s.verts[i as usize].iA;
            saveB[i as usize] = s.verts[i as usize].iB;
        }

        match s.count {
            1 => {}
            2 => simplex2(&mut s),
            3 => simplex3(&mut s),
            _ => {}
        }

        if s.count == 3 {
            hit = true;
            break;
        }

        let p = lambda(&s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = direction(&s);
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }

        let iA = support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, pB.verts[iB as usize]);

        {
            let v = &mut s.verts[s.count as usize];
            v.iA = iA;
            v.sA = sA;
            v.iB = iB;
            v.sB = sB;
            v.p = c2Sub(v.sB, v.sA);
        }

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

    let (mut a, mut b) = witness(&s);
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
        (*cache).metric = gjk_simplex_metric(&s);
        (*cache).count = s.count;
        for i in 0..s.count {
            let v = &s.verts[i as usize];
            (*cache).iA[i as usize] = v.iA;
            (*cache).iB[i as usize] = v.iB;
        }
        (*cache).div = s.div;
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

// ---------------------------------------------------------------------------
// public entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gjk_cache(
    reverse: c_char,
    _a9: *mut c2v,
    _b9: *mut c2v,
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
    // Matches the C original: `cache` is uninitialised apart from `count`.
    let mut cache = c2GJKCache::default();
    cache.count = 0;

    let A = c2Circle {
        p: v_new(0.0, 0.0),
        r: 15.0,
    };

    let B = c2Capsule {
        a: v_new(100.0, -25.0),
        b: v_new(75.0, 100.0),
        r: 10.0,
    };

    let mut a0 = c2v::default();
    let mut b0 = c2v::default();
    let mut a = c2v::default();
    let mut b = c2v::default();

    let mut iterations: c_int = -1;
    let mut cached_iterations: c_int = -1;

    let _d0 = c2GJK(
        &A as *const c2Circle as *const c_void,
        C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a0,
        &mut b0,
        1,
        &mut iterations,
        &mut cache,
    );
    let _d1 = c2GJK(
        &A as *const c2Circle as *const c_void,
        C2_TYPE_CIRCLE,
        std::ptr::null(),
        &B as *const c2Capsule as *const c_void,
        C2_TYPE_CAPSULE,
        std::ptr::null(),
        &mut a,
        &mut b,
        1,
        &mut cached_iterations,
        &mut cache,
    );

    let mut bb = c2AABB::default();
    bb.min = v_new(a1, a2);
    bb.max = v_new(a3, a4);

    let mut cap = c2Capsule::default();
    cap.a = v_new(b1, b2);
    cap.b = v_new(b3, b4);
    cap.r = b5;

    if reverse != 0 {
        c2GJK(
            &cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &bb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &mut a,
            &mut b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    } else {
        c2GJK(
            &bb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &cap as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a,
            &mut b,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
    // The C function writes nothing through a9/b9 and returns void.
}
