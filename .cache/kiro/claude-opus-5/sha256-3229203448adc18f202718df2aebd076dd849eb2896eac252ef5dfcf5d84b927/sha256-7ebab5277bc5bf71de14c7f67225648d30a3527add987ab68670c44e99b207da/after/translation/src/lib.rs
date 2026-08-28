//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is intended to be bit-for-bit identical to the original C, which
//! is compiled for x86-64 (SSE single-precision arithmetic, `cvttss2si` for the
//! float -> int conversion).

use std::ffi::c_float;

/// Turns a NaN into its quiet form by setting the most significant mantissa
/// bit, matching what SSE does when it forwards a signalling NaN operand.
#[inline]
fn quiet_nan(x: c_float) -> c_float {
    c_float::from_bits(x.to_bits() | 0x0040_0000)
}

/// NaN operand forwarding performed by the SSE scalar arithmetic instructions.
///
/// For `OP src1, src2` the Intel rules are: if `src1` is a NaN the result is
/// `src1` (quietened when it is signalling); otherwise, if `src2` is a NaN the
/// result is `src2` (likewise quietened). Only when neither operand is a NaN is
/// the arithmetic actually performed.
///
/// This matters because the NaN *payload* is observable, and Rust/LLVM is free
/// to pick either operand order for the commutative operations. The helpers
/// below therefore pin `src1`/`src2` to the order gcc emitted for the reference
/// build, which was read off the disassembly of `hsv_to_rgb`.
#[inline]
fn sse_nan_forward(src1: c_float, src2: c_float) -> Option<c_float> {
    if src1.is_nan() {
        Some(quiet_nan(src1))
    } else if src2.is_nan() {
        Some(quiet_nan(src2))
    } else {
        None
    }
}

/// `mulss src1, src2`
#[inline]
fn sse_mul(src1: c_float, src2: c_float) -> c_float {
    match sse_nan_forward(src1, src2) {
        Some(nan) => nan,
        None => src1 * src2,
    }
}

/// `subss src1, src2`, i.e. `src1 - src2`
#[inline]
fn sse_sub(src1: c_float, src2: c_float) -> c_float {
    match sse_nan_forward(src1, src2) {
        Some(nan) => nan,
        None => src1 - src2,
    }
}

/// `divss src1, src2`, i.e. `src1 / src2`
#[inline]
fn sse_div(src1: c_float, src2: c_float) -> c_float {
    match sse_nan_forward(src1, src2) {
        Some(nan) => nan,
        None => src1 / src2,
    }
}

/// Reproduces the C expression `(int)floorf(h)`.
///
/// A C cast from a floating point value that is out of range for `int` is
/// undefined behaviour; on x86-64 the generated `cvttss2si` instruction yields
/// `INT_MIN` (the "integer indefinite" value) for out-of-range inputs and for
/// NaN. Rust's `as` cast saturates instead, so the conversion is emulated here
/// to keep the observable behaviour the same as the C build.
#[inline]
fn c_floor_to_int(h: c_float) -> i32 {
    let floored = h.floor();
    // Both comparisons are false for NaN, which correctly falls through to
    // the indefinite value.
    if floored >= -2_147_483_648.0f32 && floored < 2_147_483_648.0f32 {
        floored as i32
    } else {
        i32::MIN
    }
}

/// Converts an HSV triple to RGB.
///
/// `src` points to three floats `[h, s, v]`; `dest` receives three floats
/// `[r, g, b]`.
///
/// # Safety
///
/// `dest` must be valid for writes of three `f32` values and `src` valid for
/// reads of three `f32` values, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    let src = unsafe { std::slice::from_raw_parts(src, 3) };
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, 3) };

    let mut h = src[0];
    let s = src[1];
    let v = src[2];

    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }

    // `h /= 60.0f` -> `divss h, 60.0f`
    h = sse_div(h, 60.0f32);
    let i = c_floor_to_int(h);
    // `f = h - i` -> `subss h, (float)i`
    let f = sse_sub(h, i as c_float);
    // `p = v * (1 - s)` -> gcc evaluates `(1 - s)` into the destination
    // register, so `(1 - s)` is `src1` of the multiply, not `v`.
    let p = sse_mul(sse_sub(1.0f32, s), v);
    // `q = v * (1 - s * f)` -> `mulss s, f`, then `subss 1.0f, (s*f)`, then
    // `mulss (1 - s*f), v`.
    let q = sse_mul(sse_sub(1.0f32, sse_mul(s, f)), v);
    // `t = v * (1 - s * (1 - f))` -> `subss 1.0f, f`, then `mulss (1-f), s`
    // (operands swapped relative to the C source), then `subss 1.0f, …`, then
    // `mulss (1 - s*(1-f)), v`.
    let t = sse_mul(sse_sub(1.0f32, sse_mul(sse_sub(1.0f32, f), s)), v);

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    dest[0] = r;
    dest[1] = g;
    dest[2] = b;
}
