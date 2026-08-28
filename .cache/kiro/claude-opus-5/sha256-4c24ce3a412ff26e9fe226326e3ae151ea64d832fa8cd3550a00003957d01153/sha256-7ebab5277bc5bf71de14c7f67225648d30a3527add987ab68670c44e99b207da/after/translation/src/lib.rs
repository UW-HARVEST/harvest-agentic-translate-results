//! Faithful Rust translation of `c_src/src/lib.c` (tinyc2-style 2D collision routines).
//!
//! Every function that has external linkage in the C translation unit is
//! re-exported here with the same symbol name, the same signature and the same
//! (bug-for-bug) behaviour. Floating point operations are kept in exactly the
//! same order as the C source so results are bit-identical.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_int;
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// C2_TYPE enum (a C enum with values 0..2 -> `int`)
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

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

impl c2Simplex {
    /// The C code takes `c2sv *verts = &s.a;` and indexes it; `a`, `b`, `c`, `d`
    /// are laid out contiguously so index 0..=3 maps onto those fields.
    #[inline]
    fn vert(&self, i: usize) -> c2sv {
        match i {
            0 => self.a,
            1 => self.b,
            2 => self.c,
            _ => self.d,
        }
    }

    #[inline]
    fn vert_mut(&mut self, i: usize) -> &mut c2sv {
        match i {
            0 => &mut self.a,
            1 => &mut self.b,
            2 => &mut self.c,
            _ => &mut self.d,
        }
    }
}

// ---------------------------------------------------------------------------
// Bit-exact scalar float helpers
// ---------------------------------------------------------------------------
//
// IEEE-754 leaves NaN payload propagation unspecified, and the reference C
// build (gcc, no optimisation, SSE scalar math) therefore has an *observable*
// payload order that is decided by which operand ends up in the destination
// register of each `mulss` / `addss` / `subss`.  The rule the hardware follows
// is:
//
//   1. if src1 (the destination operand) is NaN -> return it, quieted
//   2. else if src2 is NaN                      -> return it, quieted
//   3. else if the operation is invalid         -> return the default QNaN
//
// Writing plain `a * b + c * d` in Rust does not pin this down: LLVM freely
// commutes the operands of `fmul`/`fadd`, and it does so differently at
// different optimisation levels.  The helpers below make the choice explicit
// so the result is bit-identical to the C library in every build profile.  For
// every non-NaN input they are exactly `*`, `+` and `-`.

/// The "QNaN indefinite" value an SSE arithmetic instruction produces for an
/// invalid operation such as `0 * inf` or `inf - inf`.
const QNAN_INDEFINITE: f32 = f32::from_bits(0xffc0_0000);

/// Quiets a NaN the way an SSE arithmetic instruction does: the quiet bit is
/// forced on, the sign and the payload are preserved.
#[inline(always)]
fn quiet(v: f32) -> f32 {
    f32::from_bits(v.to_bits() | 0x0040_0000)
}

/// NaN result selection for a binary scalar op whose destination register holds
/// `src1`.
#[inline(always)]
fn sse_nan(src1: f32, src2: f32) -> f32 {
    if src1.is_nan() {
        quiet(src1)
    } else if src2.is_nan() {
        quiet(src2)
    } else {
        QNAN_INDEFINITE
    }
}

/// `mulss src2, src1`
#[inline(always)]
fn fmul(src1: f32, src2: f32) -> f32 {
    let r = src1 * src2;
    if r.is_nan() { sse_nan(src1, src2) } else { r }
}

/// `addss src2, src1`
#[inline(always)]
fn fadd(src1: f32, src2: f32) -> f32 {
    let r = src1 + src2;
    if r.is_nan() { sse_nan(src1, src2) } else { r }
}

/// `subss src2, src1`
#[inline(always)]
fn fsub(src1: f32, src2: f32) -> f32 {
    let r = src1 - src2;
    if r.is_nan() { sse_nan(src1, src2) } else { r }
}

/// `divss src2, src1`
#[inline(always)]
fn fdiv(src1: f32, src2: f32) -> f32 {
    let r = src1 / src2;
    if r.is_nan() { sse_nan(src1, src2) } else { r }
}

/// `sqrtss src, dst` - a NaN operand comes back quieted, a negative operand
/// yields the default QNaN.
#[inline(always)]
fn fsqrt(src: f32) -> f32 {
    if src.is_nan() {
        quiet(src)
    } else if src < 0.0 {
        QNAN_INDEFINITE
    } else {
        src.sqrt()
    }
}

// ---------------------------------------------------------------------------
// Vector math
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

/// `a.x *= b; a.y *= b`. Reference build: `mulss b, a.x` (source order).
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x = fmul(a.x, b);
    a.y = fmul(a.y, b);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    // Reproduce the C ternary (NOT fmaxf: NaN handling / tie behaviour differ).
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

/// `a.x -= b.x; a.y -= b.y`. Reference build: `subss b.x, a.x` (source order).
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x = fsub(a.x, b.x);
    a.y = fsub(a.y, b.y);
    a
}

/// `a.x * b.x + a.y * b.y`.
///
/// gcc emits `mulss b.x, a.x` / `mulss a.y, b.y` / `addss p1, p2`, i.e. the
/// NaN-payload preference order is `b.y, a.y, a.x, b.x`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    let p1 = fmul(a.x, b.x);
    let p2 = fmul(b.y, a.y);
    fadd(p2, p1)
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
    let mut x = c2x::default();
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

/// `sqrtf(c2Dot(a, a))`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    fsqrt(c2Dot(a, a))
}

/// `a.x * b.y - a.y * b.x`.
///
/// gcc emits `mulss a.x, b.y` / `mulss a.y, b.x` / `subss p2, p1`, i.e. the
/// NaN-payload preference order is `b.y, a.x, b.x, a.y`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    let p1 = fmul(b.y, a.x);
    let p2 = fmul(b.x, a.y);
    fsub(p1, p2)
}

/// `c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)`.
///
/// Operand order taken from the reference build: the `x` component is
/// `subss(mulss(b.x, a.c), mulss(b.y, a.s))` and the `y` component is
/// `addss(mulss(a.s, b.x), mulss(b.y, a.c))`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    let x = fsub(fmul(b.x, a.c), fmul(b.y, a.s));
    let y = fadd(fmul(a.s, b.x), fmul(b.y, a.c));
    c2V(x, y)
}

/// `a.x += b.x; a.y += b.y`.
///
/// The reference build emits `addss a.x, b.x`, i.e. the destination register
/// holds `b`, so `b`'s NaN payload wins - the reverse of the source order.
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x = fadd(b.x, a.x);
    a.y = fadd(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
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

/// `c2Mulvs(a, 1.0f / b)`. Reference build: `divss b, 1.0f`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, fdiv(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

/// `c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)`.
///
/// The reference build negates `a.s` with `xorps` (a pure sign flip that keeps
/// the NaN payload, exactly like Rust's unary `-`) before the multiply.
#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    let x = fadd(fmul(a.c, b.x), fmul(b.y, a.s));
    let y = fadd(fmul(-a.s, b.x), fmul(b.y, a.c));
    c2V(x, y)
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
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe {
        let s = &*s;
        match s.count {
            2 => c2Len(c2Sub(s.b.p, s.a.p)),
            3 => c2Det2(c2Sub(s.b.p, s.a.p), c2Sub(s.c.p, s.a.p)),
            // `default:` falls through to `case 1:` in the C source.
            _ => 0.0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let s = &mut *s;
        let a = s.a.p;
        let b = s.b.p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            s.a.u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        } else if u <= 0.0 {
            s.a = s.b;
            s.a.u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        } else {
            s.a.u = u;
            s.b.u = v;
            s.div = fadd(u, v);
            s.count = 2;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let s = &mut *s;
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
        let uABC = fmul(c2Det2(b, c), area);
        let vABC = fmul(c2Det2(c, a), area);
        let wABC = fmul(c2Det2(a, b), area);
        if vAB <= 0.0 && uCA <= 0.0 {
            s.a.u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            s.a = s.b;
            s.a.u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            s.a = s.c;
            s.a.u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            s.a.u = uAB;
            s.b.u = vAB;
            s.div = fadd(uAB, vAB);
            s.count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            s.a = s.b;
            s.b = s.c;
            s.a.u = uBC;
            s.b.u = vBC;
            s.div = fadd(uBC, vBC);
            s.count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            s.b = s.a;
            s.a = s.c;
            s.a.u = uCA;
            s.b.u = vCA;
            s.div = fadd(uCA, vCA);
            s.count = 2;
        } else {
            s.a.u = uABC;
            s.b.u = vABC;
            s.c.u = wABC;
            s.div = fadd(fadd(uABC, vABC), wABC);
            s.count = 3;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        let s = &*s;
        match s.count {
            1 => c2Neg(s.a.p),
            2 => {
                let ab = c2Sub(s.b.p, s.a.p);
                if c2Det2(ab, c2Neg(s.a.p)) > 0.0 {
                    return c2Skew(ab);
                }
                c2CCW90(ab)
            }
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
    unsafe {
        let mut imax: c_int = 0;
        let mut dmax = c2Dot(*verts.add(0), d);
        let mut i: c_int = 1;
        while i < count {
            let dot = c2Dot(*verts.add(i as usize), d);
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
        let den = fdiv(1.0f32, s.div);
        match s.count {
            1 => {
                *a = s.a.sA;
                *b = s.a.sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs(s.a.sA, fmul(s.a.u, den)),
                    c2Mulvs(s.b.sA, fmul(s.b.u, den)),
                );
                *b = c2Add(
                    c2Mulvs(s.a.sB, fmul(s.a.u, den)),
                    c2Mulvs(s.b.sB, fmul(s.b.u, den)),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs(s.a.sA, fmul(s.a.u, den)),
                        c2Mulvs(s.b.sA, fmul(s.b.u, den)),
                    ),
                    c2Mulvs(s.c.sA, fmul(s.c.u, den)),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs(s.a.sB, fmul(s.a.u, den)),
                        c2Mulvs(s.b.sB, fmul(s.b.u, den)),
                    ),
                    c2Mulvs(s.c.sB, fmul(s.c.u, den)),
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
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe {
        let s = &*s;
        let den = fdiv(1.0f32, s.div);
        match s.count {
            1 => s.a.p,
            2 => c2Add(
                c2Mulvs(s.a.p, fmul(s.a.u, den)),
                c2Mulvs(s.b.p, fmul(s.b.u, den)),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

const FLT_MAX: f32 = 3.402_823_466_385_288_6e38_f32;
const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;

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
        let ax: c2x;
        let bx: c2x;
        if ax_ptr.is_null() {
            ax = c2xIdentity();
        } else {
            ax = *ax_ptr;
        }
        if bx_ptr.is_null() {
            bx = c2xIdentity();
        } else {
            bx = *bx_ptr;
        }
        let mut pA = c2Proxy::default();
        let mut pB = c2Proxy::default();
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);
        let mut s = c2Simplex::default();
        let mut cache_was_read: c_int = 0;
        if !cache.is_null() {
            let cache = &mut *cache;
            let cache_was_good = (cache.count != 0) as c_int;
            if cache_was_good != 0 {
                // Signed loop counter: `cache->count` is an `int` and the C
                // loop simply does not execute for a negative count.  Casting
                // to `usize` first would wrap to a huge bound instead.
                let mut i: c_int = 0;
                while i < cache.count {
                    let iA = cache.iA[(i as usize) & 3];
                    let iB = cache.iB[(i as usize) & 3];
                    let sA = c2Mulxv(ax, pA.verts[(iA as usize) & 7]);
                    let sB = c2Mulxv(bx, pB.verts[(iB as usize) & 7]);
                    let v = s.vert_mut(i as usize);
                    v.iA = iA;
                    v.sA = sA;
                    v.iB = iB;
                    v.sB = sB;
                    v.p = c2Sub(v.sB, v.sA);
                    v.u = 0.0;
                    i += 1;
                }
                s.count = cache.count;
                s.div = cache.div;
                let metric_old = cache.metric;
                let metric = c2GJKSimplexMetric(&mut s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < fadd(max_metric, max_metric) && metric < -1.0e8f32) {
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
            s.a.u = 1.0f32;
            s.div = 1.0f32;
            s.count = 1;
        }
        let mut saveA: [c_int; 3] = [0; 3];
        let mut saveB: [c_int; 3] = [0; 3];
        let mut save_count: c_int;
        let mut d0 = FLT_MAX;
        let mut d1;
        let mut iter: c_int = 0;
        let mut hit: c_int = 0;
        while iter < 20 {
            save_count = s.count;
            // Signed counter: for `s.count <= 0` the C loop body never runs.
            {
                let mut i: c_int = 0;
                while i < save_count && i < 3 {
                    saveA[i as usize] = s.vert(i as usize).iA;
                    saveB[i as usize] = s.vert(i as usize).iB;
                    i += 1;
                }
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
            if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
                break;
            }
            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, pA.verts[iA as usize]);
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, pB.verts[iB as usize]);
            {
                let v = s.vert_mut(s.count as usize);
                v.iA = iA;
                v.sA = sA;
                v.iB = iB;
                v.sB = sB;
                v.p = c2Sub(v.sB, v.sA);
            }
            let mut dup = 0;
            {
                let mut i: c_int = 0;
                while i < save_count && i < 3 {
                    if iA == saveA[i as usize] && iB == saveB[i as usize] {
                        dup = 1;
                        break;
                    }
                    i += 1;
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
        c2Witness(&mut s, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));
        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            // The reference build emits `addss rA, rB`, so `rB` occupies the
            // destination register and its NaN payload wins.
            if dist > fadd(rB, rA) && dist > FLT_EPSILON {
                dist = fsub(dist, fadd(rB, rA));
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
            let cache = &mut *cache;
            cache.metric = c2GJKSimplexMetric(&mut s);
            cache.count = s.count;
            // Signed counter: nothing is written back for `s.count <= 0`.
            let mut i: c_int = 0;
            while i < s.count && i < 3 {
                let v = s.vert(i as usize);
                cache.iA[i as usize] = v.iA;
                cache.iB[i as usize] = v.iB;
                i += 1;
            }
            cache.div = s.div;
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
    unsafe {
        if c2GJK(
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
        ) != 0.0
        {
            return 0;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    unsafe {
        if c2GJK(
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
        ) != 0.0
        {
            return 0;
        }
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
pub extern "C" fn reverse_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let mut circle_in = c2Circle::default();
    circle_in.p = c2V(x, y);
    circle_in.r = r;

    let mut circle = c2Circle::default();
    circle.p = c2V(-70.0f32, 0.0);
    circle.r = 20.0f32;

    let mut aabb = c2AABB::default();
    aabb.min = c2V(-40.0f32, -40.0f32);
    aabb.max = c2V(-15.0f32, -15.0f32);

    let mut capsule = c2Capsule::default();
    capsule.a = c2V(-40.0f32, 40.0f32);
    capsule.b = c2V(-20.0f32, 100.0f32);
    capsule.r = 10.0f32;

    unsafe {
        result += c2Collided(
            &circle as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            &circle_in as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        );

        result += c2Collided(
            &aabb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            &circle_in as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        ) << 1;

        result += c2Collided(
            &capsule as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            &circle_in as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
        ) << 2;
    }

    result
}
