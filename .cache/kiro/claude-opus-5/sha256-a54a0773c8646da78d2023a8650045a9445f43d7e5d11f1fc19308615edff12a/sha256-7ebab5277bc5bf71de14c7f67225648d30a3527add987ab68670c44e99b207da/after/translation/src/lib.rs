//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is a faithful, byte-for-byte reproduction of the original C,
//! including the bug in the third hue branch (the C source tests
//! `h < 120.0f && h < 180.0f` where `h >= 120.0f && h < 180.0f` was clearly
//! intended). That branch is therefore only reachable for `h < 0.0f`, which is
//! preserved here deliberately.
//!
//! # Bit-exact NaN propagation
//!
//! `float` arithmetic on x86-64 is performed by the scalar SSE instructions
//! (`addss`/`subss`/`mulss`/`divss`), whose NaN handling is *asymmetric*: the
//! result is the first (destination) operand when that operand is a NaN, and
//! only otherwise the second operand — in both cases forced to a quiet NaN by
//! setting the mantissa's most-significant bit, while the sign bit and the rest
//! of the payload are carried through untouched.
//!
//! Writing the expressions as plain Rust `+`/`-`/`*` does *not* reproduce this,
//! because LLVM freely rewrites them in ways that are value-preserving but not
//! NaN-payload-preserving: it turns `l - 0.5 * c` into `(-0.5 * c) + l` (folding
//! the negation into the literal) and commutes multiplications to save a
//! register. Either rewrite swaps which operand is "first", so a NaN input
//! surfaces with the wrong sign or payload.
//!
//! The arithmetic is therefore routed through the [`fadd`], [`fsub`], [`fmul`],
//! [`fdiv`] and [`fabs`] helpers, which spell out the NaN selection explicitly
//! and only fall back on the hardware for NaN-free operands. That makes the
//! observable behaviour independent of how LLVM schedules the code, and it
//! keeps the operand order of the reference build visible at each call site.

use std::ffi::c_float;

/// Quiet a NaN the way SSE does: set the mantissa's high bit, keep the sign
/// bit and the remaining payload bits as they are.
#[inline]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// The NaN an SSE scalar op would produce for these operands, if any.
///
/// `a` is the destination operand (`src1`), `b` the source operand (`src2`).
#[inline]
fn propagated_nan(a: f32, b: f32) -> Option<f32> {
    if a.is_nan() {
        Some(quiet_nan(a))
    } else if b.is_nan() {
        Some(quiet_nan(b))
    } else {
        None
    }
}

/// `addss a, b` — `a + b`.
#[inline]
fn fadd(a: f32, b: f32) -> f32 {
    match propagated_nan(a, b) {
        Some(n) => n,
        // Neither operand is a NaN, so the hardware result (including the
        // `inf + -inf` indefinite) needs no fixing up.
        None => a + b,
    }
}

/// `subss a, b` — `a - b`.
#[inline]
fn fsub(a: f32, b: f32) -> f32 {
    match propagated_nan(a, b) {
        Some(n) => n,
        None => a - b,
    }
}

/// `mulss a, b` — `a * b`.
#[inline]
fn fmul(a: f32, b: f32) -> f32 {
    match propagated_nan(a, b) {
        Some(n) => n,
        None => a * b,
    }
}

/// `divss a, b` — `a / b`.
#[inline]
fn fdiv(a: f32, b: f32) -> f32 {
    match propagated_nan(a, b) {
        Some(n) => n,
        None => a / b,
    }
}

/// `fabsf` as the reference build implements it: `andps` against `0x7fffffff`,
/// i.e. a pure bit operation that clears the sign of NaNs without quieting
/// them.
#[inline]
fn fabs(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7fff_ffff)
}

/// `fmodf(a, b)` — the truncated remainder, which Rust's `%` on `f32` lowers to
/// the very same libm entry point the C calls.
#[inline]
fn fmodf(a: f32, b: f32) -> f32 {
    a % b
}

/// Pure computation kernel: HSL triple in, RGB triple out.
///
/// Mirrors the C statement-for-statement so float rounding matches exactly.
/// The operand order of each helper call is the order the reference build's
/// instructions use, which for the two multiplications marked below is the
/// reverse of the C source text (commuting them is free for the compiler and
/// only observable in NaN payloads).
// The comparison chains are written exactly as the C spells them, including the
// third branch's redundant `h < 120.0 && h < 180.0`, which Clippy would
// otherwise simplify away and thereby change behaviour.
#[allow(
    clippy::redundant_comparisons,
    clippy::overly_complex_bool_expr,
    clippy::nonminimal_bool,
    clippy::manual_range_contains
)]
fn hsl_to_rgb_impl(src: [c_float; 3]) -> [c_float; 3] {
    let h = src[0];
    let s = src[1];
    let l = src[2];

    if s == 0.0 {
        return [l, l, l];
    }

    // c = (1.0f - fabsf(2.0f * l - 1.0f)) * s;
    // `2.0f * l` is emitted as `l + l`; both spellings select `l`'s NaN.
    let t1 = fsub(fadd(l, l), 1.0);
    let c = fmul(fsub(1.0, fabs(t1)), s);

    // m = 1.0f * (l - 0.5f * c);
    // `0.5f * c` is emitted with `c` first; the `1.0f *` is a no-op the
    // reference build drops entirely, so it is kept only as documentation.
    let m = fmul(1.0, fsub(l, fmul(c, 0.5)));

    // x = c * (1.0f - fabsf(fmodf(h / 60.0f, 2) - 1.0f));
    let t2 = fsub(fmodf(fdiv(h, 60.0), 2.0), 1.0);
    // Emitted with the parenthesised factor first, hence the swap.
    let x = fmul(fsub(1.0, fabs(t2)), c);

    if h >= 0.0f32 && h < 60.0f32 {
        [fadd(c, m), fadd(x, m), m]
    } else if h >= 60.0f32 && h < 120.0f32 {
        [fadd(x, m), fadd(c, m), m]
    } else if h < 120.0f32 && h < 180.0f32 {
        // Faithful reproduction of the original C condition (see module docs).
        [m, fadd(c, m), fadd(x, m)]
    } else if h >= 180.0f32 && h < 240.0f32 {
        [m, fadd(x, m), fadd(c, m)]
    } else if h >= 240.0f32 && h < 300.0f32 {
        [fadd(x, m), m, fadd(c, m)]
    } else if h >= 300.0f32 && h < 360.0f32 {
        [fadd(c, m), m, fadd(x, m)]
    } else {
        [m, m, m]
    }
}

/// C ABI entry point: `void hsl_to_rgb(float *dest, const float *src)`.
///
/// # Safety
///
/// `src` must point to at least 3 readable `float`s and `dest` to at least 3
/// writable `float`s. As in the C original, `dest` and `src` may overlap: the
/// source values are fully consumed before anything is written back.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    // Read the inputs up front (as the C does into locals) so that overlapping
    // `dest`/`src` buffers behave identically and no aliasing slices coexist.
    let input = [
        unsafe { *src.add(0) },
        unsafe { *src.add(1) },
        unsafe { *src.add(2) },
    ];

    let out = hsl_to_rgb_impl(input);

    let dest = unsafe { std::slice::from_raw_parts_mut(dest, 3) };
    dest.copy_from_slice(&out);
}
