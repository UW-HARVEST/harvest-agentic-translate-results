#![allow(non_snake_case)]

use std::ffi::{c_int, c_void};

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn add_dot_products(x_product: f32, mut y_product: f32) -> f32 {
    // GCC evaluates the C expression with the Y product as the ADDSS destination.
    unsafe {
        std::arch::asm!(
            "addss {y}, {x}",
            x = in(xmm_reg) x_product,
            y = inout(xmm_reg) y_product,
            options(pure, nomem, nostack)
        );
    }
    y_product
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
fn add_dot_products(x_product: f32, y_product: f32) -> f32 {
    y_product + x_product
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: f32, y: f32) -> c2v {
    c2v { x, y }
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
    let x_product = a.x * b.x;
    let y_product = a.y * b.y;
    add_dot_products(x_product, y_product)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(A: c2Circle, B: c2Circle) -> c_int {
    let c = c2Sub(B.p, A.p);
    let d2 = c2Dot(c, c);
    let mut r2 = A.r + B.r;
    r2 *= r2;
    c_int::from(d2 < r2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(A: c2Circle, B: c2AABB) -> c_int {
    let L = c2Clampv(A.p, B.min, B.max);
    let ab = c2Sub(A.p, L);
    let d2 = c2Dot(ab, ab);
    let r2 = A.r * A.r;
    c_int::from(d2 < r2)
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
    c_int::from(d2 < r * r)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(A: *const c_void, B: *const c_void, typeB: c_int) -> c_int {
    match typeB {
        C2_TYPE_CIRCLE => {
            // SAFETY: The C ABI requires A and B to point to aligned c2Circle values.
            unsafe { c2CircletoCircle(*A.cast::<c2Circle>(), *B.cast::<c2Circle>()) }
        }
        C2_TYPE_AABB => {
            // SAFETY: The C ABI requires A to be c2Circle and B to be c2AABB.
            unsafe { c2CircletoAABB(*A.cast::<c2Circle>(), *B.cast::<c2AABB>()) }
        }
        C2_TYPE_CAPSULE => {
            // SAFETY: The C ABI requires A to be c2Circle and B to be c2Capsule.
            unsafe { c2CircletoCapsule(*A.cast::<c2Circle>(), *B.cast::<c2Capsule>()) }
        }
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: f32, y: f32, r: f32) -> c_int {
    let circle_in = c2Circle { p: c2V(x, y), r };
    let circle = c2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };
    let aabb = c2AABB {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };
    let capsule = c2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    let mut result = 0;
    // SAFETY: Every pointer refers to the matching aligned local C-layout value.
    unsafe {
        result += c2Collided(
            (&circle_in as *const c2Circle).cast(),
            (&circle as *const c2Circle).cast(),
            C2_TYPE_CIRCLE,
        );
        result += c2Collided(
            (&circle_in as *const c2Circle).cast(),
            (&aabb as *const c2AABB).cast(),
            C2_TYPE_AABB,
        ) << 1;
        result += c2Collided(
            (&circle_in as *const c2Circle).cast(),
            (&capsule as *const c2Capsule).cast(),
            C2_TYPE_CAPSULE,
        ) << 2;
    }
    result
}
