//! Rust translation of `c_src/src/lib.c` (a trimmed-down slice of cute_c2).
//!
//! The public entry point declared in `c_src/include/lib.h` is `circle_collide`.
//! The header contains no namespacing macros, so the linker symbols keep their
//! source-level names.
//!
//! All arithmetic is single precision (`f32`) and every comparison is
//! reproduced literally (including the NaN-propagation behaviour of the C
//! ternary based min/max helpers) so that results are bit-identical to the C.

// C identifiers are kept verbatim so the exported symbols match the C library.
#![allow(non_camel_case_types, non_snake_case)]

use std::ffi::{c_int, c_void};

// C2_TYPE enumerators from lib.c
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: f32) -> c2v {
    c2v {
        x: a.x * b,
        y: a.y * b,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    // `(a.x) > (b.x) ? (a.x) : (b.x)` -- not f32::max: a NaN comparison is
    // false, so `b` wins, which differs from `f32::max`.
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
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
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

/// `A` is unconditionally reinterpreted as a `c2Circle`, exactly as the C does.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(A: *const c_void, B: *const c_void, typeB: c_int) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => c2CircletoCircle(
            unsafe { (A as *const c2Circle).read_unaligned() },
            unsafe { (B as *const c2Circle).read_unaligned() },
        ),
        C2_TYPE_AABB => c2CircletoAABB(
            unsafe { (A as *const c2Circle).read_unaligned() },
            unsafe { (B as *const c2AABB).read_unaligned() },
        ),
        C2_TYPE_CAPSULE => c2CircletoCapsule(
            unsafe { (A as *const c2Circle).read_unaligned() },
            unsafe { (B as *const c2Capsule).read_unaligned() },
        ),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let mut result: c_int = 0;

    let circle_in = c2Circle {
        p: c2V(x, y),
        r,
    };

    let circle = c2Circle {
        p: c2V(-70.0f32, 0.0),
        r: 20.0f32,
    };

    let aabb = c2AABB {
        min: c2V(-40.0f32, -40.0f32),
        max: c2V(-15.0f32, -15.0f32),
    };

    let capsule = c2Capsule {
        a: c2V(-40.0f32, 40.0f32),
        b: c2V(-20.0f32, 100.0f32),
        r: 10.0f32,
    };

    let a = (&circle_in as *const c2Circle).cast::<c_void>();

    result += unsafe {
        c2Collided(
            a,
            (&circle as *const c2Circle).cast::<c_void>(),
            C2_TYPE_CIRCLE,
        )
    };

    result += unsafe {
        c2Collided(a, (&aabb as *const c2AABB).cast::<c_void>(), C2_TYPE_AABB)
    } << 1;

    result += unsafe {
        c2Collided(
            a,
            (&capsule as *const c2Capsule).cast::<c_void>(),
            C2_TYPE_CAPSULE,
        )
    } << 2;

    result
}
