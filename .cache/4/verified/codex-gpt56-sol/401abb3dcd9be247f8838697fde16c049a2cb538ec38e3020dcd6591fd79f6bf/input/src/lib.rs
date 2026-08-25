use std::ffi::{c_float, c_int, c_void};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::asm;

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2v {
    pub x: c_float,
    pub y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Circle {
    pub p: C2v,
    pub r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: c_float,
}

#[inline(always)]
fn ordered_mul(left: c_float, right: c_float) -> c_float {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut result = left;
        // SAFETY: This is a register-only scalar multiplication. Operand order
        // preserves the same NaN payload selection as the reference build.
        unsafe {
            asm!(
                "mulss {result}, {right}",
                result = inout(xmm_reg) result,
                right = in(xmm_reg) right,
                options(pure, nomem, nostack, preserves_flags),
            );
        }
        result
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        left * right
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> C2v {
    C2v { x, y }
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(mut a: C2v, b: c_float) -> C2v {
    a.x = ordered_mul(b, a.x);
    a.y = ordered_mul(b, a.y);
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: C2v, b: C2v) -> C2v {
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: C2v, lo: C2v, hi: C2v) -> C2v {
    c2Maxv(lo, c2Minv(a, hi))
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Sub(mut a: C2v, b: C2v) -> C2v {
    a.x -= b.x;
    a.y -= b.y;
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn c2Dot(a: C2v, b: C2v) -> c_float {
    ordered_mul(a.x, b.x) + ordered_mul(a.y, b.y)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCircle(a: C2Circle, b: C2Circle) -> c_int {
    let c = c2Sub(b.p, a.p);
    let d2 = c2Dot(c, c);
    let mut r2 = a.r + b.r;
    r2 *= r2;
    c_int::from(d2 < r2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoAABB(a: C2Circle, b: C2Aabb) -> c_int {
    let l = c2Clampv(a.p, b.min, b.max);
    let ab = c2Sub(a.p, l);
    let d2 = c2Dot(ab, ab);
    let r2 = a.r * a.r;
    c_int::from(d2 < r2)
}

#[unsafe(no_mangle)]
pub extern "C" fn c2CircletoCapsule(a: C2Circle, b: C2Capsule) -> c_int {
    let n = c2Sub(b.b, b.a);
    let ap = c2Sub(a.p, b.a);
    let da = c2Dot(ap, n);
    let d2;

    if da < 0.0 {
        d2 = c2Dot(ap, ap);
    } else {
        let db = c2Dot(c2Sub(a.p, b.b), n);
        if db < 0.0 {
            let e = c2Sub(ap, c2Mulvs(n, da / c2Dot(n, n)));
            d2 = c2Dot(e, e);
        } else {
            let bp = c2Sub(a.p, b.b);
            d2 = c2Dot(bp, bp);
        }
    }

    let r = a.r + b.r;
    c_int::from(d2 < r * r)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn c2Collided(a: *const c_void, b: *const c_void, type_b: c_int) -> c_int {
    match type_b {
        C2_TYPE_CIRCLE => {
            // SAFETY: This mirrors the C function's unchecked typed dereferences.
            c2CircletoCircle(unsafe { (a.cast::<C2Circle>()).read() }, unsafe {
                (b.cast::<C2Circle>()).read()
            })
        }
        C2_TYPE_AABB => c2CircletoAABB(unsafe { (a.cast::<C2Circle>()).read() }, unsafe {
            (b.cast::<C2Aabb>()).read()
        }),
        C2_TYPE_CAPSULE => c2CircletoCapsule(unsafe { (a.cast::<C2Circle>()).read() }, unsafe {
            (b.cast::<C2Capsule>()).read()
        }),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn circle_collide(x: c_float, y: c_float, r: c_float) -> c_int {
    let circle_in = C2Circle { p: c2V(x, y), r };
    let circle = C2Circle {
        p: c2V(-70.0, 0.0),
        r: 20.0,
    };
    let aabb = C2Aabb {
        min: c2V(-40.0, -40.0),
        max: c2V(-15.0, -15.0),
    };
    let capsule = C2Capsule {
        a: c2V(-40.0, 40.0),
        b: c2V(-20.0, 100.0),
        r: 10.0,
    };

    let mut result = unsafe {
        c2Collided(
            (&circle_in as *const C2Circle).cast(),
            (&circle as *const C2Circle).cast(),
            C2_TYPE_CIRCLE,
        )
    };
    result += unsafe {
        c2Collided(
            (&circle_in as *const C2Circle).cast(),
            (&aabb as *const C2Aabb).cast(),
            C2_TYPE_AABB,
        )
    } << 1;
    result += unsafe {
        c2Collided(
            (&circle_in as *const C2Circle).cast(),
            (&capsule as *const C2Capsule).cast(),
            C2_TYPE_CAPSULE,
        )
    } << 2;
    result
}
