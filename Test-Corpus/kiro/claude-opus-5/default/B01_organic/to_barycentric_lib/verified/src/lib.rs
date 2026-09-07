//! Rust translation of `c_src/src/lib.c` (public header: `c_src/include/lib.h`).
//!
//! Public ABI surface of the C shared library (per `nm -D --defined-only`):
//!   * `to_barycentric`
//!
//! `lm_v2` / `lm_sub2` / `lm_dot2` are `static` in the C source, so they are not
//! exported; they are reproduced here as private helpers with identical
//! semantics.
//!
//! # Bit-exactness
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no optimisation flags,
//! so the reference library is built at `-O0`. At `-O0` GCC does not inline
//! `lm_v2`/`lm_sub2`/`lm_dot2` and lowers every C operator to exactly one scalar
//! SSE instruction (`subss` / `mulss` / `addss` / `divss`). Two properties of
//! that instruction stream are observable in the results and must be mirrored to
//! get byte-identical output over the whole input domain:
//!
//! 1. **Scalar, not packed.** Left to itself LLVM fuses the `x` and `y` lanes
//!    into packed `subps`/`mulps`/`addps`, which changes which operand a NaN is
//!    taken from.
//! 2. **Operand order of the commutative operations.** An x86 SSE binary op
//!    returns its *first* (destination) operand when both operands are NaN, so
//!    the operand order is observable through the resulting NaN sign and payload
//!    even though it is numerically irrelevant. GCC's `-O0` register allocator
//!    emits `lm_dot2` with the `y` product and the final addition *swapped*
//!    relative to the C source text:
//!
//!    ```text
//!    lm_dot2:                     ; a in xmm0 -> [-0x8], b in xmm1 -> [-0x10]
//!      movss -0x8(%rbp),%xmm1     ; a.x
//!      movss -0x10(%rbp),%xmm0    ; b.x
//!      mulss %xmm0,%xmm1          ; px = a.x * b.x   (destination = a.x)
//!      movss -0x4(%rbp),%xmm2     ; a.y
//!      movss -0xc(%rbp),%xmm0     ; b.y
//!      mulss %xmm2,%xmm0          ; py = b.y * a.y   (destination = b.y)
//!      addss %xmm1,%xmm0          ; py + px          (destination = py)
//!    ```
//!
//! Neither property survives compilation of ordinary Rust float expressions:
//! LLVM canonicalises commutative operands and vectorises the lanes. Emitting
//! each step as its own `asm!` statement is not sufficient either — the operand
//! order of a two-address inline-asm template is decided by the register
//! allocator, so it changes with the surrounding code and was observed to flip
//! for some of the `addss`/`mulss` steps here.
//!
//! Therefore, on x86-64 the whole arithmetic core is a single [`global_asm!`]
//! block ([`to_barycentric_core`]) that reproduces the C instruction stream
//! operation for operation, with the destination operand of every instruction
//! chosen to match GCC's. That is the only construction that is stable against
//! LLVM's register allocation.
//!
//! On non-x86-64 targets the arithmetic falls back to plain Rust `f32`
//! operations in C source order. That is identical for every finite, infinite
//! and single-NaN input, and can differ only in the payload chosen when two
//! different NaNs meet in one commutative operation.

#![allow(non_camel_case_types)]

/// ```c
/// typedef struct lm_vec2 {
///     float x, y;
/// } lm_vec2;
/// ```
///
/// Two consecutive `float`s: under the SysV x86-64 ABI this classifies as a
/// single SSE eightbyte, so it is passed and returned packed in one XMM
/// register. `repr(C)` makes rustc apply the same platform classification.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct lm_vec2 {
    pub x: f32,
    pub y: f32,
}

/// `static lm_vec2 lm_v2(float x, float y)`
///
/// A pure constructor; no arithmetic, so it needs no special treatment. On
/// x86-64 the packing it performs is done by the `unpcklps` at the end of
/// [`to_barycentric_core`], so this function is only reachable from the portable
/// fallback.
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
#[inline]
fn lm_v2(x: f32, y: f32) -> lm_vec2 {
    lm_vec2 { x, y }
}

// ---------------------------------------------------------------------------
// x86-64: exact instruction-for-instruction transcription of the C.
// ---------------------------------------------------------------------------

// The arithmetic core, transcribed from the `-O0` GCC instruction stream of
// `to_barycentric` (including the inlined bodies of `lm_sub2` and `lm_dot2`).
//
// The C's stack traffic is elided — spilling a `float` to memory and reloading
// it is bit-preserving — but every arithmetic instruction appears in the same
// order, with the same opcode, and with the same operand acting as the
// destination, which is what fixes NaN provenance.
//
// Marked `.hidden` so that the Rust `.so` exports exactly the same dynamic
// symbol set as the C `.so` (see `SYMBOLS.md`).
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".text",
    ".p2align 4",
    ".hidden to_barycentric_core",
    ".type to_barycentric_core,@function",
    "to_barycentric_core:",
    // SysV: xmm0 = p1, xmm1 = p2, xmm2 = p3, xmm3 = p, each packed as
    // { x = bits 0..32, y = bits 32..64 }. Return value: packed { u, v } in xmm0.
    //
    // ---- extract the `y` lanes into scalar position -----------------------
    "movaps xmm4, xmm0",
    "psrlq  xmm4, 32", // xmm4 = p1.y
    "movaps xmm5, xmm1",
    "psrlq  xmm5, 32", // xmm5 = p2.y
    "movaps xmm6, xmm2",
    "psrlq  xmm6, 32", // xmm6 = p3.y
    "movaps xmm7, xmm3",
    "psrlq  xmm7, 32", // xmm7 = p.y
    //
    // ---- lm_vec2 v0 = lm_sub2(p3, p1); v1 = lm_sub2(p2, p1); v2 = lm_sub2(p, p1)
    // `subss` is not commutative, so GCC has no operand freedom: the left C
    // operand is always the destination.
    "subss xmm2, xmm0", // xmm2 = v0.x = p3.x - p1.x
    "subss xmm6, xmm4", // xmm6 = v0.y = p3.y - p1.y
    "subss xmm1, xmm0", // xmm1 = v1.x = p2.x - p1.x
    "subss xmm5, xmm4", // xmm5 = v1.y = p2.y - p1.y
    "subss xmm3, xmm0", // xmm3 = v2.x = p.x  - p1.x
    "subss xmm7, xmm4", // xmm7 = v2.y = p.y  - p1.y
    //
    // ---- float dot00 = lm_dot2(v0, v0)  (a = b = v0) ----------------------
    "movaps xmm8, xmm2",
    "mulss  xmm8, xmm2", // px = v0.x * v0.x   (dest = a.x)
    "movaps xmm9, xmm6",
    "mulss  xmm9, xmm6", // py = v0.y * v0.y   (dest = b.y)
    "addss  xmm9, xmm8", // dot00 = py + px    (dest = py)
    //
    // ---- float dot01 = lm_dot2(v0, v1)  (a = v0, b = v1) ------------------
    "movaps xmm8, xmm2",
    "mulss  xmm8, xmm1", // px = v0.x * v1.x   (dest = a.x = v0.x)
    "movaps xmm10, xmm5",
    "mulss  xmm10, xmm6", // py = v1.y * v0.y  (dest = b.y = v1.y)
    "addss  xmm10, xmm8", // dot01 = py + px   (dest = py)
    //
    // ---- float dot02 = lm_dot2(v0, v2)  (a = v0, b = v2) ------------------
    "movaps xmm8, xmm2",
    "mulss  xmm8, xmm3", // px = v0.x * v2.x   (dest = a.x = v0.x)
    "movaps xmm11, xmm7",
    "mulss  xmm11, xmm6", // py = v2.y * v0.y  (dest = b.y = v2.y)
    "addss  xmm11, xmm8", // dot02 = py + px   (dest = py)
    //
    // ---- float dot11 = lm_dot2(v1, v1)  (a = b = v1) ----------------------
    "movaps xmm8, xmm1",
    "mulss  xmm8, xmm1", // px = v1.x * v1.x
    "movaps xmm12, xmm5",
    "mulss  xmm12, xmm5", // py = v1.y * v1.y
    "addss  xmm12, xmm8", // dot11 = py + px  (dest = py)
    //
    // ---- float dot12 = lm_dot2(v1, v2)  (a = v1, b = v2) ------------------
    "movaps xmm8, xmm1",
    "mulss  xmm8, xmm3", // px = v1.x * v2.x   (dest = a.x = v1.x)
    "movaps xmm13, xmm7",
    "mulss  xmm13, xmm5", // py = v2.y * v1.y  (dest = b.y = v2.y)
    "addss  xmm13, xmm8", // dot12 = py + px   (dest = py)
    //
    // live: xmm9 = dot00, xmm10 = dot01, xmm11 = dot02, xmm12 = dot11,
    //       xmm13 = dot12
    //
    // ---- float invDenom = 1.0f / (dot00 * dot11 - dot01 * dot01) ----------
    "movaps xmm0, xmm9",
    "mulss  xmm0, xmm12", // dot00 * dot11   (dest = dot00)
    "movaps xmm1, xmm10",
    "mulss  xmm1, xmm10", // dot01 * dot01
    "subss  xmm0, xmm1",  // denom           (dest = dot00*dot11)
    "mov    eax, 0x3f800000",
    "movd   xmm14, eax",  // 1.0f
    "divss  xmm14, xmm0", // invDenom = 1.0f / denom  (dest = 1.0f)
    //
    // ---- float u = (dot11 * dot02 - dot01 * dot12) * invDenom -------------
    "movaps xmm0, xmm12",
    "mulss  xmm0, xmm11", // dot11 * dot02   (dest = dot11)
    "movaps xmm1, xmm10",
    "mulss  xmm1, xmm13", // dot01 * dot12   (dest = dot01)
    "subss  xmm0, xmm1",  // numerator
    "mulss  xmm0, xmm14", // u = numerator * invDenom  (dest = numerator)
    //
    // ---- float v = (dot00 * dot12 - dot01 * dot02) * invDenom -------------
    "movaps xmm2, xmm9",
    "mulss  xmm2, xmm13", // dot00 * dot12   (dest = dot00)
    "movaps xmm1, xmm10",
    "mulss  xmm1, xmm11", // dot01 * dot02   (dest = dot01)
    "subss  xmm2, xmm1",  // numerator
    "mulss  xmm2, xmm14", // v = numerator * invDenom  (dest = numerator)
    //
    // ---- return lm_v2(u, v) ----------------------------------------------
    "unpcklps xmm0, xmm2", // xmm0 = { u, v, .. }
    "movq     xmm0, xmm0", // zero the upper eightbyte, as GCC's `movq` return does
    "ret",
    ".size to_barycentric_core,.-to_barycentric_core",
);

#[cfg(target_arch = "x86_64")]
extern "C" {
    /// See the `global_asm!` block above.
    fn to_barycentric_core(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2;
}

/// `lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p)`
///
/// The exported wrapper. It performs no arithmetic of its own; the sequence of
/// floating-point operations lives in [`to_barycentric_core`] (x86-64) or
/// [`to_barycentric_portable`] (everything else), in both cases matching the C
/// exactly — including the reciprocal-then-multiply form (`invDenom = 1.0f /
/// denom` followed by two multiplies) rather than a direct division, so the
/// single-precision rounding is identical.
///
/// A degenerate triangle gives a zero denominator and hence infinite/NaN
/// components exactly as the C does; that behaviour is reproduced, not "fixed".
/// There are no error returns to mirror: the C never rejects an input (see
/// `ERRORS.md`).
#[unsafe(no_mangle)]
pub extern "C" fn to_barycentric(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: `to_barycentric_core` is the `extern "C"` assembly routine
        // defined above. It reads only its four register arguments, touches no
        // memory, and clobbers only caller-saved registers (`eax`, `xmm0`-`xmm14`).
        unsafe { to_barycentric_core(p1, p2, p3, p) }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        to_barycentric_portable(p1, p2, p3, p)
    }
}

// ---------------------------------------------------------------------------
// Portable fallback.
// ---------------------------------------------------------------------------

/// `static lm_vec2 lm_sub2(lm_vec2 a, lm_vec2 b)`
#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn lm_sub2(a: lm_vec2, b: lm_vec2) -> lm_vec2 {
    lm_v2(a.x - b.x, a.y - b.y)
}

/// `static float lm_dot2(lm_vec2 a, lm_vec2 b)`
#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn lm_dot2(a: lm_vec2, b: lm_vec2) -> f32 {
    a.x * b.x + a.y * b.y
}

/// Straight transcription of the C for targets without the assembly core.
#[cfg(not(target_arch = "x86_64"))]
fn to_barycentric_portable(p1: lm_vec2, p2: lm_vec2, p3: lm_vec2, p: lm_vec2) -> lm_vec2 {
    let v0 = lm_sub2(p3, p1);
    let v1 = lm_sub2(p2, p1);
    let v2 = lm_sub2(p, p1);
    let dot00 = lm_dot2(v0, v0);
    let dot01 = lm_dot2(v0, v1);
    let dot02 = lm_dot2(v0, v2);
    let dot11 = lm_dot2(v1, v1);
    let dot12 = lm_dot2(v1, v2);
    let inv_denom = 1.0f32 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    lm_v2(u, v)
}
