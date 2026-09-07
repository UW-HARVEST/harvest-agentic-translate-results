//! Rust translation of `c_src/src/lib.c`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `hsv_to_rgb`
//!
//! Behaviour is reproduced exactly, including the original code's quirks:
//! no range clamping or wrapping of the hue, the exact `s == 0` float
//! comparison (which `-0.0` also satisfies), the `default:` switch arm being
//! reached for any sector index outside `0..=4` (negative hues included), and
//! the undefined `(int)floorf(...)` conversion.
//!
//! It also reproduces the *bit-level* results for non-finite inputs. The C
//! compiler lowers each arithmetic operation to a single SSE instruction, and
//! SSE NaN propagation is operand-order sensitive: `MULSS`/`SUBSS`/`DIVSS`
//! return the FIRST (destination) operand when both operands are NaN, and the
//! x86 "QNaN floating-point indefinite" `0xFFC0_0000` (note the set sign bit)
//! for an invalid operation such as `0 * inf` or `inf - inf`. Plain Rust
//! `*`/`-`/`/` leave the operand order to LLVM, which is free to commute, so
//! the operations are spelled out below with the exact `src1`/`src2` ordering
//! taken from the reference build's disassembly.

use std::ffi::c_float;

/// x86 "QNaN floating-point indefinite" — the result SSE produces for an
/// invalid operation on non-NaN operands. Its sign bit is set.
const INDEFINITE: u32 = 0xFFC0_0000;

/// Force a NaN to be quiet, preserving its sign and payload, exactly as SSE
/// does when propagating a signalling NaN.
#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// Shared SSE NaN/invalid-operation dispatch.
///
/// `a` is the instruction's first (destination) operand, `b` the second, which
/// is the priority order SSE uses:
/// 1. `src1` NaN  -> quieted `src1`
/// 2. `src2` NaN  -> quieted `src2`
/// 3. invalid op  -> `0xFFC0_0000`
/// 4. otherwise   -> the IEEE result
#[inline]
fn sse_op(a: f32, b: f32, op: impl FnOnce(f32, f32) -> f32) -> f32 {
    if a.is_nan() {
        return quiet(a);
    }
    if b.is_nan() {
        return quiet(b);
    }
    let r = op(a, b);
    if r.is_nan() {
        // Neither operand was NaN, so a NaN result can only come from an
        // invalid operation (`0 * inf`, `inf - inf`, `0 / 0`, `inf / inf`).
        return f32::from_bits(INDEFINITE);
    }
    r
}

/// `MULSS a, b` — `a` is the destination operand.
#[inline]
fn fmul(a: f32, b: f32) -> f32 {
    sse_op(a, b, |x, y| x * y)
}

/// `SUBSS a, b` — computes `a - b`, `a` is the destination operand.
#[inline]
fn fsub(a: f32, b: f32) -> f32 {
    sse_op(a, b, |x, y| x - y)
}

/// `DIVSS a, b` — computes `a / b`, `a` is the destination operand.
#[inline]
fn fdiv(a: f32, b: f32) -> f32 {
    sse_op(a, b, |x, y| x / y)
}

unsafe extern "C" {
    /// The very same `floorf` the C build calls (`call floorf@plt`), so hue
    /// flooring — including NaN quieting and sign handling — is identical by
    /// construction rather than by assumption.
    safe fn floorf(x: c_float) -> c_float;
}

/// Emulates the C cast `(int)x` for a `float` as the reference build performs
/// it (`cvttss2si`).
///
/// The C standard leaves out-of-range float-to-int conversions undefined; the
/// hardware produces the "integer indefinite" value `INT_MIN` for NaN and for
/// anything outside the representable range. Rust's `as` cast instead
/// saturates, so those cases are handled explicitly.
#[inline]
fn cvttss2si(x: f32) -> i32 {
    // 2147483648.0 == 2^31 is exactly representable as f32; -2^31 likewise.
    if x >= -2147483648.0f32 && x < 2147483648.0f32 {
        // In range: truncation toward zero, same as the C cast.
        x as i32
    } else {
        // Out of range, or NaN.
        i32::MIN
    }
}

/// Convert an HSV triple to RGB.
///
/// `src` must point to at least 3 readable `float`s (`h`, `s`, `v`) and `dest`
/// to at least 3 writable `float`s. Hue is expressed in degrees; saturation and
/// value are passed through untouched in the achromatic case. Nothing is
/// validated or clamped, matching the C.
///
/// All three inputs are read before anything is written, exactly as the C does,
/// so callers that alias `dest` with `src` observe the same results.
///
/// # Safety
///
/// Both pointers must be valid and properly aligned for at least three `float`
/// elements, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    // Raw reads/writes rather than slices: the C performs three independent
    // `movss` loads and three independent `movss` stores, with no aliasing
    // assumptions and no pointer validity checks, and an invalid pointer must
    // fault here just as it does there.
    let mut h: f32 = unsafe { src.read() };
    let s: f32 = unsafe { src.add(1).read() };
    let v: f32 = unsafe { src.add(2).read() };

    if s == 0.0 {
        unsafe {
            dest.write(v);
            dest.add(1).write(v);
            dest.add(2).write(v);
        }
        return;
    }

    // divss %xmm1(60.0), %xmm0(h)
    h = fdiv(h, 60.0f32);
    // call floorf ; cvttss2si %xmm0, %eax
    let i: i32 = cvttss2si(floorf(h));
    // cvtsi2ssl i, %xmm1 ; subss %xmm1, %xmm0(h)
    let f: f32 = fsub(h, i as f32);
    // subss s, %xmm0(1.0) ; mulss %xmm1(v), %xmm0
    let p: f32 = fmul(fsub(1.0f32, s), v);
    // mulss f, %xmm1(s) ; subss %xmm1, %xmm0(1.0) ; mulss %xmm1(v), %xmm0
    let q: f32 = fmul(fsub(1.0f32, fmul(s, f)), v);
    // subss f, %xmm0(1.0) ; mulss s, %xmm1 ; subss %xmm1, %xmm0(1.0) ; mulss v
    let t: f32 = fmul(fsub(1.0f32, fmul(fsub(1.0f32, f), s)), v);

    // The C `switch` compiles to `cmpl $0x4, i; ja default`, i.e. an unsigned
    // comparison, so every negative `i` also lands in `default:`.
    let (r, g, b): (f32, f32, f32) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    unsafe {
        dest.write(r);
        dest.add(1).write(g);
        dest.add(2).write(b);
    }
}
