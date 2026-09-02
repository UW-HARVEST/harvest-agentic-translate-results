//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (verified against `nm -D` on the C shared library):
//!   * `hsl_to_rgb`
//!
//! Source: `c_src/src/lib.c`, `c_src/include/lib.h`. There are no namespace or
//! renaming macros in the public header, so the linker symbol equals the source
//! name.
//!
//! # Fidelity notes
//!
//! The original C is reproduced verbatim, bugs included:
//!
//! * The third arm of the hue cascade reads `h < 120.0f && h < 180.0f` instead
//!   of `h >= 120.0f && h < 180.0f`. The two preceding arms already consume
//!   `0 <= h < 120`, so this arm is only reachable for `h < 0`; hues in
//!   `[120, 180)` therefore fall through to the final `else` and come out as
//!   the flat grey `(m, m, m)`. This is preserved, not fixed.
//! * The redundant `1.0f *` in `m = 1.0f * (l - 0.5f * c)` is an exact identity
//!   for every `f32` bit pattern, and the C compiler elides it; it is dropped
//!   here with no observable change.
//! * `fmodf` maps onto Rust's `%` for `f32`, which is the same truncated
//!   remainder and lowers to the same libm call.
//! * A NaN hue makes every predicate in the cascade false, so control reaches
//!   the final `else`, exactly as in C.
//!
//! ## Why the arithmetic goes through helper functions
//!
//! Naive `a - b` / `a * b` expressions are bit-identical to the C for every
//! non-NaN input, but they are *not* bit-identical when both operands of an
//! operation are NaN. On x86-64 a two-operand SSE instruction such as
//! `subss dst, src` returns the *destination* operand's NaN (quieted) when both
//! operands are NaN, so the result's sign bit and payload depend on which
//! operand the compiler placed in the destination register. LLVM and GCC make
//! different (equally legal) choices: LLVM rewrites `l - 0.5 * c` into
//! `(-0.5) * c + l`, which flips the destination operand and hence the returned
//! NaN.
//!
//! The `add`/`sub`/`mul`/`div` helpers below make that choice explicit instead
//! of leaving it to the optimiser: the first argument is the SSE destination
//! operand. Operand roles were read off the compiled C. For every input where
//! at most one operand is NaN the helpers are plain IEEE arithmetic, so this
//! costs nothing in fidelity elsewhere.

#![allow(clippy::missing_safety_doc)]

/// Quiet a NaN the way SSE does when it propagates an operand: set the quiet
/// bit and leave the sign and the rest of the payload alone.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `a + b` with `a` as the SSE destination operand.
#[inline(always)]
fn add(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a + b
    }
}

/// `a - b` with `a` as the SSE destination operand.
#[inline(always)]
fn sub(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a - b
    }
}

/// `a * b` with `a` as the SSE destination operand.
#[inline(always)]
fn mul(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a * b
    }
}

/// `a / b` with `a` as the SSE destination operand.
#[inline(always)]
fn div(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a / b
    }
}

/// `fabsf`: a pure bit operation (`andps`), so it never quiets a NaN.
#[inline(always)]
fn fabsf(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7fff_ffff)
}

/// `fmodf(a, b)`. Rust's `%` on `f32` is C's `fmodf`.
#[inline(always)]
fn fmodf(a: f32, b: f32) -> f32 {
    a % b
}

/// Convert an HSL triple to an RGB triple.
///
/// `src` must point to at least 3 readable `f32` values (hue in degrees,
/// saturation, lightness). `dest` must point to at least 3 writable `f32`
/// values. Mirrors `void hsl_to_rgb(float *dest, const float *src)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut f32, src: *const f32) {
    let h: f32 = unsafe { *src.add(0) };
    let s: f32 = unsafe { *src.add(1) };
    let l: f32 = unsafe { *src.add(2) };

    let c: f32;
    let m: f32;
    let x: f32;

    if s == 0.0f32 {
        unsafe {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }

    // c = (1.0f - fabsf(2.0f * l - 1.0f)) * s;
    // `2.0f * l` is emitted as `l + l`.
    c = mul(sub(1.0f32, fabsf(sub(add(l, l), 1.0f32))), s);

    // m = 1.0f * (l - 0.5f * c);  (the `1.0f *` is an exact no-op)
    m = sub(l, mul(c, 0.5f32));

    // x = c * (1.0f - fabsf(fmodf(h / 60.0f, 2) - 1.0f));
    // The literal `2` is an int converted to 2.0f for the fmodf call.
    x = mul(sub(1.0f32, fabsf(sub(fmodf(div(h, 60.0f32), 2.0f32), 1.0f32))), c);

    // Branch order and each individual predicate are kept exactly as in C.
    let (r, g, b): (f32, f32, f32) = if h >= 0.0f32 && h < 60.0f32 {
        (add(c, m), add(x, m), m)
    } else if h >= 60.0f32 && h < 120.0f32 {
        (add(x, m), add(c, m), m)
    } else if h < 120.0f32 && h < 180.0f32 {
        // NOTE: faithful copy of the original (buggy) predicate; the C says
        // `h < 120.0f`, not `h >= 120.0f`.
        (m, add(c, m), add(m, x))
    } else if h >= 180.0f32 && h < 240.0f32 {
        (m, add(x, m), add(m, c))
    } else if h >= 240.0f32 && h < 300.0f32 {
        (add(x, m), m, add(m, c))
    } else if h >= 300.0f32 && h < 360.0f32 {
        (add(c, m), m, add(m, x))
    } else {
        (m, m, m)
    };

    unsafe {
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}
