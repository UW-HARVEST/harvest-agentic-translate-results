//! Rust translation of `c_src/src/lib.c` (a stripped-down `cute_c2` 2D collision library).
//!
//! The translation is deliberately literal: operation order, comparison order and
//! partial writes through `c2Manifold*` are preserved so that the produced bytes match
//! the original C for the same inputs. Known defects of the C are reproduced, not fixed
//! (see the notes on `c2MakeProxy` and `ptr_from_parts`).

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::ffi::{c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// Types (layouts mirror the C declarations exactly)
// ---------------------------------------------------------------------------

/// `typedef enum { ... } C2_TYPE;` — all enumerators are non-negative, so the
/// underlying type chosen by GCC on this platform is `unsigned int`.
pub type C2_TYPE = c_uint;

pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_POLY: C2_TYPE = 3;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
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
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
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

/// `FLT_MAX` as spelled out in the C source.
const FLT_MAX: f32 = f32::MAX;
/// `FLT_EPSILON` as spelled out in the C source.
const FLT_EPSILON: f32 = f32::EPSILON;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// x86 SSE NaN propagation
//
// A binary SSE op returns its destination operand (quieted) when that operand is NaN,
// otherwise its source operand (quieted) when *that* is NaN. `c2Dot` and `c2Det2` are
// the two places where both operands of a commutative op can be NaN with different
// payloads, and GCC and LLVM pick opposite destination registers there. These helpers
// pin the destination explicitly so the propagated NaN matches the C. For non-NaN
// operands they are plain `*`, `+` and `-`.
// ---------------------------------------------------------------------------

#[inline]
fn sse_nan(dst: f32, src: f32) -> Option<f32> {
    if dst.is_nan() {
        Some(f32::from_bits(dst.to_bits() | 0x0040_0000))
    } else if src.is_nan() {
        Some(f32::from_bits(src.to_bits() | 0x0040_0000))
    } else {
        None
    }
}

#[inline]
fn mulss(dst: f32, src: f32) -> f32 {
    match sse_nan(dst, src) {
        Some(v) => v,
        None => dst * src,
    }
}

#[inline]
fn addss(dst: f32, src: f32) -> f32 {
    match sse_nan(dst, src) {
        Some(v) => v,
        None => dst + src,
    }
}

#[inline]
fn subss(dst: f32, src: f32) -> f32 {
    match sse_nan(dst, src) {
        Some(v) => v,
        None => dst - src,
    }
}

// ---------------------------------------------------------------------------
// Polygon element access
//
// These are literal translations of the C's `p->verts[i]` / `p->norms[i]`, including
// out-of-range indices: `c2Incident` and the `code == 0` branch of
// `c2CapsuletoPolyManifold` can keep `index == ~0` when every candidate comparison is
// false (which happens for a degenerate polygon whose normals are all NaN), and the C
// then reads `verts[-1]`. That read straddles the four bytes preceding the struct and
// the `count` field, so it is reproduced here with the same address arithmetic instead
// of being "fixed".
// ---------------------------------------------------------------------------

/// Byte offset of `c2Poly::verts` (after the leading `int count`).
const POLY_VERTS_OFFSET: isize = 4;
/// Byte offset of `c2Poly::norms`.
const POLY_NORMS_OFFSET: isize = 4 + 8 * 8;

#[inline]
unsafe fn poly_vert(p: *const c2Poly, i: c_int) -> c2v {
    unsafe {
        let base = (p as *const u8).offset(POLY_VERTS_OFFSET);
        std::ptr::read_unaligned(base.offset(i as isize * 8) as *const c2v)
    }
}

#[inline]
unsafe fn poly_norm(p: *const c2Poly, i: c_int) -> c2v {
    unsafe {
        let base = (p as *const u8).offset(POLY_NORMS_OFFSET);
        std::ptr::read_unaligned(base.offset(i as isize * 8) as *const c2v)
    }
}

// ---------------------------------------------------------------------------
// Vector / transform helpers
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
    // `a.x * b.x + a.y * b.y`, with the operand roles GCC emits.
    addss(mulss(b.y, a.y), mulss(a.x, b.x))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: c2h, p: c2v) -> f32 {
    c2Dot(h.n, p) - h.d
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2PlaneAt(p: *const c2Poly, i: c_int) -> c2h {
    let mut h = c2h::default();
    h.n = unsafe { poly_norm(p, i) };
    h.d = c2Dot(unsafe { poly_norm(p, i) }, unsafe { poly_vert(p, i) });
    h
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        *out.add(0) = (*bb).min;
        *out.add(1) = c2V((*bb).max.x, (*bb).min.y);
        *out.add(2) = (*bb).max;
        *out.add(3) = c2V((*bb).min.x, (*bb).max.y);
    }
}

/// NOTE: as in the C, there is no `C2_TYPE_POLY` case. For a polygon the proxy is
/// left exactly as the caller supplied it (in C: uninitialized stack memory).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: C2_TYPE, p: *mut c2Proxy) {
    unsafe {
        match type_ {
            C2_TYPE_CIRCLE => {
                let c = shape as *const c2Circle;
                (*p).radius = (*c).r;
                (*p).count = 1;
                (*p).verts[0] = (*c).p;
            }
            C2_TYPE_AABB => {
                let bb = shape as *mut c2AABB;
                (*p).radius = 0.0;
                (*p).count = 4;
                c2BBVerts((&raw mut (*p).verts) as *mut c2v, bb);
            }
            C2_TYPE_CAPSULE => {
                let c = shape as *const c2Capsule;
                (*p).radius = (*c).r;
                (*p).count = 2;
                (*p).verts[0] = (*c).a;
                (*p).verts[1] = (*c).b;
            }
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    // `a.x * b.y - a.y * b.x`, with the operand roles GCC emits.
    subss(mulss(b.y, a.x), mulss(b.x, a.y))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe {
        match (*s).count {
            2 => c2Len(c2Sub((*s).b.p, (*s).a.p)),
            3 => c2Det2(c2Sub((*s).b.p, (*s).a.p), c2Sub((*s).c.p, (*s).a.p)),
            // `default:` falls through into `case 1:` in the C.
            _ => 0.0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    // `c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)`, with GCC's operand roles.
    let y = addss(mulss(a.s, b.x), mulss(b.y, a.c));
    let x = subss(mulss(b.x, a.c), mulss(b.y, a.s));
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // `c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)`, with GCC's operand roles.
    let y = addss(mulss(-a.s, b.x), mulss(b.y, a.c));
    let x = addss(mulss(a.c, b.x), mulss(b.y, a.s));
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    // `a.x += b.x; a.y += b.y;` — GCC makes `b` the addss destination.
    a.x = addss(b.x, a.x);
    a.y = addss(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: c2v, b: c2v, da: f32, db: f32) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

/// `static int c2Clip(c2v *seg, c2h h)`
///
/// The C `out[2]` is uninitialized; slots that were never filled are copied into
/// `seg` as-is. Every caller discards `seg` unless 2 slots were written, so zeros
/// are used here for the unwritten slots. `out` is oversized so that the (only
/// float-underflow-reachable) `sp == 3` path cannot panic.
unsafe fn c2Clip(seg: *mut c2v, h: c2h) -> c_int {
    unsafe {
        let mut out = [c2v::default(); 4];
        let mut sp: usize = 0;
        let d0 = c2Dist(h, *seg.add(0));
        if d0 < 0.0 {
            out[sp] = *seg.add(0);
            sp += 1;
        }
        let d1 = c2Dist(h, *seg.add(1));
        if d1 < 0.0 {
            out[sp] = *seg.add(1);
            sp += 1;
        }
        if d0 == 0.0 && d1 == 0.0 {
            out[sp] = *seg.add(0);
            sp += 1;
            out[sp] = *seg.add(1);
            sp += 1;
        } else if d0 * d1 <= 0.0 {
            out[sp] = c2Intersect(*seg.add(0), *seg.add(1), d0, d1);
            sp += 1;
        }
        *seg.add(0) = out[0];
        *seg.add(1) = out[1];
        sp as c_int
    }
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
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(-a.x, -a.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v::default();
    b.x = a.y;
    b.y = -a.x;
    b
}

/// `static int c2SidePlanes(c2v *seg, c2v ra, c2v rb, c2h *h)`
unsafe fn c2SidePlanes(seg: *mut c2v, ra: c2v, rb: c2v, h: *mut c2h) -> c_int {
    unsafe {
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
        if !h.is_null() {
            (*h).n = c2CCW90(in_);
            (*h).d = c2Dot(c2CCW90(in_), ra);
        }
        1
    }
}

/// `static int c2SidePlanesFromPoly(c2v *seg, c2x x, const c2Poly *p, int e, c2h *h)`
unsafe fn c2SidePlanesFromPoly(
    seg: *mut c2v,
    x: c2x,
    p: *const c2Poly,
    e: c_int,
    h: *mut c2h,
) -> c_int {
    unsafe {
        let ra = c2Mulxv(x, poly_vert(p, e));
        let rb = c2Mulxv(
            x,
            poly_vert(p, if e + 1 == (*p).count { 0 } else { e + 1 }),
        );
        c2SidePlanes(seg, ra, rb, h)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).a.p;
        let b = (*s).b.p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if u <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else {
            (*s).a.u = u;
            (*s).b.u = v;
            (*s).div = u + v;
            (*s).count = 2;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let a = (*s).a.p;
        let b = (*s).b.p;
        let c = (*s).c.p;
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
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            (*s).a = (*s).b;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            (*s).a = (*s).c;
            (*s).a.u = 1.0;
            (*s).div = 1.0;
            (*s).count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            (*s).a.u = uAB;
            (*s).b.u = vAB;
            (*s).div = uAB + vAB;
            (*s).count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            (*s).a = (*s).b;
            (*s).b = (*s).c;
            (*s).a.u = uBC;
            (*s).b.u = vBC;
            (*s).div = uBC + vBC;
            (*s).count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            (*s).b = (*s).a;
            (*s).a = (*s).c;
            (*s).a.u = uCA;
            (*s).b.u = vCA;
            (*s).div = uCA + vCA;
            (*s).count = 2;
        } else {
            (*s).a.u = uABC;
            (*s).b.u = vABC;
            (*s).c.u = wABC;
            (*s).div = uABC + vABC + wABC;
            (*s).count = 3;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v::default();
    b.x = -a.y;
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        match (*s).count {
            1 => c2Neg((*s).a.p),
            2 => {
                let ab = c2Sub((*s).b.p, (*s).a.p);
                if c2Det2(ab, c2Neg((*s).a.p)) > 0.0 {
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
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => {
                *a = (*s).a.sA;
                *b = (*s).a.sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs((*s).a.sA, den * (*s).a.u),
                    c2Mulvs((*s).b.sA, den * (*s).b.u),
                );
                *b = c2Add(
                    c2Mulvs((*s).a.sB, den * (*s).a.u),
                    c2Mulvs((*s).b.sB, den * (*s).b.u),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.sA, den * (*s).a.u),
                        c2Mulvs((*s).b.sA, den * (*s).b.u),
                    ),
                    c2Mulvs((*s).c.sA, den * (*s).c.u),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs((*s).a.sB, den * (*s).a.u),
                        c2Mulvs((*s).b.sB, den * (*s).b.u),
                    ),
                    c2Mulvs((*s).c.sB, den * (*s).c.u),
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
        let den = 1.0 / (*s).div;
        match (*s).count {
            1 => (*s).a.p,
            2 => c2Add(
                c2Mulvs((*s).a.p, den * (*s).a.u),
                c2Mulvs((*s).b.p, den * (*s).b.u),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c_void,
    typeA: C2_TYPE,
    ax_ptr: *const c2x,
    B: *const c_void,
    typeB: C2_TYPE,
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
        // The C leaves these uninitialized and `c2MakeProxy` has no polygon case, so a
        // polygon operand keeps whatever was on the stack. Zeroing reproduces the
        // behaviour observed from the C on a clean stack (count 0, radius 0, verts 0)
        // and keeps it deterministic.
        let mut pA = c2Proxy::default();
        let mut pB = c2Proxy::default();
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);
        let mut s = c2Simplex::default();
        let verts: *mut c2sv = &raw mut s.a;
        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                let mut i: c_int = 0;
                while i < (*cache).count {
                    let iA = (*cache).iA[i as usize];
                    let iB = (*cache).iB[i as usize];
                    let sA = c2Mulxv(ax, pA.verts[iA as usize]);
                    let sB = c2Mulxv(bx, pB.verts[iB as usize]);
                    let v = verts.offset(i as isize);
                    (*v).iA = iA;
                    (*v).sA = sA;
                    (*v).iB = iB;
                    (*v).sB = sB;
                    (*v).p = c2Sub((*v).sB, (*v).sA);
                    (*v).u = 0.0;
                    i += 1;
                }
                s.count = (*cache).count;
                s.div = (*cache).div;
                let metric_old = (*cache).metric;
                let metric = c2GJKSimplexMetric(&mut s);
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
        let mut d0 = FLT_MAX;
        let mut d1;
        let mut iter: c_int = 0;
        let mut hit = 0;
        while iter < 20 {
            save_count = s.count;
            let mut i: c_int = 0;
            while i < save_count {
                saveA[i as usize] = (*verts.offset(i as isize)).iA;
                saveB[i as usize] = (*verts.offset(i as isize)).iB;
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
            if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
                break;
            }
            let iA = c2Support(
                (&raw const pA.verts) as *const c2v,
                pA.count,
                c2MulrvT(ax.r, c2Neg(d)),
            );
            let sA = c2Mulxv(ax, pA.verts[iA as usize]);
            let iB = c2Support(
                (&raw const pB.verts) as *const c2v,
                pB.count,
                c2MulrvT(bx.r, d),
            );
            let sB = c2Mulxv(bx, pB.verts[iB as usize]);
            let v = verts.offset(s.count as isize);
            (*v).iA = iA;
            (*v).sA = sA;
            (*v).iB = iB;
            (*v).sB = sB;
            (*v).p = c2Sub((*v).sB, (*v).sA);
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
        c2Witness(&mut s, &mut a, &mut b);
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
            (*cache).metric = c2GJKSimplexMetric(&mut s);
            (*cache).count = s.count;
            let mut i: c_int = 0;
            while i < s.count {
                let v = verts.offset(i as isize);
                (*cache).iA[i as usize] = (*v).iA;
                (*cache).iB[i as usize] = (*v).iB;
                i += 1;
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
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}

// ---------------------------------------------------------------------------
// Manifold generation
//
// Every write below mirrors the C exactly: fields the C never assigns are left
// untouched in the caller's `c2Manifold`, and fields the C reads back (such as
// `m->n` before negation) are read back here too.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCircleManifold(A: c2Circle, B: c2Circle, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
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
            (*m).count = 1;
            (*m).depths[0] = r - l;
            (*m).contact_points[0] = c2Sub(B.p, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoAABBManifold(A: c2Circle, B: c2AABB, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let L = c2Clampv(A.p, B.min, B.max);
        let ab = c2Sub(L, A.p);
        let d2 = c2Dot(ab, ab);
        let r2 = A.r * A.r;
        if d2 < r2 {
            if d2 != 0.0 {
                let d = d2.sqrt();
                let n = c2Norm(ab);
                (*m).count = 1;
                (*m).depths[0] = A.r - d;
                (*m).contact_points[0] = c2Add(A.p, c2Mulvs(n, d));
                (*m).n = n;
            } else {
                let mid = c2Mulvs(c2Add(B.min, B.max), 0.5);
                let e = c2Mulvs(c2Sub(B.max, B.min), 0.5);
                let d = c2Sub(A.p, mid);
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
                (*m).count = 1;
                (*m).depths[0] = A.r + depth;
                (*m).contact_points[0] = c2Sub(A.p, c2Mulvs(n, depth));
                (*m).n = n;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CircletoCapsuleManifold(A: c2Circle, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        let r = A.r + B.r;
        let d = c2GJK(
            (&raw const A) as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            (&raw const B) as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a,
            &mut b,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if d < r {
            let n;
            if d == 0.0 {
                n = c2Norm(c2Skew(c2Sub(B.b, B.a)));
            } else {
                n = c2Norm(c2Sub(b, a));
            }
            (*m).count = 1;
            (*m).depths[0] = r - d;
            (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoAABBManifold(A: c2AABB, B: c2AABB, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mid_a = c2Mulvs(c2Add(A.min, A.max), 0.5);
        let mid_b = c2Mulvs(c2Add(B.min, B.max), 0.5);
        let eA = c2Absv(c2Mulvs(c2Sub(A.max, A.min), 0.5));
        let eB = c2Absv(c2Mulvs(c2Sub(B.max, B.min), 0.5));
        let d = c2Sub(mid_b, mid_a);
        // `eA.x + eB.x - fabs(d.x)`, with GCC's addss destination (eA).
        let dx = subss(
            addss(eA.x, eB.x),
            if d.x < 0.0 { -d.x } else { d.x },
        );
        if dx < 0.0 {
            return;
        }
        let dy = subss(
            addss(eA.y, eB.y),
            if d.y < 0.0 { -d.y } else { d.y },
        );
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
        (*m).count = 1;
        (*m).contact_points[0] = p;
        (*m).depths[0] = depth;
        (*m).n = n;
    }
}

/// `static void c2KeepDeep(c2v *seg, c2h h, c2Manifold *m)`
unsafe fn c2KeepDeep(seg: *mut c2v, h: c2h, m: *mut c2Manifold) {
    unsafe {
        let mut cp: usize = 0;
        for i in 0..2usize {
            let p = *seg.add(i);
            let d = c2Dist(h, p);
            if d <= 0.0 {
                (*m).contact_points[cp] = p;
                (*m).depths[cp] = -d;
                cp += 1;
            }
        }
        (*m).count = cp as c_int;
        (*m).n = h.n;
    }
}

/// `static void c2Incident(c2v *incident, const c2Poly *ip, c2x ix, c2v rn_in_incident_space)`
unsafe fn c2Incident(incident: *mut c2v, ip: *const c2Poly, ix: c2x, rn_in_incident_space: c2v) {
    unsafe {
        let mut index: c_int = !0;
        let mut min_dot = FLT_MAX;
        let mut i: c_int = 0;
        while i < (*ip).count {
            let dot = c2Dot(rn_in_incident_space, poly_norm(ip, i));
            if dot < min_dot {
                min_dot = dot;
                index = i;
            }
            i += 1;
        }
        *incident.add(0) = c2Mulxv(ix, poly_vert(ip, index));
        *incident.add(1) = c2Mulxv(
            ix,
            poly_vert(ip, if index + 1 == (*ip).count { 0 } else { index + 1 }),
        );
    }
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)] // `sep = s1` is a dead store in the C too; kept for fidelity
pub unsafe extern "C" fn c2CapsuletoPolyManifold(
    A: c2Capsule,
    B: *const c2Poly,
    bx_ptr: *const c2x,
    m: *mut c2Manifold,
) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        let d = c2GJK(
            (&raw const A) as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            B as *const c_void,
            C2_TYPE_POLY,
            bx_ptr,
            &mut a,
            &mut b,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if d < 1.0e-6 {
            let bx = if !bx_ptr.is_null() {
                *bx_ptr
            } else {
                c2xIdentity()
            };
            let mut A_in_B = c2Capsule::default();
            A_in_B.a = c2MulxvT(bx, A.a);
            A_in_B.b = c2MulxvT(bx, A.b);
            let ab = c2Norm(c2Sub(A_in_B.a, A_in_B.b));
            let mut ab_h0 = c2h::default();
            ab_h0.n = c2CCW90(ab);
            ab_h0.d = c2Dot(A_in_B.a, ab_h0.n);
            let v0 = c2Support(
                (&raw const (*B).verts) as *const c2v,
                (*B).count,
                c2Neg(ab_h0.n),
            );
            let s0 = c2Dist(ab_h0, poly_vert(B, v0));
            let mut ab_h1 = c2h::default();
            ab_h1.n = c2Skew(ab);
            ab_h1.d = c2Dot(A_in_B.a, ab_h1.n);
            let v1 = c2Support(
                (&raw const (*B).verts) as *const c2v,
                (*B).count,
                c2Neg(ab_h1.n),
            );
            let s1 = c2Dist(ab_h1, poly_vert(B, v1));
            let mut index: c_int = !0;
            let mut sep = -FLT_MAX;
            let mut code: c_int = 0;
            let mut i: c_int = 0;
            while i < (*B).count {
                let h = c2PlaneAt(B, i);
                let da = c2Dot(A_in_B.a, c2Neg(h.n));
                let db = c2Dot(A_in_B.b, c2Neg(h.n));
                let d;
                if da > db {
                    d = c2Dist(h, A_in_B.a);
                } else {
                    d = c2Dist(h, A_in_B.b);
                }
                if d > sep {
                    sep = d;
                    index = i;
                }
                i += 1;
            }
            if s0 > sep {
                sep = s0;
                index = v0;
                code = 1;
            }
            if s1 > sep {
                sep = s1;
                index = v1;
                code = 2;
            }
            match code {
                0 => {
                    let mut seg: [c2v; 2] = [A.a, A.b];
                    let mut h = c2h::default();
                    if c2SidePlanesFromPoly(seg.as_mut_ptr(), bx, B, index, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(seg.as_mut_ptr(), h, m);
                    (*m).n = c2Neg((*m).n);
                }
                1 => {
                    let mut incident = [c2v::default(); 2];
                    c2Incident(incident.as_mut_ptr(), B, bx, ab_h0.n);
                    let mut h = c2h::default();
                    if c2SidePlanes(incident.as_mut_ptr(), A_in_B.b, A_in_B.a, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(incident.as_mut_ptr(), h, m);
                }
                2 => {
                    let mut incident = [c2v::default(); 2];
                    c2Incident(incident.as_mut_ptr(), B, bx, ab_h1.n);
                    let mut h = c2h::default();
                    if c2SidePlanes(incident.as_mut_ptr(), A_in_B.a, A_in_B.b, &mut h) == 0 {
                        return;
                    }
                    c2KeepDeep(incident.as_mut_ptr(), h, m);
                }
                _ => return,
            }
            let mut i: c_int = 0;
            while i < (*m).count {
                (*m).depths[i as usize] += A.r;
                i += 1;
            }
        } else if d < A.r {
            (*m).count = 1;
            (*m).n = c2Norm(c2Sub(b, a));
            (*m).contact_points[0] = c2Add(a, c2Mulvs((*m).n, A.r));
            (*m).depths[0] = A.r - d;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Norms(verts: *mut c2v, norms: *mut c2v, count: c_int) {
    unsafe {
        let mut i: c_int = 0;
        while i < count {
            let a = i;
            let b = if i + 1 < count { i + 1 } else { 0 };
            let e = c2Sub(*verts.offset(b as isize), *verts.offset(a as isize));
            *norms.offset(i as isize) = c2Norm(c2CCW90(e));
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2AABBtoCapsuleManifold(A: c2AABB, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        // `c2Poly p;` is uninitialized in the C, but only entries [0, count) are ever
        // read and those are all written below.
        //
        // The four bytes preceding `p` matter: for a degenerate AABB (min == max) every
        // polygon normal becomes NaN, `c2Incident` / `c2SidePlanesFromPoly` keep
        // `index == -1`, and the C reads `p.verts[-1]`, i.e. those preceding bytes plus
        // `p.count`. In the reference build `p` lives at `rbp-0xa0` with the by-value
        // parameter `A.max` at `rbp-0xa8`, so the preceding float is `A.max.y`.
        // `PolyFrame` reproduces that adjacency so the read yields the same value.
        #[repr(C)]
        struct PolyFrame {
            preceding: f32,
            poly: c2Poly,
        }
        let mut fr = PolyFrame {
            preceding: A.max.y,
            poly: c2Poly::default(),
        };
        let p = &raw mut fr.poly;
        let mut A_copy = A;
        c2BBVerts((&raw mut (*p).verts) as *mut c2v, &mut A_copy);
        (*p).count = 4;
        c2Norms(
            (&raw mut (*p).verts) as *mut c2v,
            (&raw mut (*p).norms) as *mut c2v,
            4,
        );
        c2CapsuletoPolyManifold(B, p, std::ptr::null(), m);
        // Reads back `m->n`, which `c2CapsuletoPolyManifold` may have left untouched.
        (*m).n = c2Neg((*m).n);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CapsuletoCapsuleManifold(A: c2Capsule, B: c2Capsule, m: *mut c2Manifold) {
    unsafe {
        (*m).count = 0;
        let mut a = c2v::default();
        let mut b = c2v::default();
        let r = A.r + B.r;
        let d = c2GJK(
            (&raw const A) as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            (&raw const B) as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a,
            &mut b,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if d < r {
            let n;
            if d == 0.0 {
                n = c2Norm(c2Skew(c2Sub(A.b, A.a)));
            } else {
                n = c2Norm(c2Sub(b, a));
            }
            (*m).count = 1;
            (*m).depths[0] = r - d;
            (*m).contact_points[0] = c2Sub(b, c2Mulvs(n, B.r));
            (*m).n = n;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collide(
    A: *const c_void,
    typeA: C2_TYPE,
    B: *const c_void,
    typeB: C2_TYPE,
    m: *mut c2Manifold,
) {
    unsafe {
        (*m).count = 0;
        match typeA {
            C2_TYPE_CIRCLE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCircleManifold(*(A as *const c2Circle), *(B as *const c2Circle), m)
                }
                C2_TYPE_AABB => {
                    c2CircletoAABBManifold(*(A as *const c2Circle), *(B as *const c2AABB), m)
                }
                C2_TYPE_CAPSULE => {
                    c2CircletoCapsuleManifold(*(A as *const c2Circle), *(B as *const c2Capsule), m)
                }
                _ => {}
            },
            C2_TYPE_AABB => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoAABBManifold(*(B as *const c2Circle), *(A as *const c2AABB), m);
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_AABB => {
                    c2AABBtoAABBManifold(*(A as *const c2AABB), *(B as *const c2AABB), m)
                }
                C2_TYPE_CAPSULE => {
                    c2AABBtoCapsuleManifold(*(A as *const c2AABB), *(B as *const c2Capsule), m)
                }
                _ => {}
            },
            C2_TYPE_CAPSULE => match typeB {
                C2_TYPE_CIRCLE => {
                    c2CircletoCapsuleManifold(*(B as *const c2Circle), *(A as *const c2Capsule), m);
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_AABB => {
                    c2AABBtoCapsuleManifold(*(B as *const c2AABB), *(A as *const c2Capsule), m);
                    (*m).n = c2Neg((*m).n);
                }
                C2_TYPE_CAPSULE => {
                    c2CapsuletoCapsuleManifold(*(A as *const c2Capsule), *(B as *const c2Capsule), m)
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// The C version has no `return` for `C2_TYPE_POLY` (and no `default`), so the returned
/// pointer is indeterminate there. `c2Collide` never dereferences a polygon operand, so
/// a null pointer is used for that path.
///
/// The allocation is intentionally never freed, matching the C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ptr_from_parts(
    typ: C2_TYPE,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
) -> *mut c_void {
    unsafe {
        match typ {
            C2_TYPE_CIRCLE => {
                let circle = malloc(std::mem::size_of::<c2Circle>()) as *mut c2Circle;
                (*circle).p = c2V(a, b);
                (*circle).r = c;
                circle as *mut c_void
            }
            C2_TYPE_AABB => {
                let aabb = malloc(std::mem::size_of::<c2AABB>()) as *mut c2AABB;
                (*aabb).min = c2V(a, b);
                (*aabb).max = c2V(c, d);
                aabb as *mut c_void
            }
            C2_TYPE_CAPSULE => {
                let capsule = malloc(std::mem::size_of::<c2Capsule>()) as *mut c2Capsule;
                (*capsule).a = c2V(a, b);
                (*capsule).b = c2V(c, d);
                (*capsule).r = e;
                capsule as *mut c_void
            }
            _ => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
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
    unsafe {
        let A = ptr_from_parts(type_a, a1, a2, a3, a4, a5);
        let B = ptr_from_parts(type_b, b1, b2, b3, b4, b5);

        c2Collide(A, type_a, B, type_b, m);
    }
}
