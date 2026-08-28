//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object) — exactly one symbol:
//!   * `tfm`
//!
//! The header `c_src/include/lib.h` declares no namespace/renaming macros, so
//! the linker symbol is the plain source-level name `tfm`.
//!
//! # Bit-exactness
//!
//! The C source is reproduced literally, including the operand ordering of
//! every floating point expression, so that results are bit-for-bit identical.
//!
//! One subtlety required care. A naive transliteration compiles to the same
//! *arithmetic* as GCC, but LLVM freely **commutes** the operands of `addss` /
//! `mulss` (and auto-vectorizes `mulps`), because FP add/mul are commutative
//! for ordinary values. They are *not* commutative in NaN payload propagation:
//! x86 SSE returns the quieted **destination** operand if it is NaN, and only
//! otherwise the source operand. So `(-nan) + (+nan)` yields `-nan` or `+nan`
//! depending purely on which operand the compiler put in the destination
//! register. That made expressions mixing several NaNs (reachable from inputs
//! such as `(0.0, -inf, NaN)`, where `2.0f*dx2*dy2` is `-inf*0.0` and produces
//! the x86 "QNaN indefinite" `0xFFC00000`) differ from GCC's output.
//!
//! The helpers below therefore compute the operation normally — which is
//! bit-exact whenever the result is not NaN — and, only on the rare NaN
//! outcome, impose x86 SSE's deterministic NaN selection rules. Because every
//! NaN outcome is decided explicitly, the optimizer is free to reorder or
//! vectorize the underlying arithmetic without changing the result, and the
//! operand order that GCC actually chose is then reproduced explicitly at each
//! call site.
//!
//! Worth recording: **the C library is not self-consistent here.** GCC picks
//! different operand orders at different optimization levels, so `-O0`,
//! `-O1`/`-O2`/`-Os` and `-O3` builds of `c_src` disagree with *each other* on
//! these NaN payloads. The divergence needs a NaN *input*: with finite inputs
//! every NaN that can arise is the canonical indefinite `0xFFC00000`, so
//! payload selection cannot matter and all builds agree (verified over 18M
//! random finite inputs). This translation matches the build that `c_src`'s own
//! `CMakeLists.txt` produces — `CMAKE_BUILD_TYPE` is unset, so no `-O` flag is
//! passed, i.e. `-O0`, which is also what a plain `gcc -shared` glob gives.
//!
//! Original C:
//! ```c
//! void tfm(float *dest, const float *src, int count) {
//!     int i;
//!     for (i = 0; i < count; i++) {
//!         if (src[0] < src[1]) {
//!             float dx2 = src[0];
//!             float dy2 = src[1];
//!             float dxy = src[2];
//!             float sqd = (dy2 * dy2) - (2.0f * dx2 * dy2) + (dx2 * dx2) +
//!                         (4.0f * dxy * dxy);
//!             float lambda =
//!                 0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
//!             dest[0] = dx2 - lambda;
//!             dest[1] = dxy;
//!         } else {
//!             float dy2 = src[0];
//!             float dx2 = src[1];
//!             float dxy = src[2];
//!             float sqd = (dy2 * dy2) - (2.0f * dx2 * dy2) + (dx2 * dx2) +
//!                         (4.0f * dxy * dxy);
//!             float lambda =
//!                 0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
//!             dest[0] = dxy;
//!             dest[1] = dx2 - lambda;
//!         }
//!         src += 3;
//!         dest += 2;
//!     }
//! }
//! ```

use core::ffi::c_int;

/// x86 "QNaN floating-point indefinite": the NaN produced by SSE when an
/// operation is invalid but neither operand was already a NaN (`inf - inf`,
/// `0 * inf`, `sqrt` of a negative). Note the set sign bit.
const INDEFINITE: u32 = 0xFFC0_0000;

/// Quiet a NaN the way SSE does when it propagates an operand: set the
/// mantissa MSB, leaving the sign bit and the rest of the payload alone.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// Resolve the NaN result of a binary SSE op with `a` as the destination
/// operand and `b` as the source operand.
///
/// Kept out of line and `cold`: NaN results are the rare case, so the hot path
/// stays a bare FP instruction plus a NaN test.
#[cold]
#[inline(never)]
fn nan_binop(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        // Destination operand is NaN: it wins, quieted.
        quiet(a)
    } else if b.is_nan() {
        // Otherwise the source operand, quieted.
        quiet(b)
    } else {
        // Neither operand was NaN, so the op itself was invalid.
        f32::from_bits(INDEFINITE)
    }
}

/// `a + b` with x86 SSE `ADDSS a, b` NaN semantics (`a` = destination).
#[inline(always)]
fn fadd(a: f32, b: f32) -> f32 {
    let r = a + b;
    if r.is_nan() {
        nan_binop(a, b)
    } else {
        r
    }
}

/// `a - b` with x86 SSE `SUBSS a, b` NaN semantics (`a` = destination).
#[inline(always)]
fn fsub(a: f32, b: f32) -> f32 {
    let r = a - b;
    if r.is_nan() {
        nan_binop(a, b)
    } else {
        r
    }
}

/// `a * b` with x86 SSE `MULSS a, b` NaN semantics (`a` = destination).
#[inline(always)]
fn fmul(a: f32, b: f32) -> f32 {
    let r = a * b;
    if r.is_nan() {
        nan_binop(a, b)
    } else {
        r
    }
}

/// `sqrtf(x)`.
///
/// The reference build calls glibc's `sqrtf@plt`. Measured against glibc, a NaN
/// argument comes back as exactly `quiet(x)` (sign and payload preserved,
/// mantissa MSB forced on), which is what `SQRTSS` does too.
///
/// The negative-argument case is unreachable: the preceding `comiss`/`jbe`
/// already replaced any negative `sqd` with the `0.0f` constant from `.rodata`
/// before the call, so `errno` is never set. It is handled here for
/// completeness, using SSE's QNaN indefinite.
#[inline(always)]
fn fsqrt(x: f32) -> f32 {
    if x.is_nan() {
        return quiet(x);
    }
    if x < 0.0f32 {
        // `sqrtss` of a negative operand yields QNaN indefinite. (`-0.0f` is
        // not negative by this test and correctly returns `-0.0f` below.)
        return f32::from_bits(INDEFINITE);
    }
    x.sqrt()
}

/// Faithful translation of the C macro expansion
/// `(((0) > (sqd)) ? (0) : (sqd))`.
///
/// In C the integer literal `0` is converted to `float` by the usual arithmetic
/// conversions on both the relational and the conditional operator, so the
/// whole expression has type `float`. This is deliberately *not* `f32::max` /
/// `fmaxf`:
///   * when `sqd` is NaN the comparison `0 > sqd` is false, so the NaN is
///     returned and `sqrtf` then propagates it;
///   * when `sqd` is `-0.0f` the comparison is false as well, so `-0.0f` is
///     returned and `sqrtf(-0.0f)` yields `-0.0f`.
///
/// (LLVM lowers this to `maxss`, which happens to have exactly these
/// semantics, so the transformation is sound.)
#[inline(always)]
fn clamp_nonneg_c(sqd: f32) -> f32 {
    if 0.0f32 > sqd {
        0.0f32
    } else {
        sqd
    }
}

/// The shared body of the two branches, kept as a single helper so the operand
/// order of the arithmetic is written exactly once.
///
/// Computes
/// ```text
/// sqd    = (dy2*dy2) - (2.0f*dx2*dy2) + (dx2*dx2) + (4.0f*dxy*dxy)
/// lambda = 0.5f * (dy2 + dx2 + sqrtf(max(0, sqd)))
/// ```
/// with C's left-to-right associativity:
/// `((((dy2*dy2) - ((2.0f*dx2)*dy2)) + (dx2*dx2)) + ((4.0f*dxy)*dxy))`.
///
/// Note the two C branches bind `dx2`/`dy2` to *opposite* inputs, so they are
/// genuinely different rounding sequences; passing the arguments in the right
/// order at each call site is what reproduces that.
#[inline(always)]
fn lambda_of(dx2: f32, dy2: f32, dxy: f32) -> f32 {
    // Accumulator for `(dy2*dy2) - (2.0f*dx2*dy2) + (dx2*dx2)`, in source order.
    // `2.0f * dx2` is emitted by GCC as `ADDSS dst=dx2, src=dx2`, which agrees
    // with `fmul(2.0, dx2)` on NaN propagation (only `dx2` can be the NaN) and
    // is exact for every finite value, so the plain multiply is kept.
    let acc: f32 = fadd(
        fsub(fmul(dy2, dy2), fmul(fmul(2.0f32, dx2), dy2)),
        fmul(dx2, dx2),
    );

    // `4.0f * dxy * dxy`, emitted as `MULSS dst=4.0f, src=dxy` then
    // `MULSS dst=(4.0f*dxy), src=dxy`.
    let dxy_term: f32 = fmul(fmul(4.0f32, dxy), dxy);

    // Source order is `acc + (4.0f*dxy*dxy)`, but GCC commutes this add and
    // emits `ADDSS dst=(4.0f*dxy*dxy), src=acc` (`tfm+0xac` / `tfm+0x194` in
    // the reference object). The choice is observable: when `dxy` is a NaN and
    // `acc` is also NaN -- e.g. `dy2*dy2` and `dx2*dx2` both overflow to `inf`
    // so that `inf - inf` produces the indefinite `0xFFC00000` -- the
    // destination operand's payload is the one that survives. `dxy`'s NaN must
    // therefore be the destination.
    let sqd: f32 = fadd(dxy_term, acc);

    // `0.5f * (dy2 + dx2 + sqrtf(...))`, all in source order here:
    //   `ADDSS dst=dy2,        src=dx2`
    //   `ADDSS dst=(dy2+dx2),  src=sqrtf(...)`
    //   `MULSS dst=0.5f,       src=(...)`
    // The sum is the destination of the outer add, so its payload wins over the
    // square root's. (The `* 0.5f` order is immaterial: only one of `0.5f` and
    // the sum can ever be NaN.)
    fmul(0.5f32, fadd(fadd(dy2, dx2), fsqrt(clamp_nonneg_c(sqd))))
}

/// `void tfm(float *dest, const float *src, int count);`
///
/// Consumes `count` triples of floats from `src` and writes `count` pairs of
/// floats to `dest`.
///
/// # Safety
///
/// Same contract as the C function: `src` must be valid for reading
/// `3 * count` floats and `dest` must be valid for writing `2 * count` floats
/// whenever `count > 0`. Non-positive `count` performs no accesses at all,
/// exactly as the C `for` loop does.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    // Local mutable copies of the pointer parameters; C advances the
    // parameters themselves at the bottom of each iteration.
    let mut dest: *mut f32 = dest;
    let mut src: *const f32 = src;

    let mut i: c_int = 0;
    while i < count {
        let s0: f32 = unsafe { *src.add(0) };
        let s1: f32 = unsafe { *src.add(1) };
        let s2: f32 = unsafe { *src.add(2) };

        // NaN in either operand makes this false, selecting the `else` branch,
        // exactly as C's `<` does.
        if s0 < s1 {
            let dx2: f32 = s0;
            let dy2: f32 = s1;
            let dxy: f32 = s2;

            let lambda: f32 = lambda_of(dx2, dy2, dxy);

            unsafe {
                *dest.add(0) = fsub(dx2, lambda);
                *dest.add(1) = dxy;
            }
        } else {
            let dy2: f32 = s0;
            let dx2: f32 = s1;
            let dxy: f32 = s2;

            let lambda: f32 = lambda_of(dx2, dy2, dxy);

            unsafe {
                *dest.add(0) = dxy;
                *dest.add(1) = fsub(dx2, lambda);
            }
        }

        src = unsafe { src.add(3) };
        dest = unsafe { dest.add(2) };
        i = i.wrapping_add(1);
    }
}
