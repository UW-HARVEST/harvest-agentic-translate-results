//! Rust translation of `c_src/src/lib.c` — a cute_c2-style 2D GJK distance
//! solver, built as a C shared library.
//!
//! This crate reproduces the **complete public ABI** of the C library (all 31
//! exported symbols) and is bit-for-bit behaviour compatible with it,
//! including its quirks and bugs.
//!
//! # Why the arithmetic looks the way it does
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference library
//! is compiled with `C_FLAGS = -fPIC` and **no optimisation at all** (`-O0`).
//! It is built as a shared object without `-fvisibility=hidden` and with no
//! `static` functions, so every helper is interposable; at `-O0` GCC inlines
//! nothing anyway and the disassembly shows every single call (`c2V`, `c2Dot`,
//! `c2Sub`, `c2Add`, …) going through the PLT. That means the library's entire
//! floating-point behaviour is determined by a small, enumerable set of SSE
//! scalar/packed instructions inside the leaf functions.
//!
//! For every *finite* input, IEEE-754 makes those results uniquely determined,
//! so a straightforward translation is already bit-exact. The one place where
//! "obvious" Rust diverges is **which NaN survives** an operation: `addss`,
//! `mulss`, `subss` and `divss` return the *destination* operand's NaN in
//! preference to the source operand's, and GCC's `-O0` register allocator
//! frequently ends up with the *right* C operand in the destination register
//! (it loads the two operands in source order into scratch registers and then
//! picks whichever it happens to hold). Because a NaN's sign bit and payload
//! are then observable in the output, this module routes all arithmetic through
//! `add_l`/`add_r`/`mul_l`/`mul_r`/`sub_l`/`div_l` helpers that pin the
//! destination operand to exactly what GCC emitted at each site.
//!
//! Every choice below is annotated with the actual `-O0` instruction from
//! `objdump -d` of the reference `.so` (GCC 11.5.0, x86-64). Do not "simplify"
//! them back to source order — several sites are deliberately `_r`.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr::{addr_of, addr_of_mut, null, null_mut};

// ---------------------------------------------------------------------------
// Constants, spelled out as literals in the C source
// ---------------------------------------------------------------------------

/// `FLT_MAX`
const FLT_MAX: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38_f32;
/// `FLT_EPSILON`
const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7_f32;

// ---------------------------------------------------------------------------
// SSE-exact scalar arithmetic
//
// Intel SDM, ADDSS/SUBSS/MULSS/DIVSS `xmm1, xmm2`:
//   * if the destination operand (`xmm1`) is NaN, the result is that NaN,
//     quieted (SNaN -> QNaN by setting the mantissa MSB); sign and payload
//     are otherwise preserved;
//   * otherwise, if the source operand is NaN, the result is that NaN,
//     quieted;
//   * otherwise the IEEE-754 operation is performed (which itself may raise
//     #I and produce the "QNaN indefinite" for e.g. 0*Inf or Inf-Inf — that
//     case is order independent, so plain Rust arithmetic reproduces it).
//
// `_l` means the *left* argument was GCC's destination operand; `_r` means
// GCC commuted the instruction so the *right* argument is the destination.
// `SUBSS`/`DIVSS` are not commutative, so only the `_l` form exists.
// ---------------------------------------------------------------------------

/// Quiet a NaN exactly the way an SSE arithmetic instruction does: set the
/// mantissa MSB, leave sign and the rest of the payload untouched.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// IEEE-754 negation: a pure sign-bit flip (`xorps` against `0x80000000`),
/// which never quiets a NaN. This is what GCC emits for `-x` in `c2Neg`,
/// `c2Skew` and `c2CCW90`.
#[inline(always)]
fn fneg(x: f32) -> f32 {
    f32::from_bits(x.to_bits() ^ 0x8000_0000)
}

/// `addss a, b` — destination is `a`.
#[inline(always)]
fn add_l(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a + b
    }
}

/// `addss b, a` — destination is `b` (GCC commuted the operands).
#[inline(always)]
fn add_r(a: f32, b: f32) -> f32 {
    if b.is_nan() {
        quiet(b)
    } else if a.is_nan() {
        quiet(a)
    } else {
        a + b
    }
}

/// `subss a, b` — destination is `a` (subtraction is never commuted).
#[inline(always)]
fn sub_l(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a - b
    }
}

/// `mulss a, b` — destination is `a`.
#[inline(always)]
fn mul_l(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a * b
    }
}

/// `mulss b, a` — destination is `b` (GCC commuted the operands).
#[inline(always)]
fn mul_r(a: f32, b: f32) -> f32 {
    if b.is_nan() {
        quiet(b)
    } else if a.is_nan() {
        quiet(a)
    } else {
        a * b
    }
}

/// `divss a, b` — destination is `a` (division is never commuted).
#[inline(always)]
fn div_l(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a / b
    }
}

// ---------------------------------------------------------------------------
// C2_TYPE — an unfixed C enum, so `int`-sized in the ABI
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Structures — layouts mirror the C definitions exactly
// ---------------------------------------------------------------------------

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2r { float c; float s; } c2r;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

/// `typedef struct c2x { c2v p; c2r r; } c2x;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

/// `typedef struct c2Capsule { c2v a; c2v b; float r; } c2Capsule;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

/// ```c
/// typedef struct c2GJKCache {
///         float metric; int count; int iA[3]; int iB[3]; float div;
/// } c2GJKCache;
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

/// `typedef struct { float radius; int count; c2v verts[8]; } c2Proxy;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

/// `typedef struct { c2v sA; c2v sB; c2v p; float u; int iA; int iB; } c2sv;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
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
/// The four `c2sv` members `a`, `b`, `c`, `d` are modelled as one array so the
/// C idiom `c2sv *verts = &s->a; verts[i]` can be reproduced verbatim. The
/// memory layout is byte-identical to the C struct.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Simplex {
    /// `a`, `b`, `c`, `d`
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// Compile-time layout assertions guaranteeing ABI equality with the C structs.
const _: () = {
    assert!(core::mem::size_of::<c2v>() == 8 && core::mem::align_of::<c2v>() == 4);
    assert!(core::mem::size_of::<c2r>() == 8);
    assert!(core::mem::size_of::<c2x>() == 16);
    assert!(core::mem::size_of::<c2Circle>() == 12);
    assert!(core::mem::size_of::<c2AABB>() == 16);
    assert!(core::mem::size_of::<c2Capsule>() == 20);
    assert!(core::mem::size_of::<c2GJKCache>() == 36);
    assert!(core::mem::size_of::<c2Proxy>() == 72);
    assert!(core::mem::size_of::<c2sv>() == 36);
    assert!(core::mem::size_of::<c2Simplex>() == 152);
};

const ZERO_V: c2v = c2v { x: 0.0, y: 0.0 };

impl c2Simplex {
    #[inline]
    const fn zeroed() -> c2Simplex {
        const ZSV: c2sv = c2sv {
            sA: ZERO_V,
            sB: ZERO_V,
            p: ZERO_V,
            u: 0.0,
            iA: 0,
            iB: 0,
        };
        c2Simplex {
            verts: [ZSV; 4],
            div: 0.0,
            count: 0,
        }
    }
}

impl c2Proxy {
    #[inline]
    const fn zeroed() -> c2Proxy {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [ZERO_V; 8],
        }
    }
}

// ---------------------------------------------------------------------------
// Vector / rotation helpers
// ---------------------------------------------------------------------------

/// `c2v c2V(float x, float y)`
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = ZERO_V;
    a.x = x;
    a.y = y;
    a
}

/// `c2v c2Mulvs(c2v a, float b)`
///
/// ```text
/// 1314  movss -0x8(%rbp),%xmm0    ; a.x
/// 1319  mulss -0xc(%rbp),%xmm0    ; dst = a.x, src = b (memory)
/// 1323  movss -0x4(%rbp),%xmm0    ; a.y
/// 1328  mulss -0xc(%rbp),%xmm0    ; dst = a.y, src = b (memory)
/// ```
///
/// A memory operand can only ever be the *source* of `mulss`, so at `-O0` both
/// destinations are the vector component: `a`'s NaN wins over `b`'s.
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    let mut a = a;
    a.x = mul_l(a.x, b);
    a.y = mul_l(a.y, b);
    a
}

/// `c2v c2Maxv(c2v a, c2v b)`
///
/// At `-O0` GCC emits an explicit `comiss` + `jbe` pair (`133d`..`1384`), i.e.
/// literally `a > b ? a : b`, and an unordered compare takes the `jbe` and so
/// yields `b`. Rust's `>` is also false for NaN, so the direct translation is
/// exact. Do **not** use `f32::max` here — it returns the non-NaN operand.
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

/// `c2v c2Minv(c2v a, c2v b)`
///
/// `-O0` reverses the compare into `b > a ? a : b` (`13b5 comiss %xmm1,%xmm0`
/// with `xmm0 = b`, `jbe` -> `b`). For ordered operands that is identical to
/// `a < b ? a : b`, and for unordered operands both spellings yield `b`, so
/// this mirrors `c2Maxv` exactly.
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

/// `c2v c2Clampv(c2v a, c2v lo, c2v hi)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

/// `c2v c2Sub(c2v a, c2v b)`
///
/// ```text
/// 1455  movss -0x8(%rbp),%xmm0    ; a.x
/// 145a  movss -0x10(%rbp),%xmm1   ; b.x
/// 145f  subss %xmm1,%xmm0         ; dst = a.x
/// 1468  movss -0x4(%rbp),%xmm0    ; a.y
/// 146d  movss -0xc(%rbp),%xmm1    ; b.y
/// 1472  subss %xmm1,%xmm0         ; dst = a.y
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = sub_l(a.x, b.x);
    a.y = sub_l(a.y, b.y);
    a
}

/// `c2v c2Add(c2v a, c2v b)`
///
/// ```text
/// 1852  movss -0x8(%rbp),%xmm1    ; a.x
/// 1857  movss -0x10(%rbp),%xmm0   ; b.x
/// 185c  addss %xmm1,%xmm0         ; dst = b.x  <- NOT a.x
/// 1865  movss -0x4(%rbp),%xmm1    ; a.y
/// 186a  movss -0xc(%rbp),%xmm0    ; b.y
/// 186f  addss %xmm1,%xmm0         ; dst = b.y  <- NOT a.y
/// ```
///
/// Unlike `c2Sub` (where `subss` cannot be commuted) GCC loaded `b` into the
/// accumulator, so `b`'s NaN wins over `a`'s in `c2Add`.
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x = add_r(a.x, b.x);
    a.y = add_r(a.y, b.y);
    a
}

/// `float c2Dot(c2v a, c2v b)` -> `a.x * b.x + a.y * b.y`
///
/// ```text
/// 1494  movss -0x8(%rbp),%xmm1    ; a.x
/// 1499  movss -0x10(%rbp),%xmm0   ; b.x
/// 149e  mulss %xmm0,%xmm1         ; dst = a.x   -> mul_l
/// 14a2  movss -0x4(%rbp),%xmm2    ; a.y
/// 14a7  movss -0xc(%rbp),%xmm0    ; b.y
/// 14ac  mulss %xmm2,%xmm0         ; dst = b.y   -> mul_r
/// 14b0  addss %xmm1,%xmm0         ; dst = the y term -> add_r
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    add_r(mul_l(a.x, b.x), mul_r(a.y, b.y))
}

/// `float c2Det2(c2v a, c2v b)` -> `a.x * b.y - a.y * b.x`
///
/// ```text
/// 16ec  movss -0x8(%rbp),%xmm1    ; a.x
/// 16f1  movss -0xc(%rbp),%xmm0    ; b.y
/// 16f6  mulss %xmm1,%xmm0         ; dst = b.y   -> mul_r
/// 16fa  movss -0x4(%rbp),%xmm2    ; a.y
/// 16ff  movss -0x10(%rbp),%xmm1   ; b.x
/// 1704  mulss %xmm2,%xmm1         ; dst = b.x   -> mul_r
/// 1708  subss %xmm1,%xmm0         ; dst = the x*y term -> sub_l
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> f32 {
    sub_l(mul_r(a.x, b.y), mul_r(a.y, b.x))
}

/// `c2r c2RotIdentity(void)`
#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    let mut r = c2r { c: 0.0, s: 0.0 };
    r.c = 1.0f32;
    r.s = 0.0;
    r
}

/// `c2x c2xIdentity(void)`
#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    let mut x = c2x {
        p: ZERO_V,
        r: c2r { c: 0.0, s: 0.0 },
    };
    x.p = c2V(0.0, 0.0);
    x.r = c2RotIdentity();
    x
}

/// `c2v c2Mulrv(c2r a, c2v b)`
/// -> `c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)`
///
/// `-O0` evaluates the `y` component first, then the `x` component:
///
/// ```text
/// 17e5  movss -0x4(%rbp),%xmm1    ; a.s
/// 17ea  movss -0x10(%rbp),%xmm0   ; b.x
/// 17ef  mulss %xmm0,%xmm1         ; dst = a.s      -> mul_l(a.s, b.x)
/// 17f3  movss -0x8(%rbp),%xmm2    ; a.c
/// 17f8  movss -0xc(%rbp),%xmm0    ; b.y
/// 17fd  mulss %xmm2,%xmm0         ; dst = b.y      -> mul_r(a.c, b.y)
/// 1801  movaps %xmm1,%xmm3
/// 1804  addss %xmm0,%xmm3         ; dst = a.s*b.x  -> add_l   (y)
/// 1808  movss -0x8(%rbp),%xmm1    ; a.c
/// 180d  movss -0x10(%rbp),%xmm0   ; b.x
/// 1812  mulss %xmm1,%xmm0         ; dst = b.x      -> mul_r(a.c, b.x)
/// 1816  movss -0x4(%rbp),%xmm2    ; a.s
/// 181b  movss -0xc(%rbp),%xmm1    ; b.y
/// 1820  mulss %xmm2,%xmm1         ; dst = b.y      -> mul_r(a.s, b.y)
/// 1824  subss %xmm1,%xmm0         ; dst = a.c*b.x  -> sub_l   (x)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    c2V(
        sub_l(mul_r(a.c, b.x), mul_r(a.s, b.y)),
        add_l(mul_l(a.s, b.x), mul_r(a.c, b.y)),
    )
}

/// `c2v c2MulrvT(c2r a, c2v b)`
/// -> `c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)`
///
/// `-O0` does **not** fold `-x + y` into a `subss`; it materialises `-a.s` with
/// an `xorps` sign flip and then performs a real `addss` whose destination is
/// the *first* term. The `y` component is evaluated before the `x` component:
///
/// ```text
/// 26d1  movss -0x4(%rbp),%xmm0    ; a.s
/// 26d6  movss 0x1932(%rip),%xmm1  ; 0x80000000
/// 26de  xorps %xmm0,%xmm1         ; xmm1 = -a.s   (bitwise; never quiets)
/// 26e1  movss -0x10(%rbp),%xmm0   ; b.x
/// 26e6  mulss %xmm0,%xmm1         ; dst = -a.s    -> mul_l(-a.s, b.x)
/// 26ea  movss -0x8(%rbp),%xmm2    ; a.c
/// 26ef  movss -0xc(%rbp),%xmm0    ; b.y
/// 26f4  mulss %xmm2,%xmm0         ; dst = b.y     -> mul_r(a.c, b.y)
/// 26f8  movaps %xmm1,%xmm3
/// 26fb  addss %xmm0,%xmm3         ; dst = -a.s*b.x -> add_l  (y)
/// 26ff  movss -0x8(%rbp),%xmm1    ; a.c
/// 2704  movss -0x10(%rbp),%xmm0   ; b.x
/// 2709  mulss %xmm0,%xmm1         ; dst = a.c     -> mul_l(a.c, b.x)
/// 270d  movss -0x4(%rbp),%xmm2    ; a.s
/// 2712  movss -0xc(%rbp),%xmm0    ; b.y
/// 2717  mulss %xmm2,%xmm0         ; dst = b.y     -> mul_r(a.s, b.y)
/// 271b  addss %xmm0,%xmm1         ; dst = a.c*b.x -> add_l  (x)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    c2V(
        add_l(mul_l(a.c, b.x), mul_r(a.s, b.y)),
        add_l(mul_l(fneg(a.s), b.x), mul_r(a.c, b.y)),
    )
}

/// `c2v c2Mulxv(c2x a, c2v b)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

/// `c2v c2Neg(c2v a)` — `xorps` sign flip, no NaN quieting.
#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    c2V(fneg(a.x), fneg(a.y))
}

/// `c2v c2Skew(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = ZERO_V;
    b.x = fneg(a.y);
    b.y = a.x;
    b
}

/// `c2v c2CCW90(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = ZERO_V;
    b.x = a.y;
    b.y = fneg(a.x);
    b
}

/// `c2v c2Div(c2v a, float b)`
///
/// ```text
/// 2585  movss 0x1a73(%rip),%xmm0 ; 1.0f
/// 258d  divss -0xc(%rbp),%xmm0   ; dst = 1.0f -> div_l(1.0, b)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, div_l(1.0f32, b))
}

/// `c2v c2Norm(c2v a)`
#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

/// `float c2Len(c2v a)` -> `sqrtf(c2Dot(a, a))`
///
/// At `-O0` GCC does not expand the builtin at all — `16d7 call sqrtf@plt` goes
/// straight to libm. `f32::sqrt` lowers to `sqrtss`, and glibc's `sqrtf` is
/// `sqrtss` too, so the two agree bit-for-bit. They could only differ on a
/// *negative* argument (where glibc's compat wrapper may set `errno`), but the
/// argument is `c2Dot(a, a) = a.x*a.x + a.y*a.y`, a sum of squares, which is
/// never negative — it is `>= 0` or NaN.
#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    c2Dot(a, a).sqrt()
}

// ---------------------------------------------------------------------------
// Shape helpers
// ---------------------------------------------------------------------------

/// `void c2BBVerts(c2v *out, c2AABB *bb)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2BBVerts(out: *mut c2v, bb: *mut c2AABB) {
    unsafe {
        *out.add(0) = (*bb).min;
        *out.add(1) = c2V((*bb).max.x, (*bb).min.y);
        *out.add(2) = (*bb).max;
        *out.add(3) = c2V((*bb).min.x, (*bb).max.y);
    }
}

/// `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)`
///
/// As in the C original there is no `default:` arm, so for a `type` outside
/// `{0, 1, 2}` the proxy is left completely untouched.
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
                c2BBVerts(addr_of_mut!((*p).verts) as *mut c2v, bb);
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
}

// ---------------------------------------------------------------------------
// Simplex routines
// ---------------------------------------------------------------------------

/// `float c2GJKSimplexMetric(c2Simplex *s)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2GJKSimplexMetric(s: *mut c2Simplex) -> f32 {
    unsafe {
        let verts = addr_of!((*s).verts) as *const c2sv;
        match (*s).count {
            2 => c2Len(c2Sub((*verts.add(1)).p, (*verts.add(0)).p)),
            3 => c2Det2(
                c2Sub((*verts.add(1)).p, (*verts.add(0)).p),
                c2Sub((*verts.add(2)).p, (*verts.add(0)).p),
            ),
            // `default:` falls through into `case 1:` in the C source.
            _ => 0.0,
        }
    }
}

/// `void c22(c2Simplex *s)`
///
/// ```text
/// 1a3c  movss -0x14(%rbp),%xmm0  ; u
/// 1a41  addss -0x18(%rbp),%xmm0  ; dst = u -> add_l(u, v)
/// ```
///
/// Both `v <= 0` and `u <= 0` are compiled as `pxor`/`comiss`/`jb`, i.e. as
/// `!(0 < x)`; an unordered compare takes the `jb` and so falls through to the
/// next arm, exactly like Rust's `x <= 0.0` being `false` for NaN.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c22(s: *mut c2Simplex) {
    unsafe {
        let verts = addr_of_mut!((*s).verts) as *mut c2sv;
        let sa = verts.add(0);
        let sb = verts.add(1);

        let a = (*sa).p;
        let b = (*sb).p;
        let u = c2Dot(b, c2Sub(b, a));
        let v = c2Dot(a, c2Sub(a, b));
        if v <= 0.0 {
            (*sa).u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if u <= 0.0 {
            *sa = *sb;
            (*sa).u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else {
            (*sa).u = u;
            (*sb).u = v;
            (*s).div = add_l(u, v);
            (*s).count = 2;
        }
    }
}

/// `void c23(c2Simplex *s)`
///
/// At `-O0` every local lives in a stack slot, so each `addss`/`mulss` has the
/// memory operand as its *source* and the destination is always the operand
/// that GCC loaded into `xmm0` — which is uniformly the **left** operand of the
/// C expression:
///
/// ```text
/// 1c41  movss -0x2c(%rbp),%xmm1  ; area
/// 1c46  mulss %xmm1,%xmm0        ; dst = c2Det2(b,c)  -> mul_l(det, area)
/// 1e19  movss -0x14(%rbp),%xmm0  ; uAB
/// 1e1e  addss -0x18(%rbp),%xmm0  ; dst = uAB          -> add_l(uAB, vAB)
/// 1eeb  movss -0x1c(%rbp),%xmm0  ; uBC
/// 1ef0  addss -0x20(%rbp),%xmm0  ; dst = uBC          -> add_l(uBC, vBC)
/// 1fbc  movss -0x24(%rbp),%xmm0  ; uCA
/// 1fc1  addss -0x28(%rbp),%xmm0  ; dst = uCA          -> add_l(uCA, vCA)
/// 200c  movss -0x30(%rbp),%xmm0  ; uABC
/// 2011  addss -0x34(%rbp),%xmm0  ; dst = uABC         -> add_l(uABC, vABC)
/// 2016  addss -0x38(%rbp),%xmm0  ; dst = running sum  -> add_l(sum, wABC)
/// ```
///
/// Stack slots: `uAB=-0x14 vAB=-0x18 uBC=-0x1c vBC=-0x20 uCA=-0x24 vCA=-0x28
/// area=-0x2c uABC=-0x30 vABC=-0x34 wABC=-0x38`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c23(s: *mut c2Simplex) {
    unsafe {
        let verts = addr_of_mut!((*s).verts) as *mut c2sv;
        let sa = verts.add(0);
        let sb = verts.add(1);
        let sc = verts.add(2);

        let a = (*sa).p;
        let b = (*sb).p;
        let c = (*sc).p;
        let uAB = c2Dot(b, c2Sub(b, a));
        let vAB = c2Dot(a, c2Sub(a, b));
        let uBC = c2Dot(c, c2Sub(c, b));
        let vBC = c2Dot(b, c2Sub(b, c));
        let uCA = c2Dot(a, c2Sub(a, c));
        let vCA = c2Dot(c, c2Sub(c, a));
        let area = c2Det2(c2Sub(b, a), c2Sub(c, a));
        let uABC = mul_l(c2Det2(b, c), area);
        let vABC = mul_l(c2Det2(c, a), area);
        let wABC = mul_l(c2Det2(a, b), area);
        if vAB <= 0.0 && uCA <= 0.0 {
            (*sa).u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uAB <= 0.0 && vBC <= 0.0 {
            *sa = *sb;
            (*sa).u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uBC <= 0.0 && vCA <= 0.0 {
            *sa = *sc;
            (*sa).u = 1.0f32;
            (*s).div = 1.0f32;
            (*s).count = 1;
        } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
            (*sa).u = uAB;
            (*sb).u = vAB;
            (*s).div = add_l(uAB, vAB);
            (*s).count = 2;
        } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
            *sa = *sb;
            *sb = *sc;
            (*sa).u = uBC;
            (*sb).u = vBC;
            (*s).div = add_l(uBC, vBC);
            (*s).count = 2;
        } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
            *sb = *sa;
            *sa = *sc;
            (*sa).u = uCA;
            (*sb).u = vCA;
            (*s).div = add_l(uCA, vCA);
            (*s).count = 2;
        } else {
            (*sa).u = uABC;
            (*sb).u = vABC;
            (*sc).u = wABC;
            (*s).div = add_l(add_l(uABC, vABC), wABC);
            (*s).count = 3;
        }
    }
}

/// `c2v c2D(c2Simplex *s)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2D(s: *mut c2Simplex) -> c2v {
    unsafe {
        let verts = addr_of!((*s).verts) as *const c2sv;
        match (*s).count {
            1 => c2Neg((*verts.add(0)).p),
            2 => {
                let ab = c2Sub((*verts.add(1)).p, (*verts.add(0)).p);
                if c2Det2(ab, c2Neg((*verts.add(0)).p)) > 0.0 {
                    return c2Skew(ab);
                }
                c2CCW90(ab)
            }
            // `case 3:` and `default:`
            _ => c2V(0.0, 0.0),
        }
    }
}

/// `int c2Support(const c2v *verts, int count, c2v d)`
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

/// `void c2Witness(c2Simplex *s, c2v *a, c2v *b)`
///
/// `den` lives in the stack slot `-0x14(%rbp)`, and every `den * u` product
/// loads `u` from the simplex into `xmm0` and uses `den` as the *memory source*:
///
/// ```text
/// 2291  movss 0x90(%rax),%xmm1   ; s->div
/// 2299  movss 0x1d5f(%rip),%xmm0 ; 1.0f
/// 22a1  divss %xmm1,%xmm0        ; dst = 1.0f       -> div_l(1.0, s->div)
/// 22fb  movss 0x3c(%rax),%xmm0   ; s->b.u
/// 2300  mulss -0x14(%rbp),%xmm0  ; dst = s->b.u     -> mul_r(den, u)
/// 2323  movss 0x18(%rax),%xmm0   ; s->a.u
/// 2328  mulss -0x14(%rbp),%xmm0  ; dst = s->a.u     -> mul_r(den, u)
/// ```
///
/// So the destination is the vertex weight, **not** `den`: `u`'s NaN wins.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Witness(s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) {
    unsafe {
        let verts = addr_of!((*s).verts) as *const c2sv;
        let v0 = verts.add(0);
        let v1 = verts.add(1);
        let v2 = verts.add(2);
        let den = div_l(1.0f32, (*s).div);
        match (*s).count {
            1 => {
                *a = (*v0).sA;
                *b = (*v0).sB;
            }
            2 => {
                *a = c2Add(
                    c2Mulvs((*v0).sA, mul_r(den, (*v0).u)),
                    c2Mulvs((*v1).sA, mul_r(den, (*v1).u)),
                );
                *b = c2Add(
                    c2Mulvs((*v0).sB, mul_r(den, (*v0).u)),
                    c2Mulvs((*v1).sB, mul_r(den, (*v1).u)),
                );
            }
            3 => {
                *a = c2Add(
                    c2Add(
                        c2Mulvs((*v0).sA, mul_r(den, (*v0).u)),
                        c2Mulvs((*v1).sA, mul_r(den, (*v1).u)),
                    ),
                    c2Mulvs((*v2).sA, mul_r(den, (*v2).u)),
                );
                *b = c2Add(
                    c2Add(
                        c2Mulvs((*v0).sB, mul_r(den, (*v0).u)),
                        c2Mulvs((*v1).sB, mul_r(den, (*v1).u)),
                    ),
                    c2Mulvs((*v2).sB, mul_r(den, (*v2).u)),
                );
            }
            _ => {
                *a = c2V(0.0, 0.0);
                *b = c2V(0.0, 0.0);
            }
        }
    }
}

/// `c2v c2L(c2Simplex *s)`
///
/// Identical shape to `c2Witness`: `den` is the stack slot `-0x14(%rbp)` and is
/// therefore always the `mulss` *source*, so both products have the vertex
/// weight as destination.
///
/// ```text
/// 25fd  movss 0x90(%rax),%xmm1   ; s->div
/// 2605  movss 0x19f3(%rip),%xmm0 ; 1.0f
/// 260d  divss %xmm1,%xmm0        ; dst = 1.0f     -> div_l(1.0, s->div)
/// 263a  movss 0x3c(%rax),%xmm0   ; s->b.u
/// 263f  mulss -0x14(%rbp),%xmm0  ; dst = s->b.u   -> mul_r(den, b.u)
/// 2662  movss 0x18(%rax),%xmm0   ; s->a.u
/// 2667  mulss -0x14(%rbp),%xmm0  ; dst = s->a.u   -> mul_r(den, a.u)
/// 2690  call c2Add               ; c2Add(a term, b term)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2L(s: *mut c2Simplex) -> c2v {
    unsafe {
        let verts = addr_of!((*s).verts) as *const c2sv;
        let v0 = verts.add(0);
        let v1 = verts.add(1);
        let den = div_l(1.0f32, (*s).div);
        match (*s).count {
            1 => (*v0).p,
            2 => c2Add(
                c2Mulvs((*v0).p, mul_r(den, (*v0).u)),
                c2Mulvs((*v1).p, mul_r(den, (*v1).u)),
            ),
            _ => c2V(0.0, 0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// GJK
// ---------------------------------------------------------------------------

/// ```c
/// float c2GJK(const void *A, C2_TYPE typeA, const c2x *ax_ptr,
///             const void *B, C2_TYPE typeB, const c2x *bx_ptr,
///             c2v *outA, c2v *outB, int use_radius, int *iterations,
///             c2GJKCache *cache);
/// ```
///
/// Notes on faithfulness:
/// * `c2Proxy pA, pB;` and `c2Simplex s;` are *uninitialised* in the C. They
///   are zeroed here to keep this translation deterministic. The only way the
///   difference is observable is by passing a `cache` whose `iA`/`iB` entries
///   index past the shape's real vertex count, which makes the C read
///   uninitialised stack — unreproducible undefined behaviour.
/// * Indexing that C performs without bounds checks (`verts + i`,
///   `cache->iA[i]`, `pA.verts[iA]`) is done through raw pointers so that an
///   out-of-range `cache->count` aliases neighbouring struct fields exactly as
///   the C does, instead of panicking.
/// * `max_metric * 2.0f` is emitted by GCC as `addss %xmm6,%xmm1` (i.e.
///   `m + m`), which is bit-identical to `m * 2.0` for every input.
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

        let mut pA = c2Proxy::zeroed();
        let mut pB = c2Proxy::zeroed();
        c2MakeProxy(A, typeA, &mut pA);
        c2MakeProxy(B, typeB, &mut pB);
        let pA_verts = addr_of!(pA.verts) as *const c2v;
        let pB_verts = addr_of!(pB.verts) as *const c2v;

        let mut s = c2Simplex::zeroed();
        let sp: *mut c2Simplex = &mut s;
        // `c2sv *verts = &s.a;`
        let verts = addr_of_mut!((*sp).verts) as *mut c2sv;

        let mut cache_was_read: c_int = 0;
        if !cache.is_null() {
            let cache_was_good: c_int = ((*cache).count != 0) as c_int;
            if cache_was_good != 0 {
                let cache_iA = addr_of_mut!((*cache).iA) as *mut c_int;
                let cache_iB = addr_of_mut!((*cache).iB) as *mut c_int;
                let cache_count = (*cache).count;
                let mut i: c_int = 0;
                while i < cache_count {
                    let iA = *cache_iA.offset(i as isize);
                    let iB = *cache_iB.offset(i as isize);
                    let sA = c2Mulxv(ax, *pA_verts.offset(iA as isize));
                    let sB = c2Mulxv(bx, *pB_verts.offset(iB as isize));
                    let v = verts.offset(i as isize);
                    (*v).iA = iA;
                    (*v).sA = sA;
                    (*v).iB = iB;
                    (*v).sB = sB;
                    (*v).p = c2Sub((*v).sB, (*v).sA);
                    (*v).u = 0.0;
                    i += 1;
                }
                (*sp).count = cache_count;
                (*sp).div = (*cache).div;
                let metric_old = (*cache).metric;
                let metric = c2GJKSimplexMetric(sp);
                // `minss`/`maxss` with the fresh metric as destination, which
                // is exactly what these ternaries mean.
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < add_l(max_metric, max_metric) && metric < -1.0e8f32) {
                    cache_was_read = 1;
                }
            }
        }

        if cache_was_read == 0 {
            let v0 = verts.add(0);
            (*v0).iA = 0;
            (*v0).iB = 0;
            (*v0).sA = c2Mulxv(ax, *pA_verts.add(0));
            (*v0).sB = c2Mulxv(bx, *pB_verts.add(0));
            (*v0).p = c2Sub((*v0).sB, (*v0).sA);
            (*v0).u = 1.0f32;
            (*sp).div = 1.0f32;
            (*sp).count = 1;
        }

        let mut saveA: [c_int; 3] = [0; 3];
        let mut saveB: [c_int; 3] = [0; 3];
        let saveA_p = saveA.as_mut_ptr();
        let saveB_p = saveB.as_mut_ptr();
        let mut save_count: c_int = 0;
        let mut d0: f32 = FLT_MAX;
        let mut d1: f32 = FLT_MAX;
        let mut iter: c_int = 0;
        let mut hit: c_int = 0;
        while iter < 20 {
            save_count = (*sp).count;
            let mut i: c_int = 0;
            while i < save_count {
                *saveA_p.offset(i as isize) = (*verts.offset(i as isize)).iA;
                *saveB_p.offset(i as isize) = (*verts.offset(i as isize)).iB;
                i += 1;
            }
            match (*sp).count {
                1 => {}
                2 => c22(sp),
                3 => c23(sp),
                _ => {}
            }
            if (*sp).count == 3 {
                hit = 1;
                break;
            }
            let p = c2L(sp);
            d1 = c2Dot(p, p);
            if d1 > d0 {
                break;
            }
            d0 = d1;
            let d = c2D(sp);
            if c2Dot(d, d) < FLT_EPSILON * FLT_EPSILON {
                break;
            }
            let iA = c2Support(pA_verts, pA.count, c2MulrvT(ax.r, c2Neg(d)));
            let sA = c2Mulxv(ax, *pA_verts.offset(iA as isize));
            let iB = c2Support(pB_verts, pB.count, c2MulrvT(bx.r, d));
            let sB = c2Mulxv(bx, *pB_verts.offset(iB as isize));
            let v = verts.offset((*sp).count as isize);
            (*v).iA = iA;
            (*v).sA = sA;
            (*v).iB = iB;
            (*v).sB = sB;
            (*v).p = c2Sub((*v).sB, (*v).sA);
            let mut dup: c_int = 0;
            let mut i: c_int = 0;
            while i < save_count {
                if iA == *saveA_p.offset(i as isize) && iB == *saveB_p.offset(i as isize) {
                    dup = 1;
                    break;
                }
                i += 1;
            }
            if dup != 0 {
                break;
            }
            (*sp).count += 1;
            iter += 1;
        }
        let _ = d1;

        let mut a = ZERO_V;
        let mut b = ZERO_V;
        c2Witness(sp, &mut a, &mut b);
        let mut dist = c2Len(c2Sub(a, b));
        if hit != 0 {
            a = b;
            dist = 0.0;
        } else if use_radius != 0 {
            let rA = pA.radius;
            let rB = pB.radius;
            // GCC computes `rA + rB` once (dst = rA) and reuses it for both
            // the comparison and the subtraction.
            let r_sum = add_l(rA, rB);
            if dist > r_sum && dist > FLT_EPSILON {
                dist = sub_l(dist, r_sum);
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
            (*cache).metric = c2GJKSimplexMetric(sp);
            (*cache).count = (*sp).count;
            let cache_iA = addr_of_mut!((*cache).iA) as *mut c_int;
            let cache_iB = addr_of_mut!((*cache).iB) as *mut c_int;
            let mut i: c_int = 0;
            while i < (*sp).count {
                let v = verts.offset(i as isize);
                *cache_iA.offset(i as isize) = (*v).iA;
                *cache_iB.offset(i as isize) = (*v).iB;
                i += 1;
            }
            (*cache).div = (*sp).div;
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
// Public entry point declared in include/lib.h
// ---------------------------------------------------------------------------

/// ```c
/// void gjk(char reverse, c2v *a, c2v *b, float a1, float a2, float a3,
///          float a4, float b1, float b2, float b3, float b4, float b5);
/// ```
#[unsafe(no_mangle)]
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
    unsafe {
        let mut bb = c2AABB {
            min: ZERO_V,
            max: ZERO_V,
        };
        bb.min = c2V(a1, a2);
        bb.max = c2V(a3, a4);

        let mut cap = c2Capsule {
            a: ZERO_V,
            b: ZERO_V,
            r: 0.0,
        };
        cap.a = c2V(b1, b2);
        cap.b = c2V(b3, b4);
        cap.r = b5;

        if reverse != 0 {
            c2GJK(
                addr_of_mut!(cap) as *const c_void,
                C2_TYPE_CAPSULE,
                null(),
                addr_of_mut!(bb) as *const c_void,
                C2_TYPE_AABB,
                null(),
                a,
                b,
                1,
                null_mut(),
                null_mut(),
            );
        } else {
            c2GJK(
                addr_of_mut!(bb) as *const c_void,
                C2_TYPE_AABB,
                null(),
                addr_of_mut!(cap) as *const c_void,
                C2_TYPE_CAPSULE,
                null(),
                a,
                b,
                1,
                null_mut(),
                null_mut(),
            );
        }
    }
}
