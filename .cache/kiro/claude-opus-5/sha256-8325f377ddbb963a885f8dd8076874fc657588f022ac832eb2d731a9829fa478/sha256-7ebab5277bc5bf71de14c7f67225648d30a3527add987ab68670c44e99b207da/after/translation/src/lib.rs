//! Rust translation of `c_src/src/lib.c` (a cut-down copy of the cute_c2
//! GJK distance routine).
//!
//! The translation is deliberately literal: operation order, comparison
//! order and the (occasionally odd) original logic are preserved so that the
//! floating point results are bit-for-bit identical to the C build.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `C2_TYPE` is a plain C enum, i.e. an `int` at the ABI level. It is modelled
/// as `c_int` here rather than as a Rust `enum` so that out-of-range values
/// coming across the FFI boundary behave the way the C `switch` does (no case
/// matches, nothing is written) instead of being undefined behaviour.
pub type C2_TYPE = c_int;

pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;
pub const C2_TYPE_CAPSULE: C2_TYPE = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
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

/// In C this is `struct { c2sv a, b, c, d; float div; int count; }` and the
/// code aliases `&s.a` as an array of four `c2sv`. The array is modelled
/// directly here: index 0 == a, 1 == b, 2 == c, 3 == d.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

const A: usize = 0;
const B: usize = 1;
const C: usize = 2;

const FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;
const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// Small vector helpers
// ---------------------------------------------------------------------------

fn c2Maxv_impl(a: c2v, b: c2v) -> c2v {
    // Ternary `(a.x) > (b.x) ? (a.x) : (b.x)`, *not* `f32::max`: the C form
    // yields `b` when the comparison is false, including for NaN operands.
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

fn c2Minv_impl(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

fn c2Clampv_impl(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv_impl(lo, c2Minv_impl(a, hi))
}

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

fn c2BBVerts_impl(out: &mut [c2v], bb: &c2AABB) {
    out[0] = bb.min;
    out[1] = c2V(bb.max.x, bb.min.y);
    out[2] = bb.max;
    out[3] = c2V(bb.min.x, bb.max.y);
}

/// `shape` is a `*const c2Circle`, `*const c2AABB` or `*const c2Capsule`
/// depending on `type_`, exactly as in the C original.
unsafe fn c2MakeProxy_impl(shape: *const c_void, type_: C2_TYPE, p: &mut c2Proxy) {
    match type_ {
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
            c2BBVerts_impl(&mut p.verts, bb);
        }
        C2_TYPE_CAPSULE => {
            let c = unsafe { &*(shape as *const c2Capsule) };
            p.radius = c.r;
            p.count = 2;
            p.verts[0] = c.a;
            p.verts[1] = c.b;
        }
        // No `default:` label in the C `switch`: nothing is written.
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Simplex machinery
// ---------------------------------------------------------------------------

fn c2GJKSimplexMetric_impl(s: &c2Simplex) -> f32 {
    match s.count {
        2 => c2Len(c2Sub(s.verts[B].p, s.verts[A].p)),
        3 => c2Det2(
            c2Sub(s.verts[B].p, s.verts[A].p),
            c2Sub(s.verts[C].p, s.verts[A].p),
        ),
        // `default` and `case 1` both fall through to `return 0`.
        _ => 0.0,
    }
}

fn c22_impl(s: &mut c2Simplex) {
    let a = s.verts[A].p;
    let b = s.verts[B].p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        s.verts[A].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if u <= 0.0 {
        s.verts[A] = s.verts[B];
        s.verts[A].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else {
        s.verts[A].u = u;
        s.verts[B].u = v;
        s.div = u + v;
        s.count = 2;
    }
}

fn c23_impl(s: &mut c2Simplex) {
    let a = s.verts[A].p;
    let b = s.verts[B].p;
    let c = s.verts[C].p;
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
        s.verts[A].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB <= 0.0 && vBC <= 0.0 {
        s.verts[A] = s.verts[B];
        s.verts[A].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.verts[A] = s.verts[C];
        s.verts[A].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.verts[A].u = uAB;
        s.verts[B].u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.verts[A] = s.verts[B];
        s.verts[B] = s.verts[C];
        s.verts[A].u = uBC;
        s.verts[B].u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.verts[B] = s.verts[A];
        s.verts[A] = s.verts[C];
        s.verts[A].u = uCA;
        s.verts[B].u = vCA;
        s.div = uCA + vCA;
        s.count = 2;
    } else {
        s.verts[A].u = uABC;
        s.verts[B].u = vABC;
        s.verts[C].u = wABC;
        s.div = uABC + vABC + wABC;
        s.count = 3;
    }
}

fn c2D_impl(s: &c2Simplex) -> c2v {
    match s.count {
        1 => c2Neg(s.verts[A].p),
        2 => {
            let ab = c2Sub(s.verts[B].p, s.verts[A].p);
            if c2Det2(ab, c2Neg(s.verts[A].p)) > 0.0 {
                return c2Skew(ab);
            }
            c2CCW90(ab)
        }
        _ => c2V(0.0, 0.0),
    }
}

/// Takes a raw pointer rather than a slice: the C original reads `verts[0]`
/// unconditionally and then `verts[1 .. count)`, with no notion of a length, so
/// a slice would have to invent a bound the caller never supplied.
unsafe fn c2Support_impl(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    let mut imax: c_int = 0;
    let mut dmax = c2Dot(unsafe { *verts }, d);
    let mut i: c_int = 1;
    while i < count {
        let dot = c2Dot(unsafe { *verts.offset(i as isize) }, d);
        if dot > dmax {
            imax = i;
            dmax = dot;
        }
        i += 1;
    }
    imax
}

fn c2Witness_impl(s: &c2Simplex, a: &mut c2v, b: &mut c2v) {
    let den = 1.0 / s.div;
    match s.count {
        1 => {
            *a = s.verts[A].sA;
            *b = s.verts[A].sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs(s.verts[A].sA, den * s.verts[A].u),
                c2Mulvs(s.verts[B].sA, den * s.verts[B].u),
            );
            *b = c2Add(
                c2Mulvs(s.verts[A].sB, den * s.verts[A].u),
                c2Mulvs(s.verts[B].sB, den * s.verts[B].u),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs(s.verts[A].sA, den * s.verts[A].u),
                    c2Mulvs(s.verts[B].sA, den * s.verts[B].u),
                ),
                c2Mulvs(s.verts[C].sA, den * s.verts[C].u),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs(s.verts[A].sB, den * s.verts[A].u),
                    c2Mulvs(s.verts[B].sB, den * s.verts[B].u),
                ),
                c2Mulvs(s.verts[C].sB, den * s.verts[C].u),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

fn c2L_impl(s: &c2Simplex) -> c2v {
    let den = 1.0 / s.div;
    match s.count {
        1 => s.verts[A].p,
        2 => c2Add(
            c2Mulvs(s.verts[A].p, den * s.verts[A].u),
            c2Mulvs(s.verts[B].p, den * s.verts[B].u),
        ),
        _ => c2V(0.0, 0.0),
    }
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
unsafe fn c2GJK_impl(
    a_shape: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const c_void,
    typeB: C2_TYPE,
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

    let mut pA = c2Proxy {
        radius: 0.0,
        count: 0,
        verts: [c2v::default(); 8],
    };
    let mut pB = pA;
    unsafe {
        c2MakeProxy_impl(a_shape, typeA, &mut pA);
        c2MakeProxy_impl(b_shape, typeB, &mut pB);
    }

    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div: 0.0,
        count: 0,
    };

    let mut cache_was_read = 0;
    if !cache.is_null() {
        let cache_ref = unsafe { &*cache };
        let cache_was_good = cache_ref.count != 0;
        if cache_was_good {
            // Signed loop bound, matching `for (int i = 0; i < cache->count; ++i)`:
            // a negative `count` must simply skip the body rather than wrap
            // around when converted to an unsigned range.
            let mut i: c_int = 0;
            while i < cache_ref.count {
                let iA = cache_ref.iA[i as usize];
                let iB = cache_ref.iB[i as usize];
                let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                let v = &mut s.verts[i as usize];
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
                v.u = 0.0;
                i += 1;
            }
            s.count = cache_ref.count;
            s.div = cache_ref.div;
            let metric_old = cache_ref.metric;
            let metric = c2GJKSimplexMetric_impl(&s);
            let min_metric = if metric < metric_old { metric } else { metric_old };
            let max_metric = if metric > metric_old { metric } else { metric_old };
            if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                cache_was_read = 1;
            }
        }
    }

    if cache_was_read == 0 {
        s.verts[A].iA = 0;
        s.verts[A].iB = 0;
        s.verts[A].sA = c2Mulxv(ax, pA.verts[0]);
        s.verts[A].sB = c2Mulxv(bx, pB.verts[0]);
        s.verts[A].p = c2Sub(s.verts[A].sB, s.verts[A].sA);
        s.verts[A].u = 1.0;
        s.div = 1.0;
        s.count = 1;
    }

    let mut saveA: [c_int; 3] = [0; 3];
    let mut saveB: [c_int; 3] = [0; 3];
    let mut save_count;
    let mut d0 = FLT_MAX;
    let mut d1;
    let mut iter: c_int = 0;
    let mut hit = 0;
    while iter < 20 {
        save_count = s.count;
        let mut i: c_int = 0;
        while i < save_count {
            saveA[i as usize] = s.verts[i as usize].iA;
            saveB[i as usize] = s.verts[i as usize].iB;
            i += 1;
        }

        match s.count {
            1 => {}
            2 => c22_impl(&mut s),
            3 => c23_impl(&mut s),
            _ => {}
        }

        if s.count == 3 {
            hit = 1;
            break;
        }

        let p = c2L_impl(&s);
        d1 = c2Dot(p, p);
        if d1 > d0 {
            break;
        }
        d0 = d1;

        let d = c2D_impl(&s);
        if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
            break;
        }

        let iA = unsafe { c2Support_impl(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d))) };
        let sA = c2Mulxv(ax, pA.verts[iA as usize]);
        let iB = unsafe { c2Support_impl(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d)) };
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
        let mut i: c_int = 0;
        while i < save_count {
            if iA == saveA[i as usize] && iB == saveB[i as usize] {
                dup = 1;
                break;
            }
            i += 1;
        }
        if dup != 0 {
            break;
        }

        s.count += 1;
        iter += 1;
    }

    let mut a = c2v::default();
    let mut b = c2v::default();
    c2Witness_impl(&s, &mut a, &mut b);
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
        let metric = c2GJKSimplexMetric_impl(&s);
        let cache_ref = unsafe { &mut *cache };
        cache_ref.metric = metric;
        cache_ref.count = s.count;
        let mut i: c_int = 0;
        while i < s.count {
            cache_ref.iA[i as usize] = s.verts[i as usize].iA;
            cache_ref.iB[i as usize] = s.verts[i as usize].iB;
            i += 1;
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
// Public entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn gjk(
    reverse: c_char,
    a: *mut c2v,
    b: *mut c2v,
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
    let bb = c2AABB {
        min: c2V(a1, a2),
        max: c2V(a3, a4),
    };

    let cap = c2Capsule {
        a: c2V(b1, b2),
        b: c2V(b3, b4),
        r: b5,
    };

    if reverse != 0 {
        unsafe {
            c2GJK_impl(
                &cap as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                &bb as *const c2AABB as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                a,
                b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    } else {
        unsafe {
            c2GJK_impl(
                &bb as *const c2AABB as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                &cap as *const c2Capsule as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                a,
                b,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// FFI exports
//
// The C translation unit has external linkage for every function it defines,
// so the shared library exports all of them. These wrappers reproduce that
// surface with identical symbol names and C signatures.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x *= b;
    a.y *= b;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2Maxv_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2Minv_impl(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Clampv_impl(a, lo, hi)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
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
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    let out = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    c2BBVerts_impl(out, unsafe { &*bb });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: *mut c2Proxy) {
    unsafe { c2MakeProxy_impl(shape, type_, &mut *p) }
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
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    c2GJKSimplexMetric_impl(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x += b.x;
    a.y += b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    c22_impl(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    c23_impl(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
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
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    c2D_impl(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe { c2Support_impl(verts, count, d) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe { c2Witness_impl(&*s, &mut *a, &mut *b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    c2L_impl(unsafe { &*s })
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn c2GJK(
    a_shape: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    b_shape: *const c_void,
    typeB: C2_TYPE,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    unsafe {
        c2GJK_impl(
            a_shape, typeA, ax_ptr, b_shape, typeB, bx_ptr, outA, outB, use_radius, iterations,
            cache,
        )
    }
}
