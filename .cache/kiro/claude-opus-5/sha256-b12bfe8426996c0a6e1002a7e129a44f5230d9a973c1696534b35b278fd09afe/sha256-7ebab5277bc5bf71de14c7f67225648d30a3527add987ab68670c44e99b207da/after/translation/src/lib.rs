//! Rust translation of `c_src/src/lib.c`.
//!
//! Provides `tfm`, which walks a packed array of 3-float structure-tensor
//! entries (`dx2`, `dy2`, `dxy`) and writes a 2-float eigenvector-ish result
//! per entry.
//!
//! # Bit-exactness
//!
//! The floating point expressions below are transcribed verbatim from the C,
//! including association order, so that the emitted `f32` operations match the
//! original bit for bit.
//!
//! Because plain IEEE-754 semantics leave the *payload* of a NaN result
//! unspecified, matching the C byte for byte also requires reproducing the
//! operand-selection rule of the SSE scalar instructions that the C compiles
//! to (`addss`/`subss`/`mulss`/`sqrtss`): when the first source operand is a
//! NaN it is returned (quieted), otherwise a NaN second operand is returned
//! (quieted), and an invalid operation on non-NaN operands yields the "integer
//! indefinite" QNaN `0xFFC0_0000`. Relying on the Rust code generator to pick
//! the same operand order as the C compiler would be fragile, so the rule is
//! spelled out in [`sse_add`], [`sse_sub`], [`sse_mul`] and [`sse_sqrt`], and
//! each call site passes its operands in the same order the C uses.
//!
//! The operand order was read off the reference build, i.e. the one CMake
//! produces with no `CMAKE_BUILD_TYPE` (unoptimized). gcc picks a different
//! order at `-O1` and above, so the two C builds do not even agree with each
//! other on NaN payloads; this translation matches the reference build.

use std::ffi::c_int;

/// x86 "integer indefinite" single-precision QNaN, the result of an invalid
/// operation (`inf - inf`, `0 * inf`, `sqrt` of a negative) on operands that
/// are not themselves NaN.
const INDEFINITE_NAN: u32 = 0xFFC0_0000;

/// Mask that turns a signalling NaN into the corresponding quiet NaN while
/// preserving its sign and payload, exactly as the SSE instructions do when
/// they forward a NaN operand.
const QUIET_BIT: u32 = 0x0040_0000;

#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | QUIET_BIT)
}

/// Applies the SSE scalar NaN rule around a plain arithmetic operation.
///
/// `a` is the instruction's destination (first source) operand and `b` the
/// second source operand.
#[inline]
fn sse_binop(a: f32, b: f32, value: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else if value.is_nan() {
        // Neither operand was a NaN, so a NaN result means the operation was
        // invalid and the hardware produces the indefinite QNaN.
        f32::from_bits(INDEFINITE_NAN)
    } else {
        value
    }
}

/// `addss a, b` — computes `a + b`.
#[inline]
fn sse_add(a: f32, b: f32) -> f32 {
    sse_binop(a, b, a + b)
}

/// `subss a, b` — computes `a - b`.
#[inline]
fn sse_sub(a: f32, b: f32) -> f32 {
    sse_binop(a, b, a - b)
}

/// `mulss a, b` — computes `a * b`.
#[inline]
fn sse_mul(a: f32, b: f32) -> f32 {
    sse_binop(a, b, a * b)
}

/// `sqrtf(x)`, matching `sqrtss`: NaN in, quieted NaN out; negative in, the
/// indefinite QNaN out. `-0.0` yields `-0.0`.
#[inline]
fn sse_sqrt(x: f32) -> f32 {
    if x.is_nan() {
        quiet(x)
    } else if x < 0.0f32 {
        f32::from_bits(INDEFINITE_NAN)
    } else {
        x.sqrt()
    }
}

/// Reproduces the C expression `((0) > (sqd)) ? (0) : (sqd)`.
///
/// In C the integer `0` is converted to `float` for both the comparison and
/// the result type. Note that this is *not* `f32::max`: when `sqd` is NaN the
/// comparison is false and the NaN is returned unchanged, which is the
/// behavior the C code has and which must be preserved. `-0.0` is likewise
/// returned unchanged, since `0.0 > -0.0` is false.
#[inline]
fn clamp_low_zero(sqd: f32) -> f32 {
    if 0.0f32 > sqd { 0.0f32 } else { sqd }
}

/// Computes `sqd` exactly as written in the C source:
///
/// `(dy2 * dy2) - (2.0f * dx2 * dy2) + (dx2 * dx2) + (4.0f * dxy * dxy)`
///
/// with C's left-to-right association for `-`/`+` and for the `*` chains, and
/// with each operation's operands in the order the compiled C uses.
#[inline]
fn sqd_of(dx2: f32, dy2: f32, dxy: f32) -> f32 {
    let dy2_sq = sse_mul(dy2, dy2);
    // The C compiler strength-reduces `2.0f * dx2` to `dx2 + dx2`, which is
    // exact and agrees for every input (including NaN operand forwarding).
    let two_dx2 = sse_add(dx2, dx2);
    let cross = sse_mul(two_dx2, dy2);
    let head = sse_sub(dy2_sq, cross);
    let dx2_sq = sse_mul(dx2, dx2);
    let head = sse_add(head, dx2_sq);
    let four_dxy = sse_mul(4.0f32, dxy);
    let dxy_term = sse_mul(four_dxy, dxy);
    // Note the operand order: the `4*dxy*dxy` term is the destination operand
    // of the final add, so its NaN wins over the accumulated one.
    sse_add(dxy_term, head)
}

/// Computes `lambda` as written in the C source:
/// `0.5f * (dy2 + dx2 + sqrtf(max(0, sqd)))`.
#[inline]
fn lambda_of(dx2: f32, dy2: f32, sqd: f32) -> f32 {
    let sum = sse_add(dy2, dx2);
    let root = sse_sqrt(clamp_low_zero(sqd));
    let inner = sse_add(sum, root);
    sse_mul(0.5f32, inner)
}

/// Safe core of the transform, operating on one entry at a time.
///
/// `src_chunk` is the 3-float input entry, `dest_chunk` the 2-float output.
#[inline]
fn tfm_one(dest_chunk: &mut [f32], src_chunk: &[f32]) {
    if src_chunk[0] < src_chunk[1] {
        let dx2 = src_chunk[0];
        let dy2 = src_chunk[1];
        let dxy = src_chunk[2];
        let sqd = sqd_of(dx2, dy2, dxy);
        let lambda = lambda_of(dx2, dy2, sqd);
        dest_chunk[0] = sse_sub(dx2, lambda);
        dest_chunk[1] = dxy;
    } else {
        // The C swaps which input slot feeds dy2/dx2 in this branch.
        let dy2 = src_chunk[0];
        let dx2 = src_chunk[1];
        let dxy = src_chunk[2];
        let sqd = sqd_of(dx2, dy2, dxy);
        let lambda = lambda_of(dx2, dy2, sqd);
        dest_chunk[0] = dxy;
        dest_chunk[1] = sse_sub(dx2, lambda);
    }
}

/// C entry point: `void tfm(float *dest, const float *src, int count);`
///
/// # Safety
///
/// `src` must point to at least `count * 3` readable `f32`s and `dest` to at
/// least `count * 2` writable `f32`s. As in the C, a non-positive `count` does
/// nothing and the pointers are never dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    // Matches the C loop guard `i < count`: nothing to do for count <= 0, and
    // in that case the pointers are not touched at all.
    if count <= 0 {
        return;
    }
    let n = count as usize;

    // Raw pointer stepping mirrors the C exactly (read the three inputs, then
    // write the two outputs, one entry at a time), so behavior is preserved
    // even when the caller lets `dest` and `src` overlap.
    for i in 0..n {
        let src_chunk = unsafe { std::slice::from_raw_parts(src.add(i * 3), 3) };
        let entry = [src_chunk[0], src_chunk[1], src_chunk[2]];
        let dest_chunk = unsafe { std::slice::from_raw_parts_mut(dest.add(i * 2), 2) };
        tfm_one(dest_chunk, &entry);
    }
}
