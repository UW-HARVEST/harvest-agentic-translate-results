//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI of the C shared library (the complete surface — `nm -D` on the C
//! `.so` exports exactly one public symbol):
//!
//! ```c
//! void hsl_to_rgb(float *dest, const float *src);
//! ```
//!
//! The translation is deliberately literal:
//!   * the `s == 0` early-out writes all three components and returns,
//!   * the comparison chain is evaluated in the original order,
//!   * the third branch keeps the original (buggy) `h < 120.0f && h < 180.0f`
//!     condition instead of the presumably intended `h >= 120.0f && ...`,
//!   * every floating point expression keeps its exact form and operand order,
//!     so results are bit-identical to the C build (including the sign/payload
//!     of NaN results, see the `ss_*` helpers below).

#![allow(clippy::excessive_precision)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_float;

// ---------------------------------------------------------------------------
// Scalar float helpers that reproduce the reference build bit for bit.
//
// For finite/infinite operands these are plain IEEE-754 single precision
// operations, exactly like the C code. They only differ from the naive Rust
// operators when *both* operands are NaN: the hardware (SSE `addss`/`subss`/
// `mulss`/`divss`) then returns the *first* (destination) operand, quieted,
// while an optimizing Rust/LLVM backend is free to commute the operands and
// would return the other NaN. Spelling the rule out keeps the observable
// output identical to the C library for every possible input.
// ---------------------------------------------------------------------------

/// Quiet a NaN the way SSE does: set the mantissa MSB, keep sign and payload.
#[inline]
fn quiet_nan(x: c_float) -> c_float {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `a + b` with SSE NaN propagation (destination operand wins).
#[inline]
fn ss_add(a: c_float, b: c_float) -> c_float {
    if a.is_nan() {
        quiet_nan(a)
    } else if b.is_nan() {
        quiet_nan(b)
    } else {
        a + b
    }
}

/// `a - b` with SSE NaN propagation (destination operand wins).
#[inline]
fn ss_sub(a: c_float, b: c_float) -> c_float {
    if a.is_nan() {
        quiet_nan(a)
    } else if b.is_nan() {
        quiet_nan(b)
    } else {
        a - b
    }
}

/// `a * b` with SSE NaN propagation (destination operand wins).
#[inline]
fn ss_mul(a: c_float, b: c_float) -> c_float {
    if a.is_nan() {
        quiet_nan(a)
    } else if b.is_nan() {
        quiet_nan(b)
    } else {
        a * b
    }
}

/// `a / b` with SSE NaN propagation (destination operand wins).
#[inline]
fn ss_div(a: c_float, b: c_float) -> c_float {
    if a.is_nan() {
        quiet_nan(a)
    } else if b.is_nan() {
        quiet_nan(b)
    } else {
        a / b
    }
}

/// `fabsf()` from `<math.h>`: a pure sign-bit clear (`andps`), which — unlike an
/// arithmetic operation — never quiets a signalling NaN.
#[inline]
fn fabsf(x: c_float) -> c_float {
    f32::from_bits(x.to_bits() & 0x7fff_ffff)
}

/// `fmodf()` from `<math.h>`. Rust's `%` on `f32` is the truncated (C `fmod`)
/// remainder and lowers to a call to the very same libm routine the C code
/// uses, so the result — NaN payloads included — is identical.
#[inline]
fn fmodf(x: c_float, y: c_float) -> c_float {
    x % y
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// `void hsl_to_rgb(float *dest, const float *src);`
///
/// `src` is read as `[h, s, l]`, `dest` is written as `[r, g, b]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    let h: c_float = unsafe { *src.add(0) };
    let s: c_float = unsafe { *src.add(1) };
    let l: c_float = unsafe { *src.add(2) };

    let c: c_float;
    let m: c_float;
    let x: c_float;

    if s == 0.0 {
        unsafe {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }

    // c = (1.0f - fabsf(2.0f * l - 1.0f)) * s;
    // (`2.0f * l` is emitted as `l + l`, which is exact for every input.)
    c = ss_mul(ss_sub(1.0, fabsf(ss_sub(ss_add(l, l), 1.0))), s);

    // m = 1.0f * (l - 0.5f * c);
    // The multiplication by 1.0f is a no-op that the C compiler folds away;
    // `0.5f * c` is evaluated with `c` as the destination operand.
    m = ss_sub(l, ss_mul(c, 0.5));

    // x = c * (1.0f - fabsf(fmodf(h / 60.0f, 2) - 1.0f));
    // The product is evaluated with the parenthesised term as the destination
    // operand, so it takes precedence when both it and `c` are NaN.
    x = ss_mul(
        ss_sub(1.0, fabsf(ss_sub(fmodf(ss_div(h, 60.0), 2.0), 1.0))),
        c,
    );

    let (r, g, b): (c_float, c_float, c_float) = if h >= 0.0 && h < 60.0 {
        (ss_add(c, m), ss_add(x, m), m)
    } else if h >= 60.0 && h < 120.0 {
        (ss_add(x, m), ss_add(c, m), m)
    } else if h < 120.0 && h < 180.0 {
        // Faithful to the original C source, which tests `h < 120.0f` here
        // where `h >= 120.0f` was evidently intended.
        (m, ss_add(c, m), ss_add(x, m))
    } else if h >= 180.0 && h < 240.0 {
        (m, ss_add(x, m), ss_add(c, m))
    } else if h >= 240.0 && h < 300.0 {
        (ss_add(x, m), m, ss_add(c, m))
    } else if h >= 300.0 && h < 360.0 {
        (ss_add(c, m), m, ss_add(x, m))
    } else {
        (m, m, m)
    };

    unsafe {
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}
