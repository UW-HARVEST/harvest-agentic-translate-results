//! Rust translation of `c_src/src/lib.c` (a cute_c2 / tinyc2 derived 2D
//! collision library).
//!
//! Every non-`static` function of the C translation unit has external linkage
//! and is therefore exported by the C shared object; every one of them is
//! re-exported here with the exact same linker name, signature and
//! (bit-for-bit) behaviour.
//!
//! # Fidelity notes
//!
//! * All arithmetic is `f32` and is performed in exactly the same order as the
//!   C source.
//! * The C library is built by `c_src/CMakeLists.txt` without any
//!   `CMAKE_BUILD_TYPE`, i.e. at `-O0`, so gcc emits one scalar SSE instruction
//!   per source-level operation with no reassociation and no FMA contraction.
//!   Consequently the *only* place where a naive Rust transcription can differ
//!   is NaN payload/sign propagation, which on x86 SSE is
//!   **destination-operand-wins** (see [`ssf`]). The operand that ends up in
//!   the destination register is not always the left-hand side of the C
//!   expression (gcc `-O0` picks it from its evaluation order), so each
//!   operation below records the order taken from
//!   `objdump -d` of the reference `.so`.
//! * Quirks / bugs of the original are preserved verbatim: the nonsensical
//!   `metric < -1.0e8f` cache-validation conjunct, the missing `default:` in
//!   `c2MakeProxy`, the argument swapping in `c2Collided`, `c2L`'s `case 3`
//!   falling into `default:`, `c2Support`'s unconditional `verts[0]` read, and
//!   `c2GJKSimplexMetric`'s `default:` falling through into `case 1:`.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// Constants used by the original source (spelled out there as literals).
// ---------------------------------------------------------------------------

/// `3.40282346638528859811704183484516925e+38F` (FLT_MAX), `.rodata` 0x7f7fffff
const C2_FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;
/// `1.19209289550781250000000000000000000e-7F` (FLT_EPSILON), `.rodata` 0x34000000
const C2_EPSILON: f32 = 1.192_092_895_507_812_5e-7;
/// gcc constant-folds `FLT_EPSILON * FLT_EPSILON` to `.rodata` 0x28800000.
const C2_EPSILON_SQ: f32 = f32::from_bits(0x2880_0000);

// ---------------------------------------------------------------------------
// x86 SSE scalar-arithmetic emulation (NaN propagation fidelity)
// ---------------------------------------------------------------------------
//
// For `ADDSS/SUBSS/MULSS/DIVSS xmm_dst, xmm_src`:
//   * if `dst` is a NaN, the result is `dst` with the quiet bit forced on;
//   * else if `src` is a NaN, the result is `src` with the quiet bit forced on;
//   * otherwise the IEEE-754 result (which for an invalid operation such as
//     `inf - inf`, `0 * inf` or `0 / 0` is the "real indefinite" QNaN
//     `0xffc00000`).
//
// Rust's `a op b` lowers to the same instruction but LLVM is free to choose
// which side lands in the destination register, so the NaN winner is not
// stable. Making the rule explicit removes that dependency on codegen while
// leaving the numeric result untouched for every non-NaN input.

/// Force the quiet bit of a NaN on (identity for a QNaN, quietens an SNaN),
/// preserving sign and payload — exactly what SSE does.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// Which operand's NaN an SSE arithmetic instruction returns, if any.
#[inline(always)]
fn ssf(dst: f32, src: f32) -> Option<f32> {
    if dst.is_nan() {
        Some(quiet(dst))
    } else if src.is_nan() {
        Some(quiet(src))
    } else {
        None
    }
}

/// `ADDSS dst, src`
#[inline(always)]
fn addss(dst: f32, src: f32) -> f32 {
    match ssf(dst, src) {
        Some(v) => v,
        None => dst + src,
    }
}

/// `SUBSS dst, src`
#[inline(always)]
fn subss(dst: f32, src: f32) -> f32 {
    match ssf(dst, src) {
        Some(v) => v,
        None => dst - src,
    }
}

/// `MULSS dst, src`
#[inline(always)]
fn mulss(dst: f32, src: f32) -> f32 {
    match ssf(dst, src) {
        Some(v) => v,
        None => dst * src,
    }
}

/// `DIVSS dst, src`
#[inline(always)]
fn divss(dst: f32, src: f32) -> f32 {
    match ssf(dst, src) {
        Some(v) => v,
        None => dst / src,
    }
}

/// `XORPS x, signmask` — gcc materialises C's unary `-` on a float as a plain
/// sign-bit flip, which (unlike an arithmetic negation) never quietens an SNaN.
#[inline(always)]
fn fneg(x: f32) -> f32 {
    f32::from_bits(x.to_bits() ^ 0x8000_0000)
}

/// `SQRTSS` — the C calls `sqrtf` from libm, which on x86-64 is `sqrtss`.
#[inline(always)]
fn sqrtss(x: f32) -> f32 {
    x.sqrt()
}

// ---------------------------------------------------------------------------
// C2_TYPE
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Public POD types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

/// ```c
/// typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

/// ```c
/// typedef struct { c2v sA; c2v sB; c2v p; float u; int iA; int iB; } c2sv;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// ```c
/// typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;
/// ```
///
/// The four `c2sv` members are stored as a fixed size array: a `#[repr(C)]`
/// struct with four consecutive members of the same type has exactly the same
/// layout as `[c2sv; 4]`, and the C code itself relies on that by walking
/// `c2sv *verts = &s->a;` as an array.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Layout assertions - these mirror what the C compiler reports for the same
// declarations (verified with sizeof/_Alignof/offsetof against c_src).
// ---------------------------------------------------------------------------

const _: () = {
    use std::mem::{align_of, offset_of, size_of};
    assert!(size_of::<c2v>() == 8 && align_of::<c2v>() == 4);
    assert!(size_of::<c2r>() == 8 && align_of::<c2r>() == 4);
    assert!(size_of::<c2x>() == 16 && align_of::<c2x>() == 4);
    assert!(size_of::<c2Circle>() == 12 && align_of::<c2Circle>() == 4);
    assert!(size_of::<c2AABB>() == 16 && align_of::<c2AABB>() == 4);
    assert!(size_of::<c2Capsule>() == 20 && align_of::<c2Capsule>() == 4);

    assert!(size_of::<c2GJKCache>() == 36 && align_of::<c2GJKCache>() == 4);
    assert!(offset_of!(c2GJKCache, metric) == 0);
    assert!(offset_of!(c2GJKCache, count) == 4);
    assert!(offset_of!(c2GJKCache, iA) == 8);
    assert!(offset_of!(c2GJKCache, iB) == 20);
    assert!(offset_of!(c2GJKCache, div) == 32);

    assert!(size_of::<c2Proxy>() == 72 && align_of::<c2Proxy>() == 4);
    assert!(offset_of!(c2Proxy, radius) == 0);
    assert!(offset_of!(c2Proxy, count) == 4);
    assert!(offset_of!(c2Proxy, verts) == 8);

    assert!(size_of::<c2sv>() == 36 && align_of::<c2sv>() == 4);
    assert!(offset_of!(c2sv, sA) == 0);
    assert!(offset_of!(c2sv, sB) == 8);
    assert!(offset_of!(c2sv, p) == 16);
    assert!(offset_of!(c2sv, u) == 24);
    assert!(offset_of!(c2sv, iA) == 28);
    assert!(offset_of!(c2sv, iB) == 32);

    // `verts[0..4]` must land exactly on the C members `a`, `b`, `c`, `d`.
    assert!(size_of::<c2Simplex>() == 152 && align_of::<c2Simplex>() == 4);
    assert!(offset_of!(c2Simplex, verts) == 0);
    assert!(offset_of!(c2Simplex, div) == 144);
    assert!(offset_of!(c2Simplex, count) == 148);
};

// ---------------------------------------------------------------------------
// Small vector helpers (each one is also a public C symbol)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

/// ```c
/// a.x *= b; a.y *= b;
/// ```
/// gcc: `mulss` with the *vector component* in the destination.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
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

/// ```c
/// a.x -= b.x; a.y -= b.y;
/// ```
/// gcc: `subss` with `a`'s component in the destination.
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x = subss(a.x, b.x);
    a.y = subss(a.y, b.y);
    a
}

/// ```c
/// return a.x * b.x + a.y * b.y;
/// ```
/// gcc `-O0`:
/// ```text
/// mulss %xmm0,%xmm1   ; xmm1 = a.x (dst) * b.x
/// mulss %xmm2,%xmm0   ; xmm0 = b.y (dst) * a.y
/// addss %xmm1,%xmm0   ; xmm0 = (a.y*b.y) (dst) + (a.x*b.x)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    let t_x = mulss(a.x, b.x);
    let t_y = mulss(b.y, a.y);
    addss(t_y, t_x)
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

/// `sqrtf(c2Dot(a, a))`
#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    sqrtss(c2Dot(a, a))
}

/// ```c
/// return a.x * b.y - a.y * b.x;
/// ```
/// gcc `-O0`:
/// ```text
/// mulss %xmm1,%xmm0   ; xmm0 = b.y (dst) * a.x
/// mulss %xmm2,%xmm1   ; xmm1 = b.x (dst) * a.y
/// subss %xmm1,%xmm0   ; xmm0 = (a.x*b.y) (dst) - (a.y*b.x)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    let t0 = mulss(b.y, a.x);
    let t1 = mulss(b.x, a.y);
    subss(t0, t1)
}

/// ```c
/// return c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y);
/// ```
/// gcc `-O0` (arguments evaluated right-to-left):
/// ```text
/// ; second argument
/// mulss %xmm0,%xmm1   ; xmm1 = a.s (dst) * b.x
/// mulss %xmm2,%xmm0   ; xmm0 = b.y (dst) * a.c
/// addss %xmm0,%xmm3   ; xmm3 = (a.s*b.x) (dst) + (a.c*b.y)
/// ; first argument
/// mulss %xmm1,%xmm0   ; xmm0 = b.x (dst) * a.c
/// mulss %xmm2,%xmm1   ; xmm1 = b.y (dst) * a.s
/// subss %xmm1,%xmm0   ; xmm0 = (a.c*b.x) (dst) - (a.s*b.y)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    let y = addss(mulss(a.s, b.x), mulss(b.y, a.c));
    let x = subss(mulss(b.x, a.c), mulss(b.y, a.s));
    c2V(x, y)
}

/// ```c
/// a.x += b.x; a.y += b.y;
/// ```
/// gcc `-O0` puts **`b`'s** component in the destination:
/// ```text
/// addss %xmm1,%xmm0   ; xmm0 = b.x (dst) + a.x
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x = addss(b.x, a.x);
    a.y = addss(b.y, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(fneg(a.x), fneg(a.y))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = fneg(a.y);
    b.y = a.x;
    b
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = a.y;
    b.y = fneg(a.x);
    b
}

/// ```c
/// return c2Mulvs(a, 1.0f / b);
/// ```
/// gcc: `divss` with the literal `1.0f` in the destination.
#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, divss(1.0f32, b))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

/// ```c
/// return c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y);
/// ```
/// gcc `-O0`:
/// ```text
/// ; second argument
/// xorps %xmm0,%xmm1   ; xmm1 = -a.s (plain sign flip)
/// mulss %xmm0,%xmm1   ; xmm1 = (-a.s) (dst) * b.x
/// mulss %xmm2,%xmm0   ; xmm0 = b.y (dst) * a.c
/// addss %xmm0,%xmm3   ; xmm3 = ((-a.s)*b.x) (dst) + (a.c*b.y)
/// ; first argument
/// mulss %xmm0,%xmm1   ; xmm1 = a.c (dst) * b.x
/// mulss %xmm2,%xmm0   ; xmm0 = b.y (dst) * a.s
/// addss %xmm0,%xmm1   ; xmm1 = (a.c*b.x) (dst) + (a.s*b.y)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    let y = addss(mulss(fneg(a.s), b.x), mulss(b.y, a.c));
    let x = addss(mulss(a.c, b.x), mulss(b.y, a.s));
    c2V(x, y)
}

// ---------------------------------------------------------------------------
// Proxies
// ---------------------------------------------------------------------------

/// ```c
/// void c2BBVerts(c2v *out, c2AABB *bb) {
///     out[0] = bb->min;
///     out[1] = c2V(bb->max.x, bb->min.y);
///     out[2] = bb->max;
///     out[3] = c2V(bb->min.x, bb->max.y);
/// }
/// ```
///
/// Every `bb->` load in the C happens *after* the preceding `out[...]` store, so
/// a caller whose `out` buffer overlaps `*bb` observes the partially updated
/// box — which is perfectly well defined C and is reachable in practice
/// (`c2MakeProxy` passes `p->verts` as `out`, and `p` itself as `bb` would
/// overlap). Each field is therefore re-read from the raw pointer at exactly the
/// point the C reads it, and no `&`/`&mut` reference to `*bb` is ever created,
/// so the aliasing stays legal on the Rust side too.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        use std::ptr::{addr_of, read};
        *out.add(0) = read(addr_of!((*bb).min));
        *out.add(1) = c2V(read(addr_of!((*bb).max.x)), read(addr_of!((*bb).min.y)));
        *out.add(2) = read(addr_of!((*bb).max));
        *out.add(3) = c2V(read(addr_of!((*bb).min.x)), read(addr_of!((*bb).max.y)));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2MakeProxy(shape: *const c_void, type_: c_int, p: *mut c2Proxy) {
    unsafe {
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
            // The C switch has no `default:` label - nothing is written.
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

fn gjk_simplex_metric(s: &c2Simplex) -> f32 {
    match s.count {
        // `default:` falls through into `case 1:` in the C source.
        2 => c2Len(c2Sub(s.verts[1].p, s.verts[0].p)),
        3 => c2Det2(
            c2Sub(s.verts[1].p, s.verts[0].p),
            c2Sub(s.verts[2].p, s.verts[0].p),
        ),
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
        s.div = addss(u, vv);
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
    // gcc: `mulss` with the `c2Det2` result in the destination, `area` as src.
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
        s.div = addss(uAB, vAB);
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
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe { simplex3(&mut *s) }
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
        // `case 3:` and `default:` both give the zero vector.
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe { direction(&*s) }
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

/// gcc emits `den * s->X.u` as `mulss` with **`u`** in the destination.
#[inline(always)]
fn wu(u: f32, den: f32) -> f32 {
    mulss(u, den)
}

/// `pA.verts[i]` where `i` comes from a caller-supplied `c2GJKCache`.
///
/// The C code indexes `c2v verts[8]` with the raw cached index, so anything
/// outside `[0, 8)` is an out-of-bounds read and anything in
/// `[proxy.count, 8)` is an *uninitialised* read (see `ERRORS.md` rows U4/U2).
/// Neither is a value the C source defines, and it is not reproducible, so the
/// Rust side stays memory-safe: in-range slots read the (zero-initialised)
/// array exactly like the C reads its slots, and an out-of-range index yields
/// `(0, 0)` instead of faulting. Every index the library itself ever stores
/// into a cache is `< proxy.count`, so this is unreachable for any cache that
/// was produced by `c2GJK`.
#[inline(always)]
fn proxy_vert(p: &c2Proxy, i: c_int) -> c2v {
    if i >= 0 && (i as usize) < p.verts.len() {
        p.verts[i as usize]
    } else {
        c2v { x: 0.0, y: 0.0 }
    }
}

fn witness(s: &c2Simplex) -> (c2v, c2v) {
    let den = divss(1.0f32, s.div);
    match s.count {
        1 => (s.verts[0].sA, s.verts[0].sB),
        2 => (
            c2Add(
                c2Mulvs(s.verts[0].sA, wu(s.verts[0].u, den)),
                c2Mulvs(s.verts[1].sA, wu(s.verts[1].u, den)),
            ),
            c2Add(
                c2Mulvs(s.verts[0].sB, wu(s.verts[0].u, den)),
                c2Mulvs(s.verts[1].sB, wu(s.verts[1].u, den)),
            ),
        ),
        3 => (
            c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sA, wu(s.verts[0].u, den)),
                    c2Mulvs(s.verts[1].sA, wu(s.verts[1].u, den)),
                ),
                c2Mulvs(s.verts[2].sA, wu(s.verts[2].u, den)),
            ),
            c2Add(
                c2Add(
                    c2Mulvs(s.verts[0].sB, wu(s.verts[0].u, den)),
                    c2Mulvs(s.verts[1].sB, wu(s.verts[1].u, den)),
                ),
                c2Mulvs(s.verts[2].sB, wu(s.verts[2].u, den)),
            ),
        ),
        _ => (c2V(0.0, 0.0), c2V(0.0, 0.0)),
    }
}

/// ```c
/// void c2Witness(c2Simplex *s, c2v *a, c2v *b) {
///     float den = 1.0f / s->div;
///     switch (s->count) { case 1: *a = s->a.sA; *b = s->a.sB; break; ... }
/// }
/// ```
///
/// The C stores `*a` **before** it evaluates the `*b` expression, so a caller
/// that points `a` into `*s` (legal C, e.g. `&s->a.sB`) makes the `*b`
/// computation observe the already-stored value. The internal GJK call site uses
/// two disjoint locals and so is unaffected, but the exported entry point has to
/// interleave the stores exactly like the C. All reads go through raw pointers;
/// no reference to `*s` is created, so the aliasing is legal on the Rust side.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        use std::ptr::{addr_of, read};
        macro_rules! g {
            ($i:expr, $f:ident) => {
                read(addr_of!((*s).verts[$i].$f))
            };
        }
        // `den` is computed from `s->div` before the switch, i.e. before any
        // store through `a` or `b` could disturb it.
        let den = divss(1.0f32, read(addr_of!((*s).div)));
        match read(addr_of!((*s).count)) {
            1 => {
                *a = g!(0, sA);
                *b = g!(0, sB);
            }
            2 => {
                *a = c2Add(
                    c2Mulvs(g!(0, sA), wu(g!(0, u), den)),
                    c2Mulvs(g!(1, sA), wu(g!(1, u), den)),
                );
                *b = c2Add(
                    c2Mulvs(g!(0, sB), wu(g!(0, u), den)),
                    c2Mulvs(g!(1, sB), wu(g!(1, u), den)),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs(g!(0, sA), wu(g!(0, u), den)),
                        c2Mulvs(g!(1, sA), wu(g!(1, u), den)),
                    ),
                    c2Mulvs(g!(2, sA), wu(g!(2, u), den)),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs(g!(0, sB), wu(g!(0, u), den)),
                        c2Mulvs(g!(1, sB), wu(g!(1, u), den)),
                    ),
                    c2Mulvs(g!(2, sB), wu(g!(2, u), den)),
                );
            }
            _ => {
                *a = c2V(0.0, 0.0);
                *b = c2V(0.0, 0.0);
            }
        }
    }
}

fn lerp_point(s: &c2Simplex) -> c2v {
    let den = divss(1.0f32, s.div);
    match s.count {
        1 => s.verts[0].p,
        2 => c2Add(
            c2Mulvs(s.verts[0].p, wu(s.verts[0].u, den)),
            c2Mulvs(s.verts[1].p, wu(s.verts[1].u, den)),
        ),
        _ => c2V(0.0, 0.0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe { lerp_point(&*s) }
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
        let mut cache_was_read = 0;
        if !cache.is_null() {
            let cache_was_good = (*cache).count != 0;
            if cache_was_good {
                let n = (*cache).count;
                // The C loop is `for (i = 0; i < cache->count; ++i)` and both
                // `cache->iA[i]` and `s.verts[i]` are 3/4-element arrays, so a
                // `count` above 4 is out-of-bounds (documented UB, see
                // ERRORS.md U2). Clamp to the storage the C struct actually
                // has so the Rust side stays memory-safe.
                let bound = if n > 3 { 3 } else { n };
                let mut i: c_int = 0;
                while i < bound {
                    let idx = i as usize;
                    let iA = (*cache).iA[idx];
                    let iB = (*cache).iB[idx];
                    let sA = c2Mulxv(ax, proxy_vert(&pA, iA));
                    let sB = c2Mulxv(bx, proxy_vert(&pB, iB));
                    let vtx = &mut s.verts[idx];
                    vtx.iA = iA;
                    vtx.sA = sA;
                    vtx.iB = iB;
                    vtx.sB = sB;
                    vtx.p = c2Sub(vtx.sB, vtx.sA);
                    vtx.u = 0.0;
                    i += 1;
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
                // gcc folds `max_metric * 2.0f` into `addss %xmm0,%xmm0`;
                // `mulss(max_metric, 2.0)` is bit-identical for every input
                // (same value, same NaN winner).
                if !(min_metric < mulss(max_metric, 2.0f32) && metric < -1.0e8f32) {
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

        let mut saveA: [c_int; 4] = [0; 4];
        let mut saveB: [c_int; 4] = [0; 4];
        let mut save_count: c_int;
        let mut d0: f32 = C2_FLT_MAX;
        let mut d1: f32;
        let mut iter: c_int = 0;
        let mut hit: c_int = 0;
        while iter < 20 {
            save_count = s.count;
            // `int saveA[3]` in C; `s.count` is at most 3 for any simplex this
            // library can build, so the clamp is never observable.
            let sc = if save_count > 4 { 4 } else { save_count };
            let mut i: c_int = 0;
            while i < sc {
                saveA[i as usize] = s.verts[i as usize].iA;
                saveB[i as usize] = s.verts[i as usize].iB;
                i += 1;
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

            let p = lerp_point(&s);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;

            let d = direction(&s);
            if c2Dot(d, d) < C2_EPSILON_SQ {
                break;
            }

            let iA = c2Support(pA.verts.as_ptr(), pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, proxy_vert(&pA, iA));
            let iB = c2Support(pB.verts.as_ptr(), pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, proxy_vert(&pB, iB));
            // `c2sv *v = verts + s.count;` — `s.count` is provably in `1..=2`
            // at this point (a 3-simplex `break`s above with `hit`, and any
            // `count >= 4` coming from a cache `break`s on the epsilon test
            // during the first iteration), so this is always in bounds. The
            // `get_mut` keeps that guarantee enforced rather than assumed.
            if let Some(vtx) = s.verts.get_mut(s.count as usize) {
                vtx.iA = iA;
                vtx.sA = sA;
                vtx.iB = iB;
                vtx.sB = sB;
                vtx.p = c2Sub(vtx.sB, vtx.sA);
            }

            let mut dup = 0;
            let mut i: c_int = 0;
            while i < sc {
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

        let (mut a, mut b) = witness(&s);
        let mut dist = c2Len(c2Sub(a, b));
        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            // gcc: `addss` with `rA` in the destination.
            let rsum = addss(rA, rB);
            if dist > rsum && dist > C2_EPSILON {
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
            (*cache).metric = gjk_simplex_metric(&s);
            (*cache).count = s.count;
            let n = if s.count > 3 { 3 } else { s.count };
            let mut i: c_int = 0;
            while i < n {
                let idx = i as usize;
                (*cache).iA[idx] = s.verts[idx].iA;
                (*cache).iB[idx] = s.verts[idx].iB;
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

// ---------------------------------------------------------------------------
// Boolean shape tests
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

/// gcc: `r2 = A.r + B.r` becomes `addss` with **`B.r`** in the destination.
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = addss(B.r, A.r);
    r2 = mulss(r2, r2);
    (d2 < r2) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = mulss(A.r, A.r);
    (d2 < r2) as c_int
}

/// gcc: `r = A.r + B.r` becomes `addss` with **`B.r`** in the destination;
/// `da / c2Dot(n, n)` is `divss` with `da` in the destination.
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
            let e = c2Sub(ap, c2Mulvs(n, divss(da, c2Dot(n, n))));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(A.p, B.b);
            d2 = c2Dot(bp, bp);
        }
    }
    let r = addss(B.r, A.r);
    (d2 < mulss(r, r)) as c_int
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
// Public entry point declared in include/lib.h
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
            &aabb_in as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
        );

        result += c2Collided(
            &aabb as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
            &aabb_in as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
        ) << 1;

        result += c2Collided(
            &capsule as *const c2Capsule as *const c_void,
            C2_TYPE_CAPSULE,
            &aabb_in as *const c2AABB as *const c_void,
            C2_TYPE_AABB,
        ) << 2;
    }

    result
}
