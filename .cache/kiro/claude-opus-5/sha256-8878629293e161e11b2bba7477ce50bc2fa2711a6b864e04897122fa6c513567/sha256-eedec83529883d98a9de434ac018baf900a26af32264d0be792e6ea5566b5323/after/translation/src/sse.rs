//! Scalar single-precision arithmetic with x86 SSE NaN-propagation semantics.
//!
//! The C program is compiled without optimisation (`c_src/CMakeLists.txt` sets
//! no flags), so every arithmetic expression turns into one SSE instruction
//! whose *destination* operand is fixed by the source order. That matters
//! because `MULSS`/`ADDSS`/`SUBSS` pick which NaN to return:
//!
//! ```text
//! if SRC1 (the destination register) is NaN -> QNaN(SRC1)
//! else if SRC2 is NaN                       -> QNaN(SRC2)
//! else                                      -> the arithmetic result
//! ```
//!
//! Plain `a * b` in Rust does not preserve that: LLVM treats `fmul`/`fadd` as
//! commutative and freely swaps the operands, which flips the sign of a
//! propagated NaN. These helpers pin the choice down, so `dest` is always the
//! operand that the corresponding C instruction keeps in its destination
//! register.

/// Force the quiet bit, the way the hardware does when it propagates a NaN.
#[inline]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `MULSS dest, src` — `dest * src`, NaN taken from `dest` first.
#[inline]
pub fn mul(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest * src
    }
}

/// `ADDSS dest, src` — `dest + src`, NaN taken from `dest` first.
#[inline]
pub fn add(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest + src
    }
}

/// `SUBSS dest, src` — `dest - src`, NaN taken from `dest` first. The
/// propagated NaN is returned as-is; it is *not* negated by the subtraction.
#[inline]
pub fn sub(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest - src
    }
}
