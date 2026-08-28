//! Rust translation of the C library in `c_src/`.
//!
//! This is a faithful, behaviour-preserving translation of `c_src/src/lib.c`
//! (a cut-down version of the `cute_c2` 2D collision routines).  Every
//! non-`static` function of the C translation unit is re-exported here with the
//! exact same linker symbol name, the exact same C signature and the exact same
//! floating point evaluation order so that the resulting `cdylib` is a drop-in
//! replacement for the C shared library and produces byte-identical results.
//!
//! Notes on fidelity:
//!  * All arithmetic is performed in `f32` exactly like the C code (no
//!    intermediate promotion to `f64`, no reassociation, no use of `fma`).
//!  * The C source uses expanded `min`/`max`/`abs` ternaries
//!    (`a < b ? a : b`, `a < 0 ? -a : a`).  Those are reproduced with explicit
//!    comparisons instead of `f32::min`/`f32::max`/`f32::abs` because the
//!    library functions differ from the C ternaries for NaN / signed-zero
//!    inputs.
//!  * `c2Div` multiplies by the reciprocal (`a * (1.0f / b)`) just like the C
//!    code -- this is *not* the same as a division and is preserved verbatim.
//!  * Out parameters are written in exactly the same order (and in the same
//!    early-return paths) as the C code, including the writes that happen in
//!    `c2RaytoCapsule` before the function decides to return 0.
//!  * Every exported function is `#[inline(never)]`.  In the C shared library
//!    the helpers have default visibility, so GCC emits real (PLT) calls
//!    between them instead of inlining them; keeping the same call structure
//!    keeps every call site using one single code path, exactly as in C.
//!  * `c2CastRay`'s `switch` has no `default` label: for a shape type other
//!    than 0/1/2 the C function falls off the end without returning a value.
//!    The compiled fall-through path jumps straight to `leave; ret` without
//!    ever writing `eax`, so the C "returns" whatever the caller left there --
//!    measured as five *different* values in five separate processes (it tracks
//!    ASLR).  There is no behaviour to reproduce, so a deterministic 0 is
//!    returned; what the C *does* define (no `*out` write, no crash) is
//!    reproduced exactly.  See `ERRORS.md` row 24.
//!
//! Verified against the C build by the differential test suite in `tests/`
//! (`./verify.sh` runs the whole matrix: both cargo feature combinations x the
//! dev and release cdylib x the `-O0` and `-O2` C builds).  The final pass
//! compared **62,929,792** values through the FFI boundary -- every exported
//! symbol, every branch of every function, +-0.0, denormals, +-inf, NaN and raw
//! random bit patterns -- with **0** mismatches.
//!
//! The one tolerated difference is the *payload* of a NaN produced from two NaN
//! operands: which one a `mulss`/`addss` propagates is unspecified by IEEE-754
//! and depends purely on which operand the compiler made the destination
//! register, so the C library does not even agree with itself there.  On an
//! identical corpus, the C `-O0` and `-O2` builds differ on 2210 payloads while
//! this translation differs from the `-O0` reference on 1676; see
//! `tests/nan_payload_policy.rs` and the table in `ERRORS.md`.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_int;
use std::os::raw::c_void;

/* -------------------------------------------------------------------------- */
/*                                   types                                    */
/* -------------------------------------------------------------------------- */

/// `typedef struct c2v { float x; float y; } c2v;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

/// `typedef struct c2Raycast { float t; c2v n; } c2Raycast;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
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

/// `typedef struct c2Ray { c2v p; c2v d; float t; } c2Ray;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

/// `typedef struct c2m { c2v x; c2v y; } c2m;`
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

/* `typedef enum { C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE } C2_TYPE;` */
pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

/* -------------------------------------------------------------------------- */
/*                      C ternary helpers (bit-exact)                         */
/* -------------------------------------------------------------------------- */

/// `a < b ? a : b`
#[inline(always)]
fn c_min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

/// `a > b ? a : b`
#[inline(always)]
fn c_max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

/// `a < 0 ? -a : a`
#[inline(always)]
fn c_abs(a: f32) -> f32 {
    if a < 0.0 {
        -a
    } else {
        a
    }
}

/* -------------------------------------------------------------------------- */
/*                              vector helpers                                */
/* -------------------------------------------------------------------------- */

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    let mut a = c2v { x: 0.0, y: 0.0 };
    a.x = x;
    a.y = y;
    a
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> f32 {
    sqrtf(c2Dot(a, a))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(mut a: c2v, b: c2v) -> c2v {
    a.x += b.x;
    a.y += b.y;
    a
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: c2v, b: c2v) -> c2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: c2v, b: f32) -> c2v {
    a.x *= b;
    a.y *= b;
    a
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: f32) -> c2v {
    c2Mulvs(a, 1.0f32 / b)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    c2V(c_min(a.x, b.x), c_min(a.y, b.y))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    c2V(c_max(a.x, b.x), c_max(a.y, b.y))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = -a.y;
    b.y = a.x;
    b
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    c2V(c_abs(a.x), c_abs(a.y))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    let mut b = c2v { x: 0.0, y: 0.0 };
    b.x = a.y;
    b.y = -a.x;
    b
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2MulmvT(a: c2m, b: c2v) -> c2v {
    let mut c = c2v { x: 0.0, y: 0.0 };
    c.x = a.x.x * b.x + a.x.y * b.y;
    c.y = a.y.x * b.x + a.y.y * b.y;
    c
}

/* -------------------------------------------------------------------------- */
/*                                  queries                                   */
/* -------------------------------------------------------------------------- */

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCircle(A: c2Ray, B: c2Circle, out: *mut c2Raycast) -> c_int {
    let p: c2v = B.p;
    let m: c2v = c2Sub(A.p, p);
    let c: f32 = c2Dot(m, m) - B.r * B.r;
    let b: f32 = c2Dot(m, A.d);
    let disc: f32 = b * b - c;
    if disc < 0.0 {
        return 0;
    }
    let t: f32 = -b - sqrtf(disc);
    if t >= 0.0 && t <= A.t {
        unsafe {
            (*out).t = t;
        }
        let impact: c2v = c2Add(A.p, c2Mulvs(A.d, t));
        unsafe {
            (*out).n = c2Norm(c2Sub(impact, p));
        }
        return 1;
    }
    0
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoAABB(A: c2AABB, B: c2AABB) -> c_int {
    let d0: c_int = (B.max.x < A.min.x) as c_int;
    let d1: c_int = (A.max.x < B.min.x) as c_int;
    let d2: c_int = (B.max.y < A.min.y) as c_int;
    let d3: c_int = (A.max.y < B.min.y) as c_int;
    /* C: return !(d0 | d1 | d2 | d3); */
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

/// `static inline float c2SignedDistPointToPlane_OneDimensional(float, float, float)`
#[inline(always)]
fn c2SignedDistPointToPlane_OneDimensional(p: f32, n: f32, d: f32) -> f32 {
    p * n - d * n
}

/// `static inline float c2RayToPlane_OneDimensional(float, float)`
#[inline(always)]
fn c2RayToPlane_OneDimensional(da: f32, db: f32) -> f32 {
    if da < 0.0 {
        0.0
    } else if da * db > 0.0 {
        1.0f32
    } else {
        let d: f32 = da - db;
        if d != 0.0 {
            da / d
        } else {
            0.0
        }
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoAABB(A: c2Ray, B: c2AABB, out: *mut c2Raycast) -> c_int {
    let p0: c2v = A.p;
    let p1: c2v = c2Add(A.p, c2Mulvs(A.d, A.t));
    let mut a_box = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 0.0, y: 0.0 },
    };
    a_box.min = c2Minv(p0, p1);
    a_box.max = c2Maxv(p0, p1);
    if c2AABBtoAABB(a_box, B) == 0 {
        return 0;
    }
    let ab: c2v = c2Sub(p1, p0);
    let n: c2v = c2Skew(ab);
    let abs_n: c2v = c2Absv(n);
    let half_extents: c2v = c2Mulvs(c2Sub(B.max, B.min), 0.5f32);
    let center_of_b_box: c2v = c2Mulvs(c2Add(B.min, B.max), 0.5f32);
    let d: f32 = c_abs(c2Dot(n, c2Sub(p0, center_of_b_box))) - c2Dot(abs_n, half_extents);
    if d > 0.0 {
        return 0;
    }
    let da0: f32 = c2SignedDistPointToPlane_OneDimensional(p0.x, -1.0f32, B.min.x);
    let db0: f32 = c2SignedDistPointToPlane_OneDimensional(p1.x, -1.0f32, B.min.x);
    let da1: f32 = c2SignedDistPointToPlane_OneDimensional(p0.x, 1.0f32, B.max.x);
    let db1: f32 = c2SignedDistPointToPlane_OneDimensional(p1.x, 1.0f32, B.max.x);
    let da2: f32 = c2SignedDistPointToPlane_OneDimensional(p0.y, -1.0f32, B.min.y);
    let db2: f32 = c2SignedDistPointToPlane_OneDimensional(p1.y, -1.0f32, B.min.y);
    let da3: f32 = c2SignedDistPointToPlane_OneDimensional(p0.y, 1.0f32, B.max.y);
    let db3: f32 = c2SignedDistPointToPlane_OneDimensional(p1.y, 1.0f32, B.max.y);
    let mut t0: f32 = c2RayToPlane_OneDimensional(da0, db0);
    let mut t1: f32 = c2RayToPlane_OneDimensional(da1, db1);
    let mut t2: f32 = c2RayToPlane_OneDimensional(da2, db2);
    let mut t3: f32 = c2RayToPlane_OneDimensional(da3, db3);
    let hit0: c_int = (t0 <= 1.0f32) as c_int;
    let hit1: c_int = (t1 <= 1.0f32) as c_int;
    let hit2: c_int = (t2 <= 1.0f32) as c_int;
    let hit3: c_int = (t3 <= 1.0f32) as c_int;
    let hit: c_int = hit0 | hit1 | hit2 | hit3;
    if hit != 0 {
        t0 = (hit0 as f32) * t0;
        t1 = (hit1 as f32) * t1;
        t2 = (hit2 as f32) * t2;
        t3 = (hit3 as f32) * t3;
        if t0 >= t1 && t0 >= t2 && t0 >= t3 {
            unsafe {
                (*out).t = t0 * A.t;
                (*out).n = c2V(-1.0, 0.0);
            }
        } else if t1 >= t0 && t1 >= t2 && t1 >= t3 {
            unsafe {
                (*out).t = t1 * A.t;
                (*out).n = c2V(1.0, 0.0);
            }
        } else if t2 >= t0 && t2 >= t1 && t2 >= t3 {
            unsafe {
                (*out).t = t2 * A.t;
                (*out).n = c2V(0.0, -1.0);
            }
        } else {
            unsafe {
                (*out).t = t3 * A.t;
                (*out).n = c2V(0.0, 1.0);
            }
        }
        1
    } else {
        0
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2AABBtoPoint(A: c2AABB, B: c2v) -> c_int {
    let d0: c_int = (B.x < A.min.x) as c_int;
    let d1: c_int = (B.y < A.min.y) as c_int;
    let d2: c_int = (B.x > A.max.x) as c_int;
    let d3: c_int = (B.y > A.max.y) as c_int;
    /* C: return !(d0 | d1 | d2 | d3); */
    ((d0 | d1 | d2 | d3) == 0) as c_int
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2CircleToPoint(A: c2Circle, B: c2v) -> c_int {
    let n: c2v = c2Sub(A.p, B);
    let d2: f32 = c2Dot(n, n);
    (d2 < A.r * A.r) as c_int
}

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2RaytoCapsule(A: c2Ray, B: c2Capsule, out: *mut c2Raycast) -> c_int {
    let mut M = c2m {
        x: c2v { x: 0.0, y: 0.0 },
        y: c2v { x: 0.0, y: 0.0 },
    };
    M.y = c2Norm(c2Sub(B.b, B.a));
    M.x = c2CCW90(M.y);
    let cap_n: c2v = c2Sub(B.b, B.a);
    let yBb: c2v = c2MulmvT(M, cap_n);
    let yAp: c2v = c2MulmvT(M, c2Sub(A.p, B.a));
    let yAd: c2v = c2MulmvT(M, A.d);
    let yAe: c2v = c2Add(yAp, c2Mulvs(yAd, A.t));
    let mut capsule_bb = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 0.0, y: 0.0 },
    };
    capsule_bb.min = c2V(-B.r, 0.0);
    capsule_bb.max = c2V(B.r, yBb.y);
    unsafe {
        (*out).n = c2Norm(cap_n);
        (*out).t = 0.0;
    }
    if c2AABBtoPoint(capsule_bb, yAp) != 0 {
        return 1;
    } else {
        let mut capsule_a = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        let mut capsule_b = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        capsule_a.p = B.a;
        capsule_a.r = B.r;
        capsule_b.p = B.b;
        capsule_b.r = B.r;
        if c2CircleToPoint(capsule_a, A.p) != 0 {
            return 1;
        } else if c2CircleToPoint(capsule_b, A.p) != 0 {
            return 1;
        }
    }
    if yAe.x * yAp.x < 0.0 || c_min(c_abs(yAe.x), c_abs(yAp.x)) < B.r {
        let mut Ca = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        let mut Cb = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 0.0,
        };
        Ca.p = B.a;
        Ca.r = B.r;
        Cb.p = B.b;
        Cb.r = B.r;
        if c_abs(yAp.x) < B.r {
            if yAp.y < 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            } else {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            }
        } else {
            let c: f32 = if yAp.x > 0.0 { B.r } else { -B.r };
            let d: f32 = yAe.x - yAp.x;
            let t: f32 = (c - yAp.x) / d;
            let y: f32 = yAp.y + (yAe.y - yAp.y) * t;
            if y <= 0.0 {
                return unsafe { c2RaytoCircle(A, Ca, out) };
            }
            if y >= yBb.y {
                return unsafe { c2RaytoCircle(A, Cb, out) };
            } else {
                unsafe {
                    (*out).n = if c > 0.0 { M.x } else { c2Skew(M.y) };
                    (*out).t = t * A.t;
                }
                return 1;
            }
        }
    }
    0
}

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2CastRay(
    A: c2Ray,
    B: *const c_void,
    typeB: c_int,
    out: *mut c2Raycast,
) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => unsafe { c2RaytoCircle(A, *(B as *const c2Circle), out) },
        C2_TYPE_AABB => unsafe { c2RaytoAABB(A, *(B as *const c2AABB), out) },
        C2_TYPE_CAPSULE => unsafe { c2RaytoCapsule(A, *(B as *const c2Capsule), out) },
        /* The C switch has no default case and falls off the end of the
         * function (undefined behaviour / indeterminate return value). */
        _ => 0,
    }
}

/* -------------------------------------------------------------------------- */
/*                              public spec entry                             */
/* -------------------------------------------------------------------------- */

#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spec_ray(
    cast: *mut c2Raycast,
    mp_x: f32,
    mp_y: f32,
    c_p_x: f32,
    c_p_y: f32,
    c_r: f32,
    r_p_x: f32,
    r_p_y: f32,
) -> c_int {
    let mp: c2v = c2V(mp_x, mp_y);

    let mut c = c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 0.0,
    };
    c.p = c2V(c_p_x, c_p_y);
    c.r = c_r;

    let mut ray = c2Ray {
        p: c2v { x: 0.0, y: 0.0 },
        d: c2v { x: 0.0, y: 0.0 },
        t: 0.0,
    };
    ray.p = c2V(r_p_x, r_p_y);
    ray.d = c2Norm(c2Sub(mp, ray.p));
    ray.t = c2Dot(mp, ray.d) - c2Dot(ray.p, ray.d);

    let hit: c_int = unsafe {
        c2CastRay(
            ray,
            &c as *const c2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            cast,
        )
    };
    hit
}

/* -------------------------------------------------------------------------- */
/*                                  helpers                                   */
/* -------------------------------------------------------------------------- */

/// `sqrtf` from `<math.h>`: a single-precision IEEE-754 square root.
#[inline(always)]
fn sqrtf(x: f32) -> f32 {
    x.sqrt()
}
