//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `hsv_to_rgb`
//!
//! The translation is intentionally literal: operation order, `f32` arithmetic,
//! the `(int)floorf(h)` truncation and the `switch` fall-through structure are
//! reproduced exactly so that the output is bit-identical to the C version.

use std::ffi::c_float;
use std::ffi::c_int;

/// Reproduces the target machine's native `float` -> `int` truncating
/// conversion, matching what the C compiler emits for `(int)expr`.
///
/// Rust's `as` cast *saturates* (NaN -> 0, out-of-range -> `i32::MIN`/`i32::MAX`),
/// but C leaves the out-of-range case undefined, so the observable behaviour of
/// the C shared object is whatever the hardware does. On x86-64 the compiler
/// emits `cvttss2si`, which raises #IA and returns the "integer indefinite"
/// value `0x8000_0000` for NaN *and* for any value outside `[-2^31, 2^31)`.
/// Faithfully mirroring that is required for bit-identical output (e.g. a NaN
/// hue must select the `switch` `default` arm, not `case 0`).
#[inline]
#[cfg(target_arch = "x86_64")]
fn c_float_to_int(x: c_float) -> c_int {
    if x.is_nan() || x >= 2_147_483_648.0f32 || x <= -2_147_483_648.0f32 {
        c_int::MIN
    } else {
        x as c_int
    }
}

/// Non-x86-64 fallback: AArch64's `fcvtzs` (and most other ISAs) clamp exactly
/// the way Rust's saturating `as` cast does, so the plain cast is already the
/// faithful translation there.
#[inline]
#[cfg(not(target_arch = "x86_64"))]
fn c_float_to_int(x: c_float) -> c_int {
    x as c_int
}

/// ```c
/// void hsv_to_rgb(float *dest, const float *src);
/// ```
///
/// `src` is read as `[h, s, v]`, `dest` is written as `[r, g, b]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    // float r, g, b;
    // float f, p, q, t;
    let r: c_float;
    let g: c_float;
    let b: c_float;
    let f: c_float;
    let p: c_float;
    let q: c_float;
    let t: c_float;

    // float h = src[0]; float s = src[1]; float v = src[2];
    let mut h: c_float = *src.add(0);
    let s: c_float = *src.add(1);
    let v: c_float = *src.add(2);

    // int i;
    let i: c_int;

    // if (s == 0) { dest[0] = dest[1] = dest[2] = v; return; }
    if s == 0.0 {
        *dest.add(0) = v;
        *dest.add(1) = v;
        *dest.add(2) = v;
        return;
    }

    // h /= 60.0f;
    h /= 60.0f32;
    // i = (int)floorf(h);
    i = c_float_to_int(h.floor());
    // f = h - i;
    f = h - (i as c_float);
    // p = v * (1 - s);
    p = v * (1.0f32 - s);
    // q = v * (1 - s * f);
    q = v * (1.0f32 - s * f);
    // t = v * (1 - s * (1 - f));
    t = v * (1.0f32 - s * (1.0f32 - f));

    match i {
        0 => {
            r = v;
            g = t;
            b = p;
        }
        1 => {
            r = q;
            g = v;
            b = p;
        }
        2 => {
            r = p;
            g = v;
            b = t;
        }
        3 => {
            r = p;
            g = q;
            b = v;
        }
        4 => {
            r = t;
            g = p;
            b = v;
        }
        _ => {
            r = v;
            g = p;
            b = q;
        }
    }

    // dest[0] = r; dest[1] = g; dest[2] = b;
    *dest.add(0) = r;
    *dest.add(1) = g;
    *dest.add(2) = b;
}
