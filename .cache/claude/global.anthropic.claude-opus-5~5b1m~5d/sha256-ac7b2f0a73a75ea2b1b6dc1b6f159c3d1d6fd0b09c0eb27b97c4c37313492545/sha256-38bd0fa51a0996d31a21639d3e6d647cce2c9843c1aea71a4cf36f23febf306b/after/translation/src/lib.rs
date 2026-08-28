//! Rust translation of the C library in `c_src/`.
//!
//! The C library (`c_src/src/lib.c` + `c_src/include/lib.h`) is a small 2D
//! collision-detection library (a cut-down `cute_c2`). Every function in the C
//! translation unit has external linkage, so all ten of them appear in the
//! shared object's dynamic symbol table and must be re-exported here with
//! identical names, signatures and semantics:
//!
//! ```text
//! c2V  c2Maxv  c2Minv  c2Clampv  c2Sub  c2Dot
//! c2CircletoCircle  c2CircletoAABB  c2AABBtoAABB  collided
//! ```
//!
//! Notes on bit-exactness:
//! * All arithmetic is done in `f32` exactly as in the C source (single
//!   precision throughout on the SysV x86-64 ABI, no double promotion).
//! * `c2Maxv`/`c2Minv` are written with explicit `>` / `<` ternary comparisons
//!   rather than `f32::max`/`f32::min`, because the C macros
//!   `(a > b ? a : b)` / `(a < b ? a : b)` propagate the *second* operand when
//!   an operand is NaN and always return the second operand for `±0.0`,
//!   whereas `f32::max`/`f32::min` are NaN-suppressing. Reproducing the C
//!   behaviour requires the raw comparison (this is what gcc lowers to
//!   `maxss`/`minss`).
//! * Boolean results are converted with `as c_int`, yielding exactly the 1/0
//!   that C's relational operators produce.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB } C2_TYPE;`
///
/// All enumerators are non-negative, so gcc gives the enum an `unsigned int`
/// underlying type; it is passed in a 32-bit register either way.
pub type C2_TYPE = c_uint;

pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;

/// `typedef struct c2v { float x; float y; } c2v;`
///
/// 8 bytes, one SSE eightbyte: passed/returned packed in the low half of a
/// single xmm register.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Circle { c2v p; float r; } c2Circle;`
///
/// 12 bytes, two SSE eightbytes: `p` in xmm(n), `r` in xmm(n+1).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

/// `typedef struct c2AABB { c2v min; c2v max; } c2AABB;`
///
/// 16 bytes, two SSE eightbytes: `min` in xmm(n), `max` in xmm(n+1).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

// ---------------------------------------------------------------------------
// Vector helpers
// ---------------------------------------------------------------------------

/// ```c
/// c2v c2V(float x, float y) { c2v a; a.x = x; a.y = y; return a; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

/// ```c
/// c2v c2Maxv(c2v a, c2v b) {
///     return c2V(((a.x) > (b.x) ? (a.x) : (b.x)),
///                ((a.y) > (b.y) ? (a.y) : (b.y)));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

/// ```c
/// c2v c2Minv(c2v a, c2v b) {
///     return c2V(((a.x) < (b.x) ? (a.x) : (b.x)),
///                ((a.y) < (b.y) ? (a.y) : (b.y)));
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

/// ```c
/// c2v c2Clampv(c2v a, c2v lo, c2v hi) { return c2Maxv(lo, c2Minv(a, hi)); }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
}

/// ```c
/// c2v c2Sub(c2v a, c2v b) { a.x -= b.x; a.y -= b.y; return a; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(a: c2v, b: c2v) -> c2v {
    let mut a = a;
    a.x -= b.x;
    a.y -= b.y;
    a
}

/// Quiet a NaN the way an x86 SSE arithmetic instruction does: the sign bit and
/// payload are preserved and the "quiet" mantissa bit (bit 22) is forced on, so
/// an sNaN such as `0x7f800001` becomes `0x7fc00001`.
#[inline]
fn quiet_nan(v: f32) -> f32 {
    f32::from_bits(v.to_bits() | 0x0040_0000)
}

/// `mulss dst, src` — an IEEE multiply with x86's *operand-ordered* NaN
/// propagation: if the destination operand is a NaN it wins, otherwise the
/// source operand's NaN wins, otherwise the ordinary product (which yields the
/// default qNaN `0x7fc00000` for the invalid `0 * inf` case).
///
/// This is spelled out explicitly because C's `a.x * b.x` is compiled by gcc to
/// `mulss` with `a.x` as the destination, and therefore propagates `a`'s NaN
/// when *both* operands are NaNs with different payloads. LLVM considers `fmul`
/// commutative and freely swaps the operands (it emits `mulss` with `b.y` as the
/// destination), which would propagate the other payload. Handling the NaN cases
/// by hand pins the observable bit pattern to the C library's.
#[inline]
fn mulss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst * src
    }
}

/// `addss dst, src` — an IEEE add with x86's operand-ordered NaN propagation
/// (see [`mulss`]). The non-NaN path still produces the default qNaN for the
/// invalid `inf + (-inf)` case.
#[inline]
fn addss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst + src
    }
}

/// ```c
/// float c2Dot(c2v a, c2v b) { return a.x * b.x + a.y * b.y; }
/// ```
///
/// The value is bit-exact for every finite/infinite input. The one detail that
/// needs care is *which* NaN payload comes out when several operands are
/// distinct NaNs, since that depends on the `mulss`/`addss` operand order the
/// compiler happens to choose. gcc, building this very `CMakeLists.txt`
/// (which sets no `CMAKE_BUILD_TYPE`, hence no optimisation), emits:
///
/// ```text
/// movss a.x,xmm1 ; movss b.x,xmm0 ; mulss xmm0,xmm1  -> dst = a.x
/// movss a.y,xmm2 ; movss b.y,xmm0 ; mulss xmm2,xmm0  -> dst = b.y
/// addss xmm1,xmm0                                    -> dst = y product
/// ```
///
/// i.e. the x product keeps `a.x` as destination, the y product keeps `b.y`,
/// and the sum keeps the *y* product. That ordering is mirrored below.
///
/// Note that this is the one place where the C library's own behaviour is not
/// stable across compiler settings: at `-O2` gcc reassociates to
/// `dst = a.y` for the y product and `dst = x product` for the sum, which
/// yields a different NaN payload for the same inputs. Inputs containing at
/// most one distinct NaN payload (the realistic case) produce identical results
/// under either ordering.
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    let x_product = mulss(a.x, b.x);
    let y_product = mulss(b.y, a.y);
    addss(y_product, x_product)
}

// ---------------------------------------------------------------------------
// Collision routines
// ---------------------------------------------------------------------------

/// ```c
/// int c2CircletoCircle(c2Circle A, c2Circle B) {
///     c2v c = c2Sub(B.p, A.p);
///     float d2 = c2Dot(c, c);
///     float r2 = A.r + B.r;
///     r2 = r2 * r2;
///     return d2 < r2;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 = r2 * r2;
    (d2 < r2) as c_int
}

/// ```c
/// int c2CircletoAABB(c2Circle A, c2AABB B) {
///     c2v L = c2Clampv(A.p, B.min, B.max);
///     c2v ab = c2Sub(A.p, L);
///     float d2 = c2Dot(ab, ab);
///     float r2 = A.r * A.r;
///     return d2 < r2;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    (d2 < r2) as c_int
}

/// ```c
/// int c2AABBtoAABB(c2AABB A, c2AABB B) {
///     int d0 = B.max.x < A.min.x;
///     int d1 = A.max.x < B.min.x;
///     int d2 = B.max.y < A.min.y;
///     int d3 = A.max.y < B.min.y;
///     return !(d0 | d1 | d2 | d3);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0 = (B.max.x < A.min.x) as c_int;
    let d1 = (A.max.x < B.min.x) as c_int;
    let d2 = (B.max.y < A.min.y) as c_int;
    let d3 = (A.max.y < B.min.y) as c_int;
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// ```c
/// int collided(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB);
/// ```
///
/// Dispatches on the pair of shape tags, exactly reproducing the C switch
/// nest (including the `default: return 0;` arms for out-of-range tags and the
/// argument swap in the AABB-vs-circle case).
///
/// # Safety
/// `A` and `B` must point to objects of the type indicated by `typeA` /
/// `typeB`, just as the C function requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn collided(
    A: *const c_void,
    typeA: C2_TYPE,
    B: *const c_void,
    typeB: C2_TYPE,
) -> c_int {
    match typeA {
        C2_TYPE_CIRCLE => match typeB {
            C2_TYPE_CIRCLE => c2CircletoCircle(
                unsafe { (A as *const c2Circle).read_unaligned() },
                unsafe { (B as *const c2Circle).read_unaligned() },
            ),
            C2_TYPE_AABB => c2CircletoAABB(
                unsafe { (A as *const c2Circle).read_unaligned() },
                unsafe { (B as *const c2AABB).read_unaligned() },
            ),
            _ => 0,
        },
        C2_TYPE_AABB => match typeB {
            C2_TYPE_CIRCLE => c2CircletoAABB(
                unsafe { (B as *const c2Circle).read_unaligned() },
                unsafe { (A as *const c2AABB).read_unaligned() },
            ),
            C2_TYPE_AABB => c2AABBtoAABB(
                unsafe { (A as *const c2AABB).read_unaligned() },
                unsafe { (B as *const c2AABB).read_unaligned() },
            ),
            _ => 0,
        },
        _ => 0,
    }
}
