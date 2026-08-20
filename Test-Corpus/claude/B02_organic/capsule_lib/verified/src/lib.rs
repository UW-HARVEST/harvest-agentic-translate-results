//! Rust translation of c_src/src/lib.c (cute_c2 derived collision routines).
//!
//! The whole public ABI of the C shared library is reproduced here: every
//! non-static function in `c_src/src/lib.c` is exported with the exact same
//! linker name and the exact same C signature / calling convention.
//!
//! Behaviour (including the quirks of the original code, such as the swapped
//! arguments in `c2Collided` for the AABB/circle case) is reproduced verbatim.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// FLT_MAX / FLT_EPSILON, exactly as spelled in the C source.
// ---------------------------------------------------------------------------
const C2_FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;
const C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// The C declaration is
/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
/// `a`, `b`, `c` and `d` are contiguous (the C code walks them via
/// `c2sv* verts = &s->a;`), so an array of four elements has identical layout.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

impl c2Simplex {
    #[inline]
    fn a(&self) -> c2sv {
        self.verts[0]
    }
    #[inline]
    fn b(&self) -> c2sv {
        self.verts[1]
    }
    #[inline]
    fn c(&self) -> c2sv {
        self.verts[2]
    }
}

/// Index helper used only where the original C code would perform an
/// out-of-bounds access (i.e. where the C behaviour is already undefined).
/// For every in-range value it is the identity, so well-formed inputs behave
/// exactly like the C original while malformed ones cannot corrupt memory or
/// panic across the FFI boundary.
#[inline]
fn ix(i: c_int, n: c_int) -> usize {
    if i < 0 {
        0
    } else if i >= n {
        (n - 1) as usize
    } else {
        i as usize
    }
}

// ---------------------------------------------------------------------------
// Basic math
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
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
    let mut r = c2r { c: 0.0, s: 0.0 };
    r.c = 1.0f32;
    r.s = 0.0;
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x = c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 0.0, s: 0.0 },
    };
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        let bb = &*bb;
        *out.add(0) = bb.min;
        *out.add(1) = c2V(bb.max.x, bb.min.y);
        *out.add(2) = bb.max;
        *out.add(3) = c2V(bb.min.x, bb.max.y);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    unsafe {
        let p = &mut *p;
        match type_ {
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
            // No `default` label in the C switch: nothing happens.
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// More math
// ---------------------------------------------------------------------------

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
    let s = unsafe { &*s };
    match s.count {
        2 => c2Len(c2Sub(s.b().p, s.a().p)),
        3 => c2Det2(c2Sub(s.b().p, s.a().p), c2Sub(s.c().p, s.a().p)),
        // `default:` falls through to `case 1:` which returns 0.
        _ => 0.0,
    }
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

// ---------------------------------------------------------------------------
// Simplex solvers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let s = unsafe { &mut *s };
    let a = s.a().p;
    let b = s.b().p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else if u <= 0.0 {
        s.verts[0] = s.b();
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else {
        s.verts[0].u = u;
        s.verts[1].u = v;
        s.div = u + v;
        s.count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let s = unsafe { &mut *s };
    let a = s.a().p;
    let b = s.b().p;
    let c = s.c().p;
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
        s.verts[0] = s.b();
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else if uBC <= 0.0 && vCA <= 0.0 {
        s.verts[0] = s.c();
        s.verts[0].u = 1.0f32;
        s.div = 1.0f32;
        s.count = 1;
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        s.verts[0].u = uAB;
        s.verts[1].u = vAB;
        s.div = uAB + vAB;
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.verts[0] = s.b();
        s.verts[1] = s.c();
        s.verts[0].u = uBC;
        s.verts[1].u = vBC;
        s.div = uBC + vBC;
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.verts[1] = s.a();
        s.verts[0] = s.c();
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
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = a.y;
    b.y = -a.x;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    let s = unsafe { &*s };
    match s.count {
        1 => c2Neg(s.a().p),
        2 => {
            let ab = c2Sub(s.b().p, s.a().p);
            if c2Det2(ab, c2Neg(s.a().p)) > 0.0 {
                return c2Skew(ab);
            }
            c2CCW90(ab)
        }
        // case 3 and default
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe {
        let mut imax: c_int = 0;
        let mut dmax = c2Dot(*verts.add(0), d);
        let mut i: c_int = 1;
        while i < count {
            let dot = c2Dot(*verts.offset(i as isize), d);
            if dot > dmax {
                imax = i;
                dmax = dot;
            }
            i += 1;
        }
        imax
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        let s = &*s;
        let den = 1.0f32 / s.div;
        match s.count {
            1 => {
                *a = s.a().sA;
                *b = s.a().sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs(s.a().sA, den * s.a().u),
                    c2Mulvs(s.b().sA, den * s.b().u),
                );
                *b = c2Add(
                    c2Mulvs(s.a().sB, den * s.a().u),
                    c2Mulvs(s.b().sB, den * s.b().u),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs(s.a().sA, den * s.a().u),
                        c2Mulvs(s.b().sA, den * s.b().u),
                    ),
                    c2Mulvs(s.c().sA, den * s.c().u),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs(s.a().sB, den * s.a().u),
                        c2Mulvs(s.b().sB, den * s.b().u),
                    ),
                    c2Mulvs(s.c().sB, den * s.c().u),
                );
            }
            _ => {
                *a = c2V(0.0, 0.0);
                *b = c2V(0.0, 0.0);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let s = unsafe { &*s };
    let den = 1.0f32 / s.div;
    match s.count {
        1 => s.a().p,
        2 => c2Add(
            c2Mulvs(s.a().p, den * s.a().u),
            c2Mulvs(s.b().p, den * s.b().u),
        ),
        _ => c2V(0.0, 0.0),
    }
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
    unsafe {
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
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);

        let mut s = c2Simplex::default();
        let mut cache_was_read: c_int = 0;
        if !cache.is_null() {
            let cache_ref = &mut *cache;
            let cache_was_good = cache_ref.count != 0;
            if cache_was_good {
                let n = if cache_ref.count > 3 { 3 } else { cache_ref.count };
                let mut i: c_int = 0;
                while i < n {
                    let k = i as usize;
                    let iA = cache_ref.iA[k];
                    let iB = cache_ref.iB[k];
                    let sA = c2Mulxv(ax, pA.verts[ix(iA, 8)]);
                    let sB = c2Mulxv(bx, pB.verts[ix(iB, 8)]);
                    let v = &mut s.verts[k];
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
                let metric = c2GJKSimplexMetric(&mut s);
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
                if !(min_metric < max_metric * 2.0f32 && metric < -1.0e8f32) {
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
            s.verts[0].u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        }

        let mut saveA: [c_int; 3] = [0; 3];
        let mut saveB: [c_int; 3] = [0; 3];
        let mut d0: f32 = C2_FLT_MAX;
        let mut d1: f32 = C2_FLT_MAX;
        let mut iter: c_int = 0;
        let mut hit: c_int = 0;
        let _ = d1;
        while iter < 20 {
            // `save_count` in the C source.
            let save_count = s.count;
            let saved = if save_count > 3 { 3 } else { save_count };
            let mut i: c_int = 0;
            while i < saved {
                saveA[i as usize] = s.verts[i as usize].iA;
                saveB[i as usize] = s.verts[i as usize].iB;
                i += 1;
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

            let p = c2L(&mut s);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;

            let d = c2D(&mut s);
            if c2Dot(d, d) < C2_FLT_EPSILON * C2_FLT_EPSILON {
                break;
            }

            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, pA.verts[ix(iA, 8)]);
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, pB.verts[ix(iB, 8)]);

            {
                let v = &mut s.verts[ix(s.count, 4)];
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
            }

            let mut dup: c_int = 0;
            let mut i: c_int = 0;
            while i < saved {
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

        let mut a = c2v { x: 0.0, y: 0.0 };
        let mut b = c2v { x: 0.0, y: 0.0 };
        c2Witness(&mut s, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));

        if hit != 0 {
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
            cache_ref.metric = c2GJKSimplexMetric(&mut s);
            cache_ref.count = s.count;
            let n = if s.count > 3 { 3 } else { s.count };
            let mut i: c_int = 0;
            while i < n {
                let v = s.verts[i as usize];
                cache_ref.iA[i as usize] = v.iA;
                cache_ref.iB[i as usize] = v.iB;
                i += 1;
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
    let a = A;
    let b = B;
    let r = unsafe {
        c2GJK(
            &a as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if r != 0.0 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let a = A;
    let b = B;
    let r = unsafe {
        c2GJK(
            &a as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &b as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if r != 0.0 {
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
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
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
                C2_TYPE_CIRCLE => c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle)),
                C2_TYPE_AABB => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => {
                    c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule))
                }
                _ => 0,
            },
            C2_TYPE_AABB => match typeB {
                // NOTE: the original C source swaps the operands here; the
                // behaviour is reproduced verbatim (not "fixed").
                C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),
                C2_TYPE_AABB => c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB)),
                C2_TYPE_CAPSULE => c2AABBtoCapsule(*(A as *const c2AABB), *(B as *const c2Capsule)),
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
    circle.p = c2V(-70.0f32, 0.0);
    circle.r = 20.0f32;

    let mut aabb = c2AABB::default();
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);

    let mut capsule_shape = c2Capsule::default();
    capsule_shape.a = c2V(-40.0f32, 40.0f32);
    capsule_shape.b = c2V(-20.0f32, 100.0f32);
    capsule_shape.r = 10.0f32;

    unsafe {
        result += c2Collided(
            &circle as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &capsule_in as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        );

        result += c2Collided(
            &aabb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            &capsule_in as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        ) << 1;

        result += c2Collided(
            &capsule_shape as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            &capsule_in as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
        ) << 2;
    }

    result
}
