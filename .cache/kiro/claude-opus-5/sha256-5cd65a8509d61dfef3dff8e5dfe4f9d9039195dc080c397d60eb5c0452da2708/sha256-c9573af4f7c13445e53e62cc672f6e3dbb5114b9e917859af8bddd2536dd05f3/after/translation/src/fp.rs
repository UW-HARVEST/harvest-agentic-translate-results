//! x86-64 SSE-faithful floating-point primitives.
//!
//! For finite/infinite operands these are plain IEEE-754 operations and the
//! helpers below are indistinguishable from `+` / `*`. They exist purely to pin
//! down **NaN payload propagation**, which IEEE-754 leaves unspecified and
//! which therefore differs between the GCC-compiled C and LLVM-compiled Rust.
//!
//! SSE rules reproduced here:
//!   * For `ADDSD dst, src` / `MULSD dst, src` / `MULSS dst, src`, if the
//!     *destination* operand is a NaN the result is that NaN, quieted; only if
//!     the destination is not a NaN does the *source* NaN propagate.
//!   * "Quieting" sets the most significant significand bit and leaves the sign
//!     and the remaining payload bits alone.
//!
//! Why this matters: `fadd` and `fmul` are commutative, so LLVM is free to pick
//! either operand as the two-address destination, and with two NaN inputs the
//! choice changes the result's payload. The C is built by `c_src/CMakeLists.txt`
//! with **no `CMAKE_BUILD_TYPE`, i.e. `-O0`**, and GCC's `-O0` code generator has
//! a very regular shape: it materialises the operands in the order they appear
//! in the expression tree and issues the arithmetic with the operand it loaded
//! *last* as the destination. Verified per call site against
//! `objdump -d` on the shared object:
//!
//! | C expression | emitted | destination operand |
//! |---|---|---|
//! | `match.c:7`  `sum += v[i]`             | `movsd v[i],%xmm0` / `movsd sum,%xmm1` / `addsd %xmm1,%xmm0` | `v[i]` |
//! | `match.c:17` `sum += v[i+j]`           | `movsd v[i+j],%xmm0` / `movsd sum,%xmm1` / `addsd %xmm1,%xmm0` | `v[i+j]` |
//! | `match.c:37` `threshold * total(ref)`  | `movq total_ref,%xmm1` / `mulsd -0x70(%rbp),%xmm1` | `total(ref)` |
//! | `sc.c:6`     `a[i] * b[i]`             | `movss a[i],%xmm1` / `movss b[i],%xmm0` / `mulss %xmm1,%xmm0` | `b[i]` |
//! | `sc.c:6`     `sum += a[i]*b[i]`        | `cvtss2sd %xmm0,%xmm0` / `movsd sum,%xmm1` / `addsd %xmm1,%xmm0` | the product |
//!
//! Note the accumulator is the *source*, not the destination, in every `+=`.
//!
//! Non-commutative operations (`SUBSD`, `DIVSD`, `SQRTSD`) need no helper: their
//! destination operand is fixed by the source order, and LLVM cannot swap it.

#[inline(always)]
fn quiet_f64(x: f64) -> f64 {
    f64::from_bits(x.to_bits() | 0x0008_0000_0000_0000)
}

#[inline(always)]
fn quiet_f32(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `ADDSD dst, src` -> `dst + src`, with the destination NaN winning.
#[inline(always)]
pub(crate) fn add_sd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet_f64(dst)
    } else if src.is_nan() {
        quiet_f64(src)
    } else {
        dst + src
    }
}

/// `MULSD dst, src` -> `dst * src`, with the destination NaN winning.
#[inline(always)]
pub(crate) fn mul_sd(dst: f64, src: f64) -> f64 {
    if dst.is_nan() {
        quiet_f64(dst)
    } else if src.is_nan() {
        quiet_f64(src)
    } else {
        dst * src
    }
}

/// `MULSS dst, src` -> `dst * src` in single precision, destination NaN winning.
#[inline(always)]
pub(crate) fn mul_ss(dst: f32, src: f32) -> f32 {
    if dst.is_nan() {
        quiet_f32(dst)
    } else if src.is_nan() {
        quiet_f32(src)
    } else {
        dst * src
    }
}

/// `CVTSS2SD` -- widen `float` to `double`.
///
/// Hardware quiets a signalling NaN and shifts the payload left by 29 bits.
/// LLVM's `fpext` lowers to exactly this instruction, but it is also allowed to
/// constant-fold it differently, so the NaN case is spelled out.
#[inline(always)]
pub(crate) fn cvtss2sd(x: f32) -> f64 {
    if x.is_nan() {
        let b = x.to_bits();
        let sign = ((b as u64) & 0x8000_0000) << 32;
        let payload = ((b as u64) & 0x003F_FFFF) << 29;
        f64::from_bits(sign | 0x7FF8_0000_0000_0000 | payload)
    } else {
        x as f64
    }
}

/// `CVTSD2SS` -- narrow `double` to `float` (round-to-nearest-even, overflow to
/// infinity). Hardware quiets a signalling NaN and truncates the payload.
#[inline(always)]
pub(crate) fn cvtsd2ss(x: f64) -> f32 {
    if x.is_nan() {
        let b = x.to_bits();
        let sign = ((b >> 32) as u32) & 0x8000_0000;
        let payload = ((b >> 29) as u32) & 0x003F_FFFF;
        f32::from_bits(sign | 0x7FC0_0000 | payload)
    } else {
        x as f32
    }
}
