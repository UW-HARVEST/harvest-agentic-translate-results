//! Rust translation of `c_src/src/lib.c`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `hsv_to_rgb`
//!
//! Behaviour is reproduced exactly, including the original code's quirks
//! (no range clamping of the hue, `s == 0` exact float comparison, and the
//! `default:` switch arm being reached for any sector index outside `0..=4`,
//! which includes negative hues).

use std::ffi::c_float;

/// Quiet a NaN the way x86-64 SSE does when it forwards a source operand: a
/// signalling NaN gets the significand MSB set, a quiet NaN passes through
/// unchanged. Sign and payload are preserved in both cases.
#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

// The three helpers below reproduce the NaN-propagation rule of the SSE scalar
// arithmetic instructions that GCC emits for `c_src/src/lib.c`:
//
//   "If either source operand is a NaN, the result is the FIRST source operand,
//    converted to a quiet NaN."
//
// Float multiplication is commutative in value but NOT in NaN sign/payload
// propagation, and the order of the machine operands is not always the order
// written in the C source. From `objdump -d` of the reference build:
//
//   h / 60.0f          -> divss with `h`            first
//   h - (float)i       -> subss with `h`            first
//   1.0f - s           -> subss with `1.0f`         first
//   v * (1.0f - s)     -> mulss with `(1.0f - s)`   first
//   s * f              -> mulss with `s`            first
//   v * (1.0f - s*f)   -> mulss with `(1.0f - s*f)` first
//   1.0f - f           -> subss with `1.0f`         first
//   s * (1.0f - f)     -> mulss with `(1.0f - f)`   first   <-- reversed vs. source
//   v * (1.0f - ...)   -> mulss with `(1.0f - ...)` first
//
// Writing the helpers explicitly makes the result independent of whatever
// operand order LLVM happens to pick for a commutative `fmul`, which otherwise
// differs between the debug and release profiles.

/// `a * b` with SSE NaN propagation, `a` being the first source operand.
#[inline]
fn c_mul(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a * b
    }
}

/// `a - b` with SSE NaN propagation, `a` being the first source operand.
#[inline]
fn c_sub(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a - b
    }
}

/// `a / b` with SSE NaN propagation, `a` being the first source operand.
#[inline]
fn c_div(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a / b
    }
}

/// `floorf(x)`, matching the NaN handling of the libm the C links against
/// (`roundss`-style: a NaN source operand is forwarded, quieted).
#[inline]
fn c_floorf(x: f32) -> f32 {
    if x.is_nan() { quiet(x) } else { x.floor() }
}

/// Emulates the C cast `(int)x` for a `float` on x86-64 / AArch64 with the
/// standard SSE / NEON conversion instructions.
///
/// The C standard leaves out-of-range float-to-int conversions undefined; the
/// hardware used by the reference build produces the "integer indefinite"
/// value `INT_MIN` for NaN and for anything outside `[INT_MIN, INT_MAX]`.
/// Rust's `as` cast instead saturates, so the out-of-range cases are handled
/// explicitly to keep the observable results identical.
#[inline]
fn c_float_to_int(x: f32) -> i32 {
    // 2147483648.0 == 2^31 is exactly representable as f32; -2^31 likewise.
    if x >= -2147483648.0f32 && x < 2147483648.0f32 {
        // In range: truncation toward zero, same as the C cast.
        x as i32
    } else {
        // Out of range or NaN.
        i32::MIN
    }
}

/// Convert an HSV triple to RGB.
///
/// `src` must point to at least 3 readable `float`s (`h`, `s`, `v`) and `dest`
/// to at least 3 writable `float`s. Hue is expressed in degrees; saturation and
/// value are passed through untouched in the achromatic case.
///
/// # Safety
///
/// Both pointers must be valid, non-null and properly aligned for at least
/// three `float` elements, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    // Raw pointer reads/writes rather than slices: the C function loads all
    // three inputs into locals before its first store, so callers may pass
    // overlapping (or identical) `dest` and `src` for an in-place conversion.
    // Materialising a `&[f32]` and a `&mut [f32]` over the same memory would be
    // undefined behaviour in Rust and could let LLVM's `noalias` reasoning
    // reorder the stores ahead of the loads.
    let mut h: f32 = unsafe { std::ptr::read(src) };
    let s: f32 = unsafe { std::ptr::read(src.add(1)) };
    let v: f32 = unsafe { std::ptr::read(src.add(2)) };

    if s == 0.0 {
        unsafe {
            std::ptr::write(dest, v);
            std::ptr::write(dest.add(1), v);
            std::ptr::write(dest.add(2), v);
        }
        return;
    }

    h = c_div(h, 60.0f32);
    let i: i32 = c_float_to_int(c_floorf(h));
    let f: f32 = c_sub(h, i as f32);
    // Operand order below mirrors the emitted SSE instructions, not the C
    // source's textual order (see the note on `c_mul`).
    let p: f32 = c_mul(c_sub(1.0f32, s), v);
    let q: f32 = c_mul(c_sub(1.0f32, c_mul(s, f)), v);
    let t: f32 = c_mul(c_sub(1.0f32, c_mul(c_sub(1.0f32, f), s)), v);

    // `switch (i)` compiles to an UNSIGNED `cmpl $4 / ja`, so every negative
    // index also lands in `default:`.
    let (r, g, b): (f32, f32, f32) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    unsafe {
        std::ptr::write(dest, r);
        std::ptr::write(dest.add(1), g);
        std::ptr::write(dest.add(2), b);
    }
}
