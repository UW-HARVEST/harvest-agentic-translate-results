//! Translation of the parts of `c_src/inc/q_shared.h` that carry executable
//! meaning: the `typedef`s, the vector macros, the `static ID_INLINE`
//! functions and the numeric constants.
//!
//! Everything is expressed in `f32` (`typedef float vec_t;`) exactly like the C
//! code, so the results are bit-for-bit identical on targets where
//! `FLT_EVAL_METHOD == 0` (x86-64 SSE, aarch64).
//!
//! The `static ID_INLINE` functions have internal linkage in C, so they are not
//! part of the shared library's symbol table.  They are still translated here
//! (they are used by `q_math.c`) and additionally re-exported under a `w_`
//! prefix by [`crate::wrappers`] so that the differential test suite can reach
//! them.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use core::ffi::c_int;

/// `typedef float vec_t;`
pub type vec_t = f32;

// ---------------------------------------------------------------------------
// numeric constants
// ---------------------------------------------------------------------------

/// `#define NUMVERTEXNORMALS 162`
pub const NUMVERTEXNORMALS: usize = 162;

/// `#define PITCH 0` (up / down)
pub const PITCH: usize = 0;
/// `#define YAW 1` (left / right)
pub const YAW: usize = 1;
/// `#define ROLL 2` (fall over)
pub const ROLL: usize = 2;

/// `#define nanmask (255<<23)`
pub const nanmask: i32 = 255 << 23;

/// `q_shared.h` guards its own `M_PI` with `#ifndef M_PI`, and `<math.h>` is
/// included first, so the `double` `M_PI` from libm wins.  Every use of `M_PI`
/// in this library therefore happens in `f64`.
pub const M_PI: f64 = core::f64::consts::PI;

/// `PLANE_X`
pub const PLANE_X: c_int = 0;
/// `PLANE_Y`
pub const PLANE_Y: c_int = 1;
/// `PLANE_Z`
pub const PLANE_Z: c_int = 2;
/// `PLANE_NON_AXIAL`
pub const PLANE_NON_AXIAL: c_int = 3;

/// `typedef enum {qfalse, qtrue} qboolean;` — an `enum` is an `int` here.
pub const qfalse: c_int = 0;
/// see [`qfalse`]
pub const qtrue: c_int = 1;

// ---------------------------------------------------------------------------
// structs
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct cplane_s {
///     vec3_t  normal;
///     float   dist;
///     byte    type;      // for fast side tests: 0,1,2 = axial, 3 = nonaxial
///     byte    signbits;  // signx + (signy<<1) + (signz<<2)
///     byte    pad[2];
/// } cplane_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct cplane_t {
    pub normal: [vec_t; 3],
    pub dist: f32,
    pub type_: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

// ---------------------------------------------------------------------------
// C float -> integer conversion, with the exact behaviour of the generated
// x86-64 code
// ---------------------------------------------------------------------------

/// `(int)f` for a `float` operand.
///
/// C leaves the result undefined when the truncated value does not fit into an
/// `int`; gcc emits `cvttss2si`, which yields the "integer indefinite" value
/// `0x80000000` for NaN and for every out-of-range operand.  Rust's `as i32`
/// saturates instead, so the conversion has to be spelled out.
#[inline]
pub fn f32_to_i32(f: f32) -> i32 {
    if f.is_nan() || f >= 2147483648.0f32 || f < -2147483648.0f32 {
        i32::MIN
    } else {
        f as i32
    }
}

/// `(int)d` for a `double` operand (`cvttsd2si`), see [`f32_to_i32`].
#[inline]
pub fn f64_to_i32(d: f64) -> i32 {
    if d.is_nan() || d >= 2147483648.0f64 || d < -2147483648.0f64 {
        i32::MIN
    } else {
        d as i32
    }
}

/// `(byte)f`: gcc emits `cvttss2si` into a 32-bit register and stores its low
/// byte, so an out-of-range or NaN operand produces `0x80000000 & 0xff == 0`.
#[inline]
pub fn f32_to_byte(f: f32) -> u8 {
    (f32_to_i32(f) & 0xff) as u8
}

// ---------------------------------------------------------------------------
// the vector macros
// ---------------------------------------------------------------------------
//
// `#if 1` is taken in q_shared.h, so DotProduct/VectorSubtract/... are the
// macro versions (the `_`-prefixed functions in q_math.c are unused by the
// library itself but still exported).

/// `#define DotProduct(x,y) ((x)[0]*(y)[0]+(x)[1]*(y)[1]+(x)[2]*(y)[2])`
#[inline]
pub unsafe fn DotProduct(x: *const vec_t, y: *const vec_t) -> vec_t {
    *x.add(0) * *y.add(0) + *x.add(1) * *y.add(1) + *x.add(2) * *y.add(2)
}

/// `#define VectorSubtract(a,b,c) ((c)[0]=(a)[0]-(b)[0],...)`
#[inline]
pub unsafe fn VectorSubtract(a: *const vec_t, b: *const vec_t, c: *mut vec_t) {
    *c.add(0) = *a.add(0) - *b.add(0);
    *c.add(1) = *a.add(1) - *b.add(1);
    *c.add(2) = *a.add(2) - *b.add(2);
}

/// `#define VectorAdd(a,b,c) ((c)[0]=(a)[0]+(b)[0],...)`
#[inline]
pub unsafe fn VectorAdd(a: *const vec_t, b: *const vec_t, c: *mut vec_t) {
    *c.add(0) = *a.add(0) + *b.add(0);
    *c.add(1) = *a.add(1) + *b.add(1);
    *c.add(2) = *a.add(2) + *b.add(2);
}

/// `#define VectorCopy(a,b) ((b)[0]=(a)[0],(b)[1]=(a)[1],(b)[2]=(a)[2])`
#[inline]
pub unsafe fn VectorCopy(a: *const vec_t, b: *mut vec_t) {
    *b.add(0) = *a.add(0);
    *b.add(1) = *a.add(1);
    *b.add(2) = *a.add(2);
}

/// `#define VectorScale(v,s,o) ((o)[0]=(v)[0]*(s),...)`
#[inline]
pub unsafe fn VectorScale(v: *const vec_t, s: vec_t, o: *mut vec_t) {
    *o.add(0) = *v.add(0) * s;
    *o.add(1) = *v.add(1) * s;
    *o.add(2) = *v.add(2) * s;
}

/// `#define VectorMA(v,s,b,o) ((o)[0]=(v)[0]+(b)[0]*(s),...)`
#[inline]
pub unsafe fn VectorMA(v: *const vec_t, s: vec_t, b: *const vec_t, o: *mut vec_t) {
    *o.add(0) = *v.add(0) + *b.add(0) * s;
    *o.add(1) = *v.add(1) + *b.add(1) * s;
    *o.add(2) = *v.add(2) + *b.add(2) * s;
}

/// `#define VectorClear(a) ((a)[0]=(a)[1]=(a)[2]=0)`
#[inline]
pub unsafe fn VectorClear(a: *mut vec_t) {
    *a.add(0) = 0.0;
    *a.add(1) = 0.0;
    *a.add(2) = 0.0;
}

/// `#define VectorNegate(a,b) ((b)[0]=-(a)[0],(b)[1]=-(a)[1],(b)[2]=-(a)[2])`
#[inline]
pub unsafe fn VectorNegate(a: *const vec_t, b: *mut vec_t) {
    *b.add(0) = -*a.add(0);
    *b.add(1) = -*a.add(1);
    *b.add(2) = -*a.add(2);
}

/// `#define VectorSet(v,x,y,z) ((v)[0]=(x),(v)[1]=(y),(v)[2]=(z))`
#[inline]
pub unsafe fn VectorSet(v: *mut vec_t, x: vec_t, y: vec_t, z: vec_t) {
    *v.add(0) = x;
    *v.add(1) = y;
    *v.add(2) = z;
}

/// `#define Vector4Copy(a,b) ((b)[0]=(a)[0],...,(b)[3]=(a)[3])`
#[inline]
pub unsafe fn Vector4Copy(a: *const vec_t, b: *mut vec_t) {
    *b.add(0) = *a.add(0);
    *b.add(1) = *a.add(1);
    *b.add(2) = *a.add(2);
    *b.add(3) = *a.add(3);
}

/// `#define SnapVector(v) {v[0]=((int)(v[0]));v[1]=((int)(v[1]));v[2]=((int)(v[2]));}`
#[inline]
pub unsafe fn SnapVector(v: *mut vec_t) {
    *v.add(0) = f32_to_i32(*v.add(0)) as vec_t;
    *v.add(1) = f32_to_i32(*v.add(1)) as vec_t;
    *v.add(2) = f32_to_i32(*v.add(2)) as vec_t;
}

/// `#define IS_NAN(x) (((*(int *)&x)&nanmask)==nanmask)`
#[inline]
pub fn IS_NAN(x: vec_t) -> bool {
    ((x.to_bits() as i32) & nanmask) == nanmask
}

/// `#define SQRTFAST(x) ((x) * Q_rsqrt(x))`
#[inline]
pub fn SQRTFAST(x: vec_t) -> vec_t {
    x * crate::q_math::Q_rsqrt(x)
}

/// `#define DEG2RAD(a) (((a) * M_PI) / 180.0F)` — `M_PI` is a `double`, so the
/// whole expression is evaluated in `f64`.
#[inline]
pub fn DEG2RAD(a: f32) -> f64 {
    (a as f64 * M_PI) / 180.0f32 as f64
}

/// `#define RAD2DEG(a) (((a) * 180.0f) / M_PI)`
///
/// `180.0f` is a `float` literal, so -- unlike [`DEG2RAD`], where `M_PI` pulls
/// the whole expression into `double` -- the multiplication is done in `f32` and
/// only the division by `M_PI` promotes to `f64`.
#[inline]
pub fn RAD2DEG(a: f32) -> f64 {
    (a * 180.0f32) as f64 / M_PI
}

/// `#define ANGLE2SHORT(x) ((int)((x)*65536/360) & 65535)`
#[inline]
pub fn ANGLE2SHORT(x: f32) -> c_int {
    // (x)*65536 is float*int -> float; /360 is float/int -> float.
    f32_to_i32(x * 65536.0f32 / 360.0f32) & 65535
}

/// `#define SHORT2ANGLE(x) ((x)*(360.0/65536))`
#[inline]
pub fn SHORT2ANGLE(x: c_int) -> f64 {
    x as f64 * (360.0f64 / 65536.0f64)
}

/// `#define ColorIndex(c) (((c) - '0') & 7)`
///
/// `wrapping_sub` because `c - '0'` overflows for `c < INT_MIN + 48`, which is
/// UB in C but wraps with gcc (and Rust would panic on it in a debug build).
#[inline]
pub fn ColorIndex(c: c_int) -> c_int {
    c.wrapping_sub('0' as c_int) & 7
}

/// `#define Square(x) ((x)*(x))`
#[inline]
pub fn Square(x: f32) -> f32 {
    x * x
}

/// `#define PlaneTypeForNormal(x) (x[0] == 1.0 ? PLANE_X : (x[1] == 1.0 ? PLANE_Y : (x[2] == 1.0 ? PLANE_Z : PLANE_NON_AXIAL)))`
#[inline]
pub unsafe fn PlaneTypeForNormal(x: *const vec_t) -> c_int {
    if *x.add(0) as f64 == 1.0 {
        PLANE_X
    } else if *x.add(1) as f64 == 1.0 {
        PLANE_Y
    } else if *x.add(2) as f64 == 1.0 {
        PLANE_Z
    } else {
        PLANE_NON_AXIAL
    }
}

/// `#define Q_COLOR_ESCAPE '^'`
pub const Q_COLOR_ESCAPE: u8 = b'^';

/// ```c
/// #define Q_IsColorString(p) ( p && *(p) == Q_COLOR_ESCAPE && *((p)+1) && *((p)+1) != Q_COLOR_ESCAPE )
/// ```
///
/// The `&&` chain evaluates to an `int` 0/1 in C.
#[inline]
pub unsafe fn Q_IsColorString(p: *const core::ffi::c_char) -> c_int {
    c_int::from(
        !p.is_null()
            && *p as u8 == Q_COLOR_ESCAPE
            && *p.add(1) != 0
            && *p.add(1) as u8 != Q_COLOR_ESCAPE,
    )
}

extern "C" {
    /// `int rand(void)` from libc -- the very same generator (and the very same
    /// process-global state) the C library's `random()` macro uses.
    fn rand() -> c_int;
}

/// `#define random() ((rand () & 0x7fff) / ((float)0x7fff))`
#[inline]
pub fn random_() -> f32 {
    (unsafe { rand() } & 0x7fff) as f32 / 0x7fff as f32
}

/// `#define crandom() (2.0 * (random() - 0.5))` -- `2.0` and `0.5` are `double`s.
#[inline]
pub fn crandom_() -> f32 {
    (2.0f64 * (random_() as f64 - 0.5f64)) as f32
}

/// `#define MAKERGB(v,r,g,b) v[0]=r;v[1]=g;v[2]=b`
#[inline]
pub unsafe fn MAKERGB(v: *mut vec_t, r: vec_t, g: vec_t, b: vec_t) {
    *v.add(0) = r;
    *v.add(1) = g;
    *v.add(2) = b;
}

/// `#define MAKERGBA(v,r,g,b,a) v[0]=r;v[1]=g;v[2]=b;v[3]=a`
#[inline]
pub unsafe fn MAKERGBA(v: *mut vec_t, r: vec_t, g: vec_t, b: vec_t, a: vec_t) {
    *v.add(0) = r;
    *v.add(1) = g;
    *v.add(2) = b;
    *v.add(3) = a;
}

// ---------------------------------------------------------------------------
// the `static ID_INLINE` functions
// ---------------------------------------------------------------------------

/// ```c
/// static ID_INLINE int VectorCompare( const vec3_t v1, const vec3_t v2 ) {
///     if (v1[0] != v2[0] || v1[1] != v2[1] || v1[2] != v2[2]) {
///         return 0;
///     }
///     return 1;
/// }
/// ```
#[inline]
pub unsafe fn VectorCompare(v1: *const vec_t, v2: *const vec_t) -> c_int {
    if *v1.add(0) != *v2.add(0) || *v1.add(1) != *v2.add(1) || *v1.add(2) != *v2.add(2) {
        return 0;
    }
    1
}

/// ```c
/// static ID_INLINE vec_t VectorLength( const vec3_t v ) {
///     return (vec_t)sqrt (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]);
/// }
/// ```
#[inline]
pub unsafe fn VectorLength(v: *const vec_t) -> vec_t {
    ((*v.add(0) * *v.add(0) + *v.add(1) * *v.add(1) + *v.add(2) * *v.add(2)) as f64).sqrt() as vec_t
}

/// ```c
/// static ID_INLINE vec_t VectorLengthSquared( const vec3_t v ) {
///     return (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]);
/// }
/// ```
#[inline]
pub unsafe fn VectorLengthSquared(v: *const vec_t) -> vec_t {
    *v.add(0) * *v.add(0) + *v.add(1) * *v.add(1) + *v.add(2) * *v.add(2)
}

/// ```c
/// static ID_INLINE vec_t Distance( const vec3_t p1, const vec3_t p2 ) {
///     vec3_t  v;
///     VectorSubtract (p2, p1, v);
///     return VectorLength( v );
/// }
/// ```
#[inline]
pub unsafe fn Distance(p1: *const vec_t, p2: *const vec_t) -> vec_t {
    let mut v: [vec_t; 3] = [0.0; 3];
    VectorSubtract(p2, p1, v.as_mut_ptr());
    VectorLength(v.as_ptr())
}

/// ```c
/// static ID_INLINE vec_t DistanceSquared( const vec3_t p1, const vec3_t p2 ) {
///     vec3_t  v;
///     VectorSubtract (p2, p1, v);
///     return v[0]*v[0] + v[1]*v[1] + v[2]*v[2];
/// }
/// ```
#[inline]
pub unsafe fn DistanceSquared(p1: *const vec_t, p2: *const vec_t) -> vec_t {
    let mut v: [vec_t; 3] = [0.0; 3];
    VectorSubtract(p2, p1, v.as_mut_ptr());
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

/// ```c
/// static ID_INLINE void VectorNormalizeFast( vec3_t v )
/// {
///     float ilength;
///     ilength = Q_rsqrt( DotProduct( v, v ) );
///     v[0] *= ilength;
///     v[1] *= ilength;
///     v[2] *= ilength;
/// }
/// ```
#[inline]
pub unsafe fn VectorNormalizeFast(v: *mut vec_t) {
    let ilength: f32 = crate::q_math::Q_rsqrt(DotProduct(v, v));

    *v.add(0) = *v.add(0) * ilength;
    *v.add(1) = *v.add(1) * ilength;
    *v.add(2) = *v.add(2) * ilength;
}

/// ```c
/// static ID_INLINE void VectorInverse( vec3_t v ){
///     v[0] = -v[0];
///     v[1] = -v[1];
///     v[2] = -v[2];
/// }
/// ```
#[inline]
pub unsafe fn VectorInverse(v: *mut vec_t) {
    *v.add(0) = -*v.add(0);
    *v.add(1) = -*v.add(1);
    *v.add(2) = -*v.add(2);
}

/// ```c
/// static ID_INLINE void CrossProduct( const vec3_t v1, const vec3_t v2, vec3_t cross ) {
///     cross[0] = v1[1]*v2[2] - v1[2]*v2[1];
///     cross[1] = v1[2]*v2[0] - v1[0]*v2[2];
///     cross[2] = v1[0]*v2[1] - v1[1]*v2[0];
/// }
/// ```
#[inline]
pub unsafe fn CrossProduct(v1: *const vec_t, v2: *const vec_t, cross: *mut vec_t) {
    *cross.add(0) = *v1.add(1) * *v2.add(2) - *v1.add(2) * *v2.add(1);
    *cross.add(1) = *v1.add(2) * *v2.add(0) - *v1.add(0) * *v2.add(2);
    *cross.add(2) = *v1.add(0) * *v2.add(1) - *v1.add(1) * *v2.add(0);
}
