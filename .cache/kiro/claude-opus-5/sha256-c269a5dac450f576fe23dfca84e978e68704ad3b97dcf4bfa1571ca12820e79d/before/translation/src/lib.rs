//! Rust translation of c_src/src/lib.c (cute_c2 subset).
//!
//! Behaviour, including quirks of the original, is preserved exactly.
//! Every non-static C function is re-exported with its original linker name.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Constants matching the literals used in the C source
// ---------------------------------------------------------------------------

/// FLT_MAX
const C2_FLT_MAX: f32 = 3.402_823_466_385_288_6e38_f32;
/// FLT_EPSILON
const C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;

// ---------------------------------------------------------------------------
// C2_TYPE
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// Layout-compatible with the C `c2Simplex` (`c2sv a, b, c, d; float div; int count;`).
/// The C code aliases `&s.a` as an array of four `c2sv`, so an array is used here.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Vector math
// ---------------------------------------------------------------------------

#[inline]
fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v::default();
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    v(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    v(
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
    c2r { c: 1.0f32, s: 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: v(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    v(-a.x, -a.y)
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
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    v(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

fn bb_verts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = v(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = v(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        let slice = std::slice::from_raw_parts_mut(out, 4);
        bb_verts(slice, &*bb);
    }
}

fn make_proxy_circle(c: &c2Circle, p: &mut c2Proxy) {
    p.radius = c.r;
    p.count = 1;
    p.verts[0] = c.p;
}

fn make_proxy_aabb(bb: &c2AABB, p: &mut c2Proxy) {
    p.radius = 0.0;
    p.count = 4;
    let verts = &mut p.verts;
    bb_verts(verts, bb);
}

fn make_proxy_capsule(c: &c2Capsule, p: &mut c2Proxy) {
    p.radius = c.r;
    p.count = 2;
    p.verts[0] = c.a;
    p.verts[1] = c.b;
}

/// `shape` must point to a valid shape of the kind selected by `ty`.
unsafe fn make_proxy(shape: *const (), ty: c_int, p: &mut c2Proxy) {
    unsafe {
        match ty {
            C2_TYPE_CIRCLE => make_proxy_circle(&*(shape as *const c2Circle), p),
            C2_TYPE_AABB => make_proxy_aabb(&*(shape as *const c2AABB), p),
            C2_TYPE_CAPSULE => make_proxy_capsule(&*(shape as *const c2Capsule), p),
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const (), ty: c_int, p: *mut c2Proxy) {
    unsafe { make_proxy(shape, ty, &mut *p) }
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

fn gjk_simplex_metric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.verts[1].p, s.verts[0].p)),
        3 => c2Det2(
            c2Sub(s.verts[1].p, s.verts[0].p),
            c2Sub(s.verts[2].p, s.verts[0].p),
        ),
        // `default` and `case 1` both fall through to 0 in the C source.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe { gjk_simplex_metric(&*s) }
}

fn simplex2(s: &mut c2Simplex) {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = c2Dot(b, c2Sub(b, a));
    let vv = c2Dot(a, c2Sub(a, b));
    if vv <= 0.0 {
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else if u <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else {
        s.verts[0].u = u;
        s.verts[1].u = vv;
        s.div = u + vv;
        s.count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe { simplex2(&mut *s) }
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
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.verts[0] = s.verts[2];
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
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
    unsafe { simplex3(&mut *s) }
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
        _ => v(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe { direction(&*s) }
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
    unsafe {
        let len = if count > 0 { count as usize } else { 1 };
        support(std::slice::from_raw_parts(verts, len), count, d)
    }
}

fn witness(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => {
            *a = s.verts[0].sA;
            *b = s.verts[0].sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs(s.verts[0].sA, den * s.verts[0].u),
                c2Mulvs(s.verts[1].sA, den * s.verts[1].u),
            );
            *b = c2Add(
                c2Mulvs(s.verts[0].sB, den * s.verts[0].u),
                c2Mulvs(s.verts[1].sB, den * s.verts[1].u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sA, den * s.verts[0].u),
                    c2Mulvs(s.verts[1].sA, den * s.verts[1].u),
                ),
                c2Mulvs(s.verts[2].sA, den * s.verts[2].u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sB, den * s.verts[0].u),
                    c2Mulvs(s.verts[1].sB, den * s.verts[1].u),
                ),
                c2Mulvs(s.verts[2].sB, den * s.verts[2].u),
            );
        }
        _ => {
            *a = v(0.0, 0.0);
            *b = v(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe { witness(&*s, &mut *a, &mut *b) }
}

fn closest_point(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.verts[0].p,
        2 => c2Add(
            c2Mulvs(s.verts[0].p, den * s.verts[0].u),
            c2Mulvs(s.verts[1].p, den * s.verts[1].u),
        ),
        _ => v(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe { closest_point(&*s) }
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
unsafe fn gjk(
    A: *const (),
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const (),
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    unsafe {
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

        let mut pA = c2Proxy::default();
        let mut pB = c2Proxy::default();
        make_proxy(A, typeA, &mut pA);
        make_proxy(B, typeB, &mut pB);

        let mut s = c2Simplex::default();

        let mut cache_was_read = false;
        if !cache.is_null() {
            let cache_ref = &mut *cache;
            let cache_was_good = cache_ref.count != 0;
            if cache_was_good {
                for i in 0..cache_ref.count {
                    let iA = cache_ref.iA[i as usize];
                    let iB = cache_ref.iB[i as usize];
                    let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                    let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                    let vt = &mut s.verts[i as usize];
                    vt.iA = iA;
                    vt.sA = sA;
                    vt.iB = iB;
                    vt.sB = sB;
                    vt.p = c2Sub(vt.sB, vt.sA);
                    vt.u = 0.0;
                }
                s.count = cache_ref.count;
                s.div = cache_ref.div;
                let metric_old = cache_ref.metric;
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
                // Reproduced verbatim from the C source, including the
                // seemingly-inverted condition.
                if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
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
            s.verts[0].u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        }

        let mut saveA: [c_int; 3] = [0; 3];
        let mut saveB: [c_int; 3] = [0; 3];
        let mut save_count: c_int;
        let mut d0 = C2_FLT_MAX;
        let mut d1;
        let mut iter: c_int = 0;
        let mut hit = false;

        while iter < 20 {
            save_count = s.count;
            for i in 0..save_count as usize {
                saveA[i] = s.verts[i].iA;
                saveB[i] = s.verts[i].iB;
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

            let p = closest_point(&s);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;

            let d = direction(&s);
            if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {
                break;
            }

            let iA = support(&pA.verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, pA.verts[iA as usize]);
            let iB = support(&pB.verts, pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, pB.verts[iB as usize]);

            {
                let vt = &mut s.verts[s.count as usize];
                vt.iA = iA;
                vt.sA = sA;
                vt.iB = iB;
                vt.sB = sB;
                vt.p = c2Sub(vt.sB, vt.sA);
            }

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

        let mut a = c2v::default();
        let mut b = c2v::default();
        witness(&s, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));

        if hit {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            if dist > rA + rB && dist > C2_FLT_EPSILON {
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
            let cache_ref = &mut *cache;
            cache_ref.metric = gjk_simplex_metric(&s);
            cache_ref.count = s.count;
            for i in 0..s.count as usize {
                cache_ref.iA[i] = s.verts[i].iA;
                cache_ref.iB[i] = s.verts[i].iB;
            }
            cache_ref.div = s.div;
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
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn c2GJK(
    A: *const (),
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const (),
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    unsafe {
        gjk(
            A, typeA, ax_ptr, B, typeB, bx_ptr, outA, outB, use_radius, iterations, cache,
        )
    }
}

// ---------------------------------------------------------------------------
// Boolean collision routines
// ---------------------------------------------------------------------------

fn aabb_to_aabb(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    aabb_to_aabb(A, B)
}

fn aabb_to_capsule(A: c2AABB, B: c2Capsule) -> c_int {
    let dist = unsafe {
        gjk(
            (&A as *const c2AABB).cast(),
            C2_TYPE_AABB,
            std::ptr::null(),
            (&B as *const c2Capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if dist != 0.0 { 0 } else { 1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    aabb_to_capsule(A, B)
}

fn capsule_to_capsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let dist = unsafe {
        gjk(
            (&A as *const c2Capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            (&B as *const c2Capsule).cast(),
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if dist != 0.0 { 0 } else { 1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    capsule_to_capsule(A, B)
}

fn circle_to_circle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    circle_to_circle(A, B)
}

fn circle_to_aabb(A: c2Circle, B: c2AABB) -> c_int {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    circle_to_aabb(A, B)
}

fn circle_to_capsule(A: c2Circle, B: c2Capsule) -> c_int {
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

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    circle_to_capsule(A, B)
}

unsafe fn collided(A: *const (), typeA: c_int, B: *const (), typeB: c_int) -> c_int {
    unsafe {
        match typeA {
            C2_TYPE_CIRCLE => match typeB {
                C2_TYPE_CIRCLE => {
                    circle_to_circle(*(A as *const c2Circle), *(B as *const c2Circle))
                }
                C2_TYPE_AABB => circle_to_aabb(*(A as *const c2Circle), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => {
                    circle_to_capsule(*(A as *const c2Circle), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            C2_TYPE_AABB => match typeB {
                C2_TYPE_CIRCLE => circle_to_aabb(*(B as *const c2Circle), *(A as *const c2AABB)),
                C2_TYPE_AABB => aabb_to_aabb(*(A as *const c2AABB), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => {
                    aabb_to_capsule(*(A as *const c2AABB), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            C2_TYPE_CAPSULE => match typeB {
                C2_TYPE_CIRCLE => {
                    circle_to_capsule(*(B as *const c2Circle), *(A as *const c2Capsule))
                }
                C2_TYPE_AABB => aabb_to_capsule(*(B as *const c2AABB), *(A as *const c2Capsule)),
                C2_TYPE_CAPSULE => {
                    capsule_to_capsule(*(A as *const c2Capsule), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            _ => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    A: *const (),
    typeA: c_int,
    B: *const (),
    typeB: c_int,
) -> c_int {
    unsafe { collided(A, typeA, B, typeB) }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> c_int {
    let mut result: c_int = 0;

    let mut aabb_in = c2AABB::default();
    aabb_in.min = c2V(min_x, min_y);
    aabb_in.max = c2V(max_x, max_y);

    let mut circle = c2Circle::default();
    circle.p = c2V(-70.0f32, 0.0);
    circle.r = 20.0f32;

    let mut aabb_shape = c2AABB::default();
    aabb_shape.min = c2V(-40.0f32, -40.0f32);
    aabb_shape.max = c2V(-15.0f32, -15.0f32);

    let mut capsule = c2Capsule::default();
    capsule.a = c2V(-40.0f32, 40.0f32);
    capsule.b = c2V(-20.0f32, 100.0f32);
    capsule.r = 10.0f32;

    unsafe {
        result += collided(
            (&circle as *const c2Circle).cast(),
            C2_TYPE_CIRCLE,
            (&aabb_in as *const c2AABB).cast(),
            C2_TYPE_AABB,
        );

        result += collided(
            (&aabb_shape as *const c2AABB).cast(),
            C2_TYPE_AABB,
            (&aabb_in as *const c2AABB).cast(),
            C2_TYPE_AABB,
        ) << 1;

        result += collided(
            (&capsule as *const c2Capsule).cast(),
            C2_TYPE_CAPSULE,
            (&aabb_in as *const c2AABB).cast(),
            C2_TYPE_AABB,
        ) << 2;
    }

    result
}
