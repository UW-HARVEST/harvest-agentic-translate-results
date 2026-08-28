//! Scalar single precision arithmetic that reproduces, bit for bit, what the C
//! program does when compiled for x86-64 (SSE `mulss` / `addss` / `subss`).
//!
//! For finite/infinite operands these are plain IEEE-754 operations, so the
//! ordinary Rust operators are used. The one observable difference between
//! Rust's and C's generated code is *which* NaN is propagated when an operand
//! is NaN: SSE returns the first NaN operand (destination before source), and
//! the C compiler keeps the destination as the left-hand operand of the source
//! expression. Since `printf("%f")` prints the sign of a NaN (`nan` vs `-nan`),
//! that choice is visible in the program's output, so it is modelled
//! explicitly here instead of being left to the optimizer.

/// Quiet a NaN the way the hardware does (set the quiet bit, keep sign/payload).
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `a * b` with SSE NaN propagation (`a` is the destination register).
pub fn mul(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a * b
    }
}

/// `a + b` with SSE NaN propagation (`a` is the destination register).
pub fn add(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a + b
    }
}

/// `a - b` with SSE NaN propagation (`a` is the destination register).
pub fn sub(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a - b
    }
}
