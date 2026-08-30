//! x86-64 scalar SSE arithmetic semantics for NaN operands.
//!
//! `addss`/`subss`/`mulss` return the *destination* operand when it is a NaN,
//! and otherwise the source operand when that is a NaN. Which C operand ends up
//! in the destination register is a codegen detail, and it is observable here
//! because `main.c` prints the sign of the result (`nan` vs `-nan`). The
//! functions below take the operands in `(dest, src)` order so the translated
//! expressions can mirror the instruction order gcc emits for `c_src` as built
//! by the supplied CMakeLists (no optimisation flags).
//!
//! When neither operand is a NaN these are plain `f32` operations, so ordinary
//! results are unaffected.

#[inline]
pub fn add(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        dest
    } else if src.is_nan() {
        src
    } else {
        dest + src
    }
}

#[inline]
pub fn sub(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        dest
    } else if src.is_nan() {
        src
    } else {
        dest - src
    }
}

#[inline]
pub fn mul(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        dest
    } else if src.is_nan() {
        src
    } else {
        dest * src
    }
}
