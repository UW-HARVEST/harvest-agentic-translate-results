//! Rust translation of the C library in `c_src/` (a cute_c2 / tinyc2 derivative).
//!
//! Every public symbol exported by the C shared library is re-exported here with
//! the identical linker name, C ABI, and bit-exact behaviour.  The original C is
//! reproduced faithfully, including its quirks; no bugs are "fixed".

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Bit-exact scalar float arithmetic
// ---------------------------------------------------------------------------
//
// The C library compiles to x86-64 SSE scalar instructions.  `MULSS`, `ADDSS`,
// `SUBSS` and `DIVSS` have a *defined* NaN-propagation rule:
//
//   * if the destination (first) operand is a NaN, the result is that NaN with
//     the quiet bit forced on;
//   * otherwise, if the source (second) operand is a NaN, the result is that
//     NaN quieted;
//   * otherwise the ordinary arithmetic result (an invalid operation on
//     non-NaN operands, e.g. `0 * inf`, yields the x86 "QNaN indefinite").
//
// *Which* operand gcc puts in the destination register is therefore observable
// whenever two operands are different NaNs.  LLVM commutes `fmul`/`fadd` freely
// (even at `opt-level = 0`), so a plain Rust `a * b` cannot be relied on to pick
// the same operand gcc picked.  These helpers pin the choice explicitly, which
// makes the Rust build reproduce the C binary's NaN payloads bit-for-bit while
// staying ordinary, portable Rust for every non-NaN input.  Each call site
// documents the gcc instruction it mirrors.

#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `MULSS dst, src` -> `dst * src`
#[inline(always)]
fn mulss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet(dst)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dst * src
    }
}

/// `ADDSS dst, src` -> `dst + src`
#[inline(always)]
fn addss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet(dst)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dst + src
    }
}

/// `SUBSS dst, src` -> `dst - src`
#[inline(always)]
fn subss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet(dst)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dst - src
    }
}

/// `DIVSS dst, src` -> `dst / src`
#[inline(always)]
fn divss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet(dst)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dst / src
    }
}

// ---------------------------------------------------------------------------
// C2_TYPE enum (a plain C enum => `int` in the ABI)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Public structs (layout-compatible with the C definitions)
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

/// `typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;`
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

/// `typedef struct { c2v sA; c2v sB; c2v p; float u; int iA; int iB; } c2sv;`
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

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
///
/// The four `c2sv` members are stored as an array because the C code walks them
/// with pointer arithmetic (`c2sv* verts = &s.a;`).  The memory layout is
/// identical to four consecutive `c2sv` fields.
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

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    // gcc: mulss(dst = a.x, src = b) / mulss(dst = a.y, src = b)
    let mut a = a;
    a.x = mulss(a.x, b);
    a.y = mulss(a.y, b);
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
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    // gcc: subss(dst = a.x, src = b.x) / subss(dst = a.y, src = b.y)
    let mut a = a;
    a.x = subss(a.x, b.x);
    a.y = subss(a.y, b.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    // C: `a.x * b.x + a.y * b.y`.
    //
    // The operand order below (and the `mulss`/`addss` helpers) looks
    // gratuitous but is load-bearing: it reproduces the exact SSE instruction
    // operand order the C compiler emits, which decides *which* NaN payload
    // survives when more than one operand is a NaN.  gcc emits
    //   p1 = mulss(dst = a.x, src = b.x)
    //   p2 = mulss(dst = b.y, src = a.y)
    //   r  = addss(dst = p2,  src = p1)
    // For every non-NaN input this is bit-identical to the naive spelling.
    let p1 = mulss(a.x, b.x);
    let p2 = mulss(b.y, a.y);
    addss(p2, p1)
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

// ---------------------------------------------------------------------------
// Shape helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    let bb = &*bb;
    *out.add(0) = bb.min;
    *out.add(1) = c2V(bb.max.x, bb.min.y);
    *out.add(2) = bb.max;
    *out.add(3) = c2V(bb.min.x, bb.max.y);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c2void, type_: c_int, p: *mut c2Proxy) {
    match type_ {
        C2_TYPE_CIRCLE => {
            let c = shape as *mut c2Circle;
            (*p).radius = (*c).r;
            (*p).count = 1;
            (*p).verts[0] = (*c).p;
        }
        C2_TYPE_AABB => {
            let bb = shape as *mut c2AABB;
            (*p).radius = 0.0;
            (*p).count = 4;
            c2BBVerts((*p).verts.as_mut_ptr(), bb);
        }
        C2_TYPE_CAPSULE => {
            let c = shape as *mut c2Capsule;
            (*p).radius = (*c).r;
            (*p).count = 2;
            (*p).verts[0] = (*c).a;
            (*p).verts[1] = (*c).b;
        }
        _ => {}
    }
}

/// Alias used purely to keep the `const void*` parameter spelling readable.
pub type c2void = c_void;

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    sqrtf(c2Dot(a, a))
}

#[inline]
fn sqrtf(v: f32) -> f32 {
    // IEEE-754 correctly-rounded single-precision square root, exactly as
    // C's sqrtf().
    v.sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    // C: `a.x * b.y - a.y * b.x`; gcc emits
    //   p1 = mulss(dst = b.y, src = a.x)
    //   p2 = mulss(dst = b.x, src = a.y)
    //   r  = subss(dst = p1,  src = p2)
    // (see the note in `c2Dot` about NaN-payload selection).
    let p1 = mulss(b.y, a.x);
    let p2 = mulss(b.x, a.y);
    subss(p1, p2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    let s = &mut *s;
    match s.count {
        2 => c2Len(c2Sub(s.verts[1].p, s.verts[0].p)),
        3 => c2Det2(
            c2Sub(s.verts[1].p, s.verts[0].p),
            c2Sub(s.verts[2].p, s.verts[0].p),
        ),
        // `default:` falls through into `case 1:` in the C source.
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    // C: `c2V(a.c*b.x - a.s*b.y, a.s*b.x + a.c*b.y)`; gcc emits
    //   y = addss(dst = mulss(dst = a.s, src = b.x), src = mulss(dst = b.y, src = a.c))
    //   x = subss(dst = mulss(dst = b.x, src = a.c), src = mulss(dst = b.y, src = a.s))
    // (see the note in `c2Dot` about NaN-payload selection).
    let y = addss(mulss(a.s, b.x), mulss(b.y, a.c));
    let x = subss(mulss(b.x, a.c), mulss(b.y, a.s));
    c2V(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    // C: `a.x += b.x; a.y += b.y;`.  gcc emits `addss(dst = b.x, src = a.x)`,
    // i.e. the *right-hand* operand is the destination, so a NaN in `b`
    // outranks a NaN in `a` (see the note in `c2Dot`).
    let mut a = a;
    a.x = addss(b.x, a.x);
    a.y = addss(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    let s = &mut *s;
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = c2Dot(b, c2Sub(b, a));
    let v = c2Dot(a, c2Sub(a, b));
    if v <= 0.0 {
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
        s.verts[1].u = v;
        s.div = addss(u, v); // gcc: addss(dst = u, src = v)
        s.count = 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    let s = &mut *s;
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
    // gcc: mulss(dst = <c2Det2 result>, src = area)
    let uABC = mulss(c2Det2(b, c), area);
    let vABC = mulss(c2Det2(c, a), area);
    let wABC = mulss(c2Det2(a, b), area);
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
        s.div = addss(uAB, vAB); // gcc: addss(dst = uAB, src = vAB)
        s.count = 2;
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        s.verts[0] = s.verts[1];
        s.verts[1] = s.verts[2];
        s.verts[0].u = uBC;
        s.verts[1].u = vBC;
        s.div = addss(uBC, vBC);
        s.count = 2;
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        s.verts[1] = s.verts[0];
        s.verts[0] = s.verts[2];
        s.verts[0].u = uCA;
        s.verts[1].u = vCA;
        s.div = addss(uCA, vCA);
        s.count = 2;
    } else {
        s.verts[0].u = uABC;
        s.verts[1].u = vABC;
        s.verts[2].u = wABC;
        s.div = addss(addss(uABC, vABC), wABC);
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
    let s = &mut *s;
    match s.count {
        1 => c2Neg(s.verts[0].p),
        2 => {
            let ab = c2Sub(s.verts[1].p, s.verts[0].p);
            if c2Det2(ab, c2Neg(s.verts[0].p)) > 0.0 {
                return c2Skew(ab);
            }
            c2CCW90(ab)
        }
        // `case 3:` and `default:`
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Support(verts: *const c2v, count: c_int, d: c2v) -> c_int {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    let s = &mut *s;
    let den = divss(1.0f32, s.div);
    // `(den * s->X.u)` is emitted by gcc as `mulss(dst = X.u, src = den)`, so
    // the factors are written in the reverse order here (see `c2Dot`).
    match s.count {
        1 => {
            *a = s.verts[0].sA;
            *b = s.verts[0].sB;
        }
        2 => {
            *a = c2Add(
                c2Mulvs(s.verts[0].sA, mulss(s.verts[0].u, den)),
                c2Mulvs(s.verts[1].sA, mulss(s.verts[1].u, den)),
            );
            *b = c2Add(
                c2Mulvs(s.verts[0].sB, mulss(s.verts[0].u, den)),
                c2Mulvs(s.verts[1].sB, mulss(s.verts[1].u, den)),
            );
        }
        3 => {
            *a = c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sA, mulss(s.verts[0].u, den)),
                    c2Mulvs(s.verts[1].sA, mulss(s.verts[1].u, den)),
                ),
                c2Mulvs(s.verts[2].sA, mulss(s.verts[2].u, den)),
            );
            *b = c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sB, mulss(s.verts[0].u, den)),
                    c2Mulvs(s.verts[1].sB, mulss(s.verts[1].u, den)),
                ),
                c2Mulvs(s.verts[2].sB, mulss(s.verts[2].u, den)),
            );
        }
        _ => {
            *a = c2V(0.0, 0.0);
            *b = c2V(0.0, 0.0);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    // gcc: divss(dst = 1.0f, src = b)
    c2Mulvs(a, divss(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    let s = &mut *s;
    let den = divss(1.0f32, s.div);
    // `(den * s->X.u)` -> `mulss(dst = X.u, src = den)`; see `c2Dot`.
    match s.count {
        1 => s.verts[0].p,
        2 => c2Add(
            c2Mulvs(s.verts[0].p, mulss(s.verts[0].u, den)),
            c2Mulvs(s.verts[1].p, mulss(s.verts[1].u, den)),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // C: `c2V(a.c*b.x + a.s*b.y, -a.s*b.x + a.c*b.y)`; gcc emits
    //   y = addss(dst = mulss(dst = xorps(a.s, -0.0), src = b.x),
    //             src = mulss(dst = b.y, src = a.c))
    //   x = addss(dst = mulss(dst = a.c, src = b.x), src = mulss(dst = b.y, src = a.s))
    // (see the note in `c2Dot` about NaN-payload selection).
    let y = addss(mulss(-a.s, b.x), mulss(b.y, a.c));
    let x = addss(mulss(a.c, b.x), mulss(b.y, a.s));
    c2V(x, y)
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

const C2_FLT_MAX: f32 = 3.402_823_466_385_288_6e38_f32; // FLT_MAX
const C2_FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32; // FLT_EPSILON
/// `FLT_EPSILON * FLT_EPSILON`: gcc folds this product at compile time (the C
/// spells it as a literal-times-literal), so it must be a `const` here too.
const C2_FLT_EPSILON_SQ: f32 = C2_FLT_EPSILON * C2_FLT_EPSILON;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJK(
    A: *const c2void,
    typeA: c_int,
    ax_ptr: *const c2x,
    B: *const c2void,
    typeB: c_int,
    bx_ptr: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
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
    let verts: *mut c2sv = s.verts.as_mut_ptr();
    let mut cache_was_read: c_int = 0;
    if !cache.is_null() {
        let cache_was_good: c_int = ((*cache).count != 0) as c_int;
        if cache_was_good != 0 {
            let mut i: c_int = 0;
            while i < (*cache).count {
                let iA = *(*cache).iA.as_ptr().offset(i as isize);
                let iB = *(*cache).iB.as_ptr().offset(i as isize);
                let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
                let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
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
            // gcc strength-reduces `max_metric * 2.0f` to `addss(x, x)`; the two
            // forms are bit-identical for every input, including NaN payloads.
            if !(min_metric < addss(max_metric, max_metric) && metric < -1.0e8f32) {
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
    let mut save_count: c_int = 0;
    let mut d0: f32 = C2_FLT_MAX;
    let mut d1: f32;
    let mut iter: c_int = 0;
    let mut hit: c_int = 0;
    while iter < 20 {
        save_count = s.count;
        let mut i: c_int = 0;
        while i < save_count {
            *saveA.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iA;
            *saveB.as_mut_ptr().offset(i as isize) = (*verts.offset(i as isize)).iB;
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
        if c2Dot(d, d) < C2_FLT_EPSILON_SQ {
            break;
        }
        let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
        let sA = c2Mulxv(ax, *pA.verts.as_ptr().offset(iA as isize));
        let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
        let sB = c2Mulxv(bx, *pB.verts.as_ptr().offset(iB as isize));
        let v = verts.offset(s.count as isize);
        (*v).iA = iA;
        (*v).sA = sA;
        (*v).iB = iB;
        (*v).sB = sB;
        (*v).p = c2Sub((*v).sB, (*v).sA);
        let mut dup: c_int = 0;
        let mut i: c_int = 0;
        while i < save_count {
            if iA == *saveA.as_ptr().offset(i as isize) && iB == *saveB.as_ptr().offset(i as isize) {
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
    let _ = save_count;
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
        if dist > addss(rA, rB) && dist > C2_FLT_EPSILON {
            // gcc: addss(dst = rA, src = rB) then subss(dst = dist, src = sum)
            dist = subss(dist, addss(rA, rB));
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
        (*cache).metric = c2GJKSimplexMetric(&mut s);
        (*cache).count = s.count;
        let mut i: c_int = 0;
        while i < s.count {
            let v = verts.offset(i as isize);
            *(*cache).iA.as_mut_ptr().offset(i as isize) = (*v).iA;
            *(*cache).iB.as_mut_ptr().offset(i as isize) = (*v).iB;
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

// ---------------------------------------------------------------------------
// Boolean collision routines
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0: c_int = (B.max.x < A.min.x) as c_int;
    let d1: c_int = (A.max.x < B.min.x) as c_int;
    let d2: c_int = (B.max.y < A.min.y) as c_int;
    let d3: c_int = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoCapsule(A: c2AABB, B: c2Capsule) -> c_int {
    let mut A = A;
    let mut B = B;
    unsafe {
        if c2GJK(
            &mut A as *mut c2AABB as *const c2void,
            C2_TYPE_AABB,
            core::ptr::null(),
            &mut B as *mut c2Capsule as *const c2void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        ) != 0.0
        {
            return 0;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CapsuletoCapsule(A: c2Capsule, B: c2Capsule) -> c_int {
    let mut A = A;
    let mut B = B;
    unsafe {
        if c2GJK(
            &mut A as *mut c2Capsule as *const c2void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            &mut B as *mut c2Capsule as *const c2void,
            C2_TYPE_CAPSULE,
            core::ptr::null(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
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
    // C: `float r2 = A.r + B.r;` -> `addss(dst = B.r, src = A.r)`; see `c2Dot`.
    let mut r2 = addss(B.r, A.r);
    r2 = mulss(r2, r2);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = mulss(A.r, A.r); // gcc: mulss(dst = A.r, src = A.r)
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(A: c2Circle, B: c2Capsule) -> c_int {
    let n = c2Sub(B.b, B.a);
    let ap = c2Sub(A.p, B.a);
    let da = c2Dot(ap, n);
    let d2: f32;
    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(A.p, B.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, divss(da, c2Dot(n, n))));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    // C: `float r = A.r + B.r;` -> `addss(dst = B.r, src = A.r)`; see `c2Dot`.
    let r = addss(B.r, A.r);
    (d2 < mulss(r, r)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(
    A: *const c2void,
    typeA: c_int,
    B: *const c2void,
    typeB: c_int,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(*(A as *const c2Circle), *(B as *const c2Circle)),
            C2_TYPE_AABB => c2CircletoAABB(*(A as *const c2Circle), *(B as *const c2AABB)),
            C2_TYPE_CAPSULE => c2CircletoCapsule(*(A as *const c2Circle), *(B as *const c2Capsule)),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(*(B as *const c2Circle), *(A as *const c2AABB)),
            C2_TYPE_AABB => c2AABBtoAABB(*(A as *const c2AABB), *(B as *const c2AABB)),
            C2_TYPE_CAPSULE => c2AABBtoCapsule(*(A as *const c2AABB), *(B as *const c2Capsule)),
            _ => 0,
        },
        C2_TYPE_CAPSULE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCapsule(*(B as *const c2Circle), *(A as *const c2Capsule)),
            C2_TYPE_AABB => c2AABBtoCapsule(*(B as *const c2AABB), *(A as *const c2Capsule)),
            C2_TYPE_CAPSULE => {
                c2CapsuletoCapsule(*(A as *const c2Capsule), *(B as *const c2Capsule))
            }
            _ => 0,
        },
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Public entry point declared in include/lib.h
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
            &mut circle as *mut c2Circle as *const c2void,
            C2_TYPE_CIRCLE,
            &mut circle_in as *mut c2Circle as *const c2void,
            C2_TYPE_CIRCLE,
        );

        result += c2Collided(
            &mut aabb as *mut c2AABB as *const c2void,
            C2_TYPE_AABB,
            &mut circle_in as *mut c2Circle as *const c2void,
            C2_TYPE_CIRCLE,
        ) << 1;

        result += c2Collided(
            &mut capsule as *mut c2Capsule as *const c2void,
            C2_TYPE_CAPSULE,
            &mut circle_in as *mut c2Circle as *const c2void,
            C2_TYPE_CIRCLE,
        ) << 2;
    }

    result
}
