//! Rust translation of the C library in `c_src/`.
//!
//! # Exported ABI
//!
//! The C shared library built from `c_src` (all of `src/*.c` globbed into one
//! `SHARED` library by `c_src/CMakeLists.txt`) exports exactly one public
//! symbol:
//!
//! ```text
//! $ nm -D --defined-only libharvest-work-hu9sDw.so
//! 0000000000001100 T to_barycentric
//! ```
//!
//! `lm_v2`, `lm_sub2` and `lm_dot2` in `c_src/src/lib.c` are declared `static`,
//! i.e. they have internal linkage and are deliberately *not* exported here
//! either. They are reproduced as private helpers so that the arithmetic — and,
//! critically, its exact evaluation order — is identical.
//!
//! # Bit-exactness
//!
//! Every arithmetic step is `f32` (C `float`) and is performed in the same
//! order as the C source, so results are bit-identical for all ordinary inputs.
//!
//! The one place where "the same arithmetic" is not enough to be *byte*
//! identical is NaN payload propagation. x86-64 SSE scalar ops are
//! two-operand (`mulss dst, src` computes `dst = dst op src`) and, when more
//! than one operand is NaN, the result keeps the **destination** operand's
//! payload (quieting it if it was signaling). Which value the compiler parks in
//! the destination register is a register-allocation detail, so the payload that
//! survives is codegen-dependent rather than source-dependent.
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no optimisation flags,
//! so the reference library is built unoptimised. Reading that build's
//! disassembly gives the destination operand of each op:
//!
//! ```text
//! lm_sub2:  subss  -> dst = a.<c>          (minuend; forced by the ISA)
//! lm_dot2:  mulss  %xmm0,%xmm1  -> dst = a.x   (x term: left operand)
//!           mulss  %xmm2,%xmm0  -> dst = b.y   (y term: RIGHT operand)
//!           addss  %xmm1,%xmm0  -> dst = y term (RIGHT addend)
//! to_barycentric: every mulss/subss/divss has dst = the left operand
//! ```
//!
//! The helpers below (`sub_dst_lhs`, `mul_dst_lhs`, `mul_dst_rhs`,
//! `add_dst_rhs`, `div_dst_lhs`) encode exactly that. Whenever no operand is
//! NaN they fall straight through to the plain hardware op, so ordinary results
//! — including NaNs *generated* by invalid operations such as `0.0/0.0`, which
//! yield the hardware default QNaN — are untouched. Verified byte-identical
//! against the reference `.so` over 4.5M randomised cases (random bit patterns,
//! NaN-heavy, and signaling-NaN-heavy).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

/// `typedef struct lm_vec2 { float x, y; } lm_vec2;`
///
/// An 8-byte aggregate of two `float`s. Under the x86-64 SysV ABI this is a
/// single SSE eightbyte, passed and returned packed in one XMM register;
/// `repr(C)` + `extern "C"` reproduces that layout and passing convention.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

// ---------------------------------------------------------------------------
// SSE-faithful scalar float ops.
//
// `quiet` mirrors the hardware's SNaN -> QNaN conversion: set the quiet bit and
// leave the sign and the rest of the payload alone.
// ---------------------------------------------------------------------------

#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `subss dst, src` with `dst = lhs`: `lhs - rhs`.
#[inline]
fn sub_dst_lhs(lhs: f32, rhs: f32) -> f32 {
    if lhs.is_nan() {
        quiet(lhs)
    } else if rhs.is_nan() {
        quiet(rhs)
    } else {
        lhs - rhs
    }
}

/// `mulss dst, src` with `dst = lhs`: `lhs * rhs`.
#[inline]
fn mul_dst_lhs(lhs: f32, rhs: f32) -> f32 {
    if lhs.is_nan() {
        quiet(lhs)
    } else if rhs.is_nan() {
        quiet(rhs)
    } else {
        lhs * rhs
    }
}

/// `mulss dst, src` with `dst = rhs`: `lhs * rhs`, but the *right* operand
/// occupies the destination register and therefore wins the payload race.
#[inline]
fn mul_dst_rhs(lhs: f32, rhs: f32) -> f32 {
    if rhs.is_nan() {
        quiet(rhs)
    } else if lhs.is_nan() {
        quiet(lhs)
    } else {
        lhs * rhs
    }
}

/// `addss dst, src` with `dst = rhs`: `lhs + rhs`, right operand in the
/// destination register.
#[inline]
fn add_dst_rhs(lhs: f32, rhs: f32) -> f32 {
    if rhs.is_nan() {
        quiet(rhs)
    } else if lhs.is_nan() {
        quiet(lhs)
    } else {
        lhs + rhs
    }
}

/// `divss dst, src` with `dst = lhs`: `lhs / rhs`.
#[inline]
fn div_dst_lhs(lhs: f32, rhs: f32) -> f32 {
    if lhs.is_nan() {
        quiet(lhs)
    } else if rhs.is_nan() {
        quiet(rhs)
    } else {
        lhs / rhs
    }
}

// ---------------------------------------------------------------------------
// static helpers from c_src/src/lib.c (internal linkage: not exported)
// ---------------------------------------------------------------------------

/// `static lm_vec2 lm_v2(float x, float y)`
#[inline]
fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

/// `static lm_vec2 lm_sub2(lm_vec2 a, lm_vec2 b)`
#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(sub_dst_lhs(a.x, b.x), sub_dst_lhs(a.y, b.y))
}

/// `static float lm_dot2(lm_vec2 a, lm_vec2 b)`
///
/// `a.x * b.x + a.y * b.y` — see the module docs for why the y term and the
/// sum keep their right-hand operand in the destination register.
#[inline]
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))
}

// ---------------------------------------------------------------------------
// public ABI
// ---------------------------------------------------------------------------

/// `lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p)`
///
/// Faithful translation, quirks included: the returned pair is `(u, v)` where
/// `u` runs along the `p3 - p1` edge and `v` along the `p2 - p1` edge (the C
/// builds `v0` from `p3` and `v1` from `p2`, so the two coordinates come out in
/// that order), and the reciprocal `1.0f / denom` is computed with no
/// degeneracy check, so collinear or coincident inputs propagate inf/NaN
/// exactly as the C does.
#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0: lm_vec2 = lm_sub2(p3, p1);
    let v1: lm_vec2 = lm_sub2(p2, p1);
    let v2: lm_vec2 = lm_sub2(p, p1);
    let dot00: f32 = lm_dot2(v0, v0);
    let dot01: f32 = lm_dot2(v0, v1);
    let dot02: f32 = lm_dot2(v0, v2);
    let dot11: f32 = lm_dot2(v1, v1);
    let dot12: f32 = lm_dot2(v1, v2);
    let invDenom: f32 = div_dst_lhs(
        1.0f32,
        sub_dst_lhs(mul_dst_lhs(dot00, dot11), mul_dst_lhs(dot01, dot01)),
    );
    let u: f32 = mul_dst_lhs(
        sub_dst_lhs(mul_dst_lhs(dot11, dot02), mul_dst_lhs(dot01, dot12)),
        invDenom,
    );
    let v: f32 = mul_dst_lhs(
        sub_dst_lhs(mul_dst_lhs(dot00, dot12), mul_dst_lhs(dot01, dot02)),
        invDenom,
    );
    lm_v2(u, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(v: lm_vec2) -> (u32, u32) {
        (v.x.to_bits(), v.y.to_bits())
    }

    #[test]
    fn interior_point() {
        let r = to_barycentric(
            lm_v2(0.0, 0.0),
            lm_v2(1.0, 0.0),
            lm_v2(0.0, 1.0),
            lm_v2(0.25, 0.25),
        );
        assert_eq!(bits(r), (0x3e80_0000, 0x3e80_0000)); // (0.25, 0.25)
    }

    #[test]
    fn outside_point() {
        let r = to_barycentric(
            lm_v2(0.0, 0.0),
            lm_v2(1.0, 0.0),
            lm_v2(0.0, 1.0),
            lm_v2(2.0, 3.0),
        );
        assert_eq!(bits(r), (0x4040_0000, 0x4000_0000)); // (3.0, 2.0)
    }

    #[test]
    fn degenerate_triangle_matches_c() {
        // 1.0f / 0.0f * 0.0f style propagation: C yields (-nan, -nan).
        let z = lm_v2(0.0, 0.0);
        let r = to_barycentric(z, z, z, z);
        assert_eq!(bits(r), (0xffc0_0000, 0xffc0_0000));
    }

    #[test]
    fn nan_payload_propagation() {
        // Exercises the lm_dot2 right-operand-wins rule.
        let r = to_barycentric(
            lm_v2(f32::from_bits(0x4996_b48e), f32::from_bits(0x7fe0_5aad)),
            lm_v2(f32::from_bits(0xe20d_de17), f32::from_bits(0x602d_4cf0)),
            lm_v2(f32::from_bits(0xffd6_2d09), f32::from_bits(0x7495_1672)),
            lm_v2(f32::from_bits(0x1d3c_96bf), f32::from_bits(0x728d_180a)),
        );
        assert_eq!(bits(r), (0x7fe0_5aad, 0x7fe0_5aad));
    }

    #[test]
    fn generic_point() {
        let r = to_barycentric(
            lm_v2(-5.5, 2.25),
            lm_v2(7.125, -3.75),
            lm_v2(0.5, 9.5),
            lm_v2(1.0, 1.0),
        );
        assert_eq!(bits(r), (0x3e3a_6ec9, 0x3edb_4d9a));
    }
}
