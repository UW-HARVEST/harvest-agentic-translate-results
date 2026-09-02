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
//! non-NaN input, but the result is *operand-order sensitive* when BOTH operands
//! of an operation are NaN. On x86-64 a two-operand SSE instruction such as
//! `subss dst, src` returns the *destination* operand's NaN (quieted) when both
//! operands are NaN, so the result's sign bit and payload depend on which
//! operand the compiler placed in the destination register.
//!
//! The `add`/`sub`/`mul`/`div` helpers below make that choice explicit instead of
//! leaving it to the optimiser: the first argument is the SSE destination
//! operand. Every operand role was read off the compiled C
//! (`objdump -d` on `c_src/build/lib*.so`, built at `-O0` since
//! `CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`), not guessed from source order —
//! note that `x = c * (...)` in C compiles with the *parenthesised term* as the
//! destination and `c` as the source, so the helper call is
//! `mul(term, c)`, mirroring the instruction rather than the expression.
//!
//! This is not merely defensive. It caught a real divergence: in hue arms 3-6 the
//! C computes the third channel as `x + m` / `c + m` with `x`/`c` as the `addss`
//! destination, and an earlier version of this file had the operands the other
//! way round. That is observable whenever `l` is NaN, because `fabsf` is a pure
//! `andps` that clears the sign bit without quieting — so `c` and `x` come out
//! sign-positive while `m = l - 0.5*c` re-propagates `l` and keeps `l`'s sign.
//! With `l = -NaN` the two orders therefore yield NaNs with different sign bits.
//! `tests/phase_b_configs.rs::cfg_row24_edge_lightness_patterns` pins this.
//!
//! For every input where at most one operand is NaN the helpers are plain IEEE
//! arithmetic, so this costs nothing in fidelity elsewhere.
//!
//! Verified-equivalent alternatives (checked by `mutation_check.sh`, which
//! confirms these mutations are *not* observable, i.e. they are true
//! equivalences rather than gaps in the test suite):
//!
//! * `mul(c, 0.5)` vs `mul(0.5, c)` — `0.5` is never NaN, so operand order
//!   cannot matter.
//! * `add(l, l)` vs `mul(2.0, l)` — the C emits `addss xmm0, xmm0`; rustc emits
//!   the same for `2.0 * l`, and for non-NaN input the two are exactly equal.
//! * quieting inside `fabsf` — the only consumer of `fabsf` here is
//!   `sub(1.0, ·)`, which quiets the NaN itself, so an early quiet is invisible.
//! * naive `l - 0.5 * c` — rustc rewrites this to `l + (-0.5) * c`
//!   (`mulss xmm1, -0.5` / `addss xmm0, xmm1`), which keeps `l` as the
//!   destination operand exactly as the C's `subss xmm0, xmm1` does. The
//!   explicit helper is retained so the property does not silently depend on
//!   that codegen choice.

#![allow(clippy::missing_safety_doc)]
// These lints all fire on constructs that are deliberate fidelity choices. The C
// is the ground truth and must not be "cleaned up":
//
// * `redundant_comparisons` is DENY-by-default and fires on arm 3's
//   `h < 120.0f32 && h < 180.0f32`. Clippy is right that the second test is
//   redundant — that IS the bug in the original C (`c_src/src/lib.c:27` says
//   `h < 120.0f`, not `h >= 120.0f`), and reproducing it is the whole point.
// * `manual_range_contains` would rewrite `h >= 60.0 && h < 120.0` as
//   `(60.0..120.0).contains(&h)`. Kept explicit so each predicate maps 1:1 onto
//   the `comiss`/`jb`/`jbe` pair the C emits.
// * `needless_late_init` fires on `let c: f32; let m: f32; let x: f32;`, which
//   mirrors the C's `float c, m, x;` declaration.
#![allow(clippy::redundant_comparisons)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::unusual_byte_groupings)]

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
///
/// The loads and stores go through `ptr::read`/`ptr::write` rather than `*p` /
/// `*p = v`. That is deliberate and load-bearing for fidelity: with
/// `debug-assertions` on, rustc inserts a null/alignment UB check around a raw
/// place projection (`*p`), so a null argument makes the function *abort*
/// (`SIGABRT`). The C has no such check and dies with `SIGSEGV`. `ptr::read` and
/// `ptr::write` live in the precompiled standard library, whose UB checks are
/// off, so they fault exactly like the C in every profile. Pinned by
/// `tests/phase_c_errors.rs` rows 15-18, which compare the fatal signal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut f32, src: *const f32) {
    let h: f32 = unsafe { core::ptr::read(src.add(0)) };
    let s: f32 = unsafe { core::ptr::read(src.add(1)) };
    let l: f32 = unsafe { core::ptr::read(src.add(2)) };

    let c: f32;
    let m: f32;
    let x: f32;

    if s == 0.0f32 {
        unsafe {
            core::ptr::write(dest.add(0), l);
            core::ptr::write(dest.add(1), l);
            core::ptr::write(dest.add(2), l);
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
    // Operand order within each `add` mirrors the compiled C: gcc emits
    // `movss xmm0, <first>` then `addss xmm0, <second>`, so the FIRST source
    // operand is the SSE destination and therefore the NaN that wins when both
    // operands are NaN. In every arm the C loads `c` or `x` into xmm0 and adds
    // `m` from memory, i.e. it is always `c + m` / `x + m`, never `m + c`.
    let (r, g, b): (f32, f32, f32) = if h >= 0.0f32 && h < 60.0f32 {
        (add(c, m), add(x, m), m)
    } else if h >= 60.0f32 && h < 120.0f32 {
        (add(x, m), add(c, m), m)
    } else if h < 120.0f32 && h < 180.0f32 {
        // NOTE: faithful copy of the original (buggy) predicate; the C says
        // `h < 120.0f`, not `h >= 120.0f`. Arms 1-2 already consumed
        // `[0, 120)`, so this arm is reachable only for `h < 0`.
        (m, add(c, m), add(x, m))
    } else if h >= 180.0f32 && h < 240.0f32 {
        (m, add(x, m), add(c, m))
    } else if h >= 240.0f32 && h < 300.0f32 {
        (add(x, m), m, add(c, m))
    } else if h >= 300.0f32 && h < 360.0f32 {
        (add(c, m), m, add(x, m))
    } else {
        (m, m, m)
    };

    unsafe {
        core::ptr::write(dest.add(0), r);
        core::ptr::write(dest.add(1), g);
        core::ptr::write(dest.add(2), b);
    }
}
