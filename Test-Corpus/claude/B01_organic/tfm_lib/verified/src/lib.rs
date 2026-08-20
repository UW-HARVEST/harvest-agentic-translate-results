//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (exactly matches `nm -D` on the C shared object):
//!   * `tfm`
//!
//! # Bit-exactness
//!
//! `tfm` is pure `float` arithmetic, so byte-identical output means matching
//! not only the IEEE-754 values but also the **NaN payloads** the C build
//! produces. Two hardware details make those payloads sensitive to how the C
//! was compiled:
//!
//! 1. On x86 SSE, a binary op such as `addss %src, %dst` whose operands are
//!    *both* NaN returns the (quieted) **destination** operand. Which C operand
//!    lands in the destination register is a codegen choice — `fadd`/`fmul` are
//!    commutative for values, so a compiler may freely swap them.
//! 2. When an invalid operation manufactures a NaN out of non-NaN operands
//!    (`inf - inf`, `0 * inf`, `sqrt(-x)`), x86 yields the "indefinite" QNaN
//!    `0xffc00000` — note the **set sign bit** — not `0x7fc00000`.
//!
//! Consequently `gcc` itself emits three mutually different payload behaviours
//! at `-O0`, `-O1`/`-O2`/`-Os`, and `-O3`. The reference targeted here is the
//! library exactly as `c_src/CMakeLists.txt` builds it: that file sets no
//! `CMAKE_BUILD_TYPE` and adds no optimization flags, so the reference is the
//! unoptimized build. Every *numeric* result is identical across all `-O`
//! levels; the levels differ only in NaN payloads, which are reachable only
//! when an input is NaN or when squaring an input overflows to infinity.
//!
//! To pin this down instead of trusting the optimizer, each
//! payload-sensitive operation goes through the `f*` helpers below, which model
//! x86 SSE propagation on raw bit patterns, in the operand order the reference
//! build emits. That order is plain C source order except for two spots, both
//! marked in `step`:
//!
//! * `2.0f * dx2` is emitted as `dx2 + dx2` (`addss`, destination `dx2`);
//! * the final `+ (4.0f * dxy * dxy)` is emitted **commuted**, with the
//!   `4*dxy*dxy` term as the destination, so its payload wins over the
//!   accumulator's.
//!
//! Because the helpers never let a NaN reach a hardware FP instruction, the
//! output is identical on any host, not just x86.
//!
//! No bugs are fixed: the inlined `max(0, sqd)` keeps its C semantics (it
//! propagates NaN and does not normalize `-0.0`, unlike `f32::max`), the
//! expressions keep their C evaluation order, and a non-positive `count`
//! still writes nothing.

#![allow(clippy::missing_safety_doc)]

use core::ffi::c_int;

/// The x86 "real indefinite" QNaN for `f32`, produced when an invalid
/// operation manufactures a NaN out of non-NaN operands.
const INDEFINITE: u32 = 0xffc0_0000;

/// Quiet bit of an `f32` significand.
const QUIET_BIT: u32 = 0x0040_0000;

const ZERO: u32 = 0x0000_0000; // 0.0f
const FOUR: u32 = 0x4080_0000; // 4.0f
const HALF: u32 = 0x3f00_0000; // 0.5f

#[inline(always)]
fn is_nan_bits(x: u32) -> bool {
    (x & 0x7f80_0000) == 0x7f80_0000 && (x & 0x007f_ffff) != 0
}

/// x86 quieting of a NaN operand: set the quiet bit, preserving the sign and
/// the rest of the payload. An already-quiet NaN passes through unchanged.
#[inline(always)]
fn quiet(x: u32) -> u32 {
    x | QUIET_BIT
}

/// x86 SSE NaN propagation for a two-operand op: `a` is the *destination*
/// operand, so its payload takes precedence. `None` if no operand is NaN.
#[inline(always)]
fn nan_result(a: u32, b: u32) -> Option<u32> {
    if is_nan_bits(a) {
        Some(quiet(a))
    } else if is_nan_bits(b) {
        Some(quiet(b))
    } else {
        None
    }
}

/// Maps a NaN freshly manufactured from non-NaN operands onto the x86
/// indefinite QNaN.
#[inline(always)]
fn fresh(v: f32) -> u32 {
    let bits = v.to_bits();
    if is_nan_bits(bits) {
        INDEFINITE
    } else {
        bits
    }
}

/// `addss %b, %a` — `a + b`, with `a` as the destination operand.
#[inline(always)]
fn fadd(a: u32, b: u32) -> u32 {
    match nan_result(a, b) {
        Some(r) => r,
        None => fresh(f32::from_bits(a) + f32::from_bits(b)),
    }
}

/// `subss %b, %a` — `a - b`, with `a` as the destination operand.
#[inline(always)]
fn fsub(a: u32, b: u32) -> u32 {
    match nan_result(a, b) {
        Some(r) => r,
        None => fresh(f32::from_bits(a) - f32::from_bits(b)),
    }
}

/// `mulss %b, %a` — `a * b`, with `a` as the destination operand.
#[inline(always)]
fn fmul(a: u32, b: u32) -> u32 {
    match nan_result(a, b) {
        Some(r) => r,
        None => fresh(f32::from_bits(a) * f32::from_bits(b)),
    }
}

/// `sqrtf` — quiets a NaN operand, yields the indefinite QNaN for a negative
/// operand, and leaves `-0.0` as `-0.0`.
#[inline(always)]
fn fsqrt(a: u32) -> u32 {
    if is_nan_bits(a) {
        return quiet(a);
    }
    let v = f32::from_bits(a);
    if v < 0.0 {
        return INDEFINITE;
    }
    v.sqrt().to_bits()
}

/// `comiss`-based ordered `<`: false whenever either operand is NaN.
#[inline(always)]
fn flt(a: u32, b: u32) -> bool {
    !is_nan_bits(a) && !is_nan_bits(b) && f32::from_bits(a) < f32::from_bits(b)
}

/// The body shared by both arms of the C `if`, parameterised exactly as the C
/// source names it. Returns `lambda`.
#[inline(always)]
fn lambda(dx2: u32, dy2: u32, dxy: u32) -> u32 {
    // float sqd = (dy2 * dy2) - (2.0f * dx2 * dy2) + (dx2 * dx2)
    //             + (4.0f * dxy * dxy);
    //
    // `2.0f * dx2` is emitted as the self-addition `dx2 + dx2`.
    let two_dx2_dy2 = fmul(fadd(dx2, dx2), dy2);
    let mut sqd = fsub(fmul(dy2, dy2), two_dx2_dy2);
    sqd = fadd(sqd, fmul(dx2, dx2));
    // The reference build emits this last addition commuted, with the
    // `4.0f * dxy * dxy` term as the destination operand.
    sqd = fadd(fmul(fmul(FOUR, dxy), dxy), sqd);

    // float lambda = 0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
    //
    // The ternary is *not* `f32::max`: for NaN `0 > sqd` is false, so the NaN
    // survives, and `-0.0` is likewise passed through unnormalized.
    let clamped = if flt(sqd, ZERO) { ZERO } else { sqd };
    fmul(HALF, fadd(fadd(dy2, dx2), fsqrt(clamped)))
}

/// One iteration of the C loop: consumes three input `float`s, produces two.
#[inline(always)]
fn step(s0: u32, s1: u32, s2: u32) -> (u32, u32) {
    if flt(s0, s1) {
        // if (src[0] < src[1])
        let (dx2, dy2, dxy) = (s0, s1, s2);
        let l = lambda(dx2, dy2, dxy);
        // dest[0] = dx2 - lambda;  dest[1] = dxy;
        (fsub(dx2, l), dxy)
    } else {
        let (dy2, dx2, dxy) = (s0, s1, s2);
        let l = lambda(dx2, dy2, dxy);
        // dest[0] = dxy;  dest[1] = dx2 - lambda;
        (dxy, fsub(dx2, l))
    }
}

/// ```c
/// void tfm(float *dest, const float *src, int count);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    // Operate on raw bit patterns so that no NaN payload can be laundered by
    // an incidental hardware move or arithmetic instruction.
    let src = src as *const u32;
    let dest = dest as *mut u32;

    let mut i: c_int = 0;
    while i < count {
        let inp = 3isize * i as isize;
        let s0 = src.offset(inp).read_unaligned();
        let s1 = src.offset(inp + 1).read_unaligned();
        let s2 = src.offset(inp + 2).read_unaligned();

        let (d0, d1) = step(s0, s1, s2);

        let out = 2isize * i as isize;
        dest.offset(out).write_unaligned(d0);
        dest.offset(out + 1).write_unaligned(d1);

        i += 1;
    }
}
