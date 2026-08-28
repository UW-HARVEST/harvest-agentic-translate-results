//! Rust translation of `c_src/src/lib.c` (a cute_c2-derived 2D collision library).
//!
//! The translation is intentionally literal: every arithmetic expression keeps the
//! same evaluation order, the same `f32` precision, and the same comparison
//! semantics (explicit ternaries instead of `f32::min`/`f32::max`, which differ on
//! NaN) so that results are bit-identical to the C build.
//!
//! Quirks of the original that are reproduced on purpose:
//!   * `c2MakeProxy` leaves `c2Proxy` untouched for an unknown `C2_TYPE`.
//!   * `c2GJK`'s cache-validation predicate (`metric < -1.0e8f`) is kept as written.
//!   * `c2Norm` is not guarded against a zero-length vector.
//! The C code reads a few indeterminate stack values (`c2sv::u` of a freshly
//! appended simplex vertex if the iteration cap is hit); here those bytes are
//! deterministically zero.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use core::ffi::{c_int, c_void};

// C2_TYPE enumerators.
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

// FLT_MAX / FLT_EPSILON as spelled in the C source.
const FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;
const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

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
#[derive(Copy, Clone)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
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
#[derive(Copy, Clone, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// Mirrors `struct c2Simplex { c2sv a, b, c, d; float div; int count; }`.
/// The C code aliases `&s.a` as a 4-element array of `c2sv`; the array field
/// below has exactly that layout.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

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
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
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
    let mut r = c2r::default();
    r.c = 1.0;
    r.s = 0.0;
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x = c2x::default();
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

fn bb_verts(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    let out = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    let bb = unsafe { &*bb };
    bb_verts(out, bb);
}

/// Safe core of `c2MakeProxy`. An unrecognised `type` leaves `p` alone, exactly
/// like the C switch with no `default` label.
unsafe fn make_proxy(shape: *const c_void, ty: c_int, p: &mut c2Proxy) {
    match ty {
        C2_TYPE_CIRCLE => {
            let c = unsafe { &*(shape as *const c2Circle) };
            p.radius = c.r;
            p.count = 1;
            p.verts[0] = c.p;
        }
        C2_TYPE_AABB => {
            let bb = unsafe { &*(shape as *const c2AABB) };
            p.radius = 0.0;
            p.count = 4;
            bb_verts(&mut p.verts, bb);
        }
        C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, ty: c_int, p: *mut c2Proxy) {
    unsafe { make_proxy(shape, ty, &mut *p) }
}

// ---------------------------------------------------------------------------
// Simplex math
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn gjk_simplex_metric(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.verts[1].p, s.verts[0].p)),
        3 => c2Det2(
            c2Sub(s.verts[1].p, s.verts[0].p),
            c2Sub(s.verts[2].p, s.verts[0].p),
        ),
        // `default:` falls through to `case 1:` in the C source.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    gjk_simplex_metric(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
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

fn simplex2(s: &mut c2Simplex) {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = c2Dot(b, c2Sub(b, a));
    let vv = c2Dot(a, c2Sub(a, b));
    if vv <= 0.0 {
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
        s.verts[1].u = vv;
        s.div = u + vv;
        s.count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    simplex2(unsafe { &mut *s })
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
    simplex3(unsafe { &mut *s })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v::default();
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v::default();
    b.x = a.y;
    b.y = -a.x;
    b
}

fn direction(s: &c2Simplex) -> c2v {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    direction(unsafe { &*s })
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
    let verts = unsafe { core::slice::from_raw_parts(verts, len) };
    support(verts, count, d)
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
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe { witness(&*s, &mut *a, &mut *b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

fn lambda(s: &c2Simplex) -> c2v {
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.verts[0].p,
        2 => c2Add(
            c2Mulvs(s.verts[0].p, den * s.verts[0].u),
            c2Mulvs(s.verts[1].p, den * s.verts[1].u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    lambda(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
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

    // The C locals are uninitialised; the fields actually consulted are always
    // written before use on every reachable path, so zeroing is equivalent.
    let mut pA = c2Proxy::default();
    let mut pB = c2Proxy::default();
    unsafe {
        make_proxy(A, typeA, &mut pA);
        make_proxy(B, typeB, &mut pB);
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
                let vtx = &mut s.verts[i];
                vtx.iA = iA;
                vtx.sA = sA;
                vtx.iB = iB;
                vtx.sB = sB;
                vtx.p = c2Sub(vtx.sB, vtx.sA);
                vtx.u = 0.0;
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

    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count: c_int;
    let mut d0 = FLT_MAX;
    #[allow(unused_assignments)]
    let mut d1 = FLT_MAX;
    let mut iter: c_int = 0;
    let mut hit = 0;
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
            hit = 1;
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
            let vtx = &mut s.verts[s.count as usize];
            vtx.iA = iA;
            vtx.sA = sA;
            vtx.iB = iB;
            vtx.sB = sB;
            vtx.p = c2Sub(vtx.sB, vtx.sA);
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

    let mut a = c2v::default();
    let mut b = c2v::default();
    witness(&s, &mut a, &mut b);
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
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = gjk_simplex_metric(&s);
        cache_ref.count = s.count;
        for i in 0..s.count as usize {
            cache_ref.iA[i] = s.verts[i].iA;
            cache_ref.iB[i] = s.verts[i].iB;
        }
        cache_ref.div = s.div;
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

// ---------------------------------------------------------------------------
// Boolean collision routines
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let dist = unsafe {
        c2GJK(
            &A as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            core::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        )
    };
    // `if (float)` in C is a comparison against 0.0.
    if dist != 0.0 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let dist = unsafe {
        c2GJK(
            &A as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            &B as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        )
    };
    if dist != 0.0 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let l = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
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
pub unsafe extern "C" fn c2Collided(
    A: *const c_void,
    typeA: c_int,
    B: *const c_void,
    typeB: c_int,
) -> c_int {
    unsafe {
        match typeA {
            C2_TYPE_CIRCLE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle))
                }
                C2_TYPE_AABB => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => {
                    c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            C2_TYPE_AABB => match typeB {
                C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),
                C2_TYPE_AABB => c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => {
                    c2AABBtoCapsule(*(A as *const c2AABB), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            C2_TYPE_CAPSULE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCapsule(*(B as *const c2Circle), *(A as *const c2Capsule))
                }
                C2_TYPE_AABB => c2AABBtoCapsule(*(B as *const c2AABB), *(A as *const c2Capsule)),
                C2_TYPE_CAPSULE => {
                    c2CapsuletoCapsule(*(A as *const c2Capsule), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point (declared in include/lib.h)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn capsule(min_x: f32, min_y: f32, max_x: f32, max_y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let mut capsule_in = c2Capsule::default();
    capsule_in.a = c2V(min_x, min_y);
    capsule_in.b = c2V(max_x, max_y);
    capsule_in.r = r;

    let mut circle = c2Circle::default();
    circle.p = c2V(-70.0, 0.0);
    circle.r = 20.0;

    let mut aabb = c2AABB::default();
    aabb.min = c2V(-40.0, -40.0);
    aabb.max = c2V(-15.0, -15.0);

    let mut capsule_shape = c2Capsule::default();
    capsule_shape.a = c2V(-40.0, 40.0);
    capsule_shape.b = c2V(-20.0, 100.0);
    capsule_shape.r = 10.0;

    let circle_ptr = &circle as *const c2Circle as *const c_void;
    let aabb_ptr = &aabb as *const c2AABB as *const c_void;
    let capsule_ptr = &capsule_shape as *const c2Capsule as *const c_void;
    let in_ptr = &capsule_in as *const c2Capsule as *const c_void;

    unsafe {
        result += c2Collided(circle_ptr, C2_TYPE_CIRCLE, in_ptr, C2_TYPE_CAPSULE);
        result += c2Collided(aabb_ptr, C2_TYPE_AABB, in_ptr, C2_TYPE_CAPSULE) << 1;
        result += c2Collided(capsule_ptr, C2_TYPE_CAPSULE, in_ptr, C2_TYPE_CAPSULE) << 2;
    }

    result
}
