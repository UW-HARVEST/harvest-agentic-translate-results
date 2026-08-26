//! Scalar / vector / rotation primitives.
//!
//! Every one of these is a public symbol in the C shared object, so each gets
//! `#[unsafe(no_mangle)] extern "C"`.
//!
//! Bit-exactness notes:
//! * All arithmetic is `f32` and is written in the same association order as the C
//!   source. GCC does not reassociate floating point without `-ffast-math`, and
//!   neither target enables FMA (baseline x86-64 is SSE2 only), so results match
//!   instruction-for-instruction.
//! * `c2Maxv` / `c2Minv` / `c2Absv` deliberately use raw `>` / `<` / `< 0`
//!   comparisons rather than `f32::max` / `f32::min` / `f32::abs`. The C ternaries
//!   propagate NaN from the *second* operand and return `-0.0` unchanged for
//!   `-0.0`; the Rust library functions do neither.

use crate::fp;
use crate::types::{c2r, c2v, c2x, c2h};
use core::ffi::c_float;

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2V(x: c_float, y: c_float) -> c2v {
    c2v { x, y }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulvs(a: c2v, b: c_float) -> c2v {
    // C: `a.x *= b; a.y *= b;`
    // GCC vectorises this as `movsldup` (broadcast b) + `mulps %xmm2,%xmm0`, so the
    // *scalar* b is the destination operand in both lanes and its NaN wins.
    c2v {
        x: fp::mul(b, a.x),
        y: fp::mul(b, a.y),
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Maxv(a: c2v, b: c2v) -> c2v {
    // C: c2V(((a.x) > (b.x) ? (a.x) : (b.x)), ((a.y) > (b.y) ? (a.y) : (b.y)))
    c2V(
        if a.x > b.x { a.x } else { b.x },
        if a.y > b.y { a.y } else { b.y },
    )
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Minv(a: c2v, b: c2v) -> c2v {
    // C: c2V(((a.x) < (b.x) ? (a.x) : (b.x)), ((a.y) < (b.y) ? (a.y) : (b.y)))
    c2V(
        if a.x < b.x { a.x } else { b.x },
        if a.y < b.y { a.y } else { b.y },
    )
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Clampv(a: c2v, lo: c2v, hi: c2v) -> c2v {
    c2Maxv(lo, c2Minv(a, hi))
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
pub extern "C" fn c2Dot(a: c2v, b: c2v) -> c_float {
    // GCC: mulss(dst=a.x, b.x); mulss(dst=a.y, b.y); addss(dst=first product).
    fp::add(fp::mul(a.x, b.x), fp::mul(a.y, b.y))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Dist(h: c2h, p: c2v) -> c_float {
    c2Dot(h.n, p) - h.d
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2RotIdentity() -> c2r {
    c2r { c: 1.0, s: 0.0 }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2xIdentity() -> c2x {
    c2x {
        p: c2V(0.0, 0.0),
        r: c2RotIdentity(),
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Len(a: c2v) -> c_float {
    // C: sqrtf(c2Dot(a, a))
    c2Dot(a, a).sqrt()
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Det2(a: c2v, b: c2v) -> c_float {
    // GCC: mulss(dst=a.x, b.y); mulss(dst=a.y, b.x); subss (destination forced).
    fp::mul(a.x, b.y) - fp::mul(a.y, b.x)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulrv(a: c2r, b: c2v) -> c2v {
    // C: c2V(a.c * b.x - a.s * b.y, a.s * b.x + a.c * b.y)
    // GCC's four mulss all take the rotation component as destination; note that for
    // the y lane it emits `addss %xmm5,%xmm1` with `a.c * b.y` as the destination,
    // i.e. the operands are the other way round from the source order.
    c2V(
        fp::mul(a.c, b.x) - fp::mul(a.s, b.y),
        fp::add(fp::mul(a.c, b.y), fp::mul(a.s, b.x)),
    )
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2MulrvT(a: c2r, b: c2v) -> c2v {
    // C: c2V(a.c * b.x + a.s * b.y, -a.s * b.x + a.c * b.y)
    // GCC folds the y lane's negate-then-add into `subss %xmm5,%xmm1`, i.e.
    // `(a.c * b.y) - (a.s * b.x)`. That is exactly equivalent for finite values and
    // signed zeros, and fixes the destination operand for the NaN case.
    c2V(
        fp::add(fp::mul(a.c, b.x), fp::mul(a.s, b.y)),
        fp::mul(a.c, b.y) - fp::mul(a.s, b.x),
    )
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Add(a: c2v, b: c2v) -> c2v {
    // GCC: `addps %xmm1,%xmm0` — `a` is the destination in both lanes.
    c2v {
        x: fp::add(a.x, b.x),
        y: fp::add(a.y, b.y),
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Mulxv(a: c2x, b: c2v) -> c2v {
    c2Add(c2Mulrv(a.r, b), a.p)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2MulxvT(a: c2x, b: c2v) -> c2v {
    c2MulrvT(a.r, c2Sub(b, a.p))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Intersect(a: c2v, b: c2v, da: c_float, db: c_float) -> c2v {
    c2Add(a, c2Mulvs(c2Sub(b, a), da / (da - db)))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Div(a: c2v, b: c_float) -> c2v {
    // Note: no divide-by-zero guard in C either; 1.0f/0.0f yields inf, which then
    // propagates. Reproduced verbatim.
    c2Mulvs(a, 1.0 / b)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Norm(a: c2v) -> c2v {
    c2Div(a, c2Len(a))
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Neg(a: c2v) -> c2v {
    // IEEE negation: preserves the sign of zero, exactly like C's unary minus.
    c2V(-a.x, -a.y)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2CCW90(a: c2v) -> c2v {
    c2v { x: a.y, y: -a.x }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Skew(a: c2v) -> c2v {
    c2v { x: -a.y, y: a.x }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn c2Absv(a: c2v) -> c2v {
    // C: c2V(((a.x) < 0 ? -(a.x) : (a.x)), ((a.y) < 0 ? -(a.y) : (a.y)))
    // NOT f32::abs: this returns -0.0 for -0.0 and passes NaN through unchanged.
    c2V(
        if a.x < 0.0 { -a.x } else { a.x },
        if a.y < 0.0 { -a.y } else { a.y },
    )
}
